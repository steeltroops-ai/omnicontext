//! Test coverage gap detector using dependency graph analysis.
//!
//! Identifies symbols and files that lack test coverage by analyzing
//! which production symbols are referenced (imported, called, used) by
//! test files in the dependency graph.
//!
//! ## Approach
//!
//! 1. **Classify** files as test or production based on path heuristics
//! 2. **Walk** the dependency graph from test symbols to find covered production symbols
//! 3. **Report** uncovered symbols ranked by risk (high in-degree = high risk)
//!
//! This is a static analysis approximation — it detects *structural* coverage
//! (test files that reference production code) rather than runtime line coverage.
//! It's complementary to tools like `cargo-tarpaulin` or `coverage.py`.

#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::must_use_candidate
)]

use std::collections::{HashMap, HashSet};
use std::path::Path;

/// A symbol that lacks test coverage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UncoveredSymbol {
    /// Symbol ID in the index.
    pub symbol_id: i64,
    /// Fully qualified name.
    pub fqn: String,
    /// File path containing this symbol.
    pub file_path: String,
    /// Number of downstream dependents (symbols that depend on this one).
    /// Higher = more dangerous to leave untested.
    pub dependent_count: usize,
    /// Number of upstream dependencies (symbols this one calls).
    /// Higher = more complex, needs more testing.
    pub dependency_count: usize,
    /// Composite risk score (0.0–1.0). Combines dependent count + dependency count.
    pub risk_score: f64,
}

/// Summary of test coverage analysis.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoverageReport {
    /// Total production symbols analyzed.
    pub total_production_symbols: usize,
    /// Number of production symbols covered by at least one test.
    pub covered_symbols: usize,
    /// Number of production symbols with no test coverage.
    pub uncovered_symbols: usize,
    /// Coverage ratio (0.0–1.0).
    pub coverage_ratio: f64,
    /// Total test files detected.
    pub test_file_count: usize,
    /// Uncovered symbols ranked by risk (highest risk first).
    pub gaps: Vec<UncoveredSymbol>,
}

/// Input data for coverage analysis.
/// Abstracted to avoid coupling to the full index/graph types.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// Symbol database ID.
    pub id: i64,
    /// Fully qualified name (e.g., `crate::auth::validate`).
    pub fqn: String,
    /// File path containing this symbol.
    pub file_path: String,
    /// Whether this symbol belongs to a test file.
    pub is_test_file: bool,
}

