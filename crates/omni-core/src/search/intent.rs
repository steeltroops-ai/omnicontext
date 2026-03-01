//! Query intent classification for context-aware search.
//!
//! Different query intents require different context strategies:
//! - Explain: Architectural overview, module map, high-level flow
//! - Edit: Implementation details, surrounding code, tests
//! - Debug: Error paths, recent changes, stack traces
//! - Refactor: All usages, downstream dependents, type hierarchy
//! - Generate: Similar patterns, architectural conventions

use serde::{Deserialize, Serialize};

/// Query intent classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
        assert_eq!(
            QueryIntent::classify("add a new field"),
            QueryIntent::Edit
        );
    }

    #[test]
    fn test_classify_unknown() {
        assert_eq!(
            QueryIntent::classify("authentication"),
            QueryIntent::Unknown
        );
        assert_eq!(
            QueryIntent::classify("Config"),
            QueryIntent::Unknown
        );
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
}

