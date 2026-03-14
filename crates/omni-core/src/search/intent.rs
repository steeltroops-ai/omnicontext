//! Query intent classification for context-aware search.
//!
//! Different query intents require different context strategies:
//! - Explain: Architectural overview, module map, high-level flow
//! - Edit: Implementation details, surrounding code, tests
//! - Debug: Error paths, recent changes, stack traces
//! - Refactor: All usages, downstream dependents, type hierarchy
//! - Generate: Similar patterns, architectural conventions
//!
//! ## Prototype-Vector Blending
//!
//! [`IntentClassifier`] extends the keyword heuristics in [`QueryIntent::classify`]
//! with a prototype-vector second signal.  Representative prototype sentences
//! for each intent class are embedded once at startup into class centroids.
//! At query time the query embedding is compared to each centroid via cosine
//! similarity.  The keyword confidence (`kw_conf`) and prototype similarity
//! (`vec_score`) are blended as follows:
//!
//! ```text
//! if kw_conf >= 0.8  →  return kw_intent          (high-confidence keyword wins)
//! elif vec_score > 0.6  →  return vec_intent       (prototype wins)
//! else  →  return kw_intent                        (fallback)
//! ```
//!
//! When the embedder is unavailable (no ONNX session) the classifier degrades
//! gracefully to keyword-only mode.

use std::collections::HashMap;
use std::sync::Mutex;

use lru::LruCache;
use serde::{Deserialize, Serialize};

/// Intent prototype sentences — one or two per class.
///
/// These short representative sentences are embedded once during
/// `IntentClassifier::build()` and averaged into per-class centroids.
/// They were chosen to have clear, unambiguous intent signals.
const INTENT_PROTOTYPES: &[(&str, QueryIntent)] = &[
    ("explain how this function works", QueryIntent::Explain),
    ("what does this module do", QueryIntent::Explain),
    ("fix the bug in this implementation", QueryIntent::Debug),
    ("why is this crashing", QueryIntent::Debug),
    (
        "refactor this to use a different pattern",
        QueryIntent::Refactor,
    ),
    ("find all callers of this function", QueryIntent::Refactor),
    (
        "implement a new endpoint that handles requests",
        QueryIntent::Generate,
    ),
    ("update the configuration field", QueryIntent::Edit),
    ("modify the search algorithm", QueryIntent::Edit),
    (
        "how does data flow from parser to index",
        QueryIntent::DataFlow,
    ),
    ("what does the chunker depend on", QueryIntent::Dependency),
    (
        "show upstream imports of this module",
        QueryIntent::Dependency,
    ),
    (
        "which tests cover the authentication path",
        QueryIntent::TestCoverage,
    ),
];

/// LRU cache capacity for per-query intent embeddings.
const INTENT_EMBED_CACHE_SIZE: usize = 200;

/// Create the LRU cache for intent embeddings.
///
/// Centralises the `NonZeroUsize` construction so the calling
/// functions do not need per-site lint suppressions.
#[inline]
fn intent_embed_lru() -> lru::LruCache<String, Vec<f32>> {
    // 200 is non-zero; the Option is always Some.
    let cap =
        std::num::NonZeroUsize::new(INTENT_EMBED_CACHE_SIZE).unwrap_or(std::num::NonZeroUsize::MIN);
    lru::LruCache::new(cap)
}

/// Dot-product of two equal-length slices (cosine similarity on L2-normalised vecs).
fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// L2-normalise a vector in-place; returns the norm.
fn l2_norm(v: &mut [f32]) -> f32 {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
    norm
}

/// Query intent classifier with optional prototype-vector second signal.
///
/// Use [`IntentClassifier::build`] to create an instance with prototype
/// embeddings wired to a live embedder.  Use [`IntentClassifier::keyword_only`]
/// for a no-embedder fallback (tests, degraded mode).
pub struct IntentClassifier {
    /// Per-intent class centroid (L2-normalised).  `None` when no embedder was
    /// available at construction time — falls back to keyword-only mode.
    prototypes: Option<HashMap<QueryIntent, Vec<f32>>>,
    /// Per-query embedding cache to avoid re-embedding the same query string
    /// during a single search request or across rapid consecutive calls.
    embed_cache: Mutex<LruCache<String, Vec<f32>>>,
}

