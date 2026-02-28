//! Pipeline orchestrator.
//!
//! Wires together all subsystems into a coherent indexing + query engine.
//! This is the top-level public API of omni-core.
//!
//! ## Architecture
//!
//! The Engine owns all subsystems and coordinates their lifecycle:
//!
//! ```text
//! watcher --> pipeline channel --> process_event() --> parser --> chunker
//!                                                         |
//!                                                         v
//!                                                     embedder --> vector_index
//!                                                         |
//!                                                         v
//!                                                     metadata_index
//! ```
//!
//! Search queries are handled via `SearchEngine` which reads from both indexes.

use std::path::Path;

use tokio::sync::mpsc;

use crate::chunker;
use crate::config::Config;
use crate::embedder::Embedder;
use crate::error::{OmniError, OmniResult};
use crate::graph::DependencyGraph;
use crate::index::MetadataIndex;
use crate::parser;
use crate::search::SearchEngine;
use crate::types::{DependencyEdge, DependencyKind, FileInfo, Language, PipelineEvent, SearchResult, Symbol};
use crate::vector::VectorIndex;
use crate::watcher::FileWatcher;

/// The main OmniContext engine.
///
/// This is the primary entry point for the library. It owns all subsystems
/// and coordinates their lifecycle.
pub struct Engine {
    config: Config,
    /// SQLite metadata index (files, chunks, symbols, FTS5).
    index: MetadataIndex,
    /// Vector index for semantic search.
    vector_index: VectorIndex,
    /// ONNX embedding model for semantic embeddings.
    embedder: Embedder,
    /// Hybrid search engine (RRF fusion).
    search_engine: SearchEngine,
    /// Cross-file dependency graph.
    dep_graph: DependencyGraph,
}

impl Engine {
    /// Create a new engine for the given repository.
    ///
    /// Initializes all subsystems: config, SQLite index, vector index,
    /// embedder, and search engine.
    pub fn new(repo_path: &Path) -> OmniResult<Self> {
        let config = Config::load(repo_path)?;
        Self::with_config(config)
    }

    /// Create an engine with explicit configuration (for testing).
    pub fn with_config(config: Config) -> OmniResult<Self> {
        let data_dir = config.data_dir();

        // Ensure data directory exists
        std::fs::create_dir_all(&data_dir)?;

        // Initialize SQLite index
        let db_path = data_dir.join("index.db");
        let index = MetadataIndex::open(&db_path)?;

        // Initialize vector index (auto-loads from disk if file exists)
        let vector_path = data_dir.join("vectors.bin");
        let vector_index = VectorIndex::open(&vector_path, config.embedding.dimensions)?;

        // Initialize embedder (degrades gracefully if model missing)
        let embedder = Embedder::new(&config.embedding)?;

        // Initialize search engine
        let search_engine = SearchEngine::new(
            config.search.rrf_k,
            config.search.token_budget,
        );

        // Initialize dependency graph
        let dep_graph = DependencyGraph::new();

        tracing::info!(
            repo = %config.repo_path.display(),
            data_dir = %data_dir.display(),
            embedding_available = embedder.is_available(),
            "engine initialized"
        );

        Ok(Self {
            config,
            index,
            vector_index,
            embedder,
            search_engine,
            dep_graph,
        })
    }

    /// Start the indexing pipeline.
    ///
    /// 1. Performs a full directory scan
    /// 2. Processes each discovered file (parse -> chunk -> embed -> store)
    /// 3. Saves the vector index to disk
    pub async fn run_index(&mut self) -> OmniResult<IndexResult> {
        let repo_path = self.config.repo_path.clone();
        let (tx, mut rx) = mpsc::channel::<PipelineEvent>(1024);

        // Create file watcher for scanning
        let watcher = FileWatcher::new(
            &repo_path,
            &self.config.watcher,
            &self.config.indexing,
        );

        // Full directory scan
        let file_count = watcher.full_scan(&tx)?;
        tracing::info!(files = file_count, "scan complete, processing files");

        // Close the sender side so the receiver will drain
        drop(tx);

        let mut result = IndexResult::default();

        // Process each event
        while let Some(event) = rx.recv().await {
            match event {
                PipelineEvent::FileChanged { path } => {
                    match self.process_file(&path) {
                        Ok(stats) => {
                            result.files_processed += 1;
                            result.chunks_created += stats.chunks;
                            result.symbols_extracted += stats.symbols;
                            result.embeddings_generated += stats.embeddings;
                        }
                        Err(e) => {
                            tracing::warn!(
                                path = %path.display(),
                                error = %e,
                                "failed to process file"
                            );
                            result.files_failed += 1;
                        }
                    }
                }
                PipelineEvent::FileDeleted { path } => {
                    if let Err(e) = self.index.delete_file(&path) {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "failed to delete file from index"
                        );
                    }
                }
                PipelineEvent::FullScan => {
                    // Already done above
                }
                PipelineEvent::Shutdown => {
                    break;
                }
            }
        }

