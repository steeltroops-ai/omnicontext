//! Convention and pattern recognition engine.
//!
//! Detects recurring code patterns across the codebase:
//! error handling conventions, logging patterns, authentication flows,
//! naming conventions, etc.

use crate::error::OmniResult;
use crate::index::MetadataIndex;

/// A detected code pattern / convention.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Pattern {
    /// Pattern identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Category (error_handling, logging, naming, etc.).
    pub category: PatternCategory,
    /// Number of files where this pattern was found.
    pub file_count: usize,
    /// Example file paths.
    pub examples: Vec<String>,
    /// Confidence score 0.0 - 1.0.
    pub confidence: f64,
}

/// Categories of detectable patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternCategory {
    /// Error handling patterns (Result, try/except, .unwrap, etc.)
    ErrorHandling,
    /// Logging patterns (tracing, log, print, etc.)
    Logging,
    /// Naming conventions (snake_case, camelCase, etc.)
    NamingConvention,
    /// Testing patterns (test structure, assertions, mocking)
    Testing,
    /// Documentation patterns (doc comments, README references)
    Documentation,
    /// Architecture patterns (MVC, layered, modular)
    Architecture,
}

/// Pattern recognition engine.
pub struct PatternEngine;

impl PatternEngine {
    /// Analyze the codebase for common patterns.
    pub fn analyze(index: &MetadataIndex) -> OmniResult<Vec<Pattern>> {
        let mut patterns = Vec::new();

        patterns.extend(Self::detect_error_patterns(index)?);
        patterns.extend(Self::detect_logging_patterns(index)?);
        patterns.extend(Self::detect_naming_patterns(index)?);
        patterns.extend(Self::detect_testing_patterns(index)?);
        patterns.extend(Self::detect_doc_patterns(index)?);

        // Sort by confidence descending
        patterns.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(patterns)
    }

    /// Detect error handling patterns.
    fn detect_error_patterns(index: &MetadataIndex) -> OmniResult<Vec<Pattern>> {
        let mut patterns = Vec::new();
        let conn = index.connection();

        // Check for Result/? pattern (Rust)
        let result_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE content LIKE '%-> Result<%' OR content LIKE '%-> OmniResult<%'",
            [],
            |row| row.get(0),
        )?;