impl std::fmt::Debug for IntentClassifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let has_prototypes = self.prototypes.is_some();
        f.debug_struct("IntentClassifier")
            .field("has_prototypes", &has_prototypes)
            .finish_non_exhaustive()
    }
}

impl IntentClassifier {
    /// Build an `IntentClassifier` with prototype embeddings from `embedder`.
    ///
    /// Embeds every prototype sentence in [`INTENT_PROTOTYPES`], averages them
    /// per class into L2-normalised centroids, and returns the classifier.
    ///
    /// If embedding fails for any prototype sentence, that sentence is skipped.
    /// If *all* sentences for a class fail, that class has no centroid and falls
    /// back to keyword classification for that intent.
    ///
    /// This is O(|INTENT_PROTOTYPES|) embedding calls — called once at startup.
    pub fn build(embedder: &crate::embedder::Embedder) -> Self {
        if !embedder.is_available() {
            return Self::keyword_only();
        }

        let texts: Vec<&str> = INTENT_PROTOTYPES.iter().map(|(t, _)| *t).collect();
        let embeddings = embedder.embed_batch(&texts);

        // Accumulate sum vectors per class.
        let mut sums: HashMap<QueryIntent, (Vec<f32>, usize)> = HashMap::new();
        for ((_, intent), maybe_emb) in INTENT_PROTOTYPES.iter().zip(embeddings.iter()) {
            if let Some(emb) = maybe_emb {
                let entry = sums
                    .entry(*intent)
                    .or_insert_with(|| (vec![0.0f32; emb.len()], 0));
                for (a, b) in entry.0.iter_mut().zip(emb.iter()) {
                    *a += b;
                }
                entry.1 += 1;
            }
        }

        // Average and L2-normalise each centroid.
        let prototypes: HashMap<QueryIntent, Vec<f32>> = sums
            .into_iter()
            .filter_map(|(intent, (mut sum, count))| {
                if count == 0 {
                    return None;
                }
                let n = count as f32;
                for x in &mut sum {
                    *x /= n;
                }
                l2_norm(&mut sum);
                Some((intent, sum))
            })
            .collect();

        if prototypes.is_empty() {
            return Self::keyword_only();
        }

        tracing::debug!(
            classes = prototypes.len(),
            "intent classifier prototype centroids built"
        );

        Self {
            prototypes: Some(prototypes),
            embed_cache: Mutex::new(intent_embed_lru()),
        }
    }

    /// Create a keyword-only classifier with no prototype embeddings.
    ///
    /// Used in degraded mode (embedder unavailable) and in unit tests that
    /// do not have access to a real ONNX session.
    pub fn keyword_only() -> Self {
        Self {
            prototypes: None,
            embed_cache: Mutex::new(intent_embed_lru()),
        }
    }