        // Persist vector index to disk
        if let Err(e) = self.vector_index.save() {
            tracing::warn!(error = %e, "failed to persist vector index");
        }

        tracing::info!(
            files = result.files_processed,
            chunks = result.chunks_created,
            symbols = result.symbols_extracted,
            embeddings = result.embeddings_generated,
            failed = result.files_failed,
            "indexing complete"
        );

        Ok(result)
    }

    /// Process a single file through the pipeline.
    ///
    /// Parse -> Chunk -> Embed -> Store.
    fn process_file(&mut self, path: &Path) -> OmniResult<FileProcessStats> {
        let mut stats = FileProcessStats::default();

        // Read file content
        let content = std::fs::read_to_string(path).map_err(|e| {
            OmniError::Internal(format!("failed to read {}: {e}", path.display()))
        })?;

        // Detect language
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let language = Language::from_extension(ext);

        if matches!(language, Language::Unknown) {
            return Err(OmniError::Parse {
                path: path.to_path_buf(),
                message: "unsupported language".into(),
            });
        }

        // Compute file hash for change detection
        let hash = compute_file_hash(&content);

        // Check if file has changed since last index
        if let Ok(Some(existing_hash)) = self.index.get_file_hash(path) {
            if existing_hash == hash {
                tracing::debug!(path = %path.display(), "file unchanged, skipping");
                return Ok(stats);
            }
        }

        // Parse the file into structural elements
        let elements = parser::parse_file(path, content.as_bytes(), language)?;

        // Build the FileInfo
        let file_info = FileInfo {
            id: 0, // will be set by upsert
            path: path.to_path_buf(),
            language,
            content_hash: hash.clone(),
            size_bytes: content.len() as u64,
        };

        // Upsert the file first to get a file_id
        let file_id = self.index.upsert_file(&file_info)?;

        // Chunk the elements (returns Vec<Chunk>)
        let chunks = chunker::chunk_elements(&elements, file_id, &self.config);

        // Build Symbol records from the chunks
        let symbols: Vec<Symbol> = chunks
            .iter()
            .filter(|c| !c.symbol_path.is_empty())
            .map(|c| Symbol {
                id: 0,
                name: c
                    .symbol_path
                    .rsplit(|ch: char| ch == '.' || ch == ':')
                    .next()
                    .unwrap_or(&c.symbol_path)
                    .to_string(),
                fqn: c.symbol_path.clone(),
                kind: c.kind,
                file_id,
                line: c.line_start,
                chunk_id: None,
            })
            .collect();

        stats.chunks = chunks.len();
        stats.symbols = symbols.len();

        // Atomic reindex: delete old chunks/symbols, insert new
        let (_fid, chunk_ids) = self.index.reindex_file(&file_info, &chunks, &symbols)?;

        // Generate embeddings and store in vector index
        if self.embedder.is_available() && !chunks.is_empty() {
            let texts: Vec<String> = chunks
                .iter()
                .map(|c| {
                    crate::embedder::format_chunk_for_embedding(
                        language.as_str(),
                        &c.symbol_path,
                        &format!("{:?}", c.kind),
                        &c.content,
                    )
                })
                .collect();
            let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

            let embeddings = self.embedder.embed_batch(&text_refs);
            for (i, maybe_embedding) in embeddings.into_iter().enumerate() {
                if let Some(embedding) = maybe_embedding {
                    if i < chunk_ids.len() {
                        let vector_id = chunk_ids[i] as u64;
                        if let Err(e) = self.vector_index.add(vector_id, &embedding) {
                            tracing::warn!(error = %e, "failed to add vector");
                            continue;
                        }
                        if let Err(e) =
                            self.index.set_chunk_vector_id(chunk_ids[i], vector_id)
                        {
                            tracing::warn!(error = %e, "failed to set vector_id");
                        }
                        stats.embeddings += 1;
                    }
                }
            }
        }

        // ---------------------------------------------------------------
        // Step 5: Build dependency edges from references
        // ---------------------------------------------------------------
        for element in &elements {
            if element.references.is_empty() {
                continue;
            }

            // Find the source symbol for this element
            let source_symbol = if !element.symbol_path.is_empty() {
                self.index.get_symbol_by_fqn(&element.symbol_path)?
            } else {
                None
            };

            let source_id = match source_symbol {
                Some(s) => s.id,
                None => continue,
            };

            // Resolve each reference to a target symbol
            for ref_name in &element.references {
                // Try to find target symbol by FQN match or name prefix
                let target = self.index.get_symbol_by_fqn(ref_name)?
                    .or_else(|| {
                        self.index.search_symbols_by_name(ref_name, 1)
                            .ok()
                            .and_then(|v| v.into_iter().next())
                    });

                if let Some(target_sym) = target {
                    if target_sym.id != source_id {
                        let edge = DependencyEdge {
                            source_id,
                            target_id: target_sym.id,
                            kind: DependencyKind::Calls,
                        };

                        // Store in SQLite
                        if let Err(e) = self.index.insert_dependency(&edge) {
                            tracing::trace!(error = %e, "failed to insert dependency");
                        }

                        // Store in in-memory graph
                        let _ = self.dep_graph.add_edge(&edge);
                    }
                }
            }
        }

        // ---------------------------------------------------------------
        // Step 6: Build dependency edges from import statements
        // ---------------------------------------------------------------
        let imports = parser::parse_imports(path, content.as_bytes(), language)
            .unwrap_or_default();

        if !imports.is_empty() {
            // Use the first symbol of the file (or the file_id-based ID) as source
            let file_source_id = self.index.get_first_symbol_for_file(file_id)
                .unwrap_or(None)
                .map(|s| s.id);

            if let Some(source_id) = file_source_id {
                for import in &imports {
                    // Try to resolve each imported name to a symbol
                    for name in &import.imported_names {
                        if name == "*" {
                            continue;
                        }
                        let target = self.index.search_symbols_by_name(name, 1)
                            .ok()
                            .and_then(|v| v.into_iter().next());

                        if let Some(target_sym) = target {
                            if target_sym.id != source_id {
                                let edge = DependencyEdge {
                                    source_id,
                                    target_id: target_sym.id,
                                    kind: DependencyKind::Imports,
                                };
                                if let Err(e) = self.index.insert_dependency(&edge) {
                                    tracing::trace!(error = %e, "failed to insert import dep");
                                }
                                let _ = self.dep_graph.add_edge(&edge);
                            }
                        }
                    }

                    // Also try resolving the import path itself as a module symbol
                    let target = self.index.get_symbol_by_fqn(&import.import_path)
                        .ok()
                        .flatten()
                        .or_else(|| {
                            self.index.search_symbols_by_name(&import.import_path, 1)
                                .ok()
                                .and_then(|v| v.into_iter().next())
                        });

                    if let Some(target_sym) = target {
                        if target_sym.id != source_id {
                            let edge = DependencyEdge {
                                source_id,
                                target_id: target_sym.id,
                                kind: import.kind,
                            };
                            if let Err(e) = self.index.insert_dependency(&edge) {
                                tracing::trace!(error = %e, "failed to insert import dep");
                            }
                            let _ = self.dep_graph.add_edge(&edge);
                        }
                    }
                }
            }
        }

        tracing::debug!(
            path = %path.display(),
            chunks = stats.chunks,
            symbols = stats.symbols,
            embeddings = stats.embeddings,
            imports = imports.len(),
            "file processed"
        );

        Ok(stats)
    }

    /// Execute a search query.
    pub fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> OmniResult<Vec<SearchResult>> {
        self.search_engine.search(
            query,
            limit,
            &self.index,
            &self.vector_index,
            &self.embedder,
        )
    }

    /// Get engine status information.
    pub fn status(&self) -> OmniResult<EngineStatus> {
        let stats = self.index.statistics()?;
        let dep_edges = self.index.dependency_count().unwrap_or(0);
        Ok(EngineStatus {
            repo_path: self.config.repo_path.display().to_string(),
            data_dir: self.config.data_dir().display().to_string(),
            files_indexed: stats.file_count,
            chunks_indexed: stats.chunk_count,
            symbols_indexed: stats.symbol_count,
            vectors_indexed: self.vector_index.len(),
            dep_edges,
            graph_nodes: self.dep_graph.node_count(),
            graph_edges: self.dep_graph.edge_count(),
            has_cycles: self.dep_graph.has_cycles(),
            search_mode: if self.embedder.is_available() {
                "hybrid".into()
            } else {
                "keyword-only".into()
            },
        })
    }

    /// Get a reference to the metadata index (for advanced queries).
    pub fn metadata_index(&self) -> &MetadataIndex {
        &self.index
    }

    /// Get the repository root path.
    pub fn repo_path(&self) -> &Path {
        &self.config.repo_path
    }

    /// Get a reference to the dependency graph.
    pub fn dep_graph(&self) -> &DependencyGraph {
        &self.dep_graph
    }

    /// Shut down the engine gracefully, persisting data to disk.
    pub fn shutdown(&mut self) -> OmniResult<()> {
        self.vector_index.save()?;
        tracing::info!("engine shut down");
        Ok(())
    }
}

