//! Hybrid search engine with RRF fusion and multi-signal ranking.
//!
//! Combines semantic (vector) search, keyword (FTS5) search, and
//! symbol table lookup into a single ranked result set.
//!
//! ## Search Pipeline
//!
//! 1. **Query Analysis** - Classify the query as symbol-like, keyword, or natural language
//! 2. **Multi-Signal Retrieval** - Execute parallel retrievals
//! 3. **RRF Fusion** - Combine rank lists using Reciprocal Rank Fusion
//! 4. **Boosting** - Apply structural weight, dependency proximity, recency
//! 5. **Context Building** - Assemble token-budget-aware context window
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::doc_link_with_quotes,
    clippy::doc_markdown,
    clippy::if_not_else,
    clippy::inefficient_to_string,
    clippy::manual_let_else,
    clippy::match_same_arms,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::non_canonical_partial_ord_impl,
    clippy::similar_names,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::unused_self
)]

pub mod cache;
pub mod chunk_dedup;
pub mod context_assembler;
pub mod context_formatter;
pub mod feedback;
pub mod hyde;
pub mod intent;
pub mod pack;
pub mod synonyms;

use crate::embedder::Embedder;
use crate::error::OmniResult;
use crate::graph::reasoning::ReasoningEngine;
use crate::index::MetadataIndex;
use crate::reranker::Reranker;
use crate::types::{Chunk, ContextEntry, ContextWindow, ScoreBreakdown, SearchResult};
use crate::vector::VectorIndex;

// Re-export key types for convenience
pub use cache::{CacheKey, CacheStats, TieredCacheStats, TieredQueryCache};
pub use context_assembler::ContextAssembler;
pub use context_formatter::{ContextFormat, ContextFormatter, FormatOptions};
pub use intent::{ContextStrategy, QueryIntent};

/// Hybrid search engine that fuses multiple retrieval signals.
pub struct SearchEngine {
    /// RRF constant k -- controls how much lower ranks contribute.
    /// Higher k = more uniform weighting. Default: 60.
    rrf_k: u32,

    /// Maximum results from each retrieval signal before fusion.
    retrieval_limit: usize,

    /// Token budget for context building.
    token_budget: u32,

    /// LRU cache for query embeddings (query -> embedding vector).
    /// Reduces redundant embedding computation for repeated queries.
    query_cache: std::sync::Arc<std::sync::Mutex<lru::LruCache<String, Vec<f32>>>>,

    /// Tiered result cache (L1 hot 60s / L2 warm 15min).
    /// Caches full search results to achieve sub-30ms P99 on repeated queries.
    result_cache: TieredQueryCache,

    /// Bug-prone file boost factors (file_id → boost factor 1.0–2.0).
    /// Populated from HistoricalGraphEnhancer during indexing.
    /// Files frequently involved in bug fixes get higher relevance for debug queries.
    bug_prone_boosts: std::sync::Arc<parking_lot::Mutex<std::collections::HashMap<i64, f32>>>,

    /// PageRank percentile scores (symbol_id → percentile 0.0–1.0).
    /// Populated from DependencyGraph::compute_pagerank_percentiles after indexing.
    /// Structurally central symbols receive a score lift.
    pagerank_scores: std::sync::Arc<parking_lot::Mutex<std::collections::HashMap<i64, f64>>>,

    /// Temporal freshness scores (file_id → decay factor 0.0–1.0).
    /// Populated from file `indexed_at` timestamps during pipeline init.
    /// Recently modified files receive a relevance boost.
    freshness_scores: std::sync::Arc<parking_lot::Mutex<std::collections::HashMap<i64, f64>>>,

    /// File IDs of files modified on the current branch (vs base branch).
    /// Populated from BranchTracker during pipeline init.
    /// Files changed on the active branch get a relevance boost.
    branch_changed_file_ids: std::sync::Arc<parking_lot::Mutex<std::collections::HashSet<i64>>>,
}