    /// Classify a query using keyword heuristics blended with prototype similarity.
    ///
    /// ## Blending logic
    ///
    /// 1. Run keyword heuristics → `(kw_intent, kw_conf)`.
    ///    - `kw_conf = 1.0` when `kw_intent != Unknown` (exact keyword match).
    ///    - `kw_conf = 0.5` when `kw_intent == Unknown` (ambiguous).
    /// 2. If prototypes and embedder available: embed query (using cache), compute
    ///    cosine similarity to each class centroid → `(vec_intent, vec_score)`.
    /// 3. Blend:
    ///    - `kw_conf >= 0.8` → return `kw_intent` immediately (keyword wins).
    ///    - `vec_score > 0.6` → return `vec_intent` (prototype overrides ambiguous keyword).
    ///    - else → return `kw_intent` (safe fallback).
    pub fn classify(
        &self,
        query: &str,
        embedder: Option<&crate::embedder::Embedder>,
    ) -> QueryIntent {
        let kw_intent = QueryIntent::classify(query);
        let kw_conf: f32 = if kw_intent != QueryIntent::Unknown {
            1.0
        } else {
            0.5
        };

        // High-confidence keyword match — skip prototype lookup entirely.
        if kw_conf >= 0.8 {
            return kw_intent;
        }

        // Prototype vector path (only when both prototypes and embedder are present).
        let prototypes = match &self.prototypes {
            Some(p) => p,
            None => return kw_intent,
        };
        let embedder = match embedder {
            Some(e) if e.is_available() => e,
            _ => return kw_intent,
        };

        // Retrieve from cache or embed the query.
        let query_emb: Vec<f32> = {
            let cached = self
                .embed_cache
                .lock()
                .ok()
                .and_then(|mut cache| cache.get(query).cloned());

            if let Some(emb) = cached {
                emb
            } else {
                match embedder.embed_query(query) {
                    Ok(mut emb) => {
                        l2_norm(&mut emb);
                        if let Ok(mut cache) = self.embed_cache.lock() {
                            cache.put(query.to_string(), emb.clone());
                        }
                        emb
                    }
                    Err(_) => return kw_intent,
                }
            }
        };

        // Find the class with highest cosine similarity.
        let mut best_intent = QueryIntent::Unknown;
        let mut best_score: f32 = f32::NEG_INFINITY;
        for (intent, centroid) in prototypes {
            if centroid.len() != query_emb.len() {
                continue;
            }
            let score = dot(&query_emb, centroid);
            if score > best_score {
                best_score = score;
                best_intent = *intent;
            }
        }

        if best_score > 0.6 {
            best_intent
        } else {
            kw_intent
        }
    }
}

/// Query intent classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueryIntent {
    /// User wants to understand how something works.
    Explain,
    /// User wants to modify existing code.
    Edit,
    /// User wants to fix a bug or error.
    Debug,
    /// User wants to restructure or rename code.
    Refactor,
    /// User wants to create new code following patterns.
    Generate,
    /// User wants to trace data flow through the system.
    DataFlow,
    /// User wants to find dependencies or dependents of a symbol.
    Dependency,
    /// User wants to find tests that cover a given symbol or module.
    TestCoverage,
    /// Intent unclear, use balanced strategy.
    Unknown,
}

