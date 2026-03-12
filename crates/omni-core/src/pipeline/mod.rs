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
use crate::commits::CommitEngine;
use crate::config::Config;
use crate::embedder::Embedder;
use crate::error::{OmniError, OmniResult};
use crate::graph::dependencies::FileDependencyGraph;
use crate::graph::historical::HistoricalGraphEnhancer;
use crate::graph::reasoning::ReasoningEngine;
use crate::graph::DependencyGraph;
use crate::index::MetadataIndex;
use crate::parser;
use crate::reranker::Reranker;
use crate::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerError};
use crate::resilience::health_monitor::HealthMonitor;
use crate::search::SearchEngine;
use crate::types::{
    DependencyEdge, DependencyKind, FileInfo, Language, PipelineEvent, SearchResult, Symbol,
};
use crate::vector::VectorIndex;
use crate::watcher::{hash_cache::FileHashCache, FileWatcher};

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
    /// Cross-file dependency graph (symbol-level).
    dep_graph: DependencyGraph,
    /// File-level dependency graph for architectural context.
    file_dep_graph: FileDependencyGraph,
    /// Per-branch diff tracker for branch-aware context.
    branch_tracker: BranchTracker,
    /// Token counter: uses the embedding tokenizer when available,
    /// falls back to the heuristic estimator.
    token_counter: Box<dyn chunker::token_counter::TokenCounter>,
    /// File hash cache for change detection (50-80% reduction in re-indexing).
    hash_cache: FileHashCache,
    /// Health monitor for subsystem health tracking.
    health_monitor: HealthMonitor,
    /// Circuit breaker for embedder operations.
    embedder_breaker: CircuitBreaker,
    /// Circuit breaker for reranker operations.
    reranker_breaker: CircuitBreaker,
    /// Circuit breaker for index operations.
    index_breaker: CircuitBreaker,
    /// Circuit breaker for vector operations.
    vector_breaker: CircuitBreaker,
    /// Commit history engine for git analysis.
    commit_engine: CommitEngine,
    /// Semantic reasoning engine for Graph-Augmented Retrieval (GAR).
    reasoning_engine: ReasoningEngine,
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

        // Initialize dependency graph (symbol-level)
        let dep_graph = DependencyGraph::new();

        // Initialize file dependency graph (file-level)
        let file_dep_graph = FileDependencyGraph::new();

        // Load file hash cache for change detection
        let hash_cache = FileHashCache::load(&data_dir)?;

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

        // Initialize resilience components
        let health_monitor = HealthMonitor::new();
        let embedder_breaker = CircuitBreaker::new(
            "embedder",
            5,                                  // failure threshold
            std::time::Duration::from_secs(60), // timeout
        );
        let reranker_breaker =
            CircuitBreaker::new("reranker", 5, std::time::Duration::from_secs(60));
        let index_breaker = CircuitBreaker::new("index", 5, std::time::Duration::from_secs(60));
        let vector_breaker = CircuitBreaker::new("vector", 5, std::time::Duration::from_secs(60));

        // Report initial health status
        health_monitor.report_health(
            "parser",
            crate::resilience::health_monitor::SubsystemHealth::Healthy,
        );
        health_monitor.report_health(
            "embedder",
            if embedder.is_available() {
                crate::resilience::health_monitor::SubsystemHealth::Healthy
            } else {
                crate::resilience::health_monitor::SubsystemHealth::Degraded
            },
        );
        health_monitor.report_health(
            "index",
            crate::resilience::health_monitor::SubsystemHealth::Healthy,
        );
        health_monitor.report_health(
            "vector",
            crate::resilience::health_monitor::SubsystemHealth::Healthy,
        );

        // Initialize commit engine (max 10,000 commits)
        let commit_engine = CommitEngine::new(10_000);

        // Initialize semantic reasoning engine for GAR
        let reasoning_engine = ReasoningEngine::default();

        let mut engine = Self {
            config,
            index,
            vector_index,
            embedder,
            search_engine,
            reranker,
            dep_graph,
            file_dep_graph,
            branch_tracker,
            token_counter,
            hash_cache,
            health_monitor,
            embedder_breaker,
            reranker_breaker,
            index_breaker,
            vector_breaker,
            commit_engine,
            reasoning_engine,
        };

        // Load dependency graph from SQLite index
        if let Err(e) = engine.load_graph_from_index() {
            tracing::warn!(error = %e, "failed to load dependency graph from index");
        }

        Ok(engine)
    }

    /// Load the dependency graph from the SQLite index.
    ///
    /// Populates the in-memory graph with:
    /// 1. All symbols as nodes (so isolated symbols are still queryable)
    /// 2. All dependency edges
    ///
    /// Called after engine initialization to restore graph state.
    fn load_graph_from_index(&mut self) -> OmniResult<usize> {
        // Step 1: Add every indexd symbol as a graph node.
        // This ensures symbols without edges are still reachable for
        // blast_radius / call_graph queries, and avoids stale-ID confusion.
        if let Ok(symbols) = self.index.get_all_symbols() {
            for sym in symbols {
                let _ = self.dep_graph.add_symbol(sym.id);
            }
        }

        // Step 2: Load dependency edges.
        let edges = self.index.get_all_dependencies()?;
        let edge_count = edges.len();

        if edge_count == 0 {
            tracing::debug!("no dependency edges found in index");
            return Ok(0);
        }

        tracing::info!(edges = edge_count, "loading dependency graph from index");

        for edge in edges {
            // Nodes were already added above; just wire up the edges.
            self.dep_graph.add_symbol(edge.source_id)?;
            self.dep_graph.add_symbol(edge.target_id)?;
            self.dep_graph.add_edge(&edge)?;
        }

        tracing::info!(
            nodes = self.dep_graph.node_count(),
            edges = self.dep_graph.edge_count(),
            "dependency graph loaded"
        );

        // Compute PageRank percentiles and wire into search engine
        if self.dep_graph.node_count() > 0 {
            let pr_scores = self.dep_graph.compute_pagerank_percentiles(0.85, 30);
            let pr_count = pr_scores.len();
            self.search_engine.set_pagerank_scores(pr_scores);
            if pr_count > 0 {
                tracing::info!(
                    symbols = pr_count,
                    "PageRank percentiles computed and loaded"
                );
            }
        }

        // Compute temporal freshness scores from file indexed_at timestamps.
        // Uses exponential decay with a 7-day half-life so recently modified
        // files surface higher in search results.
        if let Ok(timestamps) = self.index.get_file_freshness() {
            if !timestamps.is_empty() {
                let freshness = crate::search::SearchEngine::compute_freshness_from_timestamps(
                    &timestamps, 168.0, // 7 days half-life
                );
                let count = freshness.len();
                self.search_engine.set_freshness_scores(freshness);
                tracing::info!(
                    files = count,
                    "temporal freshness scores computed and loaded"
                );
            }
        }

        // Compute branch-aware file set: resolve git branch-changed paths
        // to file IDs so the search engine can boost files in the current branch.
        match self.branch_tracker.get_branch_changed_files() {
            Ok(changed_paths) if !changed_paths.is_empty() => {
                let mut branch_file_ids = std::collections::HashSet::new();
                for rel_path in &changed_paths {
                    let path = std::path::Path::new(rel_path);
                    if let Ok(Some(file_info)) = self.index.get_file_by_path(path) {
                        branch_file_ids.insert(file_info.id);
                    }
                }
                let count = branch_file_ids.len();
                self.search_engine.set_branch_changed_files(branch_file_ids);
                if count > 0 {
                    tracing::info!(
                        files = count,
                        total_changed = changed_paths.len(),
                        "branch-changed file IDs loaded for retrieval boost"
                    );
                }
            }
            Ok(_) => {
                tracing::debug!("no branch-changed files detected (on default branch)");
            }
            Err(e) => {
                tracing::debug!(error = %e, "branch tracker unavailable, skipping branch boost");
            }
        }

        Ok(edge_count)
    }

    /// Start the indexing pipeline.
    ///
    /// 1. Performs a full directory scan
    /// 2. Processes each discovered file (parse -> chunk -> embed -> store)
    /// 3. Saves the vector index to disk
    pub async fn run_index(&mut self, force: bool) -> OmniResult<IndexResult> {
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

        if force {
            tracing::info!("force reindex requested; clearing existing index state first");
            self.clear_index()?;
        }

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

                    if pending_embeddings.len() >= 80 {
                        if let Err(e) = self.flush_pending_embeddings(
                            &mut pending_embeddings,
                            &mut result.embeddings_generated,
                        ) {
                            tracing::error!(error = %e, "batch embedding flush failed");
                            result.embedding_failures += 1;
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
                    // Remove from hash cache
                    self.hash_cache.remove(&path);
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
            result.embedding_failures += 1;
        }

        // Automatic recovery pass: if chunks remain without vectors, retry once.
        // This avoids returning a "successful" index with silently missing embeddings.
        if self.embedder.is_available() {
            match self.index.get_chunks_without_vectors() {
                Ok(chunks_without_vectors) if !chunks_without_vectors.is_empty() => {
                    let missing = chunks_without_vectors.len();
                    tracing::warn!(
                        missing,
                        "detected chunks without embeddings after primary pass; starting automatic recovery"
                    );

                    match self.retry_failed_embeddings() {
                        Ok(retry) => {
                            result.embeddings_generated += retry.successful;
                            if retry.failed > 0 {
                                result.embedding_failures += retry.failed;
                                tracing::warn!(
                                    attempted = retry.total_attempted,
                                    successful = retry.successful,
                                    failed = retry.failed,
                                    "automatic embedding recovery left missing vectors"
                                );
                            } else {
                                tracing::info!(
                                    attempted = retry.total_attempted,
                                    successful = retry.successful,
                                    "automatic embedding recovery completed"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "automatic embedding recovery failed");
                            result.embedding_failures += 1;
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "failed to inspect chunks without vectors after indexing"
                    );
                }
            }
        }

        // Persist vector index to disk
        if let Err(e) = self.vector_index.save() {
            tracing::warn!(error = %e, "failed to persist vector index");
        }

        // Persist hash cache to disk
        if let Err(e) = self.hash_cache.save() {
            tracing::warn!(error = %e, "failed to persist hash cache");
        }

        // Historical graph enhancement: index commit history and enhance
        // the file dependency graph with co-change edges.
        if let Ok(count) = self.index_commit_history() {
            tracing::info!(commits = count, "indexed commit history");
        }
        // Build a temporary MetadataIndex for the enhancer (needs its own connection)
        if let Ok(enhancer_index) = MetadataIndex::open(&self.config.data_dir().join("index.db")) {
            let mut enhancer = HistoricalGraphEnhancer::new(enhancer_index);
            if let Ok(stats) = enhancer.analyze_history(1000) {
                tracing::info!(
                    commits_analyzed = stats.commits_analyzed,
                    co_change_pairs = stats.co_change_pairs,
                    bug_fixes = stats.bug_fixes_found,
                    "analyzed commit history"
                );
                if let Ok(enhancement) = enhancer.enhance_graph(&mut self.file_dep_graph) {
                    tracing::info!(
                        edges_added = enhancement.edges_added,
                        nodes_boosted = enhancement.nodes_boosted,
                        "enhanced graph with historical data"
                    );

                    // Bridge historical co-change edges into symbol-level DependencyGraph.
                    // For each co-change file pair, find representative symbols and create
                    // HistoricalCoChange edges so GAR can traverse them.
                    let co_change_symbol_edges = self.bridge_historical_to_symbol_graph(&enhancer);
                    if co_change_symbol_edges > 0 {
                        tracing::info!(
                            edges = co_change_symbol_edges,
                            "bridged historical co-change edges into symbol graph"
                        );
                    }
                }

                // Wire bug-prone file boost factors into the search engine.
                // Resolve file paths to file_ids and compute boost factors
                // (1.0 + bug_count/10, capped at 2.0).
                let bug_prone = enhancer.find_bug_prone_files(2); // threshold: 2+ bug fixes
                if !bug_prone.is_empty() {
                    let mut boosts = std::collections::HashMap::new();
                    for (file_path, bug_count) in &bug_prone {
                        let conn = self.index.connection();
                        if let Ok(file_id) = conn.query_row(
                            "SELECT id FROM files WHERE path = ?1",
                            rusqlite::params![file_path.to_string_lossy().as_ref()],
                            |row| row.get::<_, i64>(0),
                        ) {
                            #[allow(clippy::cast_precision_loss)]
                            let boost_factor = (1.0 + (*bug_count as f32 / 10.0)).min(2.0);
                            boosts.insert(file_id, boost_factor);
                        }
                    }
                    if !boosts.is_empty() {
                        tracing::info!(
                            files = boosts.len(),
                            "wired bug-prone file boosts into search engine"
                        );
                        self.search_engine.set_bug_prone_boosts(boosts);
                    }
                }
            }
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
            embedding_failures = result.embedding_failures,
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
        tracing::info!("Starting to process file: {}", path.display());

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

        // Check if file has changed using hash cache (50-80% reduction in re-indexing)
        if !self.hash_cache.has_changed(path)? {
            tracing::debug!(path = %rel_path.display(), "file unchanged, skipping");
            return Ok(stats);
        }

        // Parse the file into structural elements using relative path for FQN scoping
        let elements = parser::parse_file(rel_path, content.as_bytes(), language)?;

        // Compute file hash for FileInfo (still needed for metadata)
        let hash = compute_file_hash(&content);

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

        // ---------------------------------------------------------------
        // Incremental graph update: remove stale edges from the in-memory
        // dependency graph BEFORE the SQLite reindex deletes old symbol rows.
        // We read old symbol IDs first, strip their edges from petgraph, then
        // proceed with the atomic SQLite reindex which inserts fresh symbols.
        // ---------------------------------------------------------------
        {
            let old_symbols = self
                .index
                .get_all_symbols_for_file(file_id)
                .unwrap_or_default();
            if !old_symbols.is_empty() {
                let old_ids: Vec<i64> = old_symbols.iter().map(|s| s.id).collect();
                let removed = self.dep_graph.remove_edges_for_symbols(&old_ids);
                if removed > 0 {
                    tracing::debug!(
                        file_id = file_id,
                        symbols = old_ids.len(),
                        edges_removed = removed,
                        "stripped stale edges from in-memory graph before reindex"
                    );
                }
            }
        }

        // Atomic reindex: delete old chunks/symbols, insert new
        let (_fid, chunk_ids) = self
            .index_breaker
            .call_sync(|| self.index.reindex_file(&file_info, &chunks, &symbols))
            .map_err(|e| match e {
                CircuitBreakerError::Open => OmniError::Internal(
                    "index circuit breaker is open — too many recent failures".into(),
                ),
                CircuitBreakerError::OperationFailed(inner) => inner,
            })?;

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

        // ---------------------------------------------------------------
        // Step 9: Extract cross-file data flow edges
        // ---------------------------------------------------------------
        let flow_extractor = crate::graph::data_flow::DataFlowExtractor::new();
        match flow_extractor.extract_flows_for_file(&self.index, file_id, &self.dep_graph) {
            Ok(flows) if !flows.is_empty() => {
                let flow_edges =
                    crate::graph::data_flow::DataFlowExtractor::to_dependency_edges(&flows);
                let mut flow_count = 0;
                for edge in &flow_edges {
                    if let Err(e) = self.index.insert_dependency(edge) {
                        tracing::trace!(error = %e, "failed to insert data flow edge");
                    }
                    let _ = self.dep_graph.add_edge(edge);
                    flow_count += 1;
                }
                if flow_count > 0 {
                    tracing::debug!(
                        path = %path.display(),
                        flow_edges = flow_count,
                        "data flow edges extracted"
                    );
                }
            }
            Ok(_) => {} // No flows found
            Err(e) => {
                tracing::trace!(error = %e, "data flow extraction failed");
            }
        }

        tracing::debug!(
            path = %path.display(),
            chunks = stats.chunks,
            symbols = stats.symbols,
            embeddings = stats.embeddings,
            imports = imports.len(),
            call_edges = stats.call_edges,
            "file processed"
        );

        // Update hash cache after successful indexing
        let hash = FileHashCache::compute_hash(path)?;
        self.hash_cache.update_hash(path.to_path_buf(), hash);

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
        self.index_breaker
            .call_sync(|| {
                self.search_engine.search(
                    query,
                    limit,
                    &self.index,
                    &self.vector_index,
                    &self.embedder,
                    Some(&self.dep_graph),
                    Some(&self.reasoning_engine),
                    Some(&self.reranker),
                    reranker_config.as_ref(),
                    &[], // no open files in pipeline search
                )
            })
            .map_err(|e| match e {
                CircuitBreakerError::Open => OmniError::Internal(
                    "search circuit breaker is open — too many recent failures".into(),
                ),
                CircuitBreakerError::OperationFailed(inner) => inner,
            })
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
        let reranker_config = if let Some(threshold) = min_rerank_score {
            let mut cfg = self.config.search.reranker.clone();
            cfg.unranked_demotion = threshold as f64;
            Some(cfg)
        } else {
            Some(self.config.search.reranker.clone())
        };

        // Use search_with_gar to get both results AND GAR neighbor map in a single pass.
        // This eliminates the dual graph walk that previously happened in both
        // search() and assemble_context_window().
        let (results, gar_neighbors) = self
            .index_breaker
            .call_sync(|| {
                self.search_engine.search_with_gar(
                    query,
                    limit,
                    &self.index,
                    &self.vector_index,
                    &self.embedder,
                    Some(&self.dep_graph),
                    Some(&self.reasoning_engine),
                    Some(&self.reranker),
                    reranker_config.as_ref(),
                    &[], // open_files passed via dedicated API when available
                )
            })
            .map_err(|e| match e {
                CircuitBreakerError::Open => OmniError::Internal(
                    "search circuit breaker is open — too many recent failures".into(),
                ),
                CircuitBreakerError::OperationFailed(inner) => inner,
            })?;

        let budget = token_budget.unwrap_or(self.config.search.token_budget);
        let mut ctx = self.search_engine.assemble_context_window(
            &results,
            &self.index,
            Some(&self.dep_graph),
            &gar_neighbors,
            budget,
        );
        // Enrich with shadow headers when enabled
        if self.config.search.shadow_headers {
            self.enrich_shadow_headers(&mut ctx);
        }
        Ok(ctx)
    }

    /// Enrich a context window with architectural shadow headers.
    ///
    /// Each entry gets a header like:
    /// ```text
    /// // [OmniContext] File: path | Language: lang | Kind: function
    /// // [OmniContext] Dependents: 5 | Dependencies: 3 | Risk: MEDIUM
    /// // [OmniContext] Co-changes-with: config.rs, types.rs
    /// ```
    pub fn enrich_shadow_headers(&self, ctx: &mut crate::types::ContextWindow) {
        for entry in &mut ctx.entries {
            entry.shadow_header = Some(self.compute_shadow_header(entry));
        }
    }

    /// Compute a shadow header for a single context entry.
    fn compute_shadow_header(&self, entry: &crate::types::ContextEntry) -> String {
        let file_path = entry.file_path.display().to_string();
        let lang = entry
            .file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(crate::types::Language::from_extension)
            .unwrap_or(crate::types::Language::Unknown);
        let kind = entry.chunk.kind.as_str();

        // Count downstream (dependents) and upstream (dependencies) for the first symbol
        let (downstream_count, upstream_count) = if !entry.chunk.symbol_path.is_empty() {
            if let Ok(Some(sym)) = self.index.get_symbol_by_fqn(&entry.chunk.symbol_path) {
                let down = self
                    .dep_graph
                    .downstream(sym.id, 1)
                    .map(|v| v.len())
                    .unwrap_or(0);
                let up = self
                    .dep_graph
                    .upstream(sym.id, 1)
                    .map(|v| v.len())
                    .unwrap_or(0);
                (down, up)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        // Risk level based on downstream count
        let risk = if downstream_count > 20 {
            "HIGH"
        } else if downstream_count > 5 {
            "MEDIUM"
        } else {
            "LOW"
        };

        // Co-change partners (top 3)
        let co_changes =
            crate::commits::CommitEngine::co_change_files(&self.index, &file_path, 2, 3)
                .unwrap_or_default();

        let co_change_str = if co_changes.is_empty() {
            String::new()
        } else {
            let names: Vec<&str> = co_changes.iter().map(|c| c.path.as_str()).collect();
            format!("\n// [OmniContext] Co-changes-with: {}", names.join(", "))
        };

        format!(
            "// [OmniContext] File: {} | Language: {} | Kind: {}\n\
             // [OmniContext] Dependents: {} | Dependencies: {} | Risk: {}{}",
            file_path,
            lang.as_str(),
            kind,
            downstream_count,
            upstream_count,
            risk,
            co_change_str,
        )
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
            hash_cache_entries: self.hash_cache.len(),
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

        // Process in batches for efficiency.
        // Keep batch size moderate to limit ONNX arena accumulation per outer loop.
        const BATCH_SIZE: usize = 20;
        let mut outer_batch_idx: usize = 0;
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
            let embeddings = match self.embedder_breaker.call_sync(|| {
                let result = self.embedder.embed_batch(&text_refs);
                let success_count = result.iter().filter(|r| r.is_some()).count();
                if success_count == 0 && !text_refs.is_empty() {
                    Err(OmniError::Internal(
                        "all retry embeddings in batch failed".into(),
                    ))
                } else {
                    Ok(result)
                }
            }) {
                Ok(embs) => embs,
                Err(CircuitBreakerError::Open) => {
                    tracing::warn!("embedder circuit breaker open during retry — skipping batch");
                    failed += texts.len();
                    continue;
                }
                Err(CircuitBreakerError::OperationFailed(e)) => {
                    tracing::warn!(error = %e, "batch embedding failed during retry");
                    failed += texts.len();
                    continue;
                }
            };

            for (i, maybe_embedding) in embeddings.into_iter().enumerate() {
                if let Some(embedding) = maybe_embedding {
                    if i < chunk_ids.len() {
                        if let Ok(vector_id) = u64::try_from(chunk_ids[i]) {
                            let add_result = self
                                .vector_breaker
                                .call_sync(|| self.vector_index.add(vector_id, &embedding));
                            match add_result {
                                Err(CircuitBreakerError::Open) => {
                                    let remaining = chunk_ids.len().saturating_sub(i);
                                    return Err(OmniError::Internal(format!(
                                        "vector circuit breaker open during retry; {remaining} embeddings left unprocessed"
                                    )));
                                }
                                Err(CircuitBreakerError::OperationFailed(e)) => {
                                    tracing::warn!(
                                        chunk_id = chunk_ids[i],
                                        error = %e,
                                        "failed to add vector"
                                    );
                                    failed += 1;
                                    continue;
                                }
                                Ok(()) => {}
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

            // Free ONNX arena memory after each outer batch to prevent accumulation
            self.embedder.reset_session();
            outer_batch_idx += 1;

            // Persist vectors every 5 outer batches so progress survives crashes
            if outer_batch_idx % 5 == 0 {
                if let Err(e) = self.vector_index.save() {
                    tracing::warn!(error = %e, "periodic vector save during retry failed");
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

    /// Get a reference to the dependency graph (symbol-level).
    pub fn dep_graph(&self) -> &DependencyGraph {
        &self.dep_graph
    }

    /// Get a reference to the file dependency graph (file-level).
    pub fn file_dep_graph(&self) -> &FileDependencyGraph {
        &self.file_dep_graph
    }

    /// Get a reference to the reranker.
    pub fn reranker(&self) -> &Reranker {
        &self.reranker
    }

    /// Get a reference to the health monitor.
    pub fn health_monitor(&self) -> &HealthMonitor {
        &self.health_monitor
    }

    /// Get a reference to the embedder.
    pub fn embedder(&self) -> &Embedder {
        &self.embedder
    }

    /// Get a reference to the embedder circuit breaker.
    pub fn embedder_breaker(&self) -> &CircuitBreaker {
        &self.embedder_breaker
    }

    /// Get a reference to the reranker circuit breaker.
    pub fn reranker_breaker(&self) -> &CircuitBreaker {
        &self.reranker_breaker
    }

    /// Get a reference to the index circuit breaker.
    pub fn index_breaker(&self) -> &CircuitBreaker {
        &self.index_breaker
    }

    /// Get a reference to the vector circuit breaker.
    pub fn vector_breaker(&self) -> &CircuitBreaker {
        &self.vector_breaker
    }

    /// Get a reference to the commit engine.
    pub fn commit_engine(&self) -> &CommitEngine {
        &self.commit_engine
    }

    /// Get a reference to the config.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get a reference to the search engine (for cache stats, invalidation, etc.).
    pub fn search_engine(&self) -> &SearchEngine {
        &self.search_engine
    }

    /// Get a reference to the semantic reasoning engine.
    pub fn reasoning_engine(&self) -> &ReasoningEngine {
        &self.reasoning_engine
    }

    /// Bridge historical co-change file pairs into the symbol-level dependency graph.
    ///
    /// For each frequently co-changed file pair `(file_a, file_b)`, resolves a
    /// representative symbol from each file and creates a `HistoricalCoChange`
    /// edge in the symbol-level `DependencyGraph`. This allows the GAR reasoning
    /// engine to traverse historical co-change relationships during BFS walks.
    #[allow(clippy::similar_names)]
    fn bridge_historical_to_symbol_graph(&mut self, enhancer: &HistoricalGraphEnhancer) -> usize {
        let mut edges_added = 0;
        let co_change_threshold = 3; // minimum commits together

        for (file_a, file_b, _freq) in enhancer.co_change_pairs_above(co_change_threshold) {
            // Resolve file paths to file_ids
            let file_a_str = file_a.to_string_lossy();
            let file_b_str = file_b.to_string_lossy();

            let file_a_id = self.index.connection().query_row(
                "SELECT id FROM files WHERE path = ?1",
                rusqlite::params![file_a_str.as_ref()],
                |row| row.get::<_, i64>(0),
            );
            let file_b_id = self.index.connection().query_row(
                "SELECT id FROM files WHERE path = ?1",
                rusqlite::params![file_b_str.as_ref()],
                |row| row.get::<_, i64>(0),
            );

            if let (Ok(fid_a), Ok(fid_b)) = (file_a_id, file_b_id) {
                // Get a representative symbol from each file (first module-level symbol)
                let sym_a = self.index.connection().query_row(
                    "SELECT id FROM symbols WHERE file_id = ?1 ORDER BY line LIMIT 1",
                    rusqlite::params![fid_a],
                    |row| row.get::<_, i64>(0),
                );
                let sym_b = self.index.connection().query_row(
                    "SELECT id FROM symbols WHERE file_id = ?1 ORDER BY line LIMIT 1",
                    rusqlite::params![fid_b],
                    |row| row.get::<_, i64>(0),
                );

                if let (Ok(sid_a), Ok(sid_b)) = (sym_a, sym_b) {
                    if sid_a != sid_b {
                        let edge = DependencyEdge {
                            source_id: sid_a,
                            target_id: sid_b,
                            kind: DependencyKind::HistoricalCoChange,
                        };
                        if self.dep_graph.add_edge(&edge).is_ok() {
                            edges_added += 1;
                        }
                    }
                }
            }
        }

        edges_added
    }

    /// Generate a CLAUDE.md summary of the indexed repository.
    ///
    /// This produces a Markdown document suitable for LLM context priming,
    /// containing project structure, key modules, and architectural notes.
    pub fn generate_claude_md(&self) -> OmniResult<String> {
        let status = self.status()?;
        let mut out = String::with_capacity(4096);

        let languages: Vec<String> = status
            .language_distribution
            .iter()
            .map(|(lang, count)| format!("{lang} ({count})"))
            .collect();

        out.push_str("# Project Context (CLAUDE.md)\n\n");
        out.push_str(&format!("**Repository**: `{}`\n", status.repo_path));
        out.push_str(&format!("**Files indexed**: {}\n", status.files_indexed));
        out.push_str(&format!("**Symbols**: {}\n", status.symbols_indexed));
        out.push_str(&format!("**Chunks**: {}\n", status.chunks_indexed));
        out.push_str(&format!("**Languages**: {}\n\n", languages.join(", ")));

        // Graph summary
        out.push_str("## Dependency Graph\n\n");
        out.push_str(&format!("- **Nodes**: {}\n", status.graph_nodes));
        out.push_str(&format!("- **Edges**: {}\n", status.graph_edges));
        out.push_str(&format!("- **Has cycles**: {}\n\n", status.has_cycles));

        // Health status
        out.push_str("## Subsystem Health\n\n");
        out.push_str(&format!(
            "- Embedder: `{:?}`\n",
            self.embedder_breaker.state()
        ));
        out.push_str(&format!(
            "- Reranker: `{:?}`\n",
            self.reranker_breaker.state()
        ));
        out.push_str(&format!("- Index: `{:?}`\n", self.index_breaker.state()));
        out.push_str(&format!("- Vector: `{:?}`\n", self.vector_breaker.state()));

        Ok(out)
    }

    /// Generate a JSON context map of the repository structure.
    ///
    /// Returns a JSON string with file counts, symbol graph summary,
    /// and key metrics useful for agent orchestration.
    pub fn generate_context_map(&self) -> OmniResult<String> {
        let status = self.status()?;
        let cache_stats = self.search_engine.result_cache_stats();

        let map = serde_json::json!({
            "repository": status.repo_path,
            "files": status.files_indexed,
            "symbols": status.symbols_indexed,
            "chunks": status.chunks_indexed,
            "languages": status.language_distribution,
            "search_mode": status.search_mode,
            "embedding_coverage_percent": status.embedding_coverage_percent,
            "cache": {
                "l1_size": cache_stats.l1.size,
                "l2_size": cache_stats.l2.size,
                "hit_rate": cache_stats.overall_hit_rate(),
            },
            "graph": {
                "nodes": status.graph_nodes,
                "edges": status.graph_edges,
                "has_cycles": status.has_cycles,
            },
            "health": {
                "embedder": format!("{:?}", self.embedder_breaker.state()),
                "reranker": format!("{:?}", self.reranker_breaker.state()),
                "index": format!("{:?}", self.index_breaker.state()),
                "vector": format!("{:?}", self.vector_breaker.state()),
            },
        });

        serde_json::to_string_pretty(&map)
            .map_err(|e| OmniError::Internal(format!("JSON serialization failed: {e}")))
    }

    /// Index git commit history for the repository.
    ///
    /// This analyzes git history to enable:
    /// - Commit context for files
    /// - Co-change detection
    /// - Bug-prone file identification
    /// - Author statistics
    pub fn index_commit_history(&self) -> OmniResult<usize> {
        self.commit_engine
            .index_history(&self.config.repo_path, &self.index)
    }

    /// Get a mutable reference to the branch tracker.
    pub fn branch_tracker(&mut self) -> &mut BranchTracker {
        &mut self.branch_tracker
    }

    /// Clear the index (metadata, vectors, and graph).
    /// This removes all indexed data but keeps the database structure intact.
    pub fn clear_index(&mut self) -> OmniResult<()> {
        // 1) Clear SQL metadata and FTS contents.
        self.index.clear_all()?;

        // 2) Clear in-memory graphs.
        self.dep_graph.clear();
        self.file_dep_graph.clear();

        // 3) Recreate vector index from scratch by replacing on-disk file.
        let vector_path = self.config.data_dir().join("vectors.bin");
        if vector_path.exists() {
            std::fs::remove_file(&vector_path)?;
        }
        self.vector_index = VectorIndex::open(&vector_path, self.config.embedding.dimensions)?;

        // 4) Clear hash cache so next index pass fully reprocesses files.
        self.hash_cache.clear();
        self.hash_cache.save()?;

        tracing::info!("index cleared successfully");

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

            // Before deleting from SQLite, strip edges and nodes from in-memory graph
            if let Ok(Some(fi)) = self.index.get_file_by_path(rel_path) {
                let fid = fi.id;
                let old_symbols = self.index.get_all_symbols_for_file(fid).unwrap_or_default();
                if !old_symbols.is_empty() {
                    let old_ids: Vec<i64> = old_symbols.iter().map(|s| s.id).collect();
                    let edges_removed = self.dep_graph.remove_edges_for_symbols(&old_ids);
                    let nodes_removed = self.dep_graph.remove_symbols(&old_ids);
                    tracing::debug!(
                        path = %rel_path.display(),
                        edges_removed,
                        nodes_removed,
                        "purged deleted file's symbols from in-memory graph"
                    );
                }
            }

            if let Err(e) = self.index.delete_file(rel_path) {
                tracing::warn!(error = %e, "failed to delete file from index");
            }
            // Remove from hash cache
            self.hash_cache.remove(abs_path);
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

        // Invalidate tiered result cache for this file so stale results aren't served.
        if changed {
            let invalidated = self.search_engine.result_cache().invalidate_file(abs_path);
            if invalidated > 0 {
                tracing::debug!(
                    file = %abs_path.display(),
                    invalidated = invalidated,
                    "tiered result cache entries invalidated after reindex"
                );
            }
        }

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

        // Use parallel embedding for batches — guarded by the embedder circuit breaker.
        // When the breaker is open, we skip embedding and fall back to keyword-only search.
        let embeddings = match self.embedder_breaker.call_sync(|| {
            let result = self.embedder.embed_batch_parallel(&texts);
            // embed_batch_parallel returns Vec<Option<Vec<f32>>> — treat all-None as failure
            let success_count = result.iter().filter(|r| r.is_some()).count();
            if success_count == 0 && !texts.is_empty() {
                Err(OmniError::Internal("all embeddings in batch failed".into()))
            } else {
                Ok(result)
            }
        }) {
            Ok(embs) => {
                self.health_monitor.report_health(
                    "embedder",
                    crate::resilience::health_monitor::SubsystemHealth::Healthy,
                );
                embs
            }
            Err(CircuitBreakerError::Open) => {
                tracing::warn!(
                    batch_size = texts.len(),
                    "embedder circuit breaker open — skipping batch embedding (keyword-only mode)"
                );
                self.health_monitor.report_health_with_message(
                    "embedder",
                    crate::resilience::health_monitor::SubsystemHealth::Degraded,
                    "circuit breaker open — embeddings skipped",
                );
                pending.clear();
                return Ok(());
            }
            Err(CircuitBreakerError::OperationFailed(e)) => {
                tracing::error!(error = %e, "batch embedding failed — circuit breaker recorded failure");
                self.health_monitor.report_health_with_message(
                    "embedder",
                    crate::resilience::health_monitor::SubsystemHealth::Degraded,
                    format!("batch embedding failure: {e}"),
                );
                pending.clear();
                return Err(e);
            }
        };
        let total_embeddings = embeddings.len();

        for (i, maybe_embedding) in embeddings.into_iter().enumerate() {
            if let Some(embedding) = maybe_embedding {
                let chunk_id = pending[i].0;
                if let Ok(vector_id) = u64::try_from(chunk_id) {
                    let add_result = self
                        .vector_breaker
                        .call_sync(|| self.vector_index.add(vector_id, &embedding));
                    match add_result {
                        Err(CircuitBreakerError::Open) => {
                            let remaining = total_embeddings.saturating_sub(i);
                            return Err(OmniError::Internal(format!(
                                "vector circuit breaker open during flush; {remaining} embeddings left unprocessed"
                            )));
                        }
                        Err(CircuitBreakerError::OperationFailed(e)) => {
                            tracing::warn!(error = %e, "failed to add vector to HNSW");
                            continue;
                        }
                        Ok(()) => {}
                    }
                    if let Err(e) = self.index.set_chunk_vector_id(chunk_id, vector_id) {
                        tracing::warn!(error = %e, "failed to update SQL vector pointer");
                        continue;
                    }
                    *embeddings_count += 1;
                }
            }
        }

        pending.clear();

        // Recreate ONNX session to free accumulated arena memory
        self.embedder.reset_session();

        // Persist vectors to disk after each flush so progress survives crashes
        self.vector_index.save()?;

        Ok(())
    }

    /// Persist vector index to disk.
    pub fn shutdown(&mut self) -> OmniResult<()> {
        self.vector_index.save()?;

        // Prune missing files from hash cache before saving
        let pruned = self.hash_cache.prune_missing_files();
        if pruned > 0 {
            tracing::info!(pruned, "pruned missing files from hash cache");
        }

        // Save hash cache
        self.hash_cache.save()?;

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
    /// Number of embedding flush failures encountered.
    pub embedding_failures: usize,
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
    /// Number of files in the hash cache.
    pub hash_cache_entries: usize,
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
        let result = engine.run_index(false).await.expect("index");
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
        let result = engine.run_index(false).await.expect("index");

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
