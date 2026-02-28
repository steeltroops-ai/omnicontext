//! Hybrid search engine with RRF fusion and multi-signal ranking.
//!
//! Combines semantic (vector) search, keyword (FTS5) search, and
//! symbol table lookup into a single ranked result set.

use crate::types::SearchResult;
use crate::error::OmniResult;

/// Hybrid search engine that fuses multiple retrieval signals.
pub struct SearchEngine {
    /// RRF constant k.
    rrf_k: u32,
}

impl SearchEngine {
    /// Create a new search engine with the given RRF constant.
    pub fn new(rrf_k: u32) -> Self {
        Self { rrf_k }
    }

    /// Execute a hybrid search query.
    ///
    /// Steps:
    /// 1. Analyze query to determine search strategy
    /// 2. Execute semantic search (if embedder available)
    /// 3. Execute keyword search (FTS5 BM25)
    /// 4. Execute symbol lookup (exact match)
    /// 5. Fuse results with RRF
    /// 6. Apply structural weight boost
    /// 7. Apply dependency proximity boost
    /// 8. Apply recency boost
    /// 9. Build context for top results
    pub fn search(&self, _query: &str, _limit: usize) -> OmniResult<Vec<SearchResult>> {
        // TODO: Implement hybrid search pipeline
        Ok(Vec::new())
    }

    /// Compute RRF score from two rank lists.
    pub fn rrf_score(&self, semantic_rank: Option<u32>, keyword_rank: Option<u32>) -> f64 {
        let k = f64::from(self.rrf_k);
        let semantic = semantic_rank.map_or(0.0, |r| 1.0 / (k + f64::from(r)));
        let keyword = keyword_rank.map_or(0.0, |r| 1.0 / (k + f64::from(r)));
        semantic + keyword
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_score_both_signals() {
        let engine = SearchEngine::new(60);
        let score = engine.rrf_score(Some(1), Some(1));
        // 1/(60+1) + 1/(60+1) = 2/61
        let expected = 2.0 / 61.0;
        assert!((score - expected).abs() < 1e-10);
    }

    #[test]
    fn test_rrf_score_semantic_only() {
        let engine = SearchEngine::new(60);
        let score = engine.rrf_score(Some(1), None);
        let expected = 1.0 / 61.0;
        assert!((score - expected).abs() < 1e-10);
    }

    #[test]
    fn test_rrf_score_no_signal() {
        let engine = SearchEngine::new(60);
        let score = engine.rrf_score(None, None);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_rrf_higher_rank_gets_higher_score() {
        let engine = SearchEngine::new(60);
        let score_rank1 = engine.rrf_score(Some(1), Some(1));
        let score_rank10 = engine.rrf_score(Some(10), Some(10));
        assert!(score_rank1 > score_rank10);
    }
}
