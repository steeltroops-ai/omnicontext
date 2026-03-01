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
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::unused_self
)]

use crate::embedder::Embedder;
use crate::index::MetadataIndex;
use crate::types::{Chunk, SearchResult, ScoreBreakdown, ContextWindow, ContextEntry};
use crate::reranker::Reranker;
use crate::vector::VectorIndex;
use crate::error::OmniResult;

/// Hybrid search engine that fuses multiple retrieval signals.
pub struct SearchEngine {
    /// RRF constant k -- controls how much lower ranks contribute.
    /// Higher k = more uniform weighting. Default: 60.
    rrf_k: u32,

    /// Maximum results from each retrieval signal before fusion.
    retrieval_limit: usize,

    /// Token budget for context building.
    token_budget: u32,
}

impl SearchEngine {
    /// Create a new search engine with the given configuration.
    pub fn new(rrf_k: u32, token_budget: u32) -> Self {
        Self {
            rrf_k,
            retrieval_limit: 100, // fetch top-100 from each signal
            token_budget,
        }
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
        reranker: Option<&Reranker>,
        reranker_config: Option<&crate::config::RerankerConfig>,
    ) -> OmniResult<Vec<SearchResult>> {
        let query_type = analyze_query(query);
        let limit = limit.min(self.retrieval_limit);

        // ---- Query expansion for NL queries ----
        // Extract meaningful tokens for better keyword matching
        let expanded_query = if query_type == QueryType::NaturalLanguage {
            expand_query(query)
        } else {
            query.to_string()
        };

        // ---- Signal 1: Keyword (FTS5) ----
        let keyword_results = match index.keyword_search(&expanded_query, self.retrieval_limit) {
            Ok(results) => results,
            Err(e) => {
                tracing::warn!(error = %e, "keyword search failed");
                // Fallback: try original query if expansion failed
                if expanded_query != query {
                    index.keyword_search(query, self.retrieval_limit).unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
        };

        // ---- Signal 2: Semantic (Vector) ----
        let semantic_results = if embedder.is_available() && query_type != QueryType::Symbol {
            match embedder.embed_single(query) {
                Ok(query_vec) => {
                    match vector_index.search(&query_vec, self.retrieval_limit) {
                        Ok(results) => results,
                        Err(e) => {
                            tracing::warn!(error = %e, "vector search failed");
                            Vec::new()
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "query embedding failed");
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        // ---- Signal 3: Symbol lookup ----
        let symbol_results = if query_type == QueryType::Symbol || query_type == QueryType::Mixed {
            match index.search_symbols_by_name(query, self.retrieval_limit) {
                Ok(symbols) => symbols.into_iter()
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

        // ---- RRF Fusion ----
        let mut fused = self.fuse_results(
            &keyword_results,
            &semantic_results,
            &symbol_results,
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
                                item.final_score = item.final_score * rrf_weight + norm * reranker_weight;
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

        // ---- Build final results with structural and graph boosting ----
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

            // Apply structural + graph boost
            let (boosted_score, struct_weight) =
                Self::apply_structural_boost(scored.final_score, &chunk, graph_boost);

            // Check token budget
            if total_tokens + chunk.token_count > self.token_budget {
                break;
            }
            total_tokens += chunk.token_count;

            // Get file path for the result
            let file_path = self.get_file_path_for_chunk(index, &chunk)
                .unwrap_or_default();

            let mut breakdown = scored.breakdown.clone();
            breakdown.structural_weight = struct_weight;

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

        Ok(deduped)
    }

    /// Fuse multiple rank lists using Reciprocal Rank Fusion (RRF).
    ///
    /// RRF score = sum( 1 / (k + rank_i) ) for each signal where the item appears.
    /// This is a principled rank fusion method from Cormack et al. (2009).
    fn fuse_results(
        &self,
        keyword_results: &[(i64, f64)],  // (chunk_id, bm25_score)
        semantic_results: &[(u64, f32)],  // (vector_id, similarity)
        symbol_results: &[i64],           // chunk_ids from symbol match
    ) -> Vec<ScoredChunk> {
        use std::collections::HashMap;

        let mut scores: HashMap<i64, ScoredChunk> = HashMap::new();

        // Keyword signal
        for (rank, &(chunk_id, _bm25)) in keyword_results.iter().enumerate() {
            let entry = scores.entry(chunk_id).or_insert_with(|| ScoredChunk {
                chunk_id,
                breakdown: ScoreBreakdown::default(),
                final_score: 0.0,
            });
            let rank_score = 1.0 / (f64::from(self.rrf_k) + (rank as f64) + 1.0);
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
            let rank_score = 1.0 / (f64::from(self.rrf_k) + (rank as f64) + 1.0);
            entry.breakdown.semantic_rank = Some((rank + 1) as u32);
            entry.breakdown.rrf_score += rank_score;
        }

        // Symbol signal (treated as a strong boost)
        for (rank, &chunk_id) in symbol_results.iter().enumerate() {
            let entry = scores.entry(chunk_id).or_insert_with(|| ScoredChunk {
                chunk_id,
                breakdown: ScoreBreakdown::default(),
                final_score: 0.0,
            });
            // Symbol matches get a higher weight than positional RRF
            let rank_score = 1.5 / (f64::from(self.rrf_k) + (rank as f64) + 1.0);
            entry.breakdown.rrf_score += rank_score;
        }

        // Compute final scores with structural weight boost
        let mut results: Vec<ScoredChunk> = scores.into_values().collect();
        for item in &mut results {
            // Apply structural weight from chunk kind
            // Chunks of more important structural types get boosted
            item.breakdown.structural_weight = 1.0; // will be refined when chunk is fetched
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
        let struct_weight = chunk.kind.default_weight()
            * chunk.visibility.weight_multiplier();
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
                })
            },
        ).ok()
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
        ).ok()
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
    /// This is the Phase 3 context assembly engine. Instead of blindly
    /// concatenating search results, it:
    /// 1. Groups results by file
    /// 2. For files with 3+ matching chunks, includes ALL chunks from that file
    /// 3. Fetches 1-hop graph neighbors of anchor symbols
    /// 4. Packs greedily by score until token budget is hit
    ///
    /// Returns a structured context window with file grouping.
    pub fn assemble_context_window(
        &self,
        search_results: &[SearchResult],
        index: &MetadataIndex,
        dep_graph: Option<&crate::graph::DependencyGraph>,
        token_budget: u32,
    ) -> ContextWindow {
        use std::collections::{HashMap, HashSet, BinaryHeap};
        use std::cmp::Ordering;

        // Priority queue entry
        #[derive(Debug)]
        struct ScoredEntry {
            score: f64,
            chunk: Chunk,
            file_path: std::path::PathBuf,
            is_neighbor: bool,
        }

        impl PartialEq for ScoredEntry {
            fn eq(&self, other: &Self) -> bool { self.score == other.score }
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
            file_groups.entry(result.chunk.file_id)
                .or_default()
                .push(result);
        }

        // Step 2: For files with 3+ matches, include ALL chunks from that file
        for (&file_id, results) in &file_groups {
            if results.len() >= 3 {
                // This file is highly relevant -- include all its chunks
                if let Ok(all_chunks) = index.get_chunks_for_file(file_id) {
                    let file_path = results[0].file_path.clone();
                    let avg_score = results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64;
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

        // Step 3: Fetch 1-hop graph neighbors of top-scored anchor symbols
        if let Some(graph) = dep_graph {
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
                                            let fp = self.get_file_path_for_chunk(index, &chunk)
                                                .unwrap_or_default();
                                            seen_chunk_ids.insert(chunk_id);
                                            heap.push(ScoredEntry {
                                                score: result.score * 0.5, // neighbor discount
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
                                            let fp = self.get_file_path_for_chunk(index, &chunk)
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

        // Step 4: Pack greedily by score until token budget is hit
        let mut total_tokens: u32 = 0;
        let mut entries: Vec<ContextEntry> = Vec::new();

        while let Some(entry) = heap.pop() {
            if total_tokens + entry.chunk.token_count > token_budget {
                // Try to fit -- if this single chunk exceeds remaining budget, skip it
                continue;
            }
            total_tokens += entry.chunk.token_count;
            entries.push(ContextEntry {
                file_path: entry.file_path,
                chunk: entry.chunk,
                score: entry.score,
                is_graph_neighbor: entry.is_neighbor,
            });
        }

        ContextWindow {
            entries,
            total_tokens,
            token_budget,
        }
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
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "shall",
    "should", "may", "might", "can", "could", "must", "to", "of", "in",
    "for", "on", "with", "at", "by", "from", "as", "into", "through",
    "during", "before", "after", "above", "below", "and", "but", "or",
    "not", "no", "if", "then", "than", "that", "this", "these", "those",
    "it", "its", "i", "me", "my", "we", "our", "you", "your", "he",
    "she", "they", "them", "their", "what", "which", "who", "whom",
    "how", "when", "where", "why", "all", "each", "every", "both",
    "few", "more", "most", "other", "some", "such", "only", "own",
    "same", "so", "very", "just", "about", "there", "here",
    "find", "show", "get", "list", "explain", "describe",
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
        assert_eq!(analyze_query("how does authentication work?"), QueryType::NaturalLanguage);
        assert_eq!(analyze_query("what is the user service"), QueryType::NaturalLanguage);
        assert_eq!(analyze_query("find all database queries"), QueryType::NaturalLanguage);
        assert_eq!(analyze_query("where is session management implemented"), QueryType::NaturalLanguage);
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

        let fused = engine.fuse_results(&keyword, &semantic, &[]);

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
        let fused = engine.fuse_results(&[], &[], &[]);
        assert!(fused.is_empty());
    }

    #[test]
    fn test_fuse_results_symbol_boost() {
        let engine = SearchEngine::new(60, 4000);

        let keyword = vec![(1, -0.5), (2, -0.3)];
        let symbol = vec![2_i64]; // chunk_id 2 is an exact symbol match

        let fused = engine.fuse_results(&keyword, &[], &symbol);

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
}
