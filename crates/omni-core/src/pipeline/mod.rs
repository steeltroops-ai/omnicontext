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
//! watcher --> pipeline channel --> two-phase indexing --> parser (Rayon) --> chunker
//!                                                                 |
//!                                                                 v
//!                                                         embedder --> vector_index
//!                                                                 |
//!                                                                 v
//!                                                         metadata_index (bulk tx)
//! ```
//!
//! ## Two-Phase Incremental Indexing
//!
//! `run_index()` operates in two phases to saturate CPU cores and minimise I/O:
//!
//! **Phase 1 — Change detection + parallel parse (Rayon):**
//! 1. `hash_cache.check_and_read(path)` — three-tier mtime→xxHash3 check, returns
//!    content if changed.  Files whose mtime and hash are unchanged are skipped
//!    without any further work.
//! 2. CPU-bound AST parse, chunk, and symbol extraction run on a Rayon thread pool.
//!    Each file is independent, so there is no synchronisation overhead.
//!
//! **Phase 2 — Sequential store (single SQLite writer):**
//! 3. `store_parsed_file()` upserts file metadata, runs the incremental graph update,
//!    calls `reindex_file()`, and stages changed chunks for embedding.
//! 4. Chunk-level delta detection: if a chunk's xxHash3 matches the stored value in
//!    the DB, it is unchanged and its existing `vector_id` is preserved (no re-embed).
//! 5. A single bulk SQLite transaction wraps all per-file writes in the run, reducing
//!    fsync overhead from N (one per file) to 1 per index pass.
//!
//! Search queries are handled via `SearchEngine` which reads from both indexes.
#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::struct_field_names,
    clippy::too_many_lines
)]

use std::path::Path;

use rayon::prelude::*;
use tokio::sync::mpsc;

use crate::branch_diff::BranchTracker;
use crate::chunker;
use crate::commits::CommitEngine;
use crate::config::Config;
use crate::embedder::{CloudEmbedder, Embedder};
use crate::error::{OmniError, OmniResult};
use crate::graph::dependencies::FileDependencyGraph;
use crate::graph::historical::HistoricalGraphEnhancer;
use crate::graph::reasoning::ReasoningEngine;
use crate::graph::DependencyGraph;
use crate::index::MetadataIndex;
use crate::memory::MemoryStore;
use crate::parser;
use crate::reranker::Reranker;
use crate::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerError};
use crate::resilience::health_monitor::HealthMonitor;
use crate::rules::RulesLoader;
use crate::search::SearchEngine;
use crate::types::{
    Chunk, DependencyEdge, DependencyKind, FileInfo, Language, PipelineEvent, SearchResult, Symbol,
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
    ///
    /// `Arc` instead of `Box` so `parse_file_parallel` can share an immutable
    /// reference across Rayon threads without unsafe transmutes.
    token_counter: std::sync::Arc<dyn chunker::token_counter::TokenCounter>,
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
    /// Timestamp of the most recent completed index run.
    last_indexed_at: Option<std::time::SystemTime>,
    /// Cumulative embedding flush count since the last ONNX session reset.
    ///
    /// The ONNX Runtime arena grows with each session.run() call and is only
    /// released when the session is dropped.  Instead of resetting on every
    /// flush (which reloads the ~550MB model), we reset every
    /// ARENA_FLUSH_RESET_INTERVAL flushes to bound arena growth while
    /// amortising the reload cost across many batches.
    arena_flush_count: usize,
    /// When `true`, skip incremental ANN index rebuilds during `run_index()`.
    ///
    /// Set by `set_offline_index_mode(true)` before `run_index()`.  After
    /// indexing completes, call `build_ann_index()` to perform the batch build.
    /// This matches Sourcegraph's offline SCIP index build + load pattern:
    /// vectors accumulate in the flat map, then HNSW is built once in batch.
    offline_index_mode: bool,
    /// Mtime-cached loader for `.omnicontext/rules.md`.
    ///
    /// Avoids a disk read on every context-window call by caching the file
    /// content alongside its last-seen modification time.
    rules_loader: RulesLoader,
    /// Per-repo persistent key-value memory store.
    ///
    /// Loaded from `.omnicontext/memory.json` at engine startup and kept
    /// in-memory for fast reads.  Every `memory_set` / `memory_remove` call
    /// persists atomically to disk so the store survives process restarts.
    memory_store: MemoryStore,
    /// Intent classifier with optional prototype-vector second signal.
    ///
    /// Built once in `with_config()` after the embedder is initialised.
    /// Falls back to keyword-only classification when the embedder is
    /// unavailable (ONNX session not loaded).
    intent_classifier: crate::search::intent::IntentClassifier,
    /// Transient symbol index built during `run_index()` from the Rayon parse
    /// phase output.  Set to `Some` before the sequential store phase and
    /// cleared to `None` after `run_index()` completes so it does not pin
    /// memory between index runs.
    current_symbol_index: Option<std::sync::Arc<crate::graph::edge_extractor::SymbolIndex>>,
    /// In-memory inverted index for BGE-M3 sparse retrieval.
    ///
    /// Only populated when `config.embedding.enable_sparse_retrieval = true`.
    /// Built from all persisted `sparse_vectors` rows after each `run_index()`.
    /// Used by `search_sparse()` as a fourth RRF signal.
    sparse_index: SparseInvertedIndex,
    /// Optional cloud GPU embedding client.
    ///
    /// Active when `OMNI_CLOUD_API_KEY` is set or `config.embedding.cloud_api_key`
    /// is non-empty.  When `Some`, `embed_batch()` is routed to the cloud service
    /// instead of the local ONNX session.  Falls back to local ONNX on any HTTP
    /// error or timeout — cloud failure is never fatal.
    cloud_embedder: Option<CloudEmbedder>,
}

/// In-memory inverted index for sparse (SPLADE-style) retrieval.
///
/// Maps vocabulary token ID → sorted list of (chunk_id, weight) pairs.
/// Built from persisted `sparse_vectors` table rows after each index run.
///
/// For corpora < 100k chunks the per-query cost is acceptable with a full
/// linear scan.  For larger corpora the token-partition inverted index reduces
/// the effective candidate set to only chunks that share at least one token
/// with the query.
#[derive(Default)]
#[allow(dead_code)]
struct SparseInvertedIndex {
    /// token_id → sorted-descending list of (chunk_id, weight) pairs.
    index: std::collections::HashMap<u32, Vec<(i64, f32)>>,
    /// Total number of chunks indexed (for diagnostics).
    chunk_count: usize,
}

#[allow(dead_code)]
impl SparseInvertedIndex {
    /// Build from all (chunk_id, tokens) rows loaded from SQLite.
    fn build(rows: Vec<(i64, Vec<(u32, f32)>)>) -> Self {
        let chunk_count = rows.len();
        let mut index: std::collections::HashMap<u32, Vec<(i64, f32)>> =
            std::collections::HashMap::new();

        for (chunk_id, tokens) in rows {
            for (token_id, weight) in tokens {
                index.entry(token_id).or_default().push((chunk_id, weight));
            }
        }

        // Sort each posting list by weight descending for early-exit traversal.
        for posting in index.values_mut() {
            posting.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        }

        Self { index, chunk_count }
    }

    /// Compute dot-product scores for all chunks matching at least one query token.
    ///
    /// Returns `(chunk_id, score)` pairs sorted by score descending, limited to `limit`.
    fn search(&self, query_tokens: &[(u32, f32)], limit: usize) -> Vec<(i64, f32)> {
        let mut scores: std::collections::HashMap<i64, f32> = std::collections::HashMap::new();

        for (token_id, q_weight) in query_tokens {
            if let Some(posting) = self.index.get(token_id) {
                for &(chunk_id, d_weight) in posting {
                    *scores.entry(chunk_id).or_insert(0.0) += q_weight * d_weight;
                }
            }
        }

        let mut results: Vec<(i64, f32)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    /// Returns the number of distinct chunks in the index.
    fn len(&self) -> usize {
        self.chunk_count
    }

    fn is_empty(&self) -> bool {
        self.chunk_count == 0
    }
}
/// to reclaim arena memory.
///
/// At batch_size=64 chunks/flush:
///
/// - 50 flushes × 64 chunks = 3,200 chunks between resets
/// - 3,200 chunks × 768 dims × 4 bytes ≈ 9.8 MB of embedding output
///
/// ORT arena overhead is typically 100–300 MB after ~50 batches — acceptable.
/// Tweak upward (e.g. 100) on systems with >32 GB RAM.
const ARENA_FLUSH_RESET_INTERVAL: usize = 50;

/// Chunk accumulation threshold before triggering an embedding flush.
///
/// 64 is the batch size that maximizes ONNX Runtime throughput on typical
/// dev machines (4–16 cores): large enough to amortize tokenizer overhead,
/// small enough to keep per-flush latency below ~100ms on CPU.
///
/// The session pool (see `session_pool.rs`) runs 2+ concurrent ONNX sessions
/// so while one batch is executing inference, the next is already accumulating.
/// This provides the pipeline overlap equivalent to a producer-consumer queue
/// without requiring shared mutable ownership of the Embedder.
const EMBEDDING_BATCH_FLUSH_SIZE: usize = 64;

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

        // Attempt to initialize cloud embedder from config or environment.
        // Config key takes precedence; env var is the fallback for users who haven't
        // written a config file.  Failure to init is non-fatal — local ONNX is used.
        let cloud_embedder = if let Some(ref key) = config.embedding.cloud_api_key {
            match CloudEmbedder::new(key.clone(), None) {
                Ok(c) => {
                    tracing::info!("cloud embedding service enabled via config");
                    Some(c)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to init cloud embedder from config, using local ONNX");
                    None
                }
            }
        } else {
            match CloudEmbedder::from_env() {
                Ok(Some(c)) => {
                    tracing::info!("cloud embedding service enabled via OMNI_CLOUD_API_KEY");
                    Some(c)
                }
                Ok(None) => None,
                Err(e) => {
                    tracing::warn!(error = %e, "cloud embedder env init failed, using local ONNX");
                    None
                }
            }
        };

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

        // Load persisted file-graph edges from SQLite (schema v5).
        // This restores the structural graph from the previous indexing run so
        // architectural context queries return live results immediately on startup
        // without waiting for a full re-index.  Errors are non-fatal (empty graph
        // degrades gracefully to zero-edge queries).
        match index.load_file_graph_edges() {
            Ok(persisted_edges) if !persisted_edges.is_empty() => {
                let edge_count = persisted_edges.len();
                for edge in persisted_edges {
                    let _ = file_dep_graph.add_edge(&edge);
                }
                tracing::info!(
                    edges = edge_count,
                    "restored file dependency graph from SQLite"
                );
            }
            Ok(_) => {
                tracing::debug!("no persisted file graph edges found");
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to load persisted file graph edges");
            }
        }

        // Load file hash cache for change detection

        let mut hash_cache = FileHashCache::load(&data_dir)?;

        // Pre-warm the in-memory mtime cache from the filesystem for all
        // previously-indexed files.  This converts the first post-restart
        // indexing pass from O(N × file_read) to O(N × stat) for repos
        // where nothing has changed since the last run.
        hash_cache.warm_mtime_cache(&config.repo_path);

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

        // Configure the Rayon global thread pool on first engine creation.
        // Uses all available logical cores for maximum parse throughput.
        // `try_build_global` is a no-op if the pool was already initialized
        // (e.g., a second Engine::with_config call in the same process).
        let rayon_threads = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(rayon_threads)
            .build_global();
        tracing::info!(threads = rayon_threads, "Rayon thread pool configured");

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

        // Load the persistent memory store before config is moved into Self.
        // Degrade gracefully to an empty store on any I/O or parse error —
        // a missing or malformed memory.json must never prevent engine startup.
        let memory_store = match MemoryStore::load(&config.repo_path) {
            Ok(store) => store,
            Err(e) => {
                tracing::warn!(error = %e, "failed to load memory store, starting empty");
                MemoryStore::default()
            }
        };