/// Result of an indexing operation.
#[derive(Debug, Clone, Default)]
pub struct IndexResult {
    /// Number of files successfully processed.
    pub files_processed: usize,
    /// Number of files that failed to process.
    pub files_failed: usize,
    /// Total chunks created across all files.
    pub chunks_created: usize,
    /// Total symbols extracted across all files.
    pub symbols_extracted: usize,
    /// Total embeddings generated.
    pub embeddings_generated: usize,
}

/// Status information about the engine.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EngineStatus {
    /// Repository path being indexed.
    pub repo_path: String,
    /// Data directory for index files.
    pub data_dir: String,
    /// Number of files in the index.
    pub files_indexed: usize,
    /// Number of chunks in the index.
    pub chunks_indexed: usize,
    /// Number of symbols in the index.
    pub symbols_indexed: usize,
    /// Number of vectors in the index.
    pub vectors_indexed: usize,
    /// Number of dependency edges in the SQLite store.
    pub dep_edges: usize,
    /// Number of nodes in the in-memory dependency graph.
    pub graph_nodes: usize,
    /// Number of edges in the in-memory dependency graph.
    pub graph_edges: usize,
    /// Whether the dependency graph contains cycles.
    pub has_cycles: bool,
    /// Current search mode (hybrid or keyword-only).
    pub search_mode: String,
}