/// Analyze test coverage gaps using the dependency graph.
///
/// # Parameters
/// - `symbols`: All symbols in the codebase with their file classification
/// - `edges`: Dependency edges as (source_id, target_id) pairs
/// - `max_gaps`: Maximum number of gaps to return (sorted by risk)
pub fn analyze_coverage_gaps(
    symbols: &[SymbolInfo],
    edges: &[(i64, i64)],
    max_gaps: usize,
) -> CoverageReport {
    // Partition symbols into test and production
    let mut test_symbol_ids: HashSet<i64> = HashSet::new();
    let mut production_symbols: Vec<&SymbolInfo> = Vec::new();

    for sym in symbols {
        if sym.is_test_file {
            test_symbol_ids.insert(sym.id);
        } else {
            production_symbols.push(sym);
        }
    }

    if production_symbols.is_empty() {
        return CoverageReport {
            total_production_symbols: 0,
            covered_symbols: 0,
            uncovered_symbols: 0,
            coverage_ratio: 1.0,
            test_file_count: count_unique_test_files(symbols),
            gaps: Vec::new(),
        };
    }

    // Build a set of production symbol IDs for boundary checking
    let production_ids: HashSet<i64> = production_symbols.iter().map(|s| s.id).collect();

    // Build adjacency lists for forward and reverse lookups
    let mut outgoing: HashMap<i64, Vec<i64>> = HashMap::new();
    let mut incoming: HashMap<i64, Vec<i64>> = HashMap::new();

    for &(src, tgt) in edges {
        outgoing.entry(src).or_default().push(tgt);
        incoming.entry(tgt).or_default().push(src);
    }

    // Walk from test symbols to find all production symbols they cover.
    // A production symbol is "covered" if any test symbol has a direct
    // dependency edge to it (imports it, calls it, uses its type).
    let mut covered_ids: HashSet<i64> = HashSet::new();

    for &test_id in &test_symbol_ids {
        // Direct outgoing dependencies from test symbols
        if let Some(targets) = outgoing.get(&test_id) {
            for &target in targets {
                // Only count as covered if this target is a known production symbol
                if production_ids.contains(&target) {
                    covered_ids.insert(target);
                }
            }
        }
    }

    // Identify uncovered production symbols
    let mut gaps: Vec<UncoveredSymbol> = Vec::new();

    for sym in &production_symbols {
        if covered_ids.contains(&sym.id) {
            continue;
        }

        let dependent_count = incoming.get(&sym.id).map(|v| v.len()).unwrap_or(0);
        let dependency_count = outgoing.get(&sym.id).map(|v| v.len()).unwrap_or(0);

        // Risk score: weighted combination of dependents and dependencies
        // Dependents matter more (if this breaks, it affects more code)
        let risk_score = compute_risk_score(dependent_count, dependency_count);

        gaps.push(UncoveredSymbol {
            symbol_id: sym.id,
            fqn: sym.fqn.clone(),
            file_path: sym.file_path.clone(),
            dependent_count,
            dependency_count,
            risk_score,
        });
    }

    // Sort by risk score descending
    gaps.sort_by(|a, b| {
        b.risk_score
            .partial_cmp(&a.risk_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    gaps.truncate(max_gaps);

    let total = production_symbols.len();
    let covered = covered_ids.len();
    let uncovered = total.saturating_sub(covered);

    CoverageReport {
        total_production_symbols: total,
        covered_symbols: covered,
        uncovered_symbols: uncovered,
        coverage_ratio: if total > 0 {
            covered as f64 / total as f64
        } else {
            1.0
        },
        test_file_count: count_unique_test_files(symbols),
        gaps,
    }
}

/// Check if a file path looks like a test file.
pub fn is_test_file(path: &str) -> bool {
    let path_obj = Path::new(path);

    // Check filename patterns using the original case for camelCase detection
    if let Some(stem) = path_obj.file_stem().and_then(|s| s.to_str()) {
        let stem_lower = stem.to_lowercase();
        if stem_lower.starts_with("test_")
            || stem_lower.ends_with("_test")
            || stem_lower.ends_with("_spec")
            || std::path::Path::new(&stem_lower)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("test"))
            || std::path::Path::new(&stem_lower)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("spec"))
            || stem_lower == "conftest"
        {
            return true;
        }
        // camelCase: testSomething (starts with "test" and char 4 is uppercase in original)
        if stem.starts_with("test")
            && stem.len() > 4
            && stem.chars().nth(4).is_some_and(|c| c.is_uppercase())
        {
            return true;
        }
    }

    // Check directory patterns
    let path_lower = path.to_lowercase();
    let components: Vec<&str> = path_lower.split(['/', '\\']).collect();
    for component in &components {
        if *component == "tests"
            || *component == "test"
            || *component == "__tests__"
            || *component == "spec"
            || *component == "specs"
        {
            return true;
        }
    }

    false
}

/// Compute a risk score for an uncovered symbol.
/// Range: 0.0–1.0 using sigmoid-like scaling.
fn compute_risk_score(dependent_count: usize, dependency_count: usize) -> f64 {
    // Dependents weighted 3x more than dependencies
    let raw = (dependent_count as f64 * 3.0 + dependency_count as f64) / 4.0;
    // Sigmoid: 2/(1+e^(-x/5)) - 1 maps [0, inf) → [0, 1)
    let sigmoid = 2.0 / (1.0 + (-raw / 5.0_f64).exp()) - 1.0;
    sigmoid.clamp(0.0, 1.0)
}

/// Count unique test files among symbols.
fn count_unique_test_files(symbols: &[SymbolInfo]) -> usize {
    let test_files: HashSet<&str> = symbols
        .iter()
        .filter(|s| s.is_test_file)
        .map(|s| s.file_path.as_str())
        .collect();
    test_files.len()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(id: i64, fqn: &str, path: &str, is_test: bool) -> SymbolInfo {
        SymbolInfo {
            id,
            fqn: fqn.to_string(),
            file_path: path.to_string(),
            is_test_file: is_test,
        }
    }

    #[test]
    fn test_is_test_file_patterns() {
        assert!(is_test_file("src/test_auth.py"));
        assert!(is_test_file("src/auth_test.rs"));
        assert!(is_test_file("src/auth_spec.rb"));
        assert!(is_test_file("tests/integration.rs"));
        assert!(is_test_file("src/__tests__/Auth.test.tsx"));
        assert!(is_test_file("spec/models/user_spec.rb"));
        assert!(is_test_file("test/test_utils.py"));

        assert!(!is_test_file("src/auth.rs"));
        assert!(!is_test_file("src/controller.py"));
        assert!(!is_test_file("src/utils/helpers.ts"));
    }

    #[test]
    fn test_is_test_file_camel_case() {
        assert!(is_test_file("src/testUtils.ts")); // starts with test + uppercase
        assert!(!is_test_file("src/testimony.py")); // starts with test but no uppercase at char 4
    }

    #[test]
    fn test_empty_codebase() {
        let report = analyze_coverage_gaps(&[], &[], 100);
        assert_eq!(report.total_production_symbols, 0);
        assert_eq!(report.coverage_ratio, 1.0);
        assert!(report.gaps.is_empty());
    }

    #[test]
    fn test_all_covered() {
        let symbols = vec![
            sym(1, "auth::validate", "src/auth.rs", false),
            sym(2, "test_validate", "tests/test_auth.rs", true),
        ];
        let edges = vec![(2, 1)]; // test imports auth

        let report = analyze_coverage_gaps(&symbols, &edges, 100);
        assert_eq!(report.total_production_symbols, 1);
        assert_eq!(report.covered_symbols, 1);
        assert_eq!(report.uncovered_symbols, 0);
        assert_eq!(report.coverage_ratio, 1.0);
        assert!(report.gaps.is_empty());
    }

    #[test]
    fn test_partial_coverage() {
        let symbols = vec![
            sym(1, "auth::validate", "src/auth.rs", false),
            sym(2, "auth::refresh", "src/auth.rs", false),
            sym(3, "db::connect", "src/db.rs", false),
            sym(10, "test_validate", "tests/test_auth.rs", true),
        ];
        let edges = vec![
            (10, 1), // test covers validate
            (1, 3),  // validate uses db::connect (but no test covers db directly)
        ];

        let report = analyze_coverage_gaps(&symbols, &edges, 100);
        assert_eq!(report.total_production_symbols, 3);
        assert_eq!(report.covered_symbols, 1); // only validate is directly tested
        assert_eq!(report.uncovered_symbols, 2);
        assert_eq!(report.gaps.len(), 2);
    }

    #[test]
    fn test_risk_scoring() {
        let symbols = vec![
            sym(1, "core::critical_fn", "src/core.rs", false), // many dependents
            sym(2, "utils::helper", "src/utils.rs", false),    // few dependents
            sym(3, "leaf::standalone", "src/leaf.rs", false),  // no connections
            sym(10, "test1", "tests/t1.rs", true),
            sym(11, "test2", "tests/t2.rs", true),
        ];
        let edges = vec![
            // Many things depend on core::critical_fn
            (10, 2), // test covers utils::helper
            (2, 1),  // utils depends on core
            (3, 1),  // leaf depends on core (incoming edges to symbol 1)
        ];

        let report = analyze_coverage_gaps(&symbols, &edges, 100);

        // core::critical_fn and leaf::standalone are uncovered
        // core has 2 incoming edges (from 2 and 3) → higher risk
        let core_gap = report.gaps.iter().find(|g| g.fqn == "core::critical_fn");
        let leaf_gap = report.gaps.iter().find(|g| g.fqn == "leaf::standalone");

        assert!(core_gap.is_some(), "core should be in gaps");
        assert!(leaf_gap.is_some(), "leaf should be in gaps");

        assert!(
            core_gap.unwrap().risk_score > leaf_gap.unwrap().risk_score,
            "core with more dependents should have higher risk"
        );
    }

    #[test]
    fn test_max_gaps_limit() {
        let mut symbols: Vec<SymbolInfo> = (1..=100)
            .map(|i| {
                sym(
                    i,
                    &format!("mod{i}::fn{i}"),
                    &format!("src/mod{i}.rs"),
                    false,
                )
            })
            .collect();
        symbols.push(sym(200, "test_one", "tests/test.rs", true));

        let edges = vec![(200, 1)]; // only covers symbol 1

        let report = analyze_coverage_gaps(&symbols, &edges, 10);
        assert_eq!(report.gaps.len(), 10); // capped at max_gaps
        assert_eq!(report.uncovered_symbols, 99);
    }

    #[test]
    fn test_risk_score_computation() {
        // No connections → low risk
        assert!(compute_risk_score(0, 0) < 0.01);

        // Many dependents → high risk
        let high_risk = compute_risk_score(20, 0);
        let low_risk = compute_risk_score(1, 0);
        assert!(high_risk > low_risk);

        // Score is always in [0, 1]
        assert!(compute_risk_score(100, 100) <= 1.0);
        assert!(compute_risk_score(0, 0) >= 0.0);
    }

    #[test]
    fn test_test_file_count() {
        let symbols = vec![
            sym(1, "a", "src/a.rs", false),
            sym(2, "b", "tests/test_a.rs", true),
            sym(3, "c", "tests/test_a.rs", true), // same test file
            sym(4, "d", "tests/test_b.rs", true),
        ];
        let report = analyze_coverage_gaps(&symbols, &[], 100);
        assert_eq!(report.test_file_count, 2); // 2 unique test files
    }

    #[test]
    fn test_only_test_files() {
        let symbols = vec![
            sym(1, "test_a", "tests/test_a.rs", true),
            sym(2, "test_b", "tests/test_b.rs", true),
        ];
        let report = analyze_coverage_gaps(&symbols, &[], 100);
        assert_eq!(report.total_production_symbols, 0);
        assert_eq!(report.coverage_ratio, 1.0);
    }
}