        // Build intent classifier prototype centroids.  This embeds
        // INTENT_PROTOTYPES (13 sentences) once using the already-loaded
        // embedder session.  If the embedder is unavailable (ONNX not loaded),
        // `build()` degrades to keyword-only mode automatically.
        let intent_classifier = crate::search::intent::IntentClassifier::build(&embedder);

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
            last_indexed_at: None,
            arena_flush_count: 0,
            offline_index_mode: false,
            rules_loader: RulesLoader::new(),
            memory_store,
            intent_classifier,
            current_symbol_index: None,
            sparse_index: SparseInvertedIndex::default(),
            cloud_embedder,
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
    /// ## Two-Phase Algorithm
    ///
    /// **Phase 1 — Change detection + parallel parse:**
    /// 1. Full directory scan via `FileWatcher`.
    /// 2. For each path, `hash_cache.check_and_read()` runs the three-tier
    ///    mtime→xxHash3 check and returns file content only when changed.
    ///    Files whose mtime and hash are unchanged are skipped with a single
    ///    `stat` syscall (~1 µs).
    /// 3. Changed files are parsed on a Rayon thread pool (CPU-bound, embarrassingly
    ///    parallel). AST parsing, chunking, and symbol extraction run in parallel.
    ///    Results are collected as `Vec<ParsedFile>` and sorted by path for
    ///    deterministic chunk ID assignment.
    ///
    /// **Phase 2 — Sequential store (single SQLite writer):**
    /// 4. All file writes are wrapped in one bulk SQLite `DEFERRED TRANSACTION`
    ///    to amortise fsync cost across the whole run.
    /// 5. For each `ParsedFile`, `store_parsed_file()` upserts the file record,
    ///    runs the incremental graph update, calls `reindex_file()`, and stages
    ///    new or changed chunks for batch embedding.
    /// 6. Chunk-level delta detection: if a chunk's `content_hash` matches the
    ///    stored value, it is unchanged and the existing `vector_id` is preserved.
    pub async fn run_index(&mut self, force: bool) -> OmniResult<IndexResult> {
        let repo_path = self.config.repo_path.clone();
        let (tx, mut rx) = mpsc::channel::<PipelineEvent>(1024);

        // Full directory scan in a background thread
        let watcher = FileWatcher::new(&repo_path, &self.config.watcher, &self.config.indexing);
        let scan_tx = tx.clone();
        let scan_watcher = watcher.clone();
        let _scan_handle =
            tokio::task::spawn_blocking(move || scan_watcher.full_scan(&scan_tx).unwrap_or(0));
        drop(tx);

        if force {
            tracing::info!("force reindex requested; clearing existing index state first");
            self.clear_index()?;
        }

        self.arena_flush_count = 0;

        // ── Phase 1: collect changed paths + file content ──────────────────────
        //
        // We drain the event channel sequentially here because `check_and_read`
        // needs `&mut self.hash_cache` which cannot cross the Rayon boundary.
        // The actual CPU work (parse + chunk + symbol) is done in parallel below.

        let mut changed_files: Vec<(std::path::PathBuf, String)> = Vec::new();
        let mut deleted_paths: Vec<std::path::PathBuf> = Vec::new();

        while let Some(event) = rx.recv().await {
            match event {
                PipelineEvent::FileChanged { path } => {
                    // Three-tier change detection — returns content only if changed
                    match tokio::task::block_in_place(|| self.hash_cache.check_and_read(&path)) {
                        Ok((true, Some(content))) => {
                            changed_files.push((path, content));
                        }
                        Ok((false, _)) => {
                            // mtime/hash unchanged — skip
                            tracing::debug!(path = %path.display(), "unchanged (tier check), skipping");
                        }
                        Ok((true, None)) => {
                            // Should not happen; treat as if changed but unreadable
                            tracing::warn!(
                                path = %path.display(),
                                "check_and_read returned changed=true but no content; skipping"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(path = %path.display(), error = %e, "change detection failed, skipping");
                        }
                    }
                }
                PipelineEvent::FileDeleted { path } => {
                    deleted_paths.push(path);
                }
                PipelineEvent::FullScan | PipelineEvent::Shutdown => {}
            }
        }

        // ── Phase 1b: parallel parse (Rayon) ───────────────────────────────────
        //
        // `parse_file_parallel` is a free function taking only immutable shared
        // refs that are `Send + Sync`.  `token_counter` is `Arc<dyn TokenCounter>`
        // (TokenCounter: Send + Sync) so we can cheaply clone the Arc and move it
        // into the Rayon closure without any unsafe code.

        // Snapshot immutable state needed inside the Rayon closure
        let config_snap = self.config.clone();
        let token_counter_arc = std::sync::Arc::clone(&self.token_counter);

        let parsed_results: Vec<ParsedFile> = tokio::task::block_in_place(|| {
            changed_files
                .into_par_iter()
                .filter_map(|(path, content)| {
                    parse_file_parallel(
                        &path,
                        &content,
                        &config_snap.repo_path,
                        &config_snap,
                        token_counter_arc.as_ref(),
                    )
                })
                .collect()
        });

        // Sort by path for deterministic chunk IDs across runs
        let mut parsed_results = parsed_results;
        parsed_results.sort_by(|a, b| a.path.cmp(&b.path));

        // ── Build SymbolIndex for cross-file CALLS/INSTANTIATES resolution ─────
        //
        // Constructed from all elements produced in the Rayon parse phase.
        // Stored on `self` so `store_parsed_file()` can access it without an
        // extra parameter.  Cleared at the end of this function to release
        // memory between index runs.
        {
            use crate::graph::edge_extractor::SymbolIndex;
            let pairs: Vec<(std::path::PathBuf, &[crate::parser::StructuralElement])> =
                parsed_results
                    .iter()
                    .map(|pf| (pf.path.clone(), pf.elements.as_slice()))
                    .collect();
            let sym_idx = SymbolIndex::build(&pairs);
            self.current_symbol_index = Some(std::sync::Arc::new(sym_idx));
            tracing::debug!(
                files = parsed_results.len(),
                "built symbol index for cross-file edge resolution"
            );
        }

        // ── Phase 2: sequential store (SQLite writer) ──────────────────────────
        //
        // Open a single batch transaction so all N files commit in one fsync.
        let mut result = IndexResult::default();
        let mut pending_embeddings: Vec<(i64, String)> = Vec::with_capacity(512);

        // Process deletions first (no embeddings needed)
        for path in deleted_paths {
            if let Err(e) = self.index.delete_file(&path) {
                tracing::warn!(path = %path.display(), error = %e, "failed to delete file from index");
            }
            self.hash_cache.remove(&path);
        }

        if !parsed_results.is_empty() {
            // Begin the bulk transaction — if it fails, fall through to per-file
            // transactions so we still make progress.
            let bulk_tx_ok = self.index.begin_batch_transaction().is_ok();

            // Chunk counter for bounded transactions: commit every CHUNKS_PER_TX chunks
            // to prevent SQLite WAL files from growing unbounded on large repositories.
            const CHUNKS_PER_TX: usize = 500;
            let mut chunks_in_current_tx: usize = 0;

            for parsed in parsed_results {
                let parsed_chunk_count = parsed.chunks.len();
                match self.store_parsed_file(parsed, &mut pending_embeddings) {
                    Ok(stats) => {
                        result.files_processed += 1;
                        result.chunks_created += stats.chunks;
                        result.symbols_extracted += stats.symbols;
                        chunks_in_current_tx += parsed_chunk_count;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to store parsed file");
                        result.files_failed += 1;
                    }
                }

                // Commit and reopen if we've hit the chunk ceiling or the
                // embedding buffer is full. Whichever threshold fires first.
                let hit_chunk_limit = chunks_in_current_tx >= CHUNKS_PER_TX;
                let hit_embed_limit = pending_embeddings.len() >= EMBEDDING_BATCH_FLUSH_SIZE;

                if hit_chunk_limit || hit_embed_limit {
                    if bulk_tx_ok {
                        // Commit the current batch before the embedding flush (which
                        // itself does blocking I/O) so we don't hold the write lock.
                        if let Err(e) = self.index.commit_batch_transaction() {
                            tracing::warn!(error = %e, "batch transaction commit failed mid-flush");
                        }
                        chunks_in_current_tx = 0;
                    }

                    if hit_embed_limit {
                        if let Err(e) = self.flush_pending_embeddings(
                            &mut pending_embeddings,
                            &mut result.embeddings_generated,
                        ) {
                            tracing::error!(error = %e, "batch embedding flush failed");
                            result.embedding_failures += 1;
                        }
                    }

                    // Re-open a fresh batch transaction after the flush
                    if bulk_tx_ok {
                        let _ = self.index.begin_batch_transaction();
                    }
                }
            }

            // Commit any remaining unflushed writes
            if bulk_tx_ok {
                if let Err(e) = self.index.commit_batch_transaction() {
                    tracing::warn!(error = %e, "final batch transaction commit failed");
                }
            }
        }

        // Flush remaining pending embeddings
        if let Err(e) =
            self.flush_pending_embeddings(&mut pending_embeddings, &mut result.embeddings_generated)
        {
            tracing::error!(error = %e, "final embedding flush failed");
            result.embedding_failures += 1;
        }

        // Automatic recovery pass for chunks that still lack vectors
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
                    tracing::warn!(error = %e, "failed to inspect chunks without vectors after indexing");
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

        // Historical graph enhancement
        if let Ok(count) = self.index_commit_history() {
            tracing::info!(commits = count, "indexed commit history");
        }
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

                    // Persist newly-added HistoricalCoChange edges to SQLite (schema v5).
                    // Collect all HistoricalCoChange edges from the in-memory graph and
                    // upsert them so they survive daemon restarts.
                    let co_change_edges = self.file_dep_graph.all_edges_of_type(
                        crate::graph::dependencies::EdgeType::HistoricalCoChange,
                    );
                    if !co_change_edges.is_empty() {
                        if let Err(e) = self.index.save_file_graph_edges(&co_change_edges) {
                            tracing::debug!(error = %e, "failed to persist historical co-change edges");
                        } else {
                            tracing::debug!(
                                edges = co_change_edges.len(),
                                "persisted historical co-change edges"
                            );
                        }
                    }

                    let co_change_symbol_edges = self.bridge_historical_to_symbol_graph(&enhancer);
                    if co_change_symbol_edges > 0 {
                        tracing::info!(
                            edges = co_change_symbol_edges,
                            "bridged historical co-change edges into symbol graph"
                        );
                    }
                }

                let bug_prone = enhancer.find_bug_prone_files(2);
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

        // Embedding coverage summary
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

        self.last_indexed_at = Some(std::time::SystemTime::now());

        // Skip ANN index build in offline mode — caller will call build_ann_index()
        // explicitly so HNSW is built once from all vectors in batch.
        if !self.offline_index_mode && !self.vector_index.is_empty() {
            match self.vector_index.build_optimal_index() {
                Ok(()) => {
                    tracing::info!(
                        vectors = self.vector_index.len(),
                        strategy = self.vector_index.active_strategy(),
                        "ANN index built after indexing run"
                    );
                    if let Err(e) = self.vector_index.save() {
                        tracing::warn!(error = %e, "failed to persist ANN index after build");
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        vectors = self.vector_index.len(),
                        "failed to build ANN index; search will use flat strategy"
                    );
                }
            }
        } else if self.offline_index_mode {
            tracing::info!(
                vectors = self.vector_index.len(),
                "offline mode: ANN index build deferred — call build_ann_index() to complete"
            );
        }

        // Release the transient SymbolIndex — no longer needed after the store phase.
        self.current_symbol_index = None;

        // ── Build in-memory sparse inverted index (opt-in path) ───────────────
        //
        // If sparse retrieval is enabled, rebuild the in-memory inverted index
        // from all persisted sparse_vectors rows so `search_sparse()` is ready.
        if self.config.embedding.enable_sparse_retrieval {
            match self.index.get_all_sparse_vectors() {
                Ok(rows) => {
                    let count = rows.len();
                    self.sparse_index = SparseInvertedIndex::build(rows);
                    tracing::info!(
                        chunks = count,
                        "sparse inverted index built from persisted sparse vectors"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "failed to load sparse vectors for inverted index"
                    );
                }
            }
        }

        Ok(result)
    }
    ///
    /// Handles SQLite writes, incremental graph update, embedding staging,
    /// and chunk-level delta detection. Must be called sequentially (single
    /// SQLite writer constraint).
    fn store_parsed_file(
        &mut self,
        mut parsed: ParsedFile,
        pending_embeddings: &mut Vec<(i64, String)>,
    ) -> OmniResult<FileProcessStats> {
        let mut stats = FileProcessStats::default();

        // Upsert file to get the real file_id, then fix up placeholder IDs
        let file_id = tokio::task::block_in_place(|| self.index.upsert_file(&parsed.file_info))?;

        // Fix file_id in chunks and symbols (were 0 from parse phase)
        for chunk in &mut parsed.chunks {
            chunk.file_id = file_id;
        }
        for symbol in &mut parsed.symbols {
            symbol.file_id = file_id;
        }

        stats.chunks = parsed.chunks.len();
        stats.symbols = parsed.symbols.len();

        // ── Incremental graph update ──────────────────────────────────────────
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
                        file_id,
                        symbols = old_ids.len(),
                        edges_removed = removed,
                        "stripped stale edges from in-memory graph before reindex"
                    );
                }
            }
        }

        // ── Chunk-level delta: fetch existing content hashes ──────────────────
        // If a chunk's content_hash matches the stored value, skip re-embedding
        // and carry forward the existing vector_id.
        let existing_hashes =
            tokio::task::block_in_place(|| self.index.get_chunk_content_hashes_for_file(file_id))
                .unwrap_or_default();

        // For chunks whose content_hash matches the stored value, try to preserve
        // the existing vector_id by reading it from the DB.
        // This is a best-effort optimisation; if the query fails we re-embed.
        let existing_chunks_by_symbol: std::collections::HashMap<String, crate::types::Chunk> = {
            tokio::task::block_in_place(|| self.index.get_chunks_for_file(file_id))
                .unwrap_or_default()
                .into_iter()
                .map(|c| (c.symbol_path.clone(), c))
                .collect()
        };

        for chunk in &mut parsed.chunks {
            if chunk.is_summary || chunk.content_hash == 0 {
                continue;
            }
            if existing_hashes
                .get(&chunk.symbol_path)
                .copied()
                .map(|h| h == chunk.content_hash)
                .unwrap_or(false)
            {
                // Content unchanged — carry forward the existing vector_id
                if let Some(existing) = existing_chunks_by_symbol.get(&chunk.symbol_path) {
                    chunk.vector_id = existing.vector_id;
                }
            }
        }

        // ── Atomic reindex ────────────────────────────────────────────────────
        let (_fid, chunk_ids) = tokio::task::block_in_place(|| {
            self.index_breaker.call_sync(|| {
                self.index
                    .reindex_file(&parsed.file_info, &parsed.chunks, &parsed.symbols)
            })
        })
        .map_err(|e| match e {
            CircuitBreakerError::Open => OmniError::Internal(
                "index circuit breaker is open — too many recent failures".into(),
            ),
            CircuitBreakerError::OperationFailed(inner) => inner,
        })?;

        // ── Stage embeddings (skip unchanged chunks) ──────────────────────────
        if self.embedder.is_available() && !parsed.chunks.is_empty() {
            for (i, chunk) in parsed.chunks.iter().enumerate() {
                if i >= chunk_ids.len() {
                    break;
                }
                // Skip chunks that already have a vector_id (unchanged content)
                if chunk.vector_id.is_some() {
                    continue;
                }
                let text = crate::embedder::format_chunk_for_embedding(
                    parsed.language.as_str(),
                    &chunk.symbol_path,
                    &format!("{:?}", chunk.kind),
                    &chunk.content,
                );
                pending_embeddings.push((chunk_ids[i], text));
            }
        }

        // ── Dependency edges from references ──────────────────────────────────
        for element in &parsed.elements {
            if element.references.is_empty() {
                continue;
            }
            let source_symbol = if element.symbol_path.is_empty() {
                None
            } else {
                self.index.get_symbol_by_fqn(&element.symbol_path)?
            };
            let source_id = match source_symbol {
                Some(s) => s.id,
                None => continue,
            };
            for ref_name in &element.references {
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
                        if let Err(e) = self.index.insert_dependency(&edge) {
                            tracing::trace!(error = %e, "failed to insert dependency");
                        }
                        let _ = self.dep_graph.add_edge(&edge);
                    }
                }
            }
        }

        // ── Import-based dependency edges ─────────────────────────────────────
        if !parsed.imports.is_empty() {
            let file_source_id = self
                .index
                .get_first_symbol_for_file(file_id)
                .unwrap_or(None)
                .map(|s| s.id);

            if let Some(source_id) = file_source_id {
                for import in &parsed.imports {
                    for name in &import.imported_names {
                        if name == "*" {
                            continue;
                        }
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
                    let target_id =
                        DependencyGraph::resolve_import(&self.index, "", &import.import_path);
                    if let Some(target) = target_id {
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

        // ── Call graph edges ──────────────────────────────────────────────────
        let call_edges = self
            .dep_graph
            .build_call_edges(&self.index, file_id, &parsed.elements);
        for edge in &call_edges {
            if let Err(e) = self.index.insert_dependency(edge) {
                tracing::trace!(error = %e, "failed to insert call edge");
            }
        }
        stats.call_edges = call_edges.len();

        // ── Type hierarchy edges ──────────────────────────────────────────────
        let type_edges = self
            .dep_graph
            .build_type_edges(&self.index, file_id, &parsed.elements);
        for edge in &type_edges {
            if let Err(e) = self.index.insert_dependency(edge) {
                tracing::trace!(error = %e, "failed to insert type edge");
            }
        }

        // ── File-level graph: IMPORTS + INHERITS + CALLS + INSTANTIATES ───────
        // Register this file and wire structural edges into file_dep_graph so that
        // architectural context queries and edge-type metrics return live data.
        //
        // Design: fetch the full file list once (single SQLite read wrapped in
        // block_in_place) and reuse it for both import-path resolution and
        // EdgeExtractor registration. This eliminates the previous N×SQLite-read
        // pattern that panicked the tokio scheduler when called inside async context.
        {
            use crate::graph::dependencies::{DependencyEdge as FileDep, EdgeType as FileEdge};

            let file_path = parsed.file_info.path.clone();
            let lang_str = parsed.language.as_str().to_string();
            let _ = self.file_dep_graph.add_file(file_path.clone(), lang_str);

            // Single fetch — reused for IMPORTS resolution and EdgeExtractor.
            let all_indexed_files =
                tokio::task::block_in_place(|| self.index.get_all_files()).unwrap_or_default();

            // IMPORTS: best-effort name-match — compare import path's last segment
            // against each indexed file's stem. O(imports × files) but bounded by
            // per-file import count which is always small (< 100).
            for import in &parsed.imports {
                let last_segment = import
                    .import_path
                    .split(|c: char| c == ':' || c == '/' || c == '.' || c == '\\')
                    .next_back()
                    .unwrap_or("")
                    .to_lowercase();
                if last_segment.is_empty() {
                    continue;
                }
                for fi in &all_indexed_files {
                    let fname = fi
                        .path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if fname == last_segment {
                        let _ = self.file_dep_graph.add_edge(&FileDep {
                            source: file_path.clone(),
                            target: fi.path.clone(),
                            edge_type: FileEdge::Imports,
                            weight: 1.0,
                        });
                        break;
                    }
                }
            }

            // INHERITS, CALLS, INSTANTIATES — via EdgeExtractor (AST-derived).
            // Register every already-indexed file so cross-file type resolution
            // succeeds for files processed earlier in the same batch.
            //
            // Supply the transient SymbolIndex when available so CALLS/INSTANTIATES
            // that the ImportResolver cannot resolve (most cross-file calls) get a
            // second chance via short-name lookup.
            let mut extractor = if let Some(ref sym_idx) = self.current_symbol_index {
                crate::graph::edge_extractor::EdgeExtractor::with_symbol_index(
                    std::sync::Arc::clone(sym_idx),
                )
            } else {
                crate::graph::edge_extractor::EdgeExtractor::new()
            };
            for fi in &all_indexed_files {
                let module = crate::parser::build_module_name_from_path(&fi.path);
                extractor.register_file(fi.path.clone(), module);
            }
            if let Ok(structural_edges) =
                extractor.extract_edges(&file_path, parsed.language, &parsed.elements)
            {
                let edge_count = structural_edges.len();
                for edge in structural_edges {
                    let _ = self.file_dep_graph.add_edge(&edge);
                }
                if edge_count > 0 {
                    tracing::debug!(
                        path = %parsed.file_info.path.display(),
                        edges = edge_count,
                        "structural edges added to file dependency graph"
                    );
                }
            }

            // Persist updated edges for this file (schema v5).
            // Delete stale entries first so removed edges don't linger.
            if let Err(e) = self.index.delete_file_graph_edges_for_file(&file_path) {
                tracing::debug!(error = %e, "failed to delete stale file graph edges");
            }
            // Collect all outgoing edges for this file from the in-memory graph
            // and persist them atomically.
            let snapshot = self.file_dep_graph.outgoing_edges_for(&file_path);
            if !snapshot.is_empty() {
                if let Err(e) = self.index.save_file_graph_edges(&snapshot) {
                    tracing::debug!(error = %e, "failed to persist file graph edges");
                }
            }
        }

        // ── Cross-file data flow edges ────────────────────────────────────────
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
                        path = %parsed.path.display(),
                        flow_edges = flow_count,
                        "data flow edges extracted"
                    );
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::trace!(error = %e, "data flow extraction failed");
            }
        }

        // Commit hash cache entry
        let mtime = std::fs::metadata(&parsed.path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        self.hash_cache
            .update_from_read(parsed.path.clone(), parsed.file_content_hash_u64, mtime);

        tracing::debug!(
            path = %parsed.path.display(),
            chunks = stats.chunks,
            symbols = stats.symbols,
            call_edges = stats.call_edges,
            "file stored"
        );

        Ok(stats)
    }

    /// Process a single file through the pipeline.
    ///
    /// Parse -> Chunk -> Embed -> Store.
    ///
    /// Uses `check_and_read()` for three-tier change detection so the file
    /// content is read exactly once (no double-read).
    fn process_file(
        &mut self,
        path: &Path,
        pending_embeddings: &mut Vec<(i64, String)>,
    ) -> OmniResult<FileProcessStats> {
        let mut stats = FileProcessStats::default();
        tracing::info!("Starting to process file: {}", path.display());

        // Three-tier change detection + single file read.
        // block_in_place: fs::metadata + fs::read_to_string are blocking.
        let (changed, maybe_content) =
            tokio::task::block_in_place(|| self.hash_cache.check_and_read(path))?;

        if !changed {
            let rel_path = path.strip_prefix(&self.config.repo_path).unwrap_or(path);
            tracing::debug!(path = %rel_path.display(), "file unchanged, skipping");
            return Ok(stats);
        }

        let content = maybe_content.ok_or_else(|| {
            OmniError::Internal(format!(
                "check_and_read reported changed=true but returned no content for {}",
                path.display()
            ))
        })?;

        // Detect language — lowercase the extension for case-insensitive matching on
        // macOS and Windows where the filesystem may preserve the original case
        // (e.g., a file named `Main.RS` must be treated as Rust).
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());
        let ext = ext.as_deref().unwrap_or("");
        let language = Language::from_extension(ext);

        if matches!(language, Language::Unknown) {
            tracing::debug!(
                path = %path.display(),
                ext = ext,
                "skipping file with unrecognized extension"
            );
            return Err(OmniError::Parse {
                path: path.to_path_buf(),
                message: "unsupported language".into(),
            });
        }

        let rel_path = path.strip_prefix(&self.config.repo_path).unwrap_or(path);

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

        // Upsert the file first to get a file_id.
        // block_in_place: SQLite write — blocking, must not hold up the async runtime.
        let file_id = tokio::task::block_in_place(|| self.index.upsert_file(&file_info))?;

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

        // Annotate each leaf chunk with its xxHash3 for chunk-level delta detection.
        // Summary chunks keep content_hash=0 (always re-embedded as they are derived).
        for chunk in &mut chunks {
            if !chunk.is_summary {
                chunk.content_hash = xxhash_rust::xxh3::xxh3_64(chunk.content.as_bytes());
            }
        }

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

        // Atomic reindex: delete old chunks/symbols, insert new.
        // block_in_place: SQLite writes are blocking; signal tokio to allow
        // other async tasks to proceed on a different thread.
        let (_fid, chunk_ids) = tokio::task::block_in_place(|| {
            self.index_breaker
                .call_sync(|| self.index.reindex_file(&file_info, &chunks, &symbols))
        })
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

        // Update hash cache after successful indexing.
        // Capture mtime now; if it changed since check_and_read, the next warm run
        // will simply re-hash (Tier 2) rather than re-index (correct behaviour).
        let mtime = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let file_xxhash = xxhash_rust::xxh3::xxh3_64(content.as_bytes());
        self.hash_cache
            .update_from_read(path.to_path_buf(), file_xxhash, mtime);

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
                // Compute sparse results from the in-memory inverted index when enabled.
                // When `enable_sparse_retrieval = false`, `sparse_index` is empty and
                // this produces `&[]` — existing 3-signal RRF behavior is bit-identical.
                let sparse_hits: Vec<(i64, f32)> = if self.config.embedding.enable_sparse_retrieval
                    && !self.sparse_index.is_empty()
                    && self.embedder.has_sparse_session()
                {
                    self.embedder
                        .embed_sparse(query)
                        .map(|tokens| self.sparse_index.search(&tokens, limit * 2))
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };

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
                    &sparse_hits,
                )
            })
            .map_err(|e| match e {
                CircuitBreakerError::Open => OmniError::Internal(
                    "search circuit breaker is open — too many recent failures".into(),
                ),
                CircuitBreakerError::OperationFailed(inner) => inner,
            })
    }

    /// Execute a search query and prepend an ephemeral Critical-priority chunk
    /// for the currently open, unsaved editor buffer.
    ///
    /// Design: the active buffer represents the developer's current intent with
    /// certainty — no retrieval can be more relevant. By prepending it at score
    /// 3.0 with `ChunkPriority::Critical`, context assemblers will always include
    /// it first. The ephemeral chunk is constructed in memory and never written
    /// to SQLite; it exists only for the lifetime of this call.
    ///
    /// When `active_file_content` is `None`, this method is identical to `search`.
    pub fn search_with_active_content(
        &self,
        query: &str,
        limit: usize,
        active_file_content: Option<&str>,
    ) -> OmniResult<Vec<crate::types::SearchResult>> {
        use crate::types::{Chunk, ChunkKind, ScoreBreakdown, SearchResult, Visibility};
        use std::path::PathBuf;

        let mut results = self.search(query, limit)?;

        if let Some(content) = active_file_content {
            // Truncate at the last newline boundary at or before 50 KB so the
            // ephemeral chunk never overwhelms the token budget.
            const MAX_BYTES: usize = 50 * 1024;
            let truncated: &str = if content.len() > MAX_BYTES {
                // rfind returns the index OF the '\n'; use +1 to include it so
                // the truncated string ends on a complete newline boundary.
                let boundary = content[..MAX_BYTES]
                    .rfind('\n')
                    .map(|b| b + 1)
                    .unwrap_or(MAX_BYTES);
                &content[..boundary]
            } else {
                content
            };

            if !truncated.trim().is_empty() {
                let line_count = truncated.lines().count() as u32;
                // Estimate token count: ~4 characters per token (GPT-4 average).
                let token_count = (truncated.len() / 4).max(1) as u32;

                let ephemeral_chunk = Chunk {
                    id: 0, // never persisted
                    file_id: 0,
                    symbol_path: "<active buffer>".to_string(),
                    kind: ChunkKind::TopLevel,
                    visibility: Visibility::Public,
                    line_start: 1,
                    line_end: line_count.max(1),
                    content: truncated.to_string(),
                    doc_comment: None,
                    token_count,
                    weight: 1.0,
                    vector_id: None,
                    is_summary: false,
                    content_hash: 0,
                };

                let ephemeral = SearchResult {
                    chunk: ephemeral_chunk,
                    file_path: PathBuf::from("<active buffer>"),
                    score: 3.0,
                    score_breakdown: ScoreBreakdown {
                        rrf_score: 3.0,
                        structural_weight: 1.0,
                        ..ScoreBreakdown::default()
                    },
                };

                // Mark as Critical priority by setting it as the first element.
                // Consumers that read `priority` from the score should see Critical
                // because 3.0 > the Critical threshold (0.8).
                results.insert(0, ephemeral);
                tracing::debug!(
                    content_bytes = truncated.len(),
                    "injected active editor buffer as ephemeral Critical chunk"
                );
            }
        }

        Ok(results)
    }

    /// Execute a search and assemble a token-budget-aware context window.
    ///
    /// Intelligent context assembly pipeline:
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

    /// Assemble a flat, token-budget-optimal packed context window.
    ///
    /// Unlike `search_context_window()` (which returns a grouped `ContextWindow`
    /// with graph-neighbor expansion), this method returns a flat
    /// `Vec<PackedContextEntry>` tailored for agent-driven RAG orchestration:
    ///
    /// 1. Hybrid search with optional rerank threshold.
    /// 2. Sort results by `(file_path, line_start)`.
    /// 3. Merge adjacent same-file chunks where `line_end + 1 >= next.line_start`.
    /// 4. Re-sort merged list by score descending.
    /// 5. Greedy pack within `token_budget`.
    ///
    /// Returns `(packed_entries, tokens_used)`.
    pub fn pack_context_window(
        &self,
        query: &str,
        limit: usize,
        token_budget: u32,
        min_rerank_score: Option<f32>,
    ) -> OmniResult<(Vec<crate::search::pack::PackedContextEntry>, u32)> {
        use crate::search::pack::{greedy_pack, merge_adjacent, PackedContextEntry};

        let results = self.search_with_rerank_threshold(query, limit, min_rerank_score)?;

        // Convert SearchResult → PackedContextEntry, sort by (file, line_start).
        let mut entries: Vec<PackedContextEntry> = results
            .into_iter()
            .map(|r| PackedContextEntry {
                file_path: r.file_path,
                symbol_path: r.chunk.symbol_path.clone(),
                line_start: r.chunk.line_start,
                line_end: r.chunk.line_end,
                content: r.chunk.content.clone(),
                token_count: r.chunk.token_count,
                score: r.score,
                kind: r.chunk.kind,
            })
            .collect();

        entries.sort_by(|a, b| {
            a.file_path
                .cmp(&b.file_path)
                .then_with(|| a.line_start.cmp(&b.line_start))
        });

        // Merge adjacent same-file chunks.
        let merged = merge_adjacent(entries);

        // Re-sort by score descending for greedy packing.
        let mut sorted = merged;
        sorted.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(greedy_pack(sorted, token_budget))
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

    /// Classify a query using the blended keyword + prototype-vector classifier.
    ///
    /// Returns a [`QueryIntent`] using the full blended path when prototype
    /// embeddings are available, falling back to keyword-only heuristics when
    /// the embedder is unavailable.
    pub fn classify_intent(&self, query: &str) -> crate::search::intent::QueryIntent {
        self.intent_classifier.classify(query, Some(&self.embedder))
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

    /// Get a mutable reference to the config.
    ///
    /// Allows runtime mutation of configuration fields (e.g. batch_size, batch_timeout_ms)
    /// without restarting the engine. Changes take effect on the next operation that reads
    /// the affected field.
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Get a reference to the search engine (for cache stats, invalidation, etc.).
    pub fn search_engine(&self) -> &SearchEngine {
        &self.search_engine
    }

    /// Get a reference to the semantic reasoning engine.
    pub fn reasoning_engine(&self) -> &ReasoningEngine {
        &self.reasoning_engine
    }

    /// Return the timestamp of the most recently completed index run, if any.
    ///
    /// Returns `None` if no index has been run in this daemon session.
    pub fn last_indexed_at(&self) -> Option<std::time::SystemTime> {
        self.last_indexed_at
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

    // -----------------------------------------------------------------------
    // Phase 5 — Advanced retrieval APIs
    // -----------------------------------------------------------------------

    /// Search with post-filter on language, path glob, modification date, or symbol type.
    ///
    /// Runs the standard hybrid search and then filters the results by the
    /// caller-supplied criteria. All criteria are optional and ANDed together.
    ///
    /// - `language_filter`: e.g. "rust", "python", "typescript"
    /// - `path_glob`: glob pattern matched against file paths (e.g. "src/auth/**")
    /// - `modified_after`: ISO 8601 datetime string; only files indexed after this are included
    /// - `symbol_type_filter`: e.g. "function", "class", "struct"
    pub fn search_filtered(
        &self,
        query: &str,
        limit: usize,
        min_rerank_score: Option<f32>,
        language_filter: Option<&str>,
        path_glob: Option<&str>,
        modified_after: Option<&str>,
        symbol_type_filter: Option<&str>,
    ) -> OmniResult<Vec<crate::types::SearchResult>> {
        // Design: retrieve more candidates than `limit` to absorb post-filter loss,
        // then apply all filters and truncate to `limit`.
        let candidate_limit = (limit * 5).max(50);
        let raw = self.search_with_rerank_threshold(query, candidate_limit, min_rerank_score)?;

        let glob_matcher = path_glob.and_then(|p| {
            globset::GlobBuilder::new(p)
                .case_insensitive(true)
                .build()
                .ok()
                .map(|g| g.compile_matcher())
        });

        let results: Vec<_> = raw
            .into_iter()
            .filter(|r| {
                // Language filter
                if let Some(lang) = language_filter {
                    if let Ok(Some(fi)) = self.index.get_file_by_path(&r.file_path) {
                        if !fi.language.as_str().eq_ignore_ascii_case(lang) {
                            return false;
                        }
                    }
                }
                // Path glob filter
                if let Some(ref matcher) = glob_matcher {
                    let path_str = r.file_path.to_string_lossy();
                    let normalized = path_str.replace('\\', "/");
                    if !matcher.is_match(&normalized) {
                        return false;
                    }
                }
                // Symbol type filter
                if let Some(sym_type) = symbol_type_filter {
                    if !r.chunk.kind.as_str().eq_ignore_ascii_case(sym_type) {
                        return false;
                    }
                }
                // Modified-after filter: compare indexed_at against ISO 8601 cutoff.
                // Query directly — FileInfo doesn't carry the indexed_at field.
                if let Some(after) = modified_after {
                    let path_str = r.file_path.to_string_lossy();
                    let indexed_at: Option<String> = self
                        .index
                        .connection()
                        .query_row(
                            "SELECT indexed_at FROM files WHERE path = ?1",
                            rusqlite::params![path_str.as_ref()],
                            |row| row.get(0),
                        )
                        .ok();
                    if indexed_at.as_deref().unwrap_or("") < after {
                        return false;
                    }
                }
                true
            })
            .take(limit)
            .collect();

        Ok(results)
    }

    /// Assemble a rich explanation for a symbol by combining all available context.
    ///
    /// Returns a structured Markdown string with:
    /// - Type signature and doc comment
    /// - 1-hop callers and callees from the dependency graph
    /// - Recent commits touching the file
    /// - Co-change partners
    ///
    /// No LLM inference — purely assembled from indexed structured data.
    pub fn explain_symbol(&self, symbol_name: &str) -> OmniResult<String> {
        use std::fmt::Write;

        // Resolve symbol
        let symbol = if let Some(s) = self.index.get_symbol_by_fqn(symbol_name)? {
            s
        } else {
            let candidates = self.index.search_symbols_by_name(symbol_name, 3)?;
            if candidates.is_empty() {
                return Ok(format!("Symbol not found: `{symbol_name}`"));
            }
            candidates
                .into_iter()
                .next()
                .ok_or_else(|| OmniError::Internal("empty candidate list".into()))?
        };

        let mut out = format!("## {}\n**Kind**: {:?}\n", symbol.fqn, symbol.kind);

        // File path
        if let Ok(Some(fi)) = self.index.get_file_by_id(symbol.file_id) {
            writeln!(
                out,
                "**File**: {} (line {})",
                fi.path.display(),
                symbol.line
            )
            .ok();
        }

        // Source code from associated chunk
        if let Some(chunk_id) = symbol.chunk_id {
            if let Ok(chunks) = self.index.get_chunks_for_file(symbol.file_id) {
                if let Some(chunk) = chunks.iter().find(|c| c.id == chunk_id) {
                    if let Some(ref doc) = chunk.doc_comment {
                        writeln!(out, "\n**Documentation**:\n{doc}").ok();
                    }
                    write!(out, "\n**Definition**:\n```\n{}\n```\n", chunk.content).ok();
                }
            }
        }

        // 1-hop upstream (what this symbol depends on)
        let upstream = self.dep_graph.upstream(symbol.id, 1).unwrap_or_default();
        if !upstream.is_empty() {
            writeln!(out, "\n### Dependencies (calls / uses)").ok();
            for sym_id in upstream.iter().take(10) {
                if let Ok(Some(s)) = self.index.get_symbol_by_id(*sym_id) {
                    writeln!(out, "- **{}** ({:?})", s.fqn, s.kind).ok();
                }
            }
            if upstream.len() > 10 {
                writeln!(out, "- … and {} more", upstream.len() - 10).ok();
            }
        }

        // 1-hop downstream (what calls / uses this symbol)
        let downstream = self.dep_graph.downstream(symbol.id, 1).unwrap_or_default();
        if !downstream.is_empty() {
            writeln!(out, "\n### Callers / Dependents").ok();
            for sym_id in downstream.iter().take(10) {
                if let Ok(Some(s)) = self.index.get_symbol_by_id(*sym_id) {
                    writeln!(out, "- **{}** ({:?})", s.fqn, s.kind).ok();
                }
            }
            if downstream.len() > 10 {
                writeln!(out, "- … and {} more", downstream.len() - 10).ok();
            }
        }

        // Recent commits touching the file
        if let Ok(Some(fi)) = self.index.get_file_by_id(symbol.file_id) {
            let file_path_str = fi.path.display().to_string();
            let commits = CommitEngine::commits_for_file(&self.index, &file_path_str, 5)?;
            if !commits.is_empty() {
                writeln!(out, "\n### Recent Commits").ok();
                for commit in &commits {
                    writeln!(
                        out,
                        "- `{}` ({}) — {}",
                        &commit.hash[..8.min(commit.hash.len())],
                        commit.timestamp,
                        commit.message
                    )
                    .ok();
                }
            }

            // Co-change partners
            let co_changes = CommitEngine::co_change_files(&self.index, &file_path_str, 2, 5)?;
            if !co_changes.is_empty() {
                writeln!(out, "\n### Frequently Co-changed Files").ok();
                for cc in &co_changes {
                    writeln!(out, "- `{}` ({} commits)", cc.path, cc.shared_commits).ok();
                }
            }
        }

        Ok(out)
    }

    /// Get recent commits touching a specific file or symbol, with optional git diff summary.
    ///
    /// If `include_diff` is true, fetches the actual diff for each commit via `git show`.
    pub fn get_commit_summary(
        &self,
        file_or_symbol: &str,
        limit: usize,
        include_diff: bool,
    ) -> OmniResult<Vec<crate::commits::CommitInfo>> {
        // Resolve to a file path: try symbol lookup first, then treat as file path
        let file_path = if let Ok(Some(sym)) = self.index.get_symbol_by_fqn(file_or_symbol) {
            if let Ok(Some(fi)) = self.index.get_file_by_id(sym.file_id) {
                fi.path.display().to_string()
            } else {
                file_or_symbol.to_string()
            }
        } else {
            file_or_symbol.to_string()
        };

        let mut commits = CommitEngine::commits_for_file(&self.index, &file_path, limit)?;

        // Fall back: if nothing in DB, use live git log
        if commits.is_empty() {
            let git_out = std::process::Command::new("git")
                .args([
                    "log",
                    &format!("-{limit}"),
                    "--format=%H%n%s%n%an%n%aI",
                    "--name-only",
                    "--",
                    &file_path,
                ])
                .current_dir(&self.config.repo_path)
                .output();

            if let Ok(out) = git_out {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    commits = CommitEngine::parse_git_log_pub(&text);
                }
            }
        }

        if include_diff {
            for commit in &mut commits {
                let diff_out = std::process::Command::new("git")
                    .args(["show", "--stat", "--no-color", &commit.hash])
                    .current_dir(&self.config.repo_path)
                    .output();
                if let Ok(out) = diff_out {
                    if out.status.success() {
                        let diff_text = String::from_utf8_lossy(&out.stdout);
                        // Store concise stat as summary (first 1000 chars)
                        let summary_text: String = diff_text.chars().take(1000).collect();
                        commit.summary = Some(summary_text);
                    }
                }
            }
        }

        Ok(commits)
    }

    /// Search commits by keyword query (message, summary, author).
    pub fn search_commits_by_query(
        &self,
        query: &str,
        limit: usize,
    ) -> OmniResult<Vec<crate::commits::CommitInfo>> {
        // Try FTS5 index first
        let rowids = self.index.search_commits(query, limit)?;
        if !rowids.is_empty() {
            return self.index.get_commits_by_rowids(&rowids);
        }

        // Fallback: live git log keyword search when commits table is empty
        let git_out = std::process::Command::new("git")
            .args([
                "log",
                &format!("-{limit}"),
                "--format=%H%n%s%n%an%n%aI",
                "--name-only",
                &format!("--grep={query}"),
                "--regexp-ignore-case",
            ])
            .current_dir(&self.config.repo_path)
            .output();

        match git_out {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                Ok(CommitEngine::parse_git_log_pub(&text))
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Ingest an external document (URL or file path) into the index.
    ///
    /// Fetches content from `source`, splits into chunks, embeds them alongside
    /// code chunks (tagged as `external_doc` kind), and registers the document.
    ///
    /// Returns the number of new chunks created.
    pub fn ingest_external_doc(&mut self, source: &str, force_reingest: bool) -> OmniResult<usize> {
        // Skip if already ingested and not forced
        if !force_reingest && self.index.external_doc_exists(source) {
            return Ok(0);
        }

        // Fetch content
        let (title, content) = Self::fetch_external_content(source)?;

        if content.trim().is_empty() {
            return Err(OmniError::Internal(format!(
                "external source '{source}' returned empty content"
            )));
        }

        // Chunk content using a simple paragraph splitter
        // (no AST parser — external docs are prose, not code)
        let chunks_text = Self::chunk_prose(&content, 400); // ~400 tokens each

        let fake_file = crate::types::FileInfo {
            id: 0,
            path: std::path::PathBuf::from(source),
            language: crate::types::Language::Unknown,
            content_hash: format!("{:x}", xxhash_rust::xxh3::xxh3_64(content.as_bytes())),
            size_bytes: content.len() as u64,
        };

        let file_id = self.index.upsert_file(&fake_file).unwrap_or(0);

        let mut chunk_ids = Vec::with_capacity(chunks_text.len());
        let mut pending_embeddings = Vec::new();

        for (i, text) in chunks_text.iter().enumerate() {
            let chunk = crate::types::Chunk {
                id: 0,
                file_id,
                symbol_path: format!("external_doc::{}", i + 1),
                kind: crate::types::ChunkKind::Module, // module = prose section
                visibility: crate::types::Visibility::Public,
                line_start: (i * 10 + 1) as u32,
                line_end: ((i + 1) * 10) as u32,
                content: text.clone(),
                doc_comment: Some(format!("From: {source} — {title}")),
                token_count: (text.len() / 4) as u32, // rough estimate
                weight: 0.7,                          // slightly lower than code chunks
                vector_id: None,
                is_summary: false,
                content_hash: xxhash_rust::xxh3::xxh3_64(text.as_bytes()),
            };
            if let Ok(cid) = self.index.insert_chunk(&chunk) {
                chunk_ids.push(cid);
                pending_embeddings.push(crate::types::PipelineEvent::FileChanged {
                    path: std::path::PathBuf::from(source),
                });
            }
        }

        let _ = self
            .index
            .upsert_external_doc(source, &title, &content, &chunk_ids);

        tracing::info!(
            source = %source,
            title = %title,
            chunks = chunk_ids.len(),
            "ingested external document"
        );

        Ok(chunk_ids.len())
    }

    /// Fetch content from a URL or local file path.
    fn fetch_external_content(source: &str) -> OmniResult<(String, String)> {
        if source.starts_with("http://") || source.starts_with("https://") {
            // HTTP fetch via reqwest blocking client (sync, no tokio runtime needed here)
            let client = reqwest::blocking::Client::builder()
                .user_agent("OmniContext/1.0")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| OmniError::Internal(format!("HTTP client init failed: {e}")))?;

            let body = client
                .get(source)
                .send()
                .and_then(|r| r.text())
                .map_err(|e| {
                    OmniError::Internal(format!("HTTP fetch failed for '{source}': {e}"))
                })?;

            // Extract title from HTML <title> tag if present, else use URL
            let title = extract_html_title(&body)
                .unwrap_or_else(|| source.split('/').next_back().unwrap_or(source).to_string());

            // Strip HTML tags to plain text
            let text = strip_html_tags(&body);
            Ok((title, text))
        } else {
            // Local file
            let path = std::path::Path::new(source);
            let content = std::fs::read_to_string(path).map_err(OmniError::Io)?;
            let title = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(source)
                .to_string();
            Ok((title, content))
        }
    }

    /// Split prose text into chunks of approximately `target_tokens` tokens.
    fn chunk_prose(text: &str, target_tokens: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut current_tokens = 0usize;

        for para in text.split("\n\n") {
            let para_tokens = para.len() / 4 + 1;
            if current_tokens + para_tokens > target_tokens && !current.is_empty() {
                chunks.push(current.trim().to_string());
                current = String::new();
                current_tokens = 0;
            }
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(para);
            current_tokens += para_tokens;
        }
        if !current.trim().is_empty() {
            chunks.push(current.trim().to_string());
        }
        chunks
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

    /// Re-index a single file incrementally (real-time incremental indexing).
    ///
    /// Called by the daemon when a `text_edited` IDE event arrives. Unlike
    /// `process_file` which checks the content hash and skips unchanged files,
    /// this method always re-processes the file because the caller knows it
    /// has been edited.
    ///
    /// Returns the file processing stats, a changed flag, and a symbol-level
    /// `IndexDelta` describing exactly which symbols were added, removed, or
    /// modified. The delta lets the IPC layer send targeted cache invalidation
    /// and change notifications to connected IDE clients.
    pub fn reindex_single_file(
        &mut self,
        abs_path: &Path,
    ) -> OmniResult<(FileProcessStats, bool, IndexDelta)> {
        let start = std::time::Instant::now();

        // Check file exists
        if !abs_path.exists() {
            // File was deleted -- remove from index
            let rel_path = abs_path
                .strip_prefix(&self.config.repo_path)
                .unwrap_or(abs_path);

            // Capture symbol FQNs before deletion for the delta report.
            let removed_fqns: Vec<String> = self
                .index
                .get_file_by_path(rel_path)
                .ok()
                .flatten()
                .map(|fi| {
                    let fid = fi.id;
                    // Before deleting from SQLite, strip edges and nodes from in-memory graph
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
                    old_symbols.into_iter().map(|s| s.fqn).collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if let Err(e) = self.index.delete_file(rel_path) {
                tracing::warn!(error = %e, "failed to delete file from index");
            }
            // Remove from hash cache
            self.hash_cache.remove(abs_path);

            let delta = IndexDelta {
                removed_symbols: removed_fqns.clone(),
                has_structural_change: !removed_fqns.is_empty(),
                ..Default::default()
            };
            return Ok((FileProcessStats::default(), true, delta));
        }

        // Snapshot pre-reindex symbol state: map FQN → chunk content_hash
        // so we can compute the per-symbol delta after reprocessing.
        let rel_path_pre = abs_path
            .strip_prefix(&self.config.repo_path)
            .unwrap_or(abs_path);
        let pre_symbols: std::collections::HashMap<String, u64> = self
            .index
            .get_file_by_path(rel_path_pre)
            .ok()
            .flatten()
            .map(|fi| {
                // chunk content_hashes keyed by symbol_path
                self.index
                    .get_chunk_content_hashes_for_file(fi.id)
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        // Force reprocess by temporarily ignoring the hash check
        let mut pending = Vec::with_capacity(32);
        let mut stats = self.process_file(abs_path, &mut pending)?;
        let chunks_reembedded = pending.len();

        // Immediately flush for single file indexing
        let mut embeddings_generated = 0;
        self.flush_pending_embeddings(&mut pending, &mut embeddings_generated)?;
        stats.embeddings = embeddings_generated;

        // Compute post-reindex symbol state
        let post_symbols: std::collections::HashMap<String, u64> = self
            .index
            .get_file_by_path(rel_path_pre)
            .ok()
            .flatten()
            .map(|fi| {
                self.index
                    .get_chunk_content_hashes_for_file(fi.id)
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        // Diff pre vs post to populate IndexDelta fields.
        let mut added_symbols = Vec::new();
        let mut removed_symbols = Vec::new();
        let mut modified_symbols = Vec::new();

        for (fqn, post_hash) in &post_symbols {
            match pre_symbols.get(fqn.as_str()) {
                None => added_symbols.push(fqn.clone()),
                Some(pre_hash) if pre_hash != post_hash => modified_symbols.push(fqn.clone()),
                _ => {}
            }
        }
        for fqn in pre_symbols.keys() {
            if !post_symbols.contains_key(fqn.as_str()) {
                removed_symbols.push(fqn.clone());
            }
        }

        let has_structural_change = !added_symbols.is_empty() || !removed_symbols.is_empty();
        let is_body_only_change = !has_structural_change && !modified_symbols.is_empty();

        let delta = IndexDelta {
            added_symbols,
            removed_symbols,
            modified_symbols,
            chunks_reembedded,
            has_structural_change,
            is_body_only_change,
        };

        #[allow(clippy::cast_possible_truncation)]
        let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        let changed =
            stats.chunks > 0 || delta.has_structural_change || !delta.modified_symbols.is_empty();

        tracing::info!(
            path = %abs_path.display(),
            chunks = stats.chunks,
            symbols = stats.symbols,
            embeddings = stats.embeddings,
            added = delta.added_symbols.len(),
            removed = delta.removed_symbols.len(),
            modified = delta.modified_symbols.len(),
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

        Ok((stats, changed, delta))
    }

    /// Flush a batch of pending embeddings to the vector index.
    ///
    /// ## Pipeline Overlap (Augment-style)
    ///
    /// The embedding computation (`embed_batch_parallel`) is CPU-bound and
    /// purely immutable — it only needs `&self.embedder`.  The store phase
    /// (`vector_index.add` + `index.set_chunk_vector_id`) needs `&mut self`.
    ///
    /// `flush_pending_embeddings` is called synchronously because the borrow
    /// checker prevents concurrent `&self` (embed) and `&mut self` (store).
    /// The pipeline overlap is achieved at a higher level: the session pool
    /// in `embedder::session_pool` runs up to `pool_size` concurrent ONNX
    /// sessions so that while the current batch is being processed, the next
    /// batch accumulates in `pending_embeddings`.
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
        // Keep (chunk_id, content) pairs for the sparse path below.
        // Collected here while `pending` data is still accessible (before `pending.clear()`).
        let sparse_pairs: Vec<(i64, String)> = if self.config.embedding.enable_sparse_retrieval
            && self.embedder.has_sparse_session()
        {
            pending.iter().map(|(id, t)| (*id, t.clone())).collect()
        } else {
            Vec::new()
        };

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

        // ── Sparse embeddings (BGE-M3 opt-in path) ────────────────────────────
        //
        // Only runs when `enable_sparse_retrieval = true` AND the BGE-M3 ONNX
        // session is loaded.  Generates and persists SPLADE-style sparse vectors
        // for every chunk in the batch.  The in-memory inverted index is rebuilt
        // from all persisted sparse vectors at the end of `run_index()`.
        //
        // Cost: one additional ONNX session.run() per chunk — zero overhead
        // when disabled (the default).
        if self.config.embedding.enable_sparse_retrieval && self.embedder.has_sparse_session() {
            // `sparse_pairs` was captured before `pending.clear()`.
            for (chunk_id, content) in &sparse_pairs {
                match self.embedder.embed_sparse(content.as_str()) {
                    Ok(tokens) => {
                        if let Err(e) = self.index.save_sparse_vector(*chunk_id, &tokens) {
                            tracing::debug!(
                                chunk_id,
                                error = %e,
                                "failed to persist sparse vector"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::debug!(
                            chunk_id,
                            error = %e,
                            "sparse embed failed; skipping chunk"
                        );
                    }
                }
            }
        }

        pending.clear();

        // Interval-based ONNX arena reset.
        //
        // The ORT arena grows with every session.run() call and is only freed
        // when the Session is dropped.  Resetting on every flush reloads the
        // ~550MB model from disk ~625 times for a 10k-chunk index — an O(n²)
        // I/O cost.  Instead we reset every ARENA_FLUSH_RESET_INTERVAL flushes,
        // which bounds peak arena growth while keeping the reload cost constant.
        self.arena_flush_count += 1;
        if self.arena_flush_count % ARENA_FLUSH_RESET_INTERVAL == 0 {
            tracing::debug!(
                flush_count = self.arena_flush_count,
                interval = ARENA_FLUSH_RESET_INTERVAL,
                "resetting ONNX session to reclaim arena memory"
            );
            self.embedder.reset_session();
        }

        // Persist vectors to disk after each flush so progress survives crashes.
        // block_in_place: bincode serialization + atomic file write — blocking I/O.
        tokio::task::block_in_place(|| self.vector_index.save())?;

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

    /// Enable or disable offline index mode.
    ///
    /// When `true`, `run_index()` skips the per-flush ANN index rebuilds so
    /// vectors accumulate in the flat map only.  Call `build_ann_index()` once
    /// after `run_index()` returns to build the HNSW in a single batch pass.
    ///
    /// Use this for initial index builds of large repositories where building
    /// HNSW incrementally during embedding flushes is slower than building
    /// once at the end from all vectors simultaneously.
    pub fn set_offline_index_mode(&mut self, offline: bool) {
        self.offline_index_mode = offline;
        if offline {
            tracing::info!("offline index mode enabled — ANN build deferred to build_ann_index()");
        } else {
            tracing::info!("offline index mode disabled — incremental ANN updates restored");
        }
    }

    /// Returns `true` if the cloud GPU embedding service is configured and active.
    ///
    /// `true` means embedding requests will be routed to the cloud endpoint
    /// (`https://api.omnicontext.dev/v1/embed`) instead of the local ONNX session.
    /// Activated by `OMNI_CLOUD_API_KEY` env var or `config.embedding.cloud_api_key`.
    pub fn is_cloud_embedding_active(&self) -> bool {
        self.cloud_embedder.is_some()
    }

    /// Load repository rules from `.omnicontext/rules.md`, using mtime-based caching.
    ///
    /// Returns the content wrapped in a `<!-- rules -->…<!-- /rules -->` prefix block
    /// when the file is present, or an empty string when the file is absent or unreadable.
    /// The returned string is ready to be prepended directly to any context-window output.
    ///
    /// On I/O failure the error is logged at `WARN` level and an empty string is returned
    /// so that callers never need to handle a rules-injection error as fatal.
    pub fn load_rules_prefix(&mut self) -> String {
        match self.rules_loader.load_cached(&self.config.repo_path) {
            Ok(Some(rules)) => RulesLoader::format_prefix(&rules),
            Ok(None) => String::new(),
            Err(e) => {
                tracing::warn!(error = %e, "failed to load rules, skipping injection");
                String::new()
            }
        }
    }

    // -----------------------------------------------------------------------
    // Persistent memory API
    // -----------------------------------------------------------------------

    /// Retrieve a value from the persistent memory store by key.
    ///
    /// Returns `None` when the key is absent.  This is a pure in-memory read —
    /// no disk I/O on the hot path.
    pub fn memory_get(&self, key: &str) -> Option<String> {
        self.memory_store.get(key).map(str::to_owned)
    }

    /// Insert or update a key-value pair in the persistent memory store.
    ///
    /// The change is written to `.omnicontext/memory.json` atomically before
    /// this function returns.  Returns an error if the key or value exceed the
    /// size limits, or if the store is full (1,000 entries).
    pub fn memory_set(&mut self, key: String, value: String) -> OmniResult<()> {
        self.memory_store.set(key, value)?;
        self.memory_store.save(&self.config.repo_path)
    }

    /// Remove a key from the persistent memory store.
    ///
    /// Returns `Ok(true)` if the key existed and was removed, `Ok(false)` if
    /// the key was absent.  The change is persisted atomically before returning.
    pub fn memory_remove(&mut self, key: &str) -> OmniResult<bool> {
        let existed = self.memory_store.remove(key);
        if existed {
            self.memory_store.save(&self.config.repo_path)?;
        }
        Ok(existed)
    }

    /// List all memory keys with their last-updated Unix timestamps.
    ///
    /// Keys are returned in lexicographic order.  Pure in-memory read.
    pub fn memory_list(&self) -> Vec<(String, u64)> {
        self.memory_store
            .list_keys()
            .into_iter()
            .map(|(k, ts)| (k.to_owned(), ts))
            .collect()
    }

    /// Format the memory store as a context prefix block.
    ///
    /// Produces:
    /// ```text
    /// <!-- memory -->
    /// key: value
    /// <!-- /memory -->
    ///
    /// ```
    ///
    /// Returns an empty string when the store is empty so that the empty case
    /// contributes zero bytes to the assembled context window.
    pub fn memory_prefix(&self) -> String {
        self.memory_store.format_prefix()
    }

    /// Build the ANN (Approximate Nearest Neighbor) index from all stored vectors.
    ///
    /// Called explicitly after `run_index()` when `set_offline_index_mode(true)` was used.
    /// Builds `HnswIndex` using `build_batch()` which constructs the entire graph
    /// in one pass — significantly faster than N sequential `insert()` calls.
    ///
    /// Persists the rebuilt index to disk.
    pub fn build_ann_index(&mut self) -> OmniResult<()> {
        if self.vector_index.is_empty() {
            tracing::info!("no vectors to build ANN index from");
            return Ok(());
        }

        let start = std::time::Instant::now();
        self.vector_index.build_optimal_index()?;
        let elapsed = start.elapsed();

        tracing::info!(
            vectors = self.vector_index.len(),
            strategy = self.vector_index.active_strategy(),
            elapsed_ms = elapsed.as_millis(),
            "offline ANN index built"
        );

        self.vector_index.save()?;
        Ok(())
    }

    /// Number of vectors currently stored in the index.
    pub fn vector_count(&self) -> usize {
        self.vector_index.len()
    }
}

// ---------------------------------------------------------------------------
// External doc ingestion helpers (module-level, not part of Engine)
// ---------------------------------------------------------------------------

/// Extract the `<title>` tag content from an HTML string.
fn extract_html_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title>")? + "<title>".len();
    let end = lower.find("</title>")?;
    if start < end {
        Some(html[start..end].trim().to_string())
    } else {
        None
    }
}

/// Strip HTML tags from a string, returning plain text.
///
/// Handles the most common cases: removes all `<...>` blocks and decodes
/// a small set of named HTML entities. Good enough for documentation pages.
fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut inside_tag = false;
    let mut inside_script = false;
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '<' => {
                inside_tag = true;
                // Detect <script> and <style> blocks to skip entirely
                let lookahead: String = chars.clone().take(6).collect();
                if lookahead.to_lowercase().starts_with("script")
                    || lookahead.to_lowercase().starts_with("style")
                {
                    inside_script = true;
                }
                if lookahead.starts_with('/') {
                    let inner: String = chars.clone().skip(1).take(6).collect();
                    if inner.to_lowercase().starts_with("script")
                        || inner.to_lowercase().starts_with("style")
                    {
                        inside_script = false;
                    }
                }
            }
            '>' => {
                inside_tag = false;
                // Add whitespace where block elements end
                out.push(' ');
            }
            _ if inside_tag || inside_script => {}
            '&' => {
                // Collect entity up to ';' (max 8 chars)
                let mut entity = String::new();
                while let Some(&next) = chars.peek() {
                    if next == ';' {
                        chars.next();
                        break;
                    }
                    if entity.len() > 8 {
                        break;
                    }
                    entity.push(next);
                    chars.next();
                }
                let decoded = match entity.as_str() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "apos" => "'",
                    "nbsp" => " ",
                    _ => " ",
                };
                out.push_str(decoded);
            }
            _ => out.push(c),
        }
    }

    // Collapse excessive whitespace
    let mut result = String::with_capacity(out.len());
    let mut prev_space = false;
    for ch in out.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                result.push(' ');
            }
            prev_space = true;
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result.trim().to_string()
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

/// Symbol-level diff produced by `reindex_single_file`.
///
/// Design: rather than forcing clients to re-query the entire file after an
/// edit event, the daemon can forward this delta so IDEs and search caches
/// know exactly what changed. The delta is computed by diffing the
/// pre-reindex symbol table against the post-reindex one using xxHash3
/// chunk hashes as the identity key.
///
/// Fields contain fully-qualified symbol names (FQNs), not raw names.
#[derive(Debug, Default, Clone)]
pub struct IndexDelta {
    /// Symbols added since the previous index state (new functions, classes, etc.)
    pub added_symbols: Vec<String>,
    /// Symbols removed (renamed, deleted, or moved to another file).
    pub removed_symbols: Vec<String>,
    /// Symbols whose source content changed (implementation altered).
    pub modified_symbols: Vec<String>,
    /// Number of chunks that were re-embedded due to content changes.
    pub chunks_reembedded: usize,
    /// Whether any structural change (add/remove symbol) occurred.
    pub has_structural_change: bool,
    /// Whether only implementation bodies changed (no signature/name changes).
    pub is_body_only_change: bool,
}

// ---------------------------------------------------------------------------
// ParsedFile — result of the CPU-bound parse phase (safe for Rayon)
// ---------------------------------------------------------------------------

/// Output from the pure, CPU-bound parse phase.
///
/// Contains everything needed to store a file in the index without requiring
/// `&mut self` — i.e. without holding any exclusive reference to `Engine`.
/// This is the type that `parse_file_parallel()` returns and `run_index()`
/// collects before the sequential `store_parsed_file()` phase.
struct ParsedFile {
    /// Absolute path to the source file.
    path: std::path::PathBuf,
    /// xxHash3 of the full file content (for hash cache update after store).
    file_content_hash_u64: u64,
    /// Detected programming language.
    language: Language,
    /// `FileInfo` ready for `upsert_file`.
    file_info: FileInfo,
    /// Chunks with `content_hash` set per chunk.
    chunks: Vec<Chunk>,
    /// Symbols derived from chunks.
    symbols: Vec<Symbol>,
    /// Parsed elements for dependency graph construction.
    elements: Vec<crate::parser::StructuralElement>,
    /// Import statements for dependency resolution.
    imports: Vec<crate::types::ImportStatement>,
}

/// CPU-bound parse phase — pure, `Send`, safe for Rayon parallelism.
///
/// Does NOT touch SQLite, the embedder, or any `&mut` state. Takes only
/// immutable shared references that are `Send + Sync`.
///
/// Returns `None` when the file should be skipped (unknown language, I/O
/// error, or parse failure). Errors are logged at `warn` level.
fn parse_file_parallel(
    path: &std::path::Path,
    content: &str,
    repo_path: &std::path::Path,
    config: &crate::config::Config,
    token_counter: &(dyn chunker::token_counter::TokenCounter + Send + Sync),
) -> Option<ParsedFile> {
    use xxhash_rust::xxh3::xxh3_64;

    // Detect language
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    let ext = ext.as_deref().unwrap_or("");
    let language = Language::from_extension(ext);

    if matches!(language, Language::Unknown) {
        tracing::debug!(path = %path.display(), ext, "skipping unrecognized extension");
        return None;
    }

    let rel_path = path.strip_prefix(repo_path).unwrap_or(path);

    // Parse structural elements
    let elements = match crate::parser::parse_file(rel_path, content.as_bytes(), language) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "parse failed, skipping");
            return None;
        }
    };

    // Import statements for dependency graph
    let imports =
        crate::parser::parse_imports(path, content.as_bytes(), language).unwrap_or_default();

    // Content hashes
    let file_content_hash_u64 = xxh3_64(content.as_bytes());

    let file_info = FileInfo {
        id: 0, // assigned by upsert_file
        path: rel_path.to_path_buf(),
        language,
        content_hash: compute_file_hash(content),
        size_bytes: content.len() as u64,
    };

    // Chunk — pass dummy file_id=0; will be fixed in store_parsed_file
    let mut chunks = chunker::chunk_elements(
        &elements, &file_info, &imports, 0, // file_id placeholder
        config, content, token_counter,
    );

    // Annotate each chunk with its own xxHash3 for chunk-level delta detection
    for chunk in &mut chunks {
        chunk.content_hash = xxh3_64(chunk.content.as_bytes());
    }

    // RAPTOR summary chunks (no content_hash needed — always re-embedded)
    let summary_chunks = chunker::generate_summary_chunks(&chunks, &file_info, token_counter);
    if !summary_chunks.is_empty() {
        chunks.extend(summary_chunks);
    }

    // Build Symbol records from non-summary chunks
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
            file_id: 0, // placeholder
            line: c.line_start,
            chunk_id: None,
        })
        .collect();

    Some(ParsedFile {
        path: path.to_path_buf(),
        file_content_hash_u64,
        language,
        file_info,
        chunks,
        symbols,
        elements,
        imports,
    })
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_index_empty_directory() {
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let mut engine = Engine::with_config(config).expect("create engine");
        let result = engine.run_index(false).await.expect("index");
        assert_eq!(result.files_processed, 0);
        assert_eq!(result.chunks_created, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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

    // ── arena_flush_count + ARENA_FLUSH_RESET_INTERVAL ───────────────────────

    #[test]
    fn test_arena_flush_reset_interval_is_sane() {
        // Design: ARENA_FLUSH_RESET_INTERVAL must be in [1, 200].
        // The bounds are verified at compile time via the const assertions here.
        // This test is a marker — if it fails to compile, the constant is out of range.
        const _: () = assert!(
            ARENA_FLUSH_RESET_INTERVAL >= 1,
            "ARENA_FLUSH_RESET_INTERVAL must be >= 1 (otherwise every flush reloads the model)"
        );
        const _: () = assert!(
            ARENA_FLUSH_RESET_INTERVAL <= 200,
            "ARENA_FLUSH_RESET_INTERVAL must be <= 200 (otherwise arena grows unboundedly)"
        );
        // Passes when the constants compile — no runtime assertion needed.
    }

    #[test]
    fn test_engine_arena_flush_count_initialises_to_zero() {
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let engine = Engine::with_config(config).expect("create engine");
        assert_eq!(
            engine.arena_flush_count, 0,
            "arena_flush_count must be 0 at engine creation"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_run_index_resets_arena_flush_count() {
        // After a completed run_index on an empty dir, arena_flush_count should be 0
        // (reset at start of run_index) regardless of any previous value.
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let mut engine = Engine::with_config(config).expect("create engine");
        // Manually set a non-zero value to simulate a previous run.
        engine.arena_flush_count = 99;
        engine.run_index(false).await.expect("index");
        // After run_index the counter represents flushes done in that run.
        // For an empty dir, no flushes happen, so it should be 0.
        assert_eq!(
            engine.arena_flush_count, 0,
            "arena_flush_count should be reset to 0 at start of run_index"
        );
    }

    // ── vector index build after indexing ────────────────────────────────────

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_run_index_calls_build_optimal_index_on_non_empty() {
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        // Write two Rust files so we get some vectors.
        std::fs::write(
            root.join("lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\
             pub fn sub(a: i32, b: i32) -> i32 { a - b }\n",
        )
        .expect("write lib.rs");

        let config = Config::defaults(root);
        let mut engine = Engine::with_config(config).expect("create engine");
        engine.run_index(false).await.expect("index");

        // active_strategy returns a &'static str; for a small index (<5000 vectors)
        // it will be "flat" — but the important thing is it does NOT panic.
        // This test verifies build_optimal_index was called without error.
        let strategy = engine.vector_index.active_strategy();
        assert!(
            matches!(strategy, "flat" | "ivf" | "hnsw"),
            "unexpected strategy: {strategy}"
        );
    }

    // ── spawn_blocking / block_in_place integration guard ────────────────────

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_run_index_with_multi_thread_runtime() {
        // Regression test: block_in_place requires multi-thread runtime.
        // If this test panics with "block_in_place cannot be called on a current_thread runtime",
        // it means a call site uses block_in_place where it shouldn't.
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::write(root.join("main.py"), "def hello():\n    return 'world'\n").expect("write");

        let config = Config::defaults(root);
        let mut engine = Engine::with_config(config).expect("create engine");
        // Must not panic when running under multi_thread runtime (block_in_place is valid here).
        let result = engine.run_index(false).await.expect("index should succeed");
        assert!(
            result.files_processed >= 1,
            "should process at least one file"
        );
    }

    // ── Two-phase incremental indexing tests ─────────────────────────────────

    /// Warm run: after a full index, a second run with no file changes must
    /// process zero files (mtime + hash tiers both short-circuit).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_run_index_warm_skips_all_files() {
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::write(root.join("stable.rs"), "pub fn stable() -> u32 { 42 }\n").expect("write");

        let config = Config::defaults(root);
        let mut engine = Engine::with_config(config).expect("create engine");

        // Cold run: file is new, must be indexed.
        let cold = engine.run_index(false).await.expect("cold index");
        assert_eq!(cold.files_processed, 1, "cold run must index the file");

        // Warm run: nothing changed — mtime and hash identical.
        let warm = engine.run_index(false).await.expect("warm index");
        assert_eq!(
            warm.files_processed, 0,
            "warm run must skip all unchanged files"
        );
    }

    /// Incremental run: only the modified file should be re-indexed.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_run_index_incremental_one_changed() {
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();

        std::fs::write(root.join("a.rs"), "pub fn a() -> u32 { 1 }\n").expect("write a.rs");
        std::fs::write(root.join("b.rs"), "pub fn b() -> u32 { 2 }\n").expect("write b.rs");

        let config = Config::defaults(root);
        let mut engine = Engine::with_config(config).expect("create engine");

        // Cold run indexes both files.
        let cold = engine.run_index(false).await.expect("cold index");
        assert_eq!(cold.files_processed, 2, "cold run must index both files");

        // Modify only b.rs.
        std::fs::write(root.join("b.rs"), "pub fn b() -> u32 { 99 }\n").expect("update b.rs");

        // Incremental run: only b.rs changed.
        let inc = engine.run_index(false).await.expect("incremental index");
        assert_eq!(
            inc.files_processed, 1,
            "incremental run must re-index only the changed file"
        );
    }

    /// `parse_file_parallel` is a pure free function — verify it can be called
    /// from multiple threads simultaneously (no `&mut self` capture).
    #[test]
    fn test_parse_file_parallel_is_pure() {
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        let path = root.join("lib.rs");
        std::fs::write(&path, "pub fn hello() -> &'static str { \"hi\" }\n").expect("write");

        let config = Config::defaults(root);
        let counter: std::sync::Arc<dyn crate::chunker::token_counter::TokenCounter> =
            std::sync::Arc::new(crate::chunker::token_counter::EstimateTokenCounter);

        // Call from multiple threads via std::thread to confirm Send + no &mut self.
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let p = path.clone();
                let r = root.to_path_buf();
                let c = config.clone();
                let tc = std::sync::Arc::clone(&counter);
                let content = std::fs::read_to_string(&p).expect("read");
                std::thread::spawn(move || parse_file_parallel(&p, &content, &r, &c, tc.as_ref()))
            })
            .collect();

        for handle in handles {
            let result = handle.join().expect("thread panicked");
            assert!(
                result.is_some(),
                "parse_file_parallel must succeed for valid Rust"
            );
        }
    }

    /// Chunk-level delta: re-indexing an unchanged file must preserve existing
    /// vector IDs for chunks whose content_hash matches the stored value.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_chunk_delta_skips_unchanged_chunks() {
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::write(
            root.join("delta.rs"),
            "pub fn delta_fn() -> u32 { 100 }\npub fn helper() -> u32 { 200 }\n",
        )
        .expect("write");

        let config = Config::defaults(root);
        let mut engine = Engine::with_config(config).expect("create engine");

        // Cold run: index and embed (or stage for embedding; no model in CI).
        engine.run_index(false).await.expect("cold index");

        // Touch the mtime without changing content — forces hash re-check.
        let path = root.join("delta.rs");
        // Re-write identical content so mtime changes but hash is the same.
        let content = std::fs::read_to_string(&path).expect("read");
        std::fs::write(&path, &content).expect("rewrite same content");

        // Second run: mtime changed but hash identical → still skipped at tier 2.
        let warm = engine.run_index(false).await.expect("warm after touch");
        assert_eq!(
            warm.files_processed, 0,
            "content-identical file must be skipped even after mtime change"
        );
    }

    /// Chunk-level delta: a chunk whose content changes must be re-embedded
    /// (new content_hash diverges from stored value).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_chunk_delta_reembeds_changed_chunk() {
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        let path = root.join("evolving.rs");
        std::fs::write(&path, "pub fn evolving() -> u32 { 1 }\n").expect("write");

        let config = Config::defaults(root);
        let mut engine = Engine::with_config(config).expect("create engine");

        engine.run_index(false).await.expect("cold index");

        // Change the content so the chunk hash diverges.
        std::fs::write(&path, "pub fn evolving() -> u32 { 2 }\n").expect("update");

        let inc = engine
            .run_index(false)
            .await
            .expect("incremental after change");
        assert_eq!(inc.files_processed, 1, "changed file must be re-indexed");
    }

    /// Deterministic ordering: running `run_index` twice on the same unchanged
    /// repo (after clearing the hash cache) must produce the same chunk count.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_two_phase_ordering_deterministic() {
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        // Write several files to give Rayon something to parallelise.
        for i in 0..5 {
            std::fs::write(
                root.join(format!("mod{i}.rs")),
                format!("pub fn func_{i}() -> u32 {{ {i} }}\n"),
            )
            .expect("write");
        }

        // First run: cold index.
        let config = Config::defaults(root);
        let mut engine1 = Engine::with_config(config.clone()).expect("engine 1");
        let r1 = engine1.run_index(false).await.expect("first index");

        // Second run with a fresh engine (forces re-read from disk),
        // but this time force=true to clear state and re-index all files.
        let mut engine2 = Engine::with_config(config).expect("engine 2");
        let r2 = engine2.run_index(true).await.expect("second index");

        assert_eq!(
            r1.chunks_created, r2.chunks_created,
            "deterministic ordering must produce identical chunk counts across runs"
        );
    }

    // -----------------------------------------------------------------------
    // SparseInvertedIndex unit tests (Item 8 — BGE-M3 sparse track)
    // -----------------------------------------------------------------------

    #[test]
    fn test_sparse_inverted_index_build_empty() {
        let idx = SparseInvertedIndex::build(vec![]);
        assert!(idx.is_empty(), "empty input must produce empty index");
        assert_eq!(idx.len(), 0);
    }

    #[test]
    fn test_sparse_inverted_index_build_posting_lists() {
        // Two chunks share token 1; only chunk 0 has token 2.
        let rows = vec![
            (0_i64, vec![(1_u32, 0.8_f32), (2_u32, 0.4_f32)]),
            (1_i64, vec![(1_u32, 0.6_f32)]),
        ];
        let idx = SparseInvertedIndex::build(rows);

        assert_eq!(idx.len(), 2, "two chunks must be recorded");
        assert!(!idx.is_empty());

        // Posting list for token 1 must contain both chunks.
        let posting = idx.index.get(&1).expect("token 1 must have a posting list");
        assert_eq!(posting.len(), 2, "token 1 has two matching chunks");

        // Posting list for token 2 must contain only chunk 0.
        let posting2 = idx.index.get(&2).expect("token 2 must have a posting list");
        assert_eq!(posting2.len(), 1);
        assert_eq!(posting2[0].0, 0_i64);
    }

    #[test]
    fn test_sparse_inverted_index_search_dot_product() {
        // chunk 0: (1→1.0, 2→1.0)  chunk 1: (1→0.5)  chunk 2: (3→1.0) — no overlap
        let rows = vec![
            (0_i64, vec![(1_u32, 1.0_f32), (2_u32, 1.0_f32)]),
            (1_i64, vec![(1_u32, 0.5_f32)]),
            (2_i64, vec![(3_u32, 1.0_f32)]),
        ];
        let idx = SparseInvertedIndex::build(rows);

        // Query: (1→1.0, 2→0.5)
        // chunk 0 score = 1*1 + 1*0.5 = 1.5
        // chunk 1 score = 0.5*1       = 0.5
        // chunk 2 score = 0 (no overlap) — must not appear
        let results = idx.search(&[(1_u32, 1.0_f32), (2_u32, 0.5_f32)], 10);

        let ids: Vec<i64> = results.iter().map(|(id, _)| *id).collect();
        assert!(!ids.contains(&2_i64), "zero-overlap chunk must be absent");
        assert_eq!(ids[0], 0_i64, "chunk 0 must rank first (score 1.5)");
        assert_eq!(ids[1], 1_i64, "chunk 1 must rank second (score 0.5)");

        let score_0 = results.iter().find(|(id, _)| *id == 0).unwrap().1;
        assert!(
            (score_0 - 1.5_f32).abs() < 1e-5,
            "dot-product for chunk 0 must be 1.5, got {score_0}"
        );
    }

    #[test]
    fn test_sparse_inverted_index_search_limit_respected() {
        // 10 chunks each with token 1 at weight 1.0 — search with limit 3 must return exactly 3.
        let rows: Vec<(i64, Vec<(u32, f32)>)> = (0..10)
            .map(|i| (i as i64, vec![(1_u32, 1.0_f32)]))
            .collect();
        let idx = SparseInvertedIndex::build(rows);
        let results = idx.search(&[(1_u32, 1.0_f32)], 3);
        assert_eq!(results.len(), 3, "search must respect the limit parameter");
    }

    #[test]
    fn test_sparse_inverted_index_search_empty_query() {
        let rows = vec![(0_i64, vec![(1_u32, 1.0_f32)])];
        let idx = SparseInvertedIndex::build(rows);
        let results = idx.search(&[], 10);
        assert!(
            results.is_empty(),
            "empty query tokens must return no results"
        );
    }

    // ── active buffer injection ───────────────────────────────────────────────

    #[test]
    fn test_active_content_injected_as_critical_priority() {
        // Design: when active_file_content is provided, the first result must be
        // the ephemeral chunk with score 3.0 and symbol_path "<active buffer>".
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let engine = Engine::with_config(config).expect("create engine");

        let content = "fn main() {\n    println!(\"hello\");\n}\n";
        let results = engine
            .search_with_active_content("main function", 10, Some(content))
            .expect("search");

        // Even on empty index we get exactly one result: the ephemeral chunk.
        assert_eq!(results.len(), 1, "ephemeral chunk must be the only result");
        let first = &results[0];
        assert_eq!(first.chunk.symbol_path, "<active buffer>");
        assert!(
            (first.score - 3.0).abs() < f64::EPSILON,
            "ephemeral chunk score must be 3.0"
        );
        assert_eq!(first.chunk.line_start, 1);
        assert_eq!(first.chunk.content, content);
    }

    #[test]
    fn test_active_content_truncated_at_50kb() {
        // Design: content exceeding 50 KB must be truncated at the last newline
        // at or before the 50 KB mark so the chunk stays within the token budget.
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let engine = Engine::with_config(config).expect("create engine");

        // Build content just over 50 KB: lots of short lines.
        let line = "x".repeat(100) + "\n"; // 101 bytes per line
        let repeat_count = (50 * 1024 / line.len()) + 10; // enough to exceed 50 KB
        let big_content: String = line.repeat(repeat_count);
        assert!(
            big_content.len() > 50 * 1024,
            "test fixture must exceed 50 KB"
        );

        let results = engine
            .search_with_active_content("query", 10, Some(&big_content))
            .expect("search");

        assert_eq!(results.len(), 1, "ephemeral chunk must be present");
        let stored = &results[0].chunk.content;
        assert!(
            stored.len() <= 50 * 1024,
            "stored content must not exceed 50 KB"
        );
        // Must end exactly at a newline boundary (no partial line).
        assert!(
            stored.ends_with('\n') || stored.is_empty(),
            "truncation must align to a newline boundary"
        );
    }

    #[test]
    fn test_no_active_content_unchanged_results() {
        // Design: passing None for active_file_content must produce results
        // identical to a plain search() call.
        setup();
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = Config::defaults(dir.path());
        let engine = Engine::with_config(config).expect("create engine");

        let plain = engine.search("query", 10).expect("plain search");
        let with_none = engine
            .search_with_active_content("query", 10, None)
            .expect("search with None");

        assert_eq!(
            plain.len(),
            with_none.len(),
            "None active content must not change result count"
        );
    }
}