/// Stats from processing a single file.
#[derive(Debug, Default)]
struct FileProcessStats {
    chunks: usize,
    symbols: usize,
    embeddings: usize,
}

/// Compute a SHA-256 hash of file content for change detection.
fn compute_file_hash(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_file_hash() {
        let hash1 = compute_file_hash("hello world");
        let hash2 = compute_file_hash("hello world");
        let hash3 = compute_file_hash("different content");
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn test_engine_creation() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let engine = Engine::with_config(config);
        assert!(engine.is_ok(), "engine should create successfully");
    }

    #[test]
    fn test_engine_status() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let engine = Engine::with_config(config).expect("create engine");
        let status = engine.status().expect("get status");
        assert_eq!(status.files_indexed, 0);
        assert_eq!(status.chunks_indexed, 0);
        assert_eq!(status.search_mode, "keyword-only");
    }

    #[tokio::test]
    async fn test_index_empty_directory() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let mut engine = Engine::with_config(config).expect("create engine");
        let result = engine.run_index().await.expect("index");
        assert_eq!(result.files_processed, 0);
        assert_eq!(result.chunks_created, 0);
    }

    #[tokio::test]
    async fn test_index_single_file() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();

        // Create a simple Python file
        std::fs::write(
            root.join("hello.py"),
            "def greet(name):\n    \"\"\"Say hello.\"\"\"\n    return f'Hello, {name}!'\n",
        )
        .expect("write");

        let config = Config::defaults(root);
        let mut engine = Engine::with_config(config).expect("create engine");
        let result = engine.run_index().await.expect("index");

        assert_eq!(result.files_processed, 1);
        assert!(result.chunks_created > 0, "should create at least 1 chunk");

        // Verify status reflects the indexed data
        let status = engine.status().expect("status");
        assert_eq!(status.files_indexed, 1);
        assert!(status.chunks_indexed > 0);
    }

    #[test]
    fn test_search_empty_index() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let engine = Engine::with_config(config).expect("create engine");
        let results = engine.search("test query", 10).expect("search");
        assert!(results.is_empty());
    }
}