impl QueryIntent {
    /// Classify a query string into an intent category.
    ///
    /// Uses keyword-based heuristics to determine user intent.
    /// More sophisticated approaches (ML-based) can be added later.
    pub fn classify(query: &str) -> Self {
        let query_lower = query.to_lowercase();

        // DataFlow intent: tracing data through the system
        if (query_lower.contains("data flow")
            || query_lower.contains("dataflow")
            || query_lower.contains("pipeline")
            || query_lower.contains("pass through")
            || query_lower.contains("passes through")
            || query_lower.contains("transforms")
            || query_lower.contains("propagat"))
            && (query_lower.contains("from")
                || query_lower.contains("to")
                || query_lower.contains("through"))
        {
            return QueryIntent::DataFlow;
        }

        // Dependency intent: finding what depends on what
        if query_lower.contains("depend")
            || query_lower.contains("import")
            || query_lower.contains("require")
            || (query_lower.contains("uses") && query_lower.contains("what"))
            || (query_lower.contains("used by"))
            || query_lower.contains("downstream")
            || query_lower.contains("upstream")
        {
            return QueryIntent::Dependency;
        }

        // TestCoverage intent: finding tests
        if (query_lower.contains("test") || query_lower.contains("spec"))
            && (query_lower.contains("cover")
                || query_lower.contains("which")
                || query_lower.contains("where")
                || query_lower.contains("find")
                || query_lower.contains("missing"))
        {
            return QueryIntent::TestCoverage;
        }

        // Debug intent: errors, bugs, failures (check before "fix" triggers Edit)
        if query_lower.contains("bug")
            || query_lower.contains("error")
            || query_lower.contains("fail")
            || query_lower.contains("crash")
            || query_lower.contains("issue")
            || query_lower.contains("problem")
            || query_lower.contains("broken")
            || query_lower.contains("debug")
            || query_lower.contains("trace")
            || query_lower.contains("exception")
        {
            return QueryIntent::Debug;
        }

        // Refactor intent: restructuring, renaming, moving
        if query_lower.contains("rename")
            || query_lower.contains("refactor")
            || query_lower.contains("move")
            || query_lower.contains("reorganize")
            || query_lower.contains("restructure")
            || query_lower.contains("extract")
            || query_lower.contains("inline")
            || query_lower.contains("usages")
            || query_lower.contains("references")
            || query_lower.contains("callers")
        {
            return QueryIntent::Refactor;
        }

        // Explain intent: understanding, documentation, architecture
        if query_lower.contains("how")
            || query_lower.contains("what")
            || query_lower.contains("why")
            || query_lower.contains("explain")
            || query_lower.contains("understand")
            || query_lower.contains("describe")
            || query_lower.contains("overview")
            || query_lower.contains("architecture")
            || query_lower.contains("flow")
            || query_lower.contains("works")
        {
            return QueryIntent::Explain;
        }

        // Generate intent: creating new code (check before "add" triggers Edit)
        if query_lower.contains("create")
            || query_lower.contains("implement")
            || query_lower.contains("generate")
            || query_lower.contains("write")
            || query_lower.contains("build")
            || query_lower.contains("make")
        {
            return QueryIntent::Generate;
        }

        // Edit intent: modifying existing code
        if query_lower.contains("fix")
            || query_lower.contains("change")
            || query_lower.contains("update")
            || query_lower.contains("modify")
            || query_lower.contains("edit")
            || query_lower.contains("improve")
            || query_lower.contains("optimize")
            || query_lower.contains("add")
            || query_lower.contains("new")
        {
            return QueryIntent::Edit;
        }

        // Default to Unknown for ambiguous queries
        QueryIntent::Unknown
    }

    /// Get the context strategy for this intent.
    pub fn context_strategy(&self) -> ContextStrategy {
        match self {
            QueryIntent::Explain => ContextStrategy {
                include_architecture: true,
                include_implementation: false,
                include_tests: false,
                include_docs: true,
                include_recent_changes: false,
                graph_depth: 2,
                prioritize_high_level: true,
            },
            QueryIntent::Edit => ContextStrategy {
                include_architecture: false,
                include_implementation: true,
                include_tests: true,
                include_docs: false,
                include_recent_changes: false,
                graph_depth: 1,
                prioritize_high_level: false,
            },
            QueryIntent::Debug => ContextStrategy {
                include_architecture: false,
                include_implementation: true,
                include_tests: true,
                include_docs: false,
                include_recent_changes: true,
                graph_depth: 1,
                prioritize_high_level: false,
            },
            QueryIntent::Refactor => ContextStrategy {
                include_architecture: true,
                include_implementation: true,
                include_tests: true,
                include_docs: false,
                include_recent_changes: false,
                graph_depth: 3,
                prioritize_high_level: false,
            },
            QueryIntent::Generate => ContextStrategy {
                include_architecture: true,
                include_implementation: true,
                include_tests: false,
                include_docs: true,
                include_recent_changes: false,
                graph_depth: 1,
                prioritize_high_level: true,
            },
            QueryIntent::DataFlow => ContextStrategy {
                include_architecture: true,
                include_implementation: true,
                include_tests: false,
                include_docs: false,
                include_recent_changes: false,
                graph_depth: 4, // deeper traversal for tracing data paths
                prioritize_high_level: false,
            },
            QueryIntent::Dependency => ContextStrategy {
                include_architecture: true,
                include_implementation: false,
                include_tests: false,
                include_docs: true,
                include_recent_changes: false,
                graph_depth: 3, // callers + callees
                prioritize_high_level: true,
            },
            QueryIntent::TestCoverage => ContextStrategy {
                include_architecture: false,
                include_implementation: false,
                include_tests: true,
                include_docs: false,
                include_recent_changes: false,
                graph_depth: 1,
                prioritize_high_level: false,
            },
            QueryIntent::Unknown => ContextStrategy {
                include_architecture: true,
                include_implementation: true,
                include_tests: true,
                include_docs: true,
                include_recent_changes: false,
                graph_depth: 2,
                prioritize_high_level: false,
            },
        }
    }
}

