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
#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::struct_field_names,
    clippy::too_many_lines
)]

use std::path::Path;

use tokio::sync::mpsc;

use crate::branch_diff::BranchTracker;
use crate::chunker;
use crate::config::Config;
use crate::embedder::Embedder;
use crate::error::{OmniError, OmniResult};
use crate::graph::DependencyGraph;
use crate::index::MetadataIndex;
use crate::parser;
use crate::reranker::Reranker;
use crate::search::SearchEngine;
use crate::types::{
    DependencyEdge, DependencyKind, FileInfo, Language, PipelineEvent, SearchResult, Symbol,
};
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
    reranker: Reranker,
    /// Cross-file dependency graph.
    dep_graph: DependencyGraph,
    /// Per-branch diff tracker for branch-aware context.
    branch_tracker: BranchTracker,
    /// Token counter: uses the embedding tokenizer when available,
    /// falls back to the heuristic estimator.
    token_counter: Box<dyn chunker::token_counter::TokenCounter>,
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

        // Initialize embedder (degrades gracefully if model download fails after retries)
        let embedder = Embedder::new(&config.embedding)?;

        // Initialize vector index -- dimensions always match Jina (768) from config
        let vector_path = data_dir.join("vectors.bin");
        let vector_index = VectorIndex::open(&vector_path, config.embedding.dimensions)?;

        // Initialize search engine
        let search_engine = SearchEngine::new(config.search.rrf_k, config.search.token_budget);

        let reranker = Reranker::new(&config.search.reranker)?;

        // Initialize dependency graph
        let dep_graph = DependencyGraph::new();

        // Resolve the tokenizer from the model directory, if present.
        // tokenizer.json is downloaded alongside the ONNX model file.
        // Falls back to heuristic EstimateTokenCounter (~4 chars/token) when unavailable.
        let tokenizer_path = config
            .embedding
            .model_path
            .parent()
            .map(|p| p.join("tokenizer.json"));
        let token_counter = chunker::token_counter::create_token_counter(tokenizer_path.as_deref());
        tracing::info!(counter = token_counter.name(), "token counter initialized");

        tracing::info!(
            repo = %config.repo_path.display(),
            data_dir = %data_dir.display(),
            embedding_available = embedder.is_available(),
            "engine initialized"
        );

        let branch_tracker = BranchTracker::new(&config.repo_path);

        let mut engine = Self {
            config,
            index,
            vector_index,
            embedder,
            search_engine,
            reranker,
            dep_graph,
            branch_tracker,
            token_counter,
        };

        // Load dependency graph from SQLite index
        if let Err(e) = engine.load_graph_from_index() {
            tracing::warn!(error = %e, "failed to load dependency graph from index");
        }

        Ok(engine)
    }

    /// Load the dependency graph from the SQLite index.
    ///
    /// This populates the in-memory graph with all edges stored in the database.
    /// Should be called after engine initialization to restore graph state.
    fn load_graph_from_index(&mut self) -> OmniResult<usize> {
        let edges = self.index.get_all_dependencies()?;
        let edge_count = edges.len();

        if edge_count == 0 {
            tracing::debug!("no dependency edges found in index");
            return Ok(0);
        }

        tracing::info!(edges = edge_count, "loading dependency graph from index");

        for edge in edges {
            // Add nodes for source and target if they don't exist
            self.dep_graph.add_symbol(edge.source_id)?;
            self.dep_graph.add_symbol(edge.target_id)?;

            // Add the edge
            self.dep_graph.add_edge(&edge)?;
        }

        tracing::info!(
            nodes = self.dep_graph.node_count(),
            edges = self.dep_graph.edge_count(),
            "dependency graph loaded"
        );

        Ok(edge_count)
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
        let watcher = FileWatcher::new(&repo_path, &self.config.watcher, &self.config.indexing);

        // Full directory scan in a background thread to allow backpressure
        // without blocking the async receiver loop or panicking Tokio
        let scan_tx = tx.clone();
        let scan_watcher = watcher.clone();
        let _scan_handle = tokio::task::spawn_blocking(move || {
            let count = scan_watcher.full_scan(&scan_tx).unwrap_or(0);
            count
        });

        // Close our sender side so the receiver will drain when the scanner finishes
        drop(tx);

        let mut result = IndexResult::default();
        let mut pending_embeddings = Vec::with_capacity(512);

        // Process each event
        while let Some(event) = rx.recv().await {
            match event {
                PipelineEvent::FileChanged { path } => {
                    match self.process_file(&path, &mut pending_embeddings) {
                        Ok(stats) => {
                            result.files_processed += 1;
                            result.chunks_created += stats.chunks;
                            result.symbols_extracted += stats.symbols;
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

                    if pending_embeddings.len() >= 1024 {
                        if let Err(e) = self.flush_pending_embeddings(
                            &mut pending_embeddings,
                            &mut result.embeddings_generated,
                        ) {
                            tracing::error!(error = %e, "batch embedding flush failed");
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

        // Flush remaining at the end
        if let Err(e) =
            self.flush_pending_embeddings(&mut pending_embeddings, &mut result.embeddings_generated)
        {
            tracing::error!(error = %e, "final embedding flush failed");
        }

        // Persist vector index to disk
        if let Err(e) = self.vector_index.save() {
            tracing::warn!(error = %e, "failed to persist vector index");
        }

        // Calculate embedding coverage
        let total_chunks = self.index.chunk_count().unwrap_or(0);
        let embedded_chunks = self.index.embedded_chunk_count().unwrap_or(0);
        let coverage_pct = self.index.embedding_coverage().unwrap_or(0.0);

        tracing::info!(
            files = result.files_processed,
            chunks = result.chunks_created,
            symbols = result.symbols_extracted,
            embeddings = result.embeddings_generated,
            failed = result.files_failed,
            "indexing complete"
        );

        tracing::info!(
            total_chunks = total_chunks,
            embedded_chunks = embedded_chunks,
            coverage_pct = format!("{:.1}%", coverage_pct),
            "embedding coverage"
        );

        if coverage_pct < 90.0 {
            tracing::warn!(
                coverage_pct = format!("{:.1}%", coverage_pct),
                "WARNING: Embedding coverage is below 90%. \
                 Semantic search quality will be degraded. \
                 Check that the embedding model loaded correctly. \
                 Run `omnicontext embed --retry-failed` to retry failed embeddings."
            );
        }

        Ok(result)
    }

    /// Process a single file through the pipeline.
    ///
    /// Parse -> Chunk -> Embed -> Store.
    fn process_file(
        &mut self,
        path: &Path,
        pending_embeddings: &mut Vec<(i64, String)>,
    ) -> OmniResult<FileProcessStats> {
        let mut stats = FileProcessStats::default();

        // Read file content
        let content = std::fs::read_to_string(path)
            .map_err(|e| OmniError::Internal(format!("failed to read {}: {e}", path.display())))?;

        // Detect language
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let language = Language::from_extension(ext);

        if matches!(language, Language::Unknown) {
            return Err(OmniError::Parse {
                path: path.to_path_buf(),
                message: "unsupported language".into(),
            });
        }

        let rel_path = path.strip_prefix(&self.config.repo_path).unwrap_or(path);

        // Compute file hash for change detection
        let hash = compute_file_hash(&content);

        // Check if file has changed since last index
        if let Ok(Some(existing_hash)) = self.index.get_file_hash(rel_path) {
            if existing_hash == hash {
                tracing::debug!(path = %rel_path.display(), "file unchanged, skipping");
                return Ok(stats);
            }
        }

        // Parse the file into structural elements using relative path for FQN scoping
        let elements = parser::parse_file(rel_path, content.as_bytes(), language)?;

        // Build the FileInfo utilizing the relative path
        let file_info = FileInfo {
            id: 0, // will be set by upsert
            path: rel_path.to_path_buf(),
            language,
            content_hash: hash.clone(),
            size_bytes: content.len() as u64,
        };

        // Upsert the file first to get a file_id
        let file_id = self.index.upsert_file(&file_info)?;

        // Parse imports early so we can enrich chunks with them
        let imports = parser::parse_imports(path, content.as_bytes(), language).unwrap_or_default();

        // Chunk the elements (returns Vec<Chunk>)
        // Engine now owns a `token_counter` that auto-selects actual vs estimate.
        let mut chunks = chunker::chunk_elements(
            &elements,
            &file_info,
            &imports,
            file_id,
            &self.config,
            &content,
            self.token_counter.as_ref(),
        );

        // Generate RAPTOR-style summary chunks for files with enough leaf chunks
        let summary_chunks =
            chunker::generate_summary_chunks(&chunks, &file_info, self.token_counter.as_ref());
        if !summary_chunks.is_empty() {
            tracing::debug!(
                path = %rel_path.display(),
                summary_count = summary_chunks.len(),
                "generated hierarchical summary chunks"
            );
            chunks.extend(summary_chunks);
        }

        // Build Symbol records from the chunks (skip summary chunks for symbol table)
        let symbols: Vec<Symbol> = chunks
            .iter()
            .filter(|c| !c.symbol_path.is_empty() && !c.is_summary)
            .map(|c| Symbol {
                id: 0,
                name: c
                    .symbol_path
                    .rsplit(['.', ':'])
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

        // Stage for batch embedding
        if self.embedder.is_available() && !chunks.is_empty() {
            for (i, c) in chunks.iter().enumerate() {
                if i < chunk_ids.len() {
                    let text = crate::embedder::format_chunk_for_embedding(
                        language.as_str(),
                        &c.symbol_path,
                        &format!("{:?}", c.kind),
                        &c.content,
                    );
                    pending_embeddings.push((chunk_ids[i], text));
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
            let source_symbol = if element.symbol_path.is_empty() {
                None
            } else {
                self.index.get_symbol_by_fqn(&element.symbol_path)?
            };

            let source_id = match source_symbol {
                Some(s) => s.id,
                None => continue,
            };

            // Resolve each reference to a target symbol
            for ref_name in &element.references {
                // Try to find target symbol by FQN match or name prefix
                let target = self.index.get_symbol_by_fqn(ref_name)?.or_else(|| {
                    self.index
                        .search_symbols_by_name(ref_name, 1)
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
        //         using the multi-strategy import resolution engine
        // ---------------------------------------------------------------
        if !imports.is_empty() {
            let file_source_id = self
                .index
                .get_first_symbol_for_file(file_id)
                .unwrap_or(None)
                .map(|s| s.id);

            if let Some(source_id) = file_source_id {
                for import in &imports {
                    for name in &import.imported_names {
                        if name == "*" {
                            continue;
                        }

                        // Use multi-strategy resolution instead of naive name search
                        let target_id =
                            DependencyGraph::resolve_import(&self.index, &import.import_path, name);

                        if let Some(target) = target_id {
                            if target != source_id {
                                let edge = DependencyEdge {
                                    source_id,
                                    target_id: target,
                                    kind: DependencyKind::Imports,
                                };
                                if let Err(e) = self.index.insert_dependency(&edge) {
                                    tracing::trace!(error = %e, "failed to insert import dep");
                                }
                                let _ = self.dep_graph.add_edge(&edge);
                            }
                        }
                    }

                    // Resolve the module path itself
                    let target_id =
                        DependencyGraph::resolve_import(&self.index, "", &import.import_path);

                    if let Some(target) = target_id {
                        if let Some(source_id) = file_source_id {
                            if target != source_id {
                                let edge = DependencyEdge {
                                    source_id,
                                    target_id: target,
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
        }

        // ---------------------------------------------------------------
        // Step 7: Build call graph edges from element references
        // ---------------------------------------------------------------
        let call_edges = self
            .dep_graph
            .build_call_edges(&self.index, file_id, &elements);
        for edge in &call_edges {
            if let Err(e) = self.index.insert_dependency(edge) {
                tracing::trace!(error = %e, "failed to insert call edge");
            }
        }
        stats.call_edges = call_edges.len();

        // ---------------------------------------------------------------
        // Step 8: Build type hierarchy edges from element structures
        // ---------------------------------------------------------------
        let type_edges = self
            .dep_graph
            .build_type_edges(&self.index, file_id, &elements);
        for edge in &type_edges {
            if let Err(e) = self.index.insert_dependency(edge) {
                tracing::trace!(error = %e, "failed to insert type edge");
            }
        }
        // Let's reuse call_edges stat or add a new one? No, we'll just track it via the graph size later.

        tracing::debug!(
            path = %path.display(),
            chunks = stats.chunks,
            symbols = stats.symbols,
            embeddings = stats.embeddings,
            imports = imports.len(),
            call_edges = stats.call_edges,
            "file processed"
        );

        Ok(stats)
    }

    /// Execute a search query.
    pub fn search(&self, query: &str, limit: usize) -> OmniResult<Vec<SearchResult>> {
        self.search_with_rerank_threshold(query, limit, None)
    }

    /// Execute a search query with an optional reranker minimum threshold.
    ///
    /// When `min_rerank_score` is provided, the cross-encoder reranker uses it
    /// as an early termination threshold -- candidates scoring below this value
    /// are aggressively demoted. Higher values produce fewer, more precise results.
    pub fn search_with_rerank_threshold(
        &self,
        query: &str,
        limit: usize,
        min_rerank_score: Option<f32>,
    ) -> OmniResult<Vec<SearchResult>> {
        let reranker_config = if let Some(threshold) = min_rerank_score {
            let mut cfg = self.config.search.reranker.clone();
            // Use the threshold as a minimum score floor
            // Items below this get demoted via unranked_demotion
            cfg.unranked_demotion = threshold as f64;
            Some(cfg)
        } else {
            Some(self.config.search.reranker.clone())
        };
        self.search_engine.search(
            query,
            limit,
            &self.index,
            &self.vector_index,
            &self.embedder,
            Some(&self.dep_graph),
            Some(&self.reranker),
            reranker_config.as_ref(),
        )
    }

    /// Execute a search and assemble a token-budget-aware context window.
    ///
    /// This is the Phase 3 intelligent context assembly:
    /// - Runs hybrid search
    /// - Groups results by file
    /// - Includes graph-neighbor chunks
    /// - Packs optimally within token budget
    pub fn search_context_window(
        &self,
        query: &str,
        limit: usize,
        token_budget: Option<u32>,
    ) -> OmniResult<crate::types::ContextWindow> {
        self.search_context_window_with_rerank_threshold(query, limit, token_budget, None)
    }

    /// Execute a search and assemble a context window with optional reranker threshold.
    pub fn search_context_window_with_rerank_threshold(
        &self,
        query: &str,
        limit: usize,
        token_budget: Option<u32>,
        min_rerank_score: Option<f32>,
    ) -> OmniResult<crate::types::ContextWindow> {
        let results = self.search_with_rerank_threshold(query, limit, min_rerank_score)?;
        let budget = token_budget.unwrap_or(self.config.search.token_budget);
        let ctx = self.search_engine.assemble_context_window(
            &results,
            &self.index,
            Some(&self.dep_graph),
            budget,
        );
        Ok(ctx)
    }

    /// Get engine status information.
    pub fn status(&self) -> OmniResult<EngineStatus> {
        let stats = self.index.statistics()?;
        let dep_edges = self.index.dependency_count().unwrap_or(0);
        let vectors_indexed = self.vector_index.len();
        let chunks_indexed = stats.chunk_count;

        // Calculate embedding coverage percentage
        let embedding_coverage_percent = if chunks_indexed > 0 {
            (vectors_indexed as f64 / chunks_indexed as f64) * 100.0
        } else {
            0.0
        };

        Ok(EngineStatus {
            repo_path: self.config.repo_path.display().to_string(),
            data_dir: self.config.data_dir().display().to_string(),
            files_indexed: stats.file_count,
            chunks_indexed,
            symbols_indexed: stats.symbol_count,
            vectors_indexed,
            vector_memory_bytes: self.vector_index.memory_usage_bytes(),
            active_search_strategy: self.vector_index.active_strategy().to_string(),
            embedding_coverage_percent,
            dep_edges,
            graph_nodes: self.dep_graph.node_count(),
            graph_edges: self.dep_graph.edge_count(),
            has_cycles: self.dep_graph.has_cycles(),
            language_distribution: self.index.language_distribution().unwrap_or_default(),
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

    /// Retry embedding chunks that failed during initial indexing.
    ///
    /// This is useful when the embedding model was unavailable during indexing
    /// or when embeddings failed for specific chunks.
    pub fn retry_failed_embeddings(&mut self) -> OmniResult<RetryEmbeddingResult> {
        if !self.embedder.is_available() {
            return Err(OmniError::Internal(
                "Embedding model is not available. Cannot retry embeddings.".into(),
            ));
        }

        let failed_chunks = self.index.get_chunks_without_vectors()?;
        let total_failed = failed_chunks.len();

        if total_failed == 0 {
            tracing::info!("No chunks without embeddings found");
            return Ok(RetryEmbeddingResult {
                total_attempted: 0,
                successful: 0,
                failed: 0,
            });
        }

        tracing::info!(
            count = total_failed,
            "found chunks without embeddings, retrying..."
        );

        let mut successful = 0;
        let mut failed = 0;

        // Process in batches for efficiency
        const BATCH_SIZE: usize = 32;
        for batch in failed_chunks.chunks(BATCH_SIZE) {
            // Get file info for each chunk to determine language
            let mut texts = Vec::new();
            let mut chunk_ids = Vec::new();

            for chunk in batch {
                // Look up the parent file using the chunk's file_id
                let lang_str = self
                    .index
                    .get_file_by_id(chunk.file_id)
                    .ok()
                    .flatten()
                    .map(|f| f.language.as_str().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let formatted = crate::embedder::format_chunk_for_embedding(
                    &lang_str,
                    &chunk.symbol_path,
                    &format!("{:?}", chunk.kind),
                    &chunk.content,
                );
                texts.push(formatted);
                chunk_ids.push(chunk.id);
            }

            let text_refs: Vec<&str> = texts.iter().map(String::as_str).collect();
            let embeddings = self.embedder.embed_batch(&text_refs);

            for (i, maybe_embedding) in embeddings.into_iter().enumerate() {
                if let Some(embedding) = maybe_embedding {
                    if i < chunk_ids.len() {
                        if let Ok(vector_id) = u64::try_from(chunk_ids[i]) {
                            if let Err(e) = self.vector_index.add(vector_id, &embedding) {
                                tracing::warn!(
                                    chunk_id = chunk_ids[i],
                                    error = %e,
                                    "failed to add vector"
                                );
                                failed += 1;
                                continue;
                            }
                            if let Err(e) = self.index.set_chunk_vector_id(chunk_ids[i], vector_id)
                            {
                                tracing::warn!(
                                    chunk_id = chunk_ids[i],
                                    error = %e,
                                    "failed to set vector_id"
                                );
                                failed += 1;
                                continue;
                            }
                            successful += 1;
                        }
                    }
                } else {
                    failed += 1;
                }
            }
        }

        // Persist vector index
        if let Err(e) = self.vector_index.save() {
            tracing::warn!(error = %e, "failed to persist vector index");
        }

        tracing::info!(
            total = total_failed,
            successful = successful,
            failed = failed,
            "embedding retry complete"
        );

        Ok(RetryEmbeddingResult {
            total_attempted: total_failed,
            successful,
            failed,
        })
    }

    /// Get the repository root path.
    pub fn repo_path(&self) -> &Path {
        &self.config.repo_path
    }

    /// Get a reference to the dependency graph.
    pub fn dep_graph(&self) -> &DependencyGraph {
        &self.dep_graph
    }

    /// Get a mutable reference to the branch tracker.
    pub fn branch_tracker(&mut self) -> &mut BranchTracker {
        &mut self.branch_tracker
    }

    /// Clear the index (metadata, vectors, and graph).
    /// This removes all indexed data but keeps the database structure intact.
    ///
    /// Note: Currently this is a no-op placeholder. Full implementation would require:
    /// - MetadataIndex::clear() to truncate all tables
    /// - VectorIndex::clear() to remove all vectors
    /// - DependencyGraph::clear() to remove all nodes/edges
    pub fn clear_index(&mut self) -> OmniResult<()> {
        tracing::warn!("clear_index is not fully implemented yet - this is a placeholder");

        // TODO: Implement actual clearing logic:
        // self.index.clear()?;
        // self.vector_index.clear();
        // self.dep_graph.clear();

        Ok(())
    }

    /// Re-index a single file incrementally (Phase 2: real-time indexing).
    ///
    /// Called by the daemon when a `text_edited` IDE event arrives. Unlike
    /// `process_file` which checks the content hash and skips unchanged files,
    /// this method always re-processes the file because the caller knows it
    /// has been edited.
    ///
    /// Returns the file processing stats and a bool indicating whether the
    /// file content actually changed (useful for cache invalidation).
    pub fn reindex_single_file(&mut self, abs_path: &Path) -> OmniResult<(FileProcessStats, bool)> {
        let start = std::time::Instant::now();

        // Check file exists
        if !abs_path.exists() {
            // File was deleted -- remove from index
            let rel_path = abs_path
                .strip_prefix(&self.config.repo_path)
                .unwrap_or(abs_path);
            if let Err(e) = self.index.delete_file(rel_path) {
                tracing::warn!(error = %e, "failed to delete file from index");
            }
            return Ok((FileProcessStats::default(), true));
        }

        // Force reprocess by temporarily ignoring the hash check
        let mut pending = Vec::with_capacity(32);
        let mut stats = self.process_file(abs_path, &mut pending)?;

        // Immediately flush for single file indexing
        let mut embeddings_generated = 0;
        self.flush_pending_embeddings(&mut pending, &mut embeddings_generated)?;
        stats.embeddings = embeddings_generated;

        #[allow(clippy::cast_possible_truncation)]
        let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        let changed = stats.chunks > 0;

        tracing::info!(
            path = %abs_path.display(),
            chunks = stats.chunks,
            symbols = stats.symbols,
            embeddings = stats.embeddings,
            elapsed_ms = elapsed_ms,
            changed = changed,
            "single file reindex complete"
        );

        Ok((stats, changed))
    }

    /// Flush a batch of pending embeddings to the vector index.
    fn flush_pending_embeddings(
        &mut self,
        pending: &mut Vec<(i64, String)>,
        embeddings_count: &mut usize,
    ) -> OmniResult<()> {
        if pending.is_empty() || !self.embedder.is_available() {
            pending.clear();
            return Ok(());
        }

        let texts: Vec<&str> = pending.iter().map(|(_, t)| t.as_str()).collect();

        // Use parallel embedding for batches
        let embeddings = self.embedder.embed_batch_parallel(&texts);

        for (i, maybe_embedding) in embeddings.into_iter().enumerate() {
            if let Some(embedding) = maybe_embedding {
                let chunk_id = pending[i].0;
                if let Ok(vector_id) = u64::try_from(chunk_id) {
                    if let Err(e) = self.vector_index.add(vector_id, &embedding) {
                        tracing::warn!(error = %e, "failed to add vector to HNSW");
                        continue;
                    }
                    if let Err(e) = self.index.set_chunk_vector_id(chunk_id, vector_id) {
                        tracing::warn!(error = %e, "failed to update SQL vector pointer");
                    }
                    *embeddings_count += 1;
                }
            }
        }

        pending.clear();
        Ok(())
    }

    /// Persist vector index to disk.
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

/// Result of retrying failed embeddings.
#[derive(Debug, Clone, Default)]
pub struct RetryEmbeddingResult {
    /// Total number of chunks attempted.
    pub total_attempted: usize,
    /// Number of chunks successfully embedded.
    pub successful: usize,
    /// Number of chunks that failed again.
    pub failed: usize,
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
    /// Estimated heap memory used by vector storage (bytes).
    pub vector_memory_bytes: usize,
    /// Active ANN search strategy (flat, ivf, hnsw).
    pub active_search_strategy: String,
    /// Embedding coverage percentage (vectors / chunks * 100).
    pub embedding_coverage_percent: f64,
    /// Number of dependency edges in the SQLite store.
    pub dep_edges: usize,
    /// Number of nodes in the in-memory dependency graph.
    pub graph_nodes: usize,
    /// Number of edges in the in-memory dependency graph.
    pub graph_edges: usize,
    /// Whether the dependency graph contains cycles.
    pub has_cycles: bool,
    /// Breakdown of files by language.
    pub language_distribution: Vec<(String, usize)>,
    /// Current search mode (hybrid or keyword-only).
    pub search_mode: String,
}

/// Stats from processing a single file.
#[derive(Debug, Default)]
pub struct FileProcessStats {
    /// Number of chunks created.
    pub chunks: usize,
    /// Number of symbols extracted.
    pub symbols: usize,
    /// Number of embeddings generated.
    pub embeddings: usize,
    /// Number of call edges discovered.
    pub call_edges: usize,
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

    fn setup() {
        std::env::set_var("OMNI_SKIP_MODEL_DOWNLOAD", "1");
        std::env::set_var("OMNI_DISABLE_RERANKER", "1");
    }

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
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let engine = Engine::with_config(config);
        assert!(engine.is_ok(), "engine should create successfully");
    }

    #[test]
    fn test_engine_status() {
        setup();
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
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let mut engine = Engine::with_config(config).expect("create engine");
        let result = engine.run_index().await.expect("index");
        assert_eq!(result.files_processed, 0);
        assert_eq!(result.chunks_created, 0);
    }

    #[tokio::test]
    async fn test_index_single_file() {
        setup();
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
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let engine = Engine::with_config(config).expect("create engine");
        let results = engine.search("test query", 10).expect("search");
        assert!(results.is_empty());
    }
}
