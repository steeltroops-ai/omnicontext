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

use crate::embedder::Embedder;
use crate::index::MetadataIndex;
use crate::types::{Chunk, SearchResult, ScoreBreakdown};
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
        let fused = self.fuse_results(
            &keyword_results,
            &semantic_results,
            &symbol_results,
        );

        // ---- Build final results with structural boosting ----
        let mut results = Vec::new();
        let mut total_tokens: u32 = 0;

        for scored in fused.iter().take(limit * 2) {
            let chunk_id = scored.chunk_id;

            let chunk = match self.get_chunk_by_id(index, chunk_id) {
                Some(c) => c,
                None => continue,
            };

            // Apply structural boost now that we have chunk metadata
            let (boosted_score, struct_weight) =
                Self::apply_structural_boost(scored.final_score, &chunk);

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

        results.truncate(limit);

        Ok(results)
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

    /// Apply structural boosting once we have the actual chunk data.
    /// Called during result assembly when chunks are fetched from the DB.
    fn apply_structural_boost(score: f64, chunk: &Chunk) -> (f64, f64) {
        let struct_weight = chunk.kind.default_weight()
            * chunk.visibility.weight_multiplier();
        let boosted = score * (0.4 + 0.6 * struct_weight);
        (boosted, struct_weight)
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
/// Strips stop words and question marks, preserving content-bearing tokens
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

    // Join with OR for broader FTS5 matching
    tokens.join(" OR ")
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
}