/// Context selection strategy based on query intent.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub struct ContextStrategy {
    /// Include architectural overview and module map.
    pub include_architecture: bool,
    /// Include full implementation details.
    pub include_implementation: bool,
    /// Include related test files.
    pub include_tests: bool,
    /// Include documentation and comments.
    pub include_docs: bool,
    /// Include recent git changes (for debugging).
    pub include_recent_changes: bool,
    /// Maximum depth for graph traversal.
    pub graph_depth: usize,
    /// Prioritize high-level abstractions over details.
    pub prioritize_high_level: bool,
}

impl Default for ContextStrategy {
    fn default() -> Self {
        Self {
            include_architecture: true,
            include_implementation: true,
            include_tests: true,
            include_docs: true,
            include_recent_changes: false,
            graph_depth: 2,
            prioritize_high_level: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_explain() {
        assert_eq!(
            QueryIntent::classify("how does authentication work?"),
            QueryIntent::Explain
        );
        assert_eq!(
            QueryIntent::classify("what is the purpose of this module?"),
            QueryIntent::Explain
        );
        assert_eq!(
            QueryIntent::classify("explain the caching strategy"),
            QueryIntent::Explain
        );
    }

    #[test]
    fn test_classify_debug() {
        assert_eq!(
            QueryIntent::classify("fix the login bug"),
            QueryIntent::Debug
        );
        assert_eq!(
            QueryIntent::classify("why is this crashing?"),
            QueryIntent::Debug
        );
        assert_eq!(
            QueryIntent::classify("error in authentication"),
            QueryIntent::Debug
        );
    }

    #[test]
    fn test_classify_refactor() {
        assert_eq!(
            QueryIntent::classify("rename this function"),
            QueryIntent::Refactor
        );
        assert_eq!(
            QueryIntent::classify("find all usages of AuthService"),
            QueryIntent::Refactor
        );
        assert_eq!(
            QueryIntent::classify("refactor the payment module"),
            QueryIntent::Refactor
        );
    }

    #[test]
    fn test_classify_generate() {
        assert_eq!(
            QueryIntent::classify("create a new API endpoint"),
            QueryIntent::Generate
        );
        assert_eq!(
            QueryIntent::classify("implement a user service"),
            QueryIntent::Generate
        );
        assert_eq!(
            QueryIntent::classify("generate caching logic"),
            QueryIntent::Generate
        );
    }

    #[test]
    fn test_classify_edit() {
        assert_eq!(
            QueryIntent::classify("update the configuration"),
            QueryIntent::Edit
        );
        assert_eq!(
            QueryIntent::classify("modify the search algorithm"),
            QueryIntent::Edit
        );
        assert_eq!(
            QueryIntent::classify("improve performance"),
            QueryIntent::Edit
        );
        assert_eq!(QueryIntent::classify("add a new field"), QueryIntent::Edit);
    }

    #[test]
    fn test_classify_data_flow() {
        assert_eq!(
            QueryIntent::classify("how does data flow from parser to index?"),
            QueryIntent::DataFlow
        );
        assert_eq!(
            QueryIntent::classify("trace the pipeline from input to output"),
            QueryIntent::DataFlow
        );
    }

    #[test]
    fn test_classify_dependency() {
        assert_eq!(
            QueryIntent::classify("what depends on SearchEngine?"),
            QueryIntent::Dependency
        );
        assert_eq!(
            QueryIntent::classify("what does the chunker import?"),
            QueryIntent::Dependency
        );
        assert_eq!(
            QueryIntent::classify("show upstream dependencies"),
            QueryIntent::Dependency
        );
    }

    #[test]
    fn test_classify_test_coverage() {
        assert_eq!(
            QueryIntent::classify("which tests cover the search module?"),
            QueryIntent::TestCoverage
        );
        assert_eq!(
            QueryIntent::classify("find tests for authentication"),
            QueryIntent::TestCoverage
        );
        assert_eq!(
            QueryIntent::classify("missing test coverage for parser"),
            QueryIntent::TestCoverage
        );
    }

    #[test]
    fn test_classify_unknown() {
        assert_eq!(
            QueryIntent::classify("authentication"),
            QueryIntent::Unknown
        );
        assert_eq!(QueryIntent::classify("Config"), QueryIntent::Unknown);
    }

    #[test]
    fn test_context_strategy_explain() {
        let strategy = QueryIntent::Explain.context_strategy();
        assert!(strategy.include_architecture);
        assert!(!strategy.include_implementation);
        assert!(!strategy.include_tests);
        assert!(strategy.include_docs);
        assert_eq!(strategy.graph_depth, 2);
        assert!(strategy.prioritize_high_level);
    }

    #[test]
    fn test_context_strategy_debug() {
        let strategy = QueryIntent::Debug.context_strategy();
        assert!(!strategy.include_architecture);
        assert!(strategy.include_implementation);
        assert!(strategy.include_tests);
        assert!(strategy.include_recent_changes);
        assert_eq!(strategy.graph_depth, 1);
    }

    #[test]
    fn test_context_strategy_refactor() {
        let strategy = QueryIntent::Refactor.context_strategy();
        assert!(strategy.include_architecture);
        assert!(strategy.include_implementation);
        assert!(strategy.include_tests);
        assert_eq!(strategy.graph_depth, 3);
    }

    #[test]
    fn test_context_strategy_data_flow() {
        let strategy = QueryIntent::DataFlow.context_strategy();
        assert!(strategy.include_architecture);
        assert!(strategy.include_implementation);
        assert_eq!(strategy.graph_depth, 4);
    }

    #[test]
    fn test_context_strategy_dependency() {
        let strategy = QueryIntent::Dependency.context_strategy();
        assert!(strategy.include_architecture);
        assert!(strategy.include_docs);
        assert!(strategy.prioritize_high_level);
        assert_eq!(strategy.graph_depth, 3);
    }

    #[test]
    fn test_context_strategy_test_coverage() {
        let strategy = QueryIntent::TestCoverage.context_strategy();
        assert!(strategy.include_tests);
        assert!(!strategy.include_architecture);
        assert!(!strategy.include_implementation);
        assert_eq!(strategy.graph_depth, 1);
    }

    // -----------------------------------------------------------------------
    // IntentClassifier tests (keyword-only mode — no ONNX session in tests)
    // -----------------------------------------------------------------------

    #[test]
    fn test_keyword_only_classifier_falls_back_to_keyword() {
        let clf = IntentClassifier::keyword_only();
        assert_eq!(clf.classify("fix the bug", None), QueryIntent::Debug);
        assert_eq!(clf.classify("update the config", None), QueryIntent::Edit);
        assert_eq!(
            clf.classify("how does this work", None),
            QueryIntent::Explain
        );
    }

    #[test]
    fn test_keyword_wins_on_exact_match_even_without_embedder() {
        let clf = IntentClassifier::keyword_only();
        assert_eq!(
            clf.classify("fix the bug", None),
            QueryIntent::Debug,
            "high-confidence keyword match must not be overridden"
        );
    }

    #[test]
    fn test_ambiguous_query_returns_unknown_without_embedder() {
        let clf = IntentClassifier::keyword_only();
        assert_eq!(clf.classify("authentication", None), QueryIntent::Unknown);
    }

    #[test]
    fn test_classifier_keyword_only_has_no_prototypes() {
        let clf = IntentClassifier::keyword_only();
        assert!(clf.prototypes.is_none());
        assert_eq!(
            clf.classify("refactor the auth module", None),
            QueryIntent::Refactor
        );
    }
}