impl SearchEngine {
    /// Create a new search engine with the given configuration.
    #[allow(clippy::missing_panics_doc, clippy::expect_used)]
    pub fn new(rrf_k: u32, token_budget: u32) -> Self {
        // SAFETY: 100 is a non-zero constant, so this unwrap is safe
        let cache_size = std::num::NonZeroUsize::new(100).expect("100 is non-zero");

        Self {
            rrf_k,
            retrieval_limit: 100, // fetch top-100 from each signal
            token_budget,
            query_cache: std::sync::Arc::new(std::sync::Mutex::new(lru::LruCache::new(cache_size))),
            result_cache: TieredQueryCache::new(),
            bug_prone_boosts: std::sync::Arc::new(parking_lot::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pagerank_scores: std::sync::Arc::new(parking_lot::Mutex::new(
                std::collections::HashMap::new(),
            )),
            freshness_scores: std::sync::Arc::new(parking_lot::Mutex::new(
                std::collections::HashMap::new(),
            )),
            branch_changed_file_ids: std::sync::Arc::new(parking_lot::Mutex::new(
                std::collections::HashSet::new(),
            )),
        }
    }

    /// Get a reference to the tiered result cache for external invalidation.
    pub fn result_cache(&self) -> &TieredQueryCache {
        &self.result_cache
    }

    /// Get tiered result cache statistics.
    pub fn result_cache_stats(&self) -> TieredCacheStats {
        self.result_cache.stats()
    }

    /// Update bug-prone file boost factors.
    ///
    /// Called by the pipeline after analyzing git history. Files frequently
    /// involved in bug fixes get a relevance boost for debug/fix queries.
    pub fn set_bug_prone_boosts(&self, boosts: std::collections::HashMap<i64, f32>) {
        let mut guard = self.bug_prone_boosts.lock();
        *guard = boosts;
    }

    /// Update PageRank percentile scores for all symbols.
    ///
    /// Called by the pipeline after dependency graph is built.
    /// Structurally central symbols (high PageRank percentile) receive
    /// a score lift during search: `score *= 1.0 + (0.2 * percentile)`.
    pub fn set_pagerank_scores(&self, scores: std::collections::HashMap<i64, f64>) {
        let mut guard = self.pagerank_scores.lock();
        *guard = scores;
    }

    /// Look up the PageRank percentile for a symbol.
    pub fn pagerank_percentile(&self, symbol_id: i64) -> f64 {
        self.pagerank_scores
            .lock()
            .get(&symbol_id)
            .copied()
            .unwrap_or(0.0)
    }

    /// Update temporal freshness scores for all files.
    ///
    /// Called by the pipeline after computing decay factors from file timestamps.
    /// Recently modified files (freshness close to 1.0) get a relevance boost;
    /// old files (freshness close to 0.0) get no boost.
    pub fn set_freshness_scores(&self, scores: std::collections::HashMap<i64, f64>) {
        let mut guard = self.freshness_scores.lock();
        *guard = scores;
    }

    /// Look up the temporal freshness score for a file.
    /// Returns 0.0–1.0 where 1.0 = just modified, 0.0 = very old.
    pub fn file_freshness(&self, file_id: i64) -> f64 {
        self.freshness_scores
            .lock()
            .get(&file_id)
            .copied()
            .unwrap_or(0.0)
    }

    /// Compute exponential decay freshness scores from ISO 8601 timestamps.
    ///
    /// Uses the formula: `freshness = exp(-decay_rate * age_hours)` where
    /// `decay_rate` controls how quickly scores fall off. With the default
    /// half-life of 168 hours (7 days), a file modified 1 week ago gets ~0.5.
    pub fn compute_freshness_from_timestamps(
        timestamps: &[(i64, String)],
        half_life_hours: f64,
    ) -> std::collections::HashMap<i64, f64> {
        use std::collections::HashMap;

        if timestamps.is_empty() {
            return HashMap::new();
        }

        // decay_rate = ln(2) / half_life
        let decay_rate = std::f64::consts::LN_2 / half_life_hours;

        // Parse "now" as reference point: use the most recent timestamp as reference
        // to avoid depending on system clock during tests
        let mut max_epoch: f64 = 0.0;
        let mut parsed: Vec<(i64, f64)> = Vec::with_capacity(timestamps.len());

        for (file_id, ts_str) in timestamps {
            let epoch = Self::parse_sqlite_datetime(ts_str);
            if epoch <= 0.0 {
                // Malformed or NULL timestamp — skip this file (it gets freshness 0.0 by default)
                tracing::debug!(file_id, timestamp = %ts_str, "skipping file with unparseable timestamp");
                continue;
            }
            if epoch > max_epoch {
                max_epoch = epoch;
            }
            parsed.push((*file_id, epoch));
        }

        let mut result = HashMap::with_capacity(parsed.len());
        for (file_id, epoch) in parsed {
            let age_hours = (max_epoch - epoch) / 3600.0;
            let freshness = (-decay_rate * age_hours).exp();
            result.insert(file_id, freshness.clamp(0.0, 1.0));
        }

        result
    }

    /// Parse a SQLite datetime string (e.g., "2025-01-15 10:30:00") to epoch seconds.
    fn parse_sqlite_datetime(s: &str) -> f64 {
        // Format: "YYYY-MM-DD HH:MM:SS"
        // Simple manual parsing to avoid chrono dependency
        let parts: Vec<&str> = s
            .split(|c| c == '-' || c == ' ' || c == ':' || c == 'T')
            .collect();
        if parts.len() < 6 {
            return 0.0;
        }
        let year: f64 = parts[0].parse().unwrap_or(2020.0);
        let month: f64 = parts[1].parse().unwrap_or(1.0);
        let day: f64 = parts[2].parse().unwrap_or(1.0);
        let hour: f64 = parts[3].parse().unwrap_or(0.0);
        let min: f64 = parts[4].parse().unwrap_or(0.0);
        let sec: f64 = parts[5].parse().unwrap_or(0.0);

        // Approximate epoch: days since 2000-01-01 (good enough for relative ordering)
        let years_since_2000 = year - 2000.0;
        let days = years_since_2000 * 365.25 + (month - 1.0) * 30.44 + (day - 1.0);
        days * 86400.0 + hour * 3600.0 + min * 60.0 + sec
    }

    /// Update the set of file IDs modified on the current branch.
    ///
    /// Called by the pipeline after querying BranchTracker for changed files.
    /// Files on the active branch get a relevance boost during search, since
    /// developers are most likely to need context about files they're actively working on.
    pub fn set_branch_changed_files(&self, file_ids: std::collections::HashSet<i64>) {
        let mut guard = self.branch_changed_file_ids.lock();
        *guard = file_ids;
    }

    /// Check whether a file is part of the current branch diff.
    pub fn is_branch_changed(&self, file_id: i64) -> bool {
        self.branch_changed_file_ids.lock().contains(&file_id)
    }

    /// Execute a hybrid search query.
    ///
    /// Orchestrates multi-signal retrieval and fusion:
    /// 1. Analyze query type
    /// 2. Semantic search (if embedder available)
    /// 3. Keyword search (FTS5 BM25)
    /// 4. Symbol lookup (exact/prefix match)
    /// 5. RRF fusion
    /// 6. Structural weight boost
    /// 7. Token-budget-aware result assembly
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        index: &MetadataIndex,
        vector_index: &VectorIndex,
        embedder: &Embedder,
        dep_graph: Option<&crate::graph::DependencyGraph>,
        reasoning: Option<&ReasoningEngine>,
        reranker: Option<&Reranker>,
        reranker_config: Option<&crate::config::RerankerConfig>,
        open_files: &[std::path::PathBuf],
        sparse_results: &[(i64, f32)],
    ) -> OmniResult<Vec<SearchResult>> {
        // ---- Check tiered result cache ----
        let reranker_active = reranker.is_some_and(|r| r.is_available());
        let graph_available = dep_graph.is_some();
        let reasoning_available = reasoning.is_some();
        // Include the unranked_demotion threshold in the cache key because different
        // threshold values produce different result orderings after reranking.
        let min_rerank = reranker_config.map(|cfg| cfg.unranked_demotion as f32);
        let cache_key = CacheKey::with_context(
            query.to_string(),
            limit,
            min_rerank,
            reranker_active,
            graph_available,
            reasoning_available,
        );
        if let Some(cached) = self.result_cache.get(&cache_key) {
            tracing::debug!(
                query = query,
                results = cached.len(),
                "tiered result cache HIT"
            );
            return Ok(cached);
        }

        let query_type = analyze_query(query);
        let query_intent = QueryIntent::classify(query);
        let limit = limit.min(self.retrieval_limit);

        // Adaptive retrieval limits per signal source.
        // Different query types benefit from different signal depths:
        //   Symbol:  deep symbol + shallow semantic
        //   NL:      deep semantic + shallow keyword (expanded)
        //   Keyword: balanced
        //   Mixed:   balanced with slight symbol boost
        let base = self.retrieval_limit;
        let (kw_limit, sem_limit, sym_limit) = match query_type {
            QueryType::Symbol => (base / 2, base / 3, base),
            QueryType::NaturalLanguage => (base * 2 / 3, base, base / 3),
            QueryType::Keyword => (base, base * 2 / 3, base / 3),
            QueryType::Mixed => (base, base, base * 2 / 3),
        };

        // ---- Query expansion for NL queries ----
        // Extract meaningful tokens for better keyword matching
        let expanded_query =
            if query_type == QueryType::NaturalLanguage || query_type == QueryType::Mixed {
                let mut expanded = expand_query(query);
                // Add code vocabulary synonyms
                let syns = synonyms::expand_with_synonyms(query);
                if !syns.is_empty() {
                    tracing::debug!(synonyms = ?syns, "synonym expansion applied");
                    expanded.push(' ');
                    expanded.push_str(&syns.join(" "));
                }
                expanded
            } else {
                query.to_string()
            };

        // ---- Signal 1: Keyword (FTS5) ----
        let keyword_results = match index.keyword_search(&expanded_query, kw_limit) {
            Ok(results) => results,
            Err(e) => {
                tracing::warn!(error = %e, "keyword search failed");
                // Fallback: try original query if expansion failed
                if expanded_query != query {
                    index.keyword_search(query, kw_limit).unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
        };

        // ---- Signal 2: Semantic (Vector) ----
        let semantic_results = if embedder.is_available() && query_type != QueryType::Symbol {
            // Determine the best text to embed for semantic search.
            // For NL queries, HyDE generates a hypothetical code snippet whose
            // embedding is closer in vector space to relevant code.
            let embed_text = if query_type == QueryType::NaturalLanguage {
                hyde::generate_hypothetical_document(query, query_intent)
                    .unwrap_or_else(|| query.to_string())
            } else {
                query.to_string()
            };

            // Check cache first
            let cache_key = embed_text.clone();
            let cached_embedding = {
                if let Ok(mut cache) = self.query_cache.lock() {
                    cache.get(&cache_key).cloned()
                } else {
                    None
                }
            };

            let query_vec = if let Some(embedding) = cached_embedding {
                embedding
            } else {
                match embedder.embed_query(&embed_text) {
                    Ok(vec) => {
                        // Store in cache
                        if let Ok(mut cache) = self.query_cache.lock() {
                            cache.put(cache_key, vec.clone());
                        }
                        vec
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "query embedding failed");
                        Vec::new()
                    }
                }
            };

            if !query_vec.is_empty() {
                match vector_index.search(&query_vec, sem_limit) {
                    Ok(results) => results,
                    Err(e) => {
                        tracing::warn!(error = %e, "vector search failed");
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // ---- Signal 3: Symbol lookup ----
        let symbol_results = if query_type == QueryType::Symbol || query_type == QueryType::Mixed {
            match index.search_symbols_by_name(query, sym_limit) {
                Ok(symbols) => symbols
                    .into_iter()
                    .filter_map(|s| s.chunk_id)
                    .collect::<Vec<_>>(),
                Err(e) => {
                    tracing::warn!(error = %e, "symbol search failed");
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        // ---- RRF Fusion with query-type-adaptive weights ----
        let mut fused = self.fuse_results(
            query, &keyword_results, &semantic_results, &symbol_results, sparse_results, query_type,
        );

        if let Some(reranker) = reranker {
            if reranker.is_available() && !fused.is_empty() {
                let rr_cfg = reranker_config.cloned().unwrap_or_default();
                let rerank_limit = fused.len().min(rr_cfg.max_candidates);
                let rrf_weight = rr_cfg.rrf_weight;
                let reranker_weight = 1.0 - rrf_weight;
                let unranked_demotion = rr_cfg.unranked_demotion;

                let mut candidates: Vec<(i64, String)> = Vec::with_capacity(rerank_limit);
                for scored in fused.iter().take(rerank_limit) {
                    if let Some(chunk) = self.get_chunk_by_id(index, scored.chunk_id) {
                        candidates.push((scored.chunk_id, chunk.content));
                    }
                }

                if !candidates.is_empty() {
                    let texts: Vec<&str> = candidates.iter().map(|(_, c)| c.as_str()).collect();
                    let scores = reranker.rerank(query, &texts);

                    let mut min_score = f32::INFINITY;
                    let mut max_score = f32::NEG_INFINITY;
                    for score in scores.iter().flatten() {
                        if *score < min_score {
                            min_score = *score;
                        }
                        if *score > max_score {
                            max_score = *score;
                        }
                    }

                    if min_score.is_finite() && max_score.is_finite() {
                        let denom = if max_score > min_score {
                            max_score - min_score
                        } else {
                            1.0
                        };
                        let mut score_map = std::collections::HashMap::new();
                        for ((chunk_id, _), score) in candidates.iter().zip(scores.iter()) {
                            if let Some(score) = score {
                                let norm = (*score - min_score) / denom;
                                score_map.insert(*chunk_id, norm as f64);
                            }
                        }
                        for item in &mut fused {
                            if let Some(&norm) = score_map.get(&item.chunk_id) {
                                item.breakdown.reranker_score = Some(norm);
                                item.final_score =
                                    item.final_score * rrf_weight + norm * reranker_weight;
                            } else {
                                item.final_score *= unranked_demotion;
                            }
                        }
                        fused.sort_by(|a, b| {
                            b.final_score
                                .partial_cmp(&a.final_score)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                }
            }
        }

        // ---- Identify Anchor for Proximity Boosting ----
        // We find the 'best' matched chunk that maps to a symbol
        let anchor_symbol_id = if let Some(_graph) = dep_graph {
            fused.iter()
                .take(3) // Look at top 3 results
                .find_map(|scored| {
                    if let Some(chunk) = self.get_chunk_by_id(index, scored.chunk_id) {
                        index.get_symbol_by_fqn(&chunk.symbol_path).ok().flatten().map(|s| s.id)
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        // ---- Graph-Augmented Retrieval (GAR) ----
        // Walk the semantic reasoning neighborhood from each anchor symbol.
        // Discovered symbols get score boosts and potential injection into results.
        // Uses intent-driven graph_depth rather than a hardcoded constant.
        let mut gar_boosts: std::collections::HashMap<i64, f64> = std::collections::HashMap::new();
        if let (Some(engine), Some(graph)) = (reasoning, dep_graph) {
            let strategy = query_intent.context_strategy();
            let gar_depth = strategy.graph_depth.min(engine.max_hops());

            // Collect up to 3 anchor symbol IDs from top results
            let mut anchors: Vec<i64> = Vec::new();
            for scored in fused.iter().take(5) {
                if let Some(chunk) = self.get_chunk_by_id(index, scored.chunk_id) {
                    if let Ok(Some(sym)) = index.get_symbol_by_fqn(&chunk.symbol_path) {
                        if !anchors.contains(&sym.id) {
                            anchors.push(sym.id);
                            if anchors.len() >= 3 {
                                break;
                            }
                        }
                    }
                }
            }

            for anchor_id in &anchors {
                // Use intent-driven depth with top-20 neighbors per anchor
                if let Ok(hits) = engine.reasoning_neighborhood(graph, *anchor_id, gar_depth, 20) {
                    for hit in &hits {
                        // Map symbol_id → chunk_id for score injection
                        if let Ok(Some(sym)) = index.get_symbol_by_id(hit.symbol_id) {
                            if let Some(chunk_id) = sym.chunk_id {
                                // Use max rather than sum to prevent hub bias:
                                // a chunk reachable from multiple anchors is relevant,
                                // but not N× more relevant than one reachable from one anchor.
                                let entry = gar_boosts.entry(chunk_id).or_insert(0.0);
                                if hit.score > *entry {
                                    *entry = hit.score;
                                }
                            }
                        }
                    }
                }
            }

            if !gar_boosts.is_empty() {
                tracing::debug!(
                    anchors = anchors.len(),
                    gar_hits = gar_boosts.len(),
                    "GAR neighborhood discovered semantic neighbors"
                );

                // Apply GAR boosts to fused results that were already retrieved
                for scored in &mut fused {
                    if let Some(&boost) = gar_boosts.get(&scored.chunk_id) {
                        let capped = boost.min(0.5); // Cap GAR boost at +50%
                        scored.breakdown.dependency_boost = capped;
                        scored.final_score *= 1.0 + capped;
                    }
                }

                // Re-sort fused after GAR boosting
                fused.sort_by(|a, b| {
                    b.final_score
                        .partial_cmp(&a.final_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        // ---- Build final results with structural and graph boosting ----
        // Resolve open_files to file_ids for editor-aware boosting
        let open_file_ids: std::collections::HashSet<i64> = if !open_files.is_empty() {
            open_files
                .iter()
                .filter_map(|path| {
                    let conn = index.connection();
                    conn.query_row(
                        "SELECT id FROM files WHERE path = ?1",
                        rusqlite::params![path.to_string_lossy().as_ref()],
                        |row| row.get::<_, i64>(0),
                    )
                    .ok()
                })
                .collect()
        } else {
            std::collections::HashSet::new()
        };

        let mut results = Vec::new();
        let mut total_tokens: u32 = 0;

        for scored in fused.iter().take(limit * 2) {
            let chunk_id = scored.chunk_id;

            let chunk = match self.get_chunk_by_id(index, chunk_id) {
                Some(c) => c,
                None => continue,
            };

            // Compute Graph Boost
            let mut graph_boost = 1.0;
            if let Some(graph) = dep_graph {
                if !chunk.symbol_path.is_empty() {
                    if let Ok(Some(sym)) = index.get_symbol_by_fqn(&chunk.symbol_path) {
                        // Global Importance (In-degree): Highly depended upon modules get a slight score bump
                        let indegree = graph.downstream(sym.id, 1).map(|v| v.len()).unwrap_or(0);
                        graph_boost += 0.05 * ((indegree.min(20)) as f64);

                        // Local Proximity: If this chunk is closely related to the anchor, give it a big boost
                        if let Some(anchor) = anchor_symbol_id {
                            if sym.id != anchor {
                                // Shortest undirected / directed path surrogate: check both ways
                                let dist_down = graph.distance(anchor, sym.id).ok().flatten();
                                let dist_up = graph.distance(sym.id, anchor).ok().flatten();
                                let dist = match (dist_down, dist_up) {
                                    (Some(d1), Some(d2)) => std::cmp::min(d1, d2),
                                    (Some(d), None) => d,
                                    (None, Some(d)) => d,
                                    (None, None) => usize::MAX,
                                };

                                if dist == 1 {
                                    graph_boost += 0.3; // Very closely related!
                                } else if dist == 2 {
                                    graph_boost += 0.1; // Related
                                }
                            }
                        }
                    }
                }
            }

            // Editor-aware open_files boost: chunks from files currently
            // open in the editor get a 30% relevance boost
            if open_file_ids.contains(&chunk.file_id) {
                graph_boost += 0.3;
            }

            // Bug-proneness boost: files frequently involved in bug fixes get
            // a relevance boost for debug/fix queries. The boost factor (1.0–2.0)
            // is computed from historical git commit analysis and only applied
            // when the query intent suggests bug-fixing activity.
            {
                let bug_boosts = self.bug_prone_boosts.lock();
                if let Some(&boost_factor) = bug_boosts.get(&chunk.file_id) {
                    // Scale boost by intent relevance: debug/edit queries get full boost,
                    // other queries get a reduced version
                    let intent_scale = match query_intent {
                        intent::QueryIntent::Debug | intent::QueryIntent::Edit => 1.0,
                        intent::QueryIntent::Refactor => 0.6,
                        _ => 0.3,
                    };
                    // boost_factor is 1.0-2.0, so excess above 1.0 is the actual boost
                    let bug_boost = (boost_factor - 1.0) * intent_scale as f32;
                    graph_boost += bug_boost as f64;
                }
            }

            // PageRank importance boost: structurally central symbols get a
            // score lift proportional to their PageRank percentile.
            // A symbol at the 95th percentile gets a +0.19 boost (0.2 × 0.95).
            let pagerank_pct = if let Some(_graph) = dep_graph {
                if !chunk.symbol_path.is_empty() {
                    if let Ok(Some(sym)) = index.get_symbol_by_fqn(&chunk.symbol_path) {
                        self.pagerank_percentile(sym.id)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            } else {
                0.0
            };
            if pagerank_pct > 0.0 {
                graph_boost += 0.2 * pagerank_pct;
            }

            // Temporal freshness boost: recently modified files get a relevance
            // lift. A file modified today (freshness=1.0) gets +0.15 boost,
            // one modified a week ago (freshness≈0.5) gets +0.075.
            let freshness = self.file_freshness(chunk.file_id);
            if freshness > 0.0 {
                graph_boost += 0.15 * freshness;
            }

            // Branch-aware boost: files modified on the current branch get a
            // significant relevance boost. Developers querying for context are
            // most likely working on files in their current branch.
            if self.is_branch_changed(chunk.file_id) {
                graph_boost += 0.25;
            }

            // Apply structural + graph boost
            let (boosted_score, struct_weight) =
                Self::apply_structural_boost(scored.final_score, &chunk, graph_boost);

            // Check token budget
            if total_tokens + chunk.token_count > self.token_budget {
                break;
            }
            total_tokens += chunk.token_count;

            // Get file path for the result
            let file_path = self
                .get_file_path_for_chunk(index, &chunk)
                .unwrap_or_default();

            let mut breakdown = scored.breakdown.clone();
            breakdown.structural_weight = struct_weight;
            breakdown.pagerank_boost = pagerank_pct;
            breakdown.recency_boost = freshness;

            results.push(SearchResult {
                chunk,
                file_path,
                score: boosted_score,
                score_breakdown: breakdown,
            });
        }

        // Re-sort after structural boosting (order may have changed)
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // ---- Deduplication: remove overlapping chunks from same file ----
        // If two results cover the same file and their line ranges overlap by >50%,
        // keep only the higher-scored one.
        let mut deduped: Vec<SearchResult> = Vec::with_capacity(results.len());
        for result in results {
            let dominated = deduped.iter().any(|existing| {
                existing.chunk.file_id == result.chunk.file_id
                    && Self::line_overlap_ratio(
                        existing.chunk.line_start,
                        existing.chunk.line_end,
                        result.chunk.line_start,
                        result.chunk.line_end,
                    ) > 0.5
            });
            if !dominated {
                deduped.push(result);
            }
        }

        deduped.truncate(limit);

        // ---- Store in tiered result cache ----
        if !deduped.is_empty() {
            self.result_cache.insert(cache_key, deduped.clone());
            tracing::debug!(
                query = query,
                results = deduped.len(),
                "tiered result cache STORE"
            );
        }

        Ok(deduped)
    }

    /// Execute search and return both results AND the GAR neighbor map.
    ///
    /// The GAR neighbor map (`chunk_id → gar_score`) contains all chunks
    /// discovered during the semantic reasoning neighborhood walk. This
    /// sidecar data can be passed to `assemble_context_window()` to inject
    /// shadow context without redundant graph traversals.
    pub fn search_with_gar(
        &self,
        query: &str,
        limit: usize,
        index: &MetadataIndex,
        vector_index: &VectorIndex,
        embedder: &Embedder,
        dep_graph: Option<&crate::graph::DependencyGraph>,
        reasoning: Option<&ReasoningEngine>,
        reranker: Option<&Reranker>,
        reranker_config: Option<&crate::config::RerankerConfig>,
        open_files: &[std::path::PathBuf],
    ) -> OmniResult<(Vec<SearchResult>, std::collections::HashMap<i64, f64>)> {
        let results = self.search(
            query,
            limit,
            index,
            vector_index,
            embedder,
            dep_graph,
            reasoning,
            reranker,
            reranker_config,
            open_files,
            &[],
        )?;

        // Compute GAR neighbor map for context assembly
        let mut gar_neighbors: std::collections::HashMap<i64, f64> =
            std::collections::HashMap::new();
        if let (Some(engine), Some(graph)) = (reasoning, dep_graph) {
            let gar_intent = QueryIntent::classify(query);
            let strategy = gar_intent.context_strategy();
            let gar_depth = strategy.graph_depth.min(engine.max_hops());

            let mut anchors: Vec<i64> = Vec::new();
            for result in results.iter().take(5) {
                if let Ok(Some(sym)) = index.get_symbol_by_fqn(&result.chunk.symbol_path) {
                    if !anchors.contains(&sym.id) {
                        anchors.push(sym.id);
                        if anchors.len() >= 3 {
                            break;
                        }
                    }
                }
            }

            for anchor_id in &anchors {
                if let Ok(hits) = engine.reasoning_neighborhood(graph, *anchor_id, gar_depth, 20) {
                    for hit in &hits {
                        if let Ok(Some(sym)) = index.get_symbol_by_id(hit.symbol_id) {
                            if let Some(chunk_id) = sym.chunk_id {
                                let entry = gar_neighbors.entry(chunk_id).or_insert(0.0);
                                if hit.score > *entry {
                                    *entry = hit.score;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((results, gar_neighbors))
    }

    /// Fuse multiple rank lists using Reciprocal Rank Fusion (RRF)
    /// with query-type-adaptive weights.
    ///
    /// RRF score = sum( weight_i / (k + rank_i) ) for each signal.
    /// Weight multipliers per query type:
    /// - Symbol:   keyword=0.8, semantic=0.5, symbol=1.5
    /// - Keyword:  keyword=1.0, semantic=1.0, symbol=1.0
    /// - NL:       keyword=0.6, semantic=1.3, symbol=0.8
    /// - Mixed:    keyword=0.9, semantic=1.1, symbol=1.2
    fn fuse_results(
        &self,
        query: &str,
        keyword_results: &[(i64, f64)],  // (chunk_id, bm25_score)
        semantic_results: &[(u64, f32)], // (vector_id, similarity)
        symbol_results: &[i64],          // chunk_ids from symbol match
        sparse_results: &[(i64, f32)],   // (chunk_id, dot-product score) from BGE-M3
        query_type: QueryType,
    ) -> Vec<ScoredChunk> {
        use std::collections::HashMap;

        // Adaptive weights per query type.
        // Base weights differ by structural vs semantic emphasis.
        let (kw_weight, sem_weight, sym_weight) = match query_type {
            QueryType::Symbol => (0.8, 0.5, 1.5),
            QueryType::Keyword => (1.0, 1.0, 1.0),
            QueryType::NaturalLanguage => (0.6, 1.3, 0.8),
            QueryType::Mixed => (0.9, 1.1, 1.2),
        };

        // Sparse weight by query type (0.0 when sparse_results is empty → no effect).
        let sparse_weight: f64 = if sparse_results.is_empty() {
            0.0
        } else {
            match query_type {
                QueryType::Symbol => 0.6,
                QueryType::Keyword => 1.0, // replaces BM25 as primary sparse signal
                QueryType::NaturalLanguage => 0.8,
                QueryType::Mixed => 0.9,
            }
        };

        // Intent-level refinement: adjust weights based on the deeper semantic
        // understanding of what the user is trying to accomplish. This provides
        // finer-grained tuning on top of the base query-type weights.
        let intent = intent::QueryIntent::classify(query);
        let (kw_weight, sem_weight, sym_weight) = match intent {
            // Debug/Edit: need precise symbol-level matches → boost keyword+symbol
            intent::QueryIntent::Debug | intent::QueryIntent::Edit => {
                (kw_weight * 1.2, sem_weight * 0.9, sym_weight * 1.2)
            }
            // Explain/DataFlow: need broad semantic understanding → boost semantic
            intent::QueryIntent::Explain | intent::QueryIntent::DataFlow => {
                (kw_weight * 0.8, sem_weight * 1.3, sym_weight * 0.9)
            }
            // Dependency: need exact symbol resolution → heavy symbol boost
            intent::QueryIntent::Dependency => {
                (kw_weight * 0.7, sem_weight * 0.8, sym_weight * 1.5)
            }
            // Refactor: need both semantic similarity and structural precision
            intent::QueryIntent::Refactor => (kw_weight * 1.0, sem_weight * 1.2, sym_weight * 1.1),
            // TestCoverage: test files use predictable naming → boost keyword
            intent::QueryIntent::TestCoverage => {
                (kw_weight * 1.3, sem_weight * 0.8, sym_weight * 1.0)
            }
            // Generate/Unknown: no adjustment
            _ => (kw_weight, sem_weight, sym_weight),
        };

        let mut scores: HashMap<i64, ScoredChunk> = HashMap::new();

        // Keyword signal
        for (rank, &(chunk_id, _bm25)) in keyword_results.iter().enumerate() {
            let entry = scores.entry(chunk_id).or_insert_with(|| ScoredChunk {
                chunk_id,
                breakdown: ScoreBreakdown::default(),
                final_score: 0.0,
            });
            let rank_score = kw_weight / (f64::from(self.rrf_k) + (rank as f64) + 1.0);
            entry.breakdown.keyword_rank = Some((rank + 1) as u32);
            entry.breakdown.rrf_score += rank_score;
        }

        // Semantic signal
        for (rank, &(vector_id, _sim)) in semantic_results.iter().enumerate() {
            let chunk_id = vector_id as i64; // vector_id maps to chunk_id
            let entry = scores.entry(chunk_id).or_insert_with(|| ScoredChunk {
                chunk_id,
                breakdown: ScoreBreakdown::default(),
                final_score: 0.0,
            });
            let rank_score = sem_weight / (f64::from(self.rrf_k) + (rank as f64) + 1.0);
            entry.breakdown.semantic_rank = Some((rank + 1) as u32);
            entry.breakdown.rrf_score += rank_score;
        }

        // Symbol signal
        for (rank, &chunk_id) in symbol_results.iter().enumerate() {
            let entry = scores.entry(chunk_id).or_insert_with(|| ScoredChunk {
                chunk_id,
                breakdown: ScoreBreakdown::default(),
                final_score: 0.0,
            });
            let rank_score = sym_weight / (f64::from(self.rrf_k) + (rank as f64) + 1.0);
            entry.breakdown.rrf_score += rank_score;
        }

        // Sparse signal (BGE-M3 opt-in, zero-cost when disabled)
        for (rank, &(chunk_id, _dot)) in sparse_results.iter().enumerate() {
            let entry = scores.entry(chunk_id).or_insert_with(|| ScoredChunk {
                chunk_id,
                breakdown: ScoreBreakdown::default(),
                final_score: 0.0,
            });
            let rank_score = sparse_weight / (f64::from(self.rrf_k) + (rank as f64) + 1.0);
            entry.breakdown.sparse_rank = Some((rank + 1) as u32);
            entry.breakdown.rrf_score += rank_score;
        }
        let mut results: Vec<ScoredChunk> = scores.into_values().collect();
        for item in &mut results {
            item.breakdown.structural_weight = 1.0;
            item.final_score = item.breakdown.rrf_score;
        }

        // Sort by final score descending
        results.sort_by(|a, b| {
            b.final_score
                .partial_cmp(&a.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Apply structural and graph boosts once we have the actual chunk data.
    /// Called during result assembly when chunks are fetched from the DB.
    fn apply_structural_boost(score: f64, chunk: &Chunk, graph_boost: f64) -> (f64, f64) {
        let struct_weight = chunk.kind.default_weight() * chunk.visibility.weight_multiplier();
        let boosted = score * (0.4 + 0.6 * struct_weight) * graph_boost;
        (boosted, struct_weight)
    }

    /// Compute the overlap ratio between two line ranges.
    /// Returns intersection / min(len_a, len_b), so 1.0 means one range fully contains the other.
    fn line_overlap_ratio(a_start: u32, a_end: u32, b_start: u32, b_end: u32) -> f64 {
        let overlap_start = a_start.max(b_start);
        let overlap_end = a_end.min(b_end);

        if overlap_start > overlap_end {
            return 0.0;
        }

        let intersection = (overlap_end - overlap_start + 1) as f64;
        let len_a = (a_end - a_start + 1) as f64;
        let len_b = (b_end - b_start + 1) as f64;
        let min_len = len_a.min(len_b);

        if min_len == 0.0 {
            return 0.0;
        }

        intersection / min_len
    }
    /// Get a chunk by its database ID.
    ///
    /// Uses a direct SQL query instead of the chunked file-based lookup.
    fn get_chunk_by_id(&self, index: &MetadataIndex, chunk_id: i64) -> Option<Chunk> {
        let conn = index.connection();
        conn.query_row(
            "SELECT id, file_id, symbol_path, kind, visibility,
                    line_start, line_end, content, doc_comment,
                    token_count, weight, vector_id
             FROM chunks WHERE id = ?1",
            rusqlite::params![chunk_id],
            |row| {
                Ok(Chunk {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    symbol_path: row.get(2)?,
                    kind: parse_kind(&row.get::<_, String>(3)?),
                    visibility: parse_vis(&row.get::<_, String>(4)?),
                    line_start: row.get(5)?,
                    line_end: row.get(6)?,
                    content: row.get(7)?,
                    doc_comment: row.get(8)?,
                    token_count: row.get(9)?,
                    weight: row.get(10)?,
                    vector_id: row.get::<_, Option<i64>>(11)?.map(|v| v as u64),
                    is_summary: false,
                    content_hash: 0, // not needed for search results
                })
            },
        )
        .ok()
    }

    /// Get the file path for a chunk's parent file.
    fn get_file_path_for_chunk(
        &self,
        index: &MetadataIndex,
        chunk: &Chunk,
    ) -> Option<std::path::PathBuf> {
        let conn = index.connection();
        conn.query_row(
            "SELECT path FROM files WHERE id = ?1",
            rusqlite::params![chunk.file_id],
            |row| {
                let path: String = row.get(0)?;
                Ok(std::path::PathBuf::from(path))
            },
        )
        .ok()
    }

    /// Compute RRF score from rank positions.
    pub fn rrf_score(&self, semantic_rank: Option<u32>, keyword_rank: Option<u32>) -> f64 {
        let k = f64::from(self.rrf_k);
        let semantic = semantic_rank.map_or(0.0, |r| 1.0 / (k + f64::from(r)));
        let keyword = keyword_rank.map_or(0.0, |r| 1.0 / (k + f64::from(r)));
        semantic + keyword
    }

    /// Assemble a token-budget-aware context window from search results.
    ///
    /// Intelligent context assembly engine. Instead of blindly
    /// concatenating search results, it:
    /// 1. Groups results by file
    /// 2. For files with 3+ matching chunks, includes ALL chunks from that file
    /// 3. Injects GAR shadow context neighbors (pre-computed from search())
    /// 4. Falls back to 1-hop graph neighbors when no GAR data is available
    /// 5. Packs greedily by score until token budget is hit
    ///
    /// Returns a structured context window with file grouping.
    pub fn assemble_context_window(
        &self,
        search_results: &[SearchResult],
        index: &MetadataIndex,
        dep_graph: Option<&crate::graph::DependencyGraph>,
        gar_neighbors: &std::collections::HashMap<i64, f64>,
        token_budget: u32,
        file_dep_graph: Option<&crate::graph::dependencies::FileDependencyGraph>,
    ) -> ContextWindow {
        use std::cmp::Ordering;
        use std::collections::{BinaryHeap, HashMap, HashSet};

        // Priority queue entry
        #[derive(Debug)]
        struct ScoredEntry {
            score: f64,
            chunk: Chunk,
            file_path: std::path::PathBuf,
            is_neighbor: bool,
        }

        impl PartialEq for ScoredEntry {
            fn eq(&self, other: &Self) -> bool {
                self.score == other.score
            }
        }
        impl Eq for ScoredEntry {}
        impl PartialOrd for ScoredEntry {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                self.score.partial_cmp(&other.score)
            }
        }
        impl Ord for ScoredEntry {
            fn cmp(&self, other: &Self) -> Ordering {
                self.partial_cmp(other).unwrap_or(Ordering::Equal)
            }
        }

        let mut heap: BinaryHeap<ScoredEntry> = BinaryHeap::new();
        let mut seen_chunk_ids: HashSet<i64> = HashSet::new();

        // Step 1: Group search results by file
        let mut file_groups: HashMap<i64, Vec<&SearchResult>> = HashMap::new();
        for result in search_results {
            file_groups
                .entry(result.chunk.file_id)
                .or_default()
                .push(result);
        }

        // Step 2: For files with 3+ matches, include ALL chunks from that file
        for (&file_id, results) in &file_groups {
            if results.len() >= 3 {
                // This file is highly relevant -- include all its chunks
                if let Ok(all_chunks) = index.get_chunks_for_file(file_id) {
                    let file_path = results[0].file_path.clone();
                    let avg_score =
                        results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64;
                    for chunk in all_chunks {
                        if !seen_chunk_ids.contains(&chunk.id) {
                            seen_chunk_ids.insert(chunk.id);
                            heap.push(ScoredEntry {
                                score: avg_score * 0.9, // slight discount for non-matched chunks
                                chunk,
                                file_path: file_path.clone(),
                                is_neighbor: false,
                            });
                        }
                    }
                }
            } else {
                // Include only the matched chunks
                for result in results {
                    if !seen_chunk_ids.contains(&result.chunk.id) {
                        seen_chunk_ids.insert(result.chunk.id);
                        heap.push(ScoredEntry {
                            score: result.score,
                            chunk: result.chunk.clone(),
                            file_path: result.file_path.clone(),
                            is_neighbor: false,
                        });
                    }
                }
            }
        }

        // Step 3: Inject GAR shadow context neighbors
        //
        // Use pre-computed GAR neighbor chunk_ids from search() to avoid
        // redundant graph walks. Falls back to simple 1-hop structural
        // neighbors when no GAR data is available.
        if !gar_neighbors.is_empty() {
            // GAR path: inject pre-computed semantic neighbors
            let base_score = search_results.first().map(|r| r.score).unwrap_or(1.0);
            for (&chunk_id, &gar_score) in gar_neighbors {
                if !seen_chunk_ids.contains(&chunk_id) {
                    if let Some(chunk) = self.get_chunk_by_id(index, chunk_id) {
                        let fp = self
                            .get_file_path_for_chunk(index, &chunk)
                            .unwrap_or_default();
                        seen_chunk_ids.insert(chunk_id);
                        // Score: top result score * GAR relevance (capped at 0.6)
                        heap.push(ScoredEntry {
                            score: base_score * 0.5 * gar_score.min(1.0),
                            chunk,
                            file_path: fp,
                            is_neighbor: true,
                        });
                    }
                }
            }
        } else if let Some(graph) = dep_graph {
            // Fallback: simple 1-hop structural neighbors (no GAR data)
            for result in search_results.iter().take(3) {
                if result.chunk.symbol_path.is_empty() {
                    continue;
                }
                if let Ok(Some(sym)) = index.get_symbol_by_fqn(&result.chunk.symbol_path) {
                    // Get upstream dependencies (what this symbol depends on)
                    if let Ok(upstream) = graph.upstream(sym.id, 1) {
                        for dep_id in upstream {
                            if let Ok(Some(dep_sym)) = index.get_symbol_by_id(dep_id) {
                                if let Some(chunk_id) = dep_sym.chunk_id {
                                    if !seen_chunk_ids.contains(&chunk_id) {
                                        if let Some(chunk) = self.get_chunk_by_id(index, chunk_id) {
                                            let fp = self
                                                .get_file_path_for_chunk(index, &chunk)
                                                .unwrap_or_default();
                                            seen_chunk_ids.insert(chunk_id);
                                            heap.push(ScoredEntry {
                                                score: result.score * 0.5,
                                                chunk,
                                                file_path: fp,
                                                is_neighbor: true,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Get downstream dependencies (what depends on this)
                    if let Ok(downstream) = graph.downstream(sym.id, 1) {
                        for dep_id in downstream {
                            if let Ok(Some(dep_sym)) = index.get_symbol_by_id(dep_id) {
                                if let Some(chunk_id) = dep_sym.chunk_id {
                                    if !seen_chunk_ids.contains(&chunk_id) {
                                        if let Some(chunk) = self.get_chunk_by_id(index, chunk_id) {
                                            let fp = self
                                                .get_file_path_for_chunk(index, &chunk)
                                                .unwrap_or_default();
                                            seen_chunk_ids.insert(chunk_id);
                                            heap.push(ScoredEntry {
                                                score: result.score * 0.4,
                                                chunk,
                                                file_path: fp,
                                                is_neighbor: true,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Step 4: Convert heap to prioritized entries and pack with knapsack DP
        //
        // The ContextAssembler uses 0/1 knapsack dynamic programming to find the
        // mathematically optimal subset within the token budget, weighted by both
        // score and priority. Falls back to greedy for very large inputs.
        let query_for_intent = search_results
            .first()
            .map(|_| "") // We don't have the original query here; use Unknown intent
            .unwrap_or("");
        let intent = crate::search::intent::QueryIntent::classify(query_for_intent);
        let strategy = intent.context_strategy();

        // Drain heap into ContextEntry list with assigned priorities
        let mut candidate_entries: Vec<ContextEntry> = Vec::new();
        while let Some(entry) = heap.pop() {
            let is_test = matches!(entry.chunk.kind, crate::types::ChunkKind::Test);
            let priority = crate::types::ChunkPriority::from_score_and_context(
                entry.score, false, // no active file info available here
                is_test, entry.is_neighbor,
            );
            candidate_entries.push(ContextEntry {
                file_path: entry.file_path,
                chunk: entry.chunk,
                score: entry.score,
                is_graph_neighbor: entry.is_neighbor,
                priority: Some(priority),
                shadow_header: None,
            });
        }

        // Sort by priority (highest first), then by score
        candidate_entries.sort_by(|a, b| {
            let a_p = a.priority.unwrap_or(crate::types::ChunkPriority::Low);
            let b_p = b.priority.unwrap_or(crate::types::ChunkPriority::Low);
            b_p.cmp(&a_p).then_with(|| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        // Delegate to ContextAssembler's knapsack DP packer, then apply causal ordering
        let assembler = context_assembler::ContextAssembler::new(token_budget);
        assembler.pack_entries_with_strategy(candidate_entries, &strategy, token_budget, file_dep_graph)
    }
}

// ---------------------------------------------------------------------------
// Query analysis
// ---------------------------------------------------------------------------

/// Classification of a search query for routing to appropriate signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryType {
    /// Looks like a symbol name (e.g., "authenticate_user", "Config.new")
    Symbol,
    /// Short keyword search (1-2 words, no natural language structure)
    Keyword,
    /// Natural language question ("how does authentication work?")
    NaturalLanguage,
    /// Mixed -- could be either (e.g., "user authentication function")
    Mixed,
}

/// Analyze a query string to determine the best search strategy.
fn analyze_query(query: &str) -> QueryType {
    let trimmed = query.trim();

    if trimmed.is_empty() {
        return QueryType::Keyword;
    }

    // Symbol-like: contains :: or . separators, or is camelCase/snake_case without spaces
    if !trimmed.contains(' ') {
        if trimmed.contains("::") || trimmed.contains('.') || trimmed.contains("__") {
            return QueryType::Symbol;
        }
        // Single word -- check if it looks like an identifier
        if trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return QueryType::Symbol;
        }
    }

    // Natural language: starts with question words, ends with ?, or has many words
    let lower = trimmed.to_lowercase();
    let words: Vec<&str> = trimmed.split_whitespace().collect();

    if lower.ends_with('?')
        || lower.starts_with("how ")
        || lower.starts_with("what ")
        || lower.starts_with("where ")
        || lower.starts_with("why ")
        || lower.starts_with("when ")
        || lower.starts_with("which ")
        || lower.starts_with("find ")
        || lower.starts_with("show ")
    {
        return QueryType::NaturalLanguage;
    }

    // Short queries (1-3 words) that aren't questions are mixed
    if words.len() <= 3 {
        return QueryType::Mixed;
    }

    // Longer queries are likely natural language
    QueryType::NaturalLanguage
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// Intermediate scored chunk during fusion.
struct ScoredChunk {
    chunk_id: i64,
    breakdown: ScoreBreakdown,
    final_score: f64,
}

// ---------------------------------------------------------------------------
// Query expansion
// ---------------------------------------------------------------------------

/// Stop words to strip from natural language queries for FTS5.
const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
    "do", "does", "did", "will", "would", "shall", "should", "may", "might", "can", "could",
    "must", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into", "through",
    "during", "before", "after", "above", "below", "and", "but", "or", "not", "no", "if", "then",
    "than", "that", "this", "these", "those", "it", "its", "i", "me", "my", "we", "our", "you",
    "your", "he", "she", "they", "them", "their", "what", "which", "who", "whom", "how", "when",
    "where", "why", "all", "each", "every", "both", "few", "more", "most", "other", "some", "such",
    "only", "own", "same", "so", "very", "just", "about", "there", "here", "find", "show", "get",
    "list", "explain", "describe",
];

/// Expand a natural language query into better FTS5 tokens.
///
/// Strips stop words, splits code identifiers (snake_case, CamelCase,
/// dot.paths, colon::paths), and preserves content-bearing tokens
/// that are more likely to match code identifiers and documentation.
fn expand_query(query: &str) -> String {
    let tokens: Vec<&str> = query
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '_'))
        .filter(|w| !w.is_empty())
        .filter(|w| !STOP_WORDS.contains(&w.to_lowercase().as_str()))
        .collect();

    if tokens.is_empty() {
        // Fallback: return original query to avoid empty FTS5 query
        return query.to_string();
    }

    // Split code identifiers into constituent words
    let mut expanded: Vec<String> = Vec::new();
    for token in &tokens {
        // Always include the original token
        expanded.push(token.to_string());

        // Split on code delimiters and CamelCase
        let sub_tokens = split_code_token(token);
        for sub in sub_tokens {
            let lower = sub.to_lowercase();
            if lower.len() >= 2
                && !STOP_WORDS.contains(&lower.as_str())
                && !expanded.contains(&lower)
            {
                expanded.push(lower);
            }
        }
    }

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    expanded.retain(|t| seen.insert(t.to_lowercase()));

    // Join with OR for broader FTS5 matching
    expanded.join(" OR ")
}

/// Split a code token into constituent sub-words.
///
/// Handles:
/// - `snake_case` -> ["snake", "case"]
/// - `CamelCase` -> ["Camel", "Case"]
/// - `HTTPServer` -> ["HTTP", "Server"]
/// - `module::path` -> ["module", "path"]
/// - `package.Class` -> ["package", "Class"]
/// - `kebab-case-name` -> ["kebab", "case", "name"]
fn split_code_token(token: &str) -> Vec<&str> {
    let mut parts = Vec::new();

    // First split on clear separators: _ . :: - /
    for segment in token.split(['_', '.', ':', '-', '/']) {
        if segment.is_empty() {
            continue;
        }

        // Then split CamelCase within each segment
        let bytes = segment.as_bytes();
        let mut start = 0;
        for i in 1..bytes.len() {
            let cur = bytes[i] as char;
            let prev = bytes[i - 1] as char;

            // Split on case transitions: lowercase->UPPERCASE or UPPERCASE->UPPERCASE+lowercase
            let boundary = (prev.is_lowercase() && cur.is_uppercase())
                || (i + 1 < bytes.len()
                    && prev.is_uppercase()
                    && cur.is_uppercase()
                    && (bytes[i + 1] as char).is_lowercase());

            if boundary {
                let part = &segment[start..i];
                if !part.is_empty() {
                    parts.push(part);
                }
                start = i;
            }
        }
        let tail = &segment[start..];
        if !tail.is_empty() {
            parts.push(tail);
        }
    }

    parts
}

// ---------------------------------------------------------------------------
// Parse helpers (delegates to centralized methods on types)
// ---------------------------------------------------------------------------

fn parse_kind(s: &str) -> crate::types::ChunkKind {
    crate::types::ChunkKind::from_str_lossy(s)
}

fn parse_vis(s: &str) -> crate::types::Visibility {
    crate::types::Visibility::from_str_lossy(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_score_both_signals() {
        let engine = SearchEngine::new(60, 4000);
        let score = engine.rrf_score(Some(1), Some(1));
        // 1/(60+1) + 1/(60+1) = 2/61
        let expected = 2.0 / 61.0;
        assert!((score - expected).abs() < 1e-10);
    }

    #[test]
    fn test_rrf_score_semantic_only() {
        let engine = SearchEngine::new(60, 4000);
        let score = engine.rrf_score(Some(1), None);
        let expected = 1.0 / 61.0;
        assert!((score - expected).abs() < 1e-10);
    }

    #[test]
    fn test_rrf_score_no_signal() {
        let engine = SearchEngine::new(60, 4000);
        let score = engine.rrf_score(None, None);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_rrf_higher_rank_gets_higher_score() {
        let engine = SearchEngine::new(60, 4000);
        let score_rank1 = engine.rrf_score(Some(1), Some(1));
        let score_rank10 = engine.rrf_score(Some(10), Some(10));
        assert!(score_rank1 > score_rank10);
    }

    #[test]
    fn test_analyze_query_symbol() {
        assert_eq!(analyze_query("Config::new"), QueryType::Symbol);
        assert_eq!(analyze_query("user_service.get_user"), QueryType::Symbol);
        assert_eq!(analyze_query("authenticate"), QueryType::Symbol);
        assert_eq!(analyze_query("__init__"), QueryType::Symbol);
    }

    #[test]
    fn test_analyze_query_natural_language() {
        assert_eq!(
            analyze_query("how does authentication work?"),
            QueryType::NaturalLanguage
        );
        assert_eq!(
            analyze_query("what is the user service"),
            QueryType::NaturalLanguage
        );
        assert_eq!(
            analyze_query("find all database queries"),
            QueryType::NaturalLanguage
        );
        assert_eq!(
            analyze_query("where is session management implemented"),
            QueryType::NaturalLanguage
        );
    }

    #[test]
    fn test_analyze_query_mixed() {
        assert_eq!(analyze_query("user authentication"), QueryType::Mixed);
        assert_eq!(analyze_query("database connection pool"), QueryType::Mixed);
    }

    #[test]
    fn test_analyze_query_empty() {
        assert_eq!(analyze_query(""), QueryType::Keyword);
        assert_eq!(analyze_query("  "), QueryType::Keyword);
    }

    #[test]
    fn test_fuse_results_both_signals() {
        let engine = SearchEngine::new(60, 4000);

        let keyword = vec![(1, -0.5), (2, -0.3), (3, -0.1)]; // chunk_id, bm25
        let semantic = vec![(2, 0.9), (1, 0.8), (4, 0.7)]; // vector_id, similarity

        let fused = engine.fuse_results(
            "test query",
            &keyword,
            &semantic,
            &[],
            &[],
            QueryType::Keyword,
        );

        assert!(!fused.is_empty());

        // Chunk 2 appears in both signals (rank 2 keyword, rank 1 semantic)
        // Chunk 1 appears in both (rank 1 keyword, rank 2 semantic)
        // Both should score higher than single-signal results
        let chunk2 = fused.iter().find(|s| s.chunk_id == 2);
        let chunk3 = fused.iter().find(|s| s.chunk_id == 3);
        assert!(chunk2.is_some());
        assert!(chunk3.is_some());
        assert!(
            chunk2.expect("chunk2").final_score > chunk3.expect("chunk3").final_score,
            "dual-signal should outrank single-signal"
        );
    }

    #[test]
    fn test_fuse_results_empty() {
        let engine = SearchEngine::new(60, 4000);
        let fused = engine.fuse_results("", &[], &[], &[], &[], QueryType::Keyword);
        assert!(fused.is_empty());
    }

    #[test]
    fn test_fuse_results_symbol_boost() {
        let engine = SearchEngine::new(60, 4000);

        let keyword = vec![(1, -0.5), (2, -0.3)];
        let symbol = vec![2_i64]; // chunk_id 2 is an exact symbol match

        let fused = engine.fuse_results("Config", &keyword, &[], &symbol, &[], QueryType::Symbol);

        let chunk2 = fused.iter().find(|s| s.chunk_id == 2);
        let chunk1 = fused.iter().find(|s| s.chunk_id == 1);
        assert!(
            chunk2.expect("chunk2").final_score > chunk1.expect("chunk1").final_score,
            "symbol match + keyword should outrank keyword alone"
        );
    }

    #[test]
    fn test_search_engine_creation() {
        let engine = SearchEngine::new(60, 4000);
        assert_eq!(engine.rrf_k, 60);
        assert_eq!(engine.token_budget, 4000);
    }

    // -- Code token splitting tests --

    #[test]
    fn test_split_snake_case() {
        let parts = split_code_token("process_file_hash");
        assert_eq!(parts, vec!["process", "file", "hash"]);
    }

    #[test]
    fn test_split_camel_case() {
        let parts = split_code_token("SearchEngine");
        assert_eq!(parts, vec!["Search", "Engine"]);
    }

    #[test]
    fn test_split_http_server() {
        let parts = split_code_token("HTTPServer");
        assert_eq!(parts, vec!["HTTP", "Server"]);
    }

    #[test]
    fn test_split_module_path() {
        let parts = split_code_token("crate::parser::mod");
        assert_eq!(parts, vec!["crate", "parser", "mod"]);
    }

    #[test]
    fn test_split_dot_path() {
        let parts = split_code_token("package.ClassName.method");
        assert_eq!(parts, vec!["package", "Class", "Name", "method"]);
    }

    #[test]
    fn test_split_single_word() {
        let parts = split_code_token("authenticate");
        assert_eq!(parts, vec!["authenticate"]);
    }

    // -- Line overlap ratio tests --

    #[test]
    fn test_overlap_full_containment() {
        let ratio = SearchEngine::line_overlap_ratio(10, 20, 12, 18);
        assert!((ratio - 1.0).abs() < 1e-6, "inner range fully contained");
    }

    #[test]
    fn test_overlap_no_overlap() {
        let ratio = SearchEngine::line_overlap_ratio(1, 10, 20, 30);
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn test_overlap_partial() {
        let ratio = SearchEngine::line_overlap_ratio(1, 10, 5, 15);
        // intersection = 5..10 = 6 lines. min(10, 11) = 10. 6/10 = 0.6
        assert!(ratio > 0.5);
    }

    // -- Expand query with code splitting tests --

    #[test]
    fn test_expand_query_splits_identifier() {
        let expanded = expand_query("processFileHash");
        assert!(expanded.contains("process"), "should contain sub-word");
        assert!(expanded.contains("File"), "should contain sub-word");
    }

    #[test]
    fn test_expand_query_preserves_original() {
        let expanded = expand_query("processFileHash");
        assert!(expanded.contains("processFileHash"), "should keep original");
    }

    // -----------------------------------------------------------------------
    // Temporal freshness scoring tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_freshness_empty() {
        let scores = SearchEngine::compute_freshness_from_timestamps(&[], 168.0);
        assert!(scores.is_empty());
    }

    #[test]
    fn test_freshness_single_file_gets_1() {
        let timestamps = vec![(1, "2025-06-15 10:00:00".to_string())];
        let scores = SearchEngine::compute_freshness_from_timestamps(&timestamps, 168.0);
        assert_eq!(scores.len(), 1);
        // Single file is the "newest" → age = 0 → freshness = 1.0
        assert!((scores[&1] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_freshness_newer_file_scores_higher() {
        let timestamps = vec![
            (1, "2025-06-01 10:00:00".to_string()), // older
            (2, "2025-06-15 10:00:00".to_string()), // newer
        ];
        let scores = SearchEngine::compute_freshness_from_timestamps(&timestamps, 168.0);
        assert!(
            scores[&2] > scores[&1],
            "newer file ({:.3}) should score higher than older file ({:.3})",
            scores[&2],
            scores[&1]
        );
        // The newest file should be 1.0
        assert!((scores[&2] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_freshness_half_life_decay() {
        // Two files: one is exactly half_life_hours apart from the other
        // half_life = 168 hours = 7 days. 7 days ≈ 7*24 = 168 hours
        let timestamps = vec![
            (1, "2025-06-08 10:00:00".to_string()), // 7 days before
            (2, "2025-06-15 10:00:00".to_string()), // reference point
        ];
        let scores = SearchEngine::compute_freshness_from_timestamps(&timestamps, 168.0);
        // File 1 is ~7 days old → freshness ≈ 0.5
        assert!(
            (scores[&1] - 0.5).abs() < 0.1,
            "file 7 days old should have freshness ~0.5, got {:.3}",
            scores[&1]
        );
    }

    #[test]
    fn test_freshness_all_same_timestamp() {
        let timestamps = vec![
            (1, "2025-06-15 10:00:00".to_string()),
            (2, "2025-06-15 10:00:00".to_string()),
            (3, "2025-06-15 10:00:00".to_string()),
        ];
        let scores = SearchEngine::compute_freshness_from_timestamps(&timestamps, 168.0);
        // All same age → all should be 1.0
        for &id in &[1i64, 2, 3] {
            assert!(
                (scores[&id] - 1.0).abs() < 0.01,
                "all same timestamp should give 1.0, got {:.3}",
                scores[&id]
            );
        }
    }

    #[test]
    fn test_freshness_scores_range() {
        let timestamps = vec![
            (1, "2020-01-01 00:00:00".to_string()), // very old
            (2, "2025-06-15 10:00:00".to_string()), // recent
        ];
        let scores = SearchEngine::compute_freshness_from_timestamps(&timestamps, 168.0);
        // All scores should be in [0, 1]
        for &s in scores.values() {
            assert!((0.0..=1.0).contains(&s), "score {s} out of [0,1] range");
        }
        // Very old file should be close to 0
        assert!(scores[&1] < 0.01, "5-year-old file should be ~0.0");
    }

    #[test]
    fn test_parse_sqlite_datetime() {
        let epoch = SearchEngine::parse_sqlite_datetime("2025-06-15 10:30:45");
        assert!(epoch > 0.0, "should parse to positive epoch");

        let epoch2 = SearchEngine::parse_sqlite_datetime("2025-06-15 11:30:45");
        // One hour later should be ~3600 seconds more
        assert!((epoch2 - epoch - 3600.0).abs() < 1.0);
    }

    #[test]
    fn test_freshness_set_and_get() {
        let engine = SearchEngine::new(60, 4000);
        assert_eq!(engine.file_freshness(1), 0.0);

        let mut scores = std::collections::HashMap::new();
        scores.insert(1, 0.75);
        scores.insert(2, 0.3);
        engine.set_freshness_scores(scores);

        assert!((engine.file_freshness(1) - 0.75).abs() < 0.001);
        assert!((engine.file_freshness(2) - 0.3).abs() < 0.001);
        assert_eq!(engine.file_freshness(999), 0.0); // unknown file
    }

    // -----------------------------------------------------------------------
    // Branch-aware retrieval boost tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_branch_changed_empty_by_default() {
        let engine = SearchEngine::new(60, 4000);
        assert!(!engine.is_branch_changed(1));
        assert!(!engine.is_branch_changed(999));
    }

    #[test]
    fn test_branch_changed_set_and_check() {
        let engine = SearchEngine::new(60, 4000);

        let mut ids = std::collections::HashSet::new();
        ids.insert(10);
        ids.insert(20);
        ids.insert(30);
        engine.set_branch_changed_files(ids);

        assert!(engine.is_branch_changed(10));
        assert!(engine.is_branch_changed(20));
        assert!(engine.is_branch_changed(30));
        assert!(!engine.is_branch_changed(1));
        assert!(!engine.is_branch_changed(999));
    }

    #[test]
    fn test_branch_changed_can_be_updated() {
        let engine = SearchEngine::new(60, 4000);

        let mut ids = std::collections::HashSet::new();
        ids.insert(1);
        engine.set_branch_changed_files(ids);
        assert!(engine.is_branch_changed(1));

        // Replace with different set
        let mut ids2 = std::collections::HashSet::new();
        ids2.insert(2);
        engine.set_branch_changed_files(ids2);
        assert!(!engine.is_branch_changed(1)); // old file no longer marked
        assert!(engine.is_branch_changed(2)); // new file is marked
    }
}