        let unwrap_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE content LIKE '%.unwrap()%'",
            [],
            |row| row.get(0),
        )?;

        let total_fns: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE kind = 'function'",
            [],
            |row| row.get(0),
        )?;

        if total_fns > 0 {
            let result_ratio = result_count as f64 / total_fns as f64;
            if result_ratio > 0.3 {
                patterns.push(Pattern {
                    id: "rust_result_pattern".into(),
                    description: format!(
                        "Uses Result<T, E> return pattern ({:.0}% of functions)",
                        result_ratio * 100.0
                    ),
                    category: PatternCategory::ErrorHandling,
                    file_count: result_count as usize,
                    examples: Vec::new(),
                    confidence: result_ratio.min(1.0),
                });
            }

            if unwrap_count > 0 {
                let unwrap_ratio = unwrap_count as f64 / total_fns as f64;
                patterns.push(Pattern {
                    id: "unwrap_usage".into(),
                    description: format!(
                        "Uses .unwrap() in {} locations ({:.0}% of functions)",
                        unwrap_count,
                        unwrap_ratio * 100.0
                    ),
                    category: PatternCategory::ErrorHandling,
                    file_count: unwrap_count as usize,
                    examples: Vec::new(),
                    confidence: 0.9,
                });
            }
        }

        Ok(patterns)
    }

    /// Detect logging patterns.
    fn detect_logging_patterns(index: &MetadataIndex) -> OmniResult<Vec<Pattern>> {
        let mut patterns = Vec::new();
        let conn = index.connection();

        let tracing_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE content LIKE '%tracing::%'",
            [],
            |row| row.get(0),
        )?;

        let println_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE content LIKE '%println!%' OR content LIKE '%print!%'",
            [],
            |row| row.get(0),
        )?;

        if tracing_count > 0 {
            patterns.push(Pattern {
                id: "structured_logging".into(),
                description: format!(
                    "Uses structured logging (tracing) in {} chunks",
                    tracing_count
                ),
                category: PatternCategory::Logging,
                file_count: tracing_count as usize,
                examples: Vec::new(),
                confidence: 0.95,
            });
        }

        if println_count > 0 {
            patterns.push(Pattern {
                id: "println_usage".into(),
                description: format!(
                    "Uses println!/print! in {} chunks (consider tracing instead)",
                    println_count
                ),
                category: PatternCategory::Logging,
                file_count: println_count as usize,
                examples: Vec::new(),
                confidence: 0.8,
            });
        }

        Ok(patterns)
    }

    /// Detect naming convention patterns.
    fn detect_naming_patterns(index: &MetadataIndex) -> OmniResult<Vec<Pattern>> {
        let mut patterns = Vec::new();
        let conn = index.connection();

        let snake_case: i64 = conn.query_row(
            "SELECT COUNT(*) FROM symbols WHERE name LIKE '%_%' AND name NOT LIKE '%-%'",
            [],
            |row| row.get(0),
        )?;

        let total_symbols: i64 = conn.query_row(
            "SELECT COUNT(*) FROM symbols",
            [],
            |row| row.get(0),
        )?;

        if total_symbols > 0 {
            let ratio = snake_case as f64 / total_symbols as f64;
            if ratio > 0.5 {
                patterns.push(Pattern {
                    id: "snake_case_naming".into(),
                    description: format!(
                        "Uses snake_case naming convention ({:.0}% of symbols)",
                        ratio * 100.0
                    ),
                    category: PatternCategory::NamingConvention,
                    file_count: snake_case as usize,
                    examples: Vec::new(),
                    confidence: ratio.min(1.0),
                });
            }
        }

        Ok(patterns)
    }

    /// Detect testing patterns.
    fn detect_testing_patterns(index: &MetadataIndex) -> OmniResult<Vec<Pattern>> {
        let mut patterns = Vec::new();
        let conn = index.connection();

        let test_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE kind = 'test'",
            [],
            |row| row.get(0),
        )?;

        let total_fns: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE kind = 'function'",
            [],
            |row| row.get(0),
        )?;

        if test_count > 0 && total_fns > 0 {
            let ratio = test_count as f64 / total_fns as f64;
            patterns.push(Pattern {
                id: "test_coverage_pattern".into(),
                description: format!(
                    "{} test functions for {} non-test functions (ratio: {:.2})",
                    test_count, total_fns, ratio
                ),
                category: PatternCategory::Testing,
                file_count: test_count as usize,
                examples: Vec::new(),
                confidence: (ratio * 2.0).min(1.0), // 0.5 ratio = 1.0 confidence
            });
        }

        Ok(patterns)
    }

    /// Detect documentation patterns.
    fn detect_doc_patterns(index: &MetadataIndex) -> OmniResult<Vec<Pattern>> {
        let mut patterns = Vec::new();
        let conn = index.connection();

        let documented: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE doc_comment IS NOT NULL AND doc_comment != ''",
            [],
            |row| row.get(0),
        )?;

        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE kind IN ('function', 'class', 'trait')",
            [],
            |row| row.get(0),
        )?;

        if total > 0 {
            let ratio = documented as f64 / total as f64;
            patterns.push(Pattern {
                id: "documentation_coverage".into(),
                description: format!(
                    "{:.0}% of functions/classes have doc comments ({}/{})",
                    ratio * 100.0, documented, total
                ),
                category: PatternCategory::Documentation,
                file_count: documented as usize,
                examples: Vec::new(),
                confidence: ratio.min(1.0),
            });
        }

        Ok(patterns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_category_serialization() {
        let json = serde_json::to_string(&PatternCategory::ErrorHandling).expect("serialize");
        assert_eq!(json, "\"error_handling\"");

        let json = serde_json::to_string(&PatternCategory::NamingConvention).expect("serialize");
        assert_eq!(json, "\"naming_convention\"");
    }
}
