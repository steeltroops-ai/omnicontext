//! Context assembly with priority-based packing and compression.
//!
//! Assembles token-budget-aware context windows from search results,
//! prioritizing critical chunks and compressing low-priority ones to
//! fit maximum relevant context within the budget.

use std::path::PathBuf;

use crate::search::intent::{ContextStrategy, QueryIntent};
use crate::types::{Chunk, ChunkPriority, ContextEntry, ContextWindow, SearchResult};

/// Context assembler with priority-based packing.
pub struct ContextAssembler {
    /// Token budget for the context window.
    token_budget: u32,
}

impl ContextAssembler {
    /// Create a new context assembler with the given token budget.
    pub fn new(token_budget: u32) -> Self {
        Self { token_budget }
    }

    /// Assemble a context window from search results.
    ///
    /// Applies intent-based context strategy, assigns priorities,
    /// and packs chunks within token budget.
    pub fn assemble(
        &self,
        query: &str,
        search_results: Vec<SearchResult>,
        active_file: Option<&PathBuf>,
    ) -> ContextWindow {
        // Classify intent and get strategy
        let intent = QueryIntent::classify(query);
        let strategy = intent.context_strategy();

        // Convert search results to prioritized entries
        let mut entries = self.prioritize_entries(search_results, active_file, &strategy);

        // Sort by priority (highest first), then by score
        entries.sort_by(|a, b| {
            let a_priority = a.priority.unwrap_or(ChunkPriority::Low);
            let b_priority = b.priority.unwrap_or(ChunkPriority::Low);

            b_priority
                .cmp(&a_priority)
                .then_with(|| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
        });

        // Pack within token budget
        let packed = self.pack_with_budget(entries, &strategy);

        ContextWindow {
            entries: packed.entries,
            total_tokens: packed.total_tokens,
            token_budget: self.token_budget,
        }
    }

    /// Assign priorities to search results based on context and strategy.
    fn prioritize_entries(
        &self,
        results: Vec<SearchResult>,
        active_file: Option<&PathBuf>,
        strategy: &ContextStrategy,
    ) -> Vec<ContextEntry> {
        let mut entries = Vec::with_capacity(results.len());

        for result in results {
            let is_active_file = active_file
                .map(|af| af == &result.file_path)
                .unwrap_or(false);

            let is_test = matches!(
                result.chunk.kind,
                crate::types::ChunkKind::Test
            );

            let priority = ChunkPriority::from_score_and_context(
                result.score,
                is_active_file,
                is_test,
                false, // is_graph_neighbor set later if needed
            );

            // Apply strategy filters
            let should_include = match result.chunk.kind {
                crate::types::ChunkKind::Test => strategy.include_tests,
                crate::types::ChunkKind::Module => strategy.include_architecture,
                _ => strategy.include_implementation,
            };

            if should_include {
                entries.push(ContextEntry {
                    file_path: result.file_path,
                    chunk: result.chunk,
                    score: result.score,
                    is_graph_neighbor: false,
                    priority: Some(priority),
                });
            }
        }

        entries
    }

    /// Pack entries within token budget, applying compression as needed.
    fn pack_with_budget(
        &self,
        entries: Vec<ContextEntry>,
        strategy: &ContextStrategy,
    ) -> ContextWindow {
        let mut packed_entries = Vec::new();
        let mut total_tokens: u32 = 0;

        for mut entry in entries {
            let priority = entry.priority.unwrap_or(ChunkPriority::Low);
            let mut chunk_tokens = entry.chunk.token_count;

            // Try to fit without compression first
            if total_tokens + chunk_tokens <= self.token_budget {
                total_tokens += chunk_tokens;
                packed_entries.push(entry);
                continue;
            }

            // If critical priority, try compression
            if priority == ChunkPriority::Critical {
                let compressed = self.compress_chunk(&entry.chunk, priority);
                chunk_tokens = compressed.token_count;

                if total_tokens + chunk_tokens <= self.token_budget {
                    entry.chunk = compressed;
                    total_tokens += chunk_tokens;
                    packed_entries.push(entry);
                }
                continue;
            }

            // For non-critical, try compression if we have room
            let compression_factor = priority.compression_factor();
            if compression_factor > 0.0 {
                let compressed = self.compress_chunk(&entry.chunk, priority);
                chunk_tokens = compressed.token_count;

                if total_tokens + chunk_tokens <= self.token_budget {
                    entry.chunk = compressed;
                    total_tokens += chunk_tokens;
                    packed_entries.push(entry);
                    continue;
                }
            }

            // If we're prioritizing high-level and this is a detail, skip
            if strategy.prioritize_high_level && priority == ChunkPriority::Low {
                continue;
            }

            // Otherwise, we're out of budget
            break;
        }

        ContextWindow {
            entries: packed_entries,
            total_tokens,
            token_budget: self.token_budget,
        }
    }

    /// Compress a chunk based on its priority.
    ///
    /// Compression strategies:
    /// - Critical: No compression (should never be called)
    /// - High: Keep signature + first few lines
    /// - Medium: Keep signature + summary
    /// - Low: Keep signature only
    fn compress_chunk(&self, chunk: &Chunk, priority: ChunkPriority) -> Chunk {
        let compression_factor = priority.compression_factor();

        if compression_factor == 0.0 {
            return chunk.clone();
        }

        let lines: Vec<&str> = chunk.content.lines().collect();
        if lines.is_empty() {
            return chunk.clone();
        }

        // Extract signature (first line, usually function/class declaration)
        let signature = lines.first().unwrap_or(&"");

        let compressed_content = match priority {
            ChunkPriority::Critical => chunk.content.clone(), // Should never happen
            ChunkPriority::High => {
                // Keep signature + first 5 lines of body
                let keep_lines = 6.min(lines.len());
                let mut content = lines[..keep_lines].join("\n");
                if lines.len() > keep_lines {
                    content.push_str("\n  // ... (truncated)");
                }
                content
            }
            ChunkPriority::Medium => {
                // Keep signature + summary comment
                let mut content = signature.to_string();
                if let Some(doc) = &chunk.doc_comment {
                    let summary = doc.lines().next().unwrap_or("");
                    if !summary.is_empty() {
                        content.push_str(&format!("\n  // {summary}"));
                    }
                }
                content.push_str("\n  // ... (implementation omitted)");
                content
            }
            ChunkPriority::Low => {
                // Keep signature only
                let mut content = signature.to_string();
                content.push_str(" { /* ... */ }");
                content
            }
        };

        // Estimate new token count
        let new_token_count = (compressed_content.len() / 4).max(1) as u32;

        Chunk {
            id: chunk.id,
            file_id: chunk.file_id,
            symbol_path: chunk.symbol_path.clone(),
            kind: chunk.kind,
            visibility: chunk.visibility,
            line_start: chunk.line_start,
            line_end: chunk.line_end,
            content: compressed_content,
            doc_comment: chunk.doc_comment.clone(),
            token_count: new_token_count,
            weight: chunk.weight,
            vector_id: chunk.vector_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChunkKind, Visibility};

    fn make_test_chunk(content: &str, token_count: u32) -> Chunk {
        Chunk {
            id: 1,
            file_id: 1,
            symbol_path: "test::function".to_string(),
            kind: ChunkKind::Function,
            visibility: Visibility::Public,
            line_start: 1,
            line_end: 10,
            content: content.to_string(),
            doc_comment: Some("Test function".to_string()),
            token_count,
            weight: 0.85,
            vector_id: Some(1),
        }
    }

    fn make_test_result(chunk: Chunk, score: f64) -> SearchResult {
        SearchResult {
            chunk,
            file_path: PathBuf::from("test.rs"),
            score,
            score_breakdown: Default::default(),
        }
    }

    #[test]
    fn test_priority_from_score() {
        // High score
        let priority = ChunkPriority::from_score_and_context(0.9, false, false, false);
        assert_eq!(priority, ChunkPriority::High);

        // Medium score
        let priority = ChunkPriority::from_score_and_context(0.6, false, false, false);
        assert_eq!(priority, ChunkPriority::Medium);

        // Low score
        let priority = ChunkPriority::from_score_and_context(0.3, false, false, false);
        assert_eq!(priority, ChunkPriority::Low);
    }

    #[test]
    fn test_priority_active_file() {
        // Active file is always critical
        let priority = ChunkPriority::from_score_and_context(0.1, true, false, false);
        assert_eq!(priority, ChunkPriority::Critical);
    }

    #[test]
    fn test_priority_test_file() {
        // Test files are high priority
        let priority = ChunkPriority::from_score_and_context(0.5, false, true, false);
        assert_eq!(priority, ChunkPriority::High);
    }

    #[test]
    fn test_compression_factors() {
        assert_eq!(ChunkPriority::Critical.compression_factor(), 0.0);
        assert_eq!(ChunkPriority::High.compression_factor(), 0.1);
        assert_eq!(ChunkPriority::Medium.compression_factor(), 0.3);
        assert_eq!(ChunkPriority::Low.compression_factor(), 0.6);
    }

    #[test]
    fn test_assemble_within_budget() {
        let assembler = ContextAssembler::new(1000);

        let chunk1 = make_test_chunk("fn test1() { }", 100);
        let chunk2 = make_test_chunk("fn test2() { }", 100);

        let results = vec![
            make_test_result(chunk1, 0.9),
            make_test_result(chunk2, 0.8),
        ];

        let context = assembler.assemble("fix the bug", results, None);

        assert_eq!(context.entries.len(), 2);
        assert!(context.total_tokens <= 1000);
    }

    #[test]
    fn test_assemble_exceeds_budget() {
        let assembler = ContextAssembler::new(150);

        let chunk1 = make_test_chunk("fn test1() { }", 100);
        let chunk2 = make_test_chunk("fn test2() { }", 100);

        let results = vec![
            make_test_result(chunk1, 0.9),
            make_test_result(chunk2, 0.8),
        ];

        let context = assembler.assemble("fix the bug", results, None);

        // Should only include first chunk
        assert_eq!(context.entries.len(), 1);
        assert!(context.total_tokens <= 150);
    }

    #[test]
    fn test_compress_high_priority() {
        let assembler = ContextAssembler::new(1000);

        let content = "fn test() {\n  line1();\n  line2();\n  line3();\n  line4();\n  line5();\n  line6();\n  line7();\n}";
        let chunk = make_test_chunk(content, 200);

        let compressed = assembler.compress_chunk(&chunk, ChunkPriority::High);

        // Should keep signature + first 5 lines
        assert!(compressed.content.contains("fn test()"));
        assert!(compressed.content.contains("line1"));
        assert!(compressed.token_count < chunk.token_count);
    }

    #[test]
    fn test_compress_medium_priority() {
        let assembler = ContextAssembler::new(1000);

        let content = "fn test() {\n  line1();\n  line2();\n}";
        let chunk = make_test_chunk(content, 100);

        let compressed = assembler.compress_chunk(&chunk, ChunkPriority::Medium);

        // Should keep signature + doc comment summary
        assert!(compressed.content.contains("fn test()"));
        assert!(compressed.content.contains("Test function"));
        assert!(compressed.content.contains("implementation omitted"));
        assert!(compressed.token_count < chunk.token_count);
    }

    #[test]
    fn test_compress_low_priority() {
        let assembler = ContextAssembler::new(1000);

        let content = "fn test() {\n  line1();\n  line2();\n}";
        let chunk = make_test_chunk(content, 100);

        let compressed = assembler.compress_chunk(&chunk, ChunkPriority::Low);

        // Should keep signature only
        assert!(compressed.content.contains("fn test()"));
        assert!(compressed.content.contains("{ /* ... */ }"));
        assert!(compressed.token_count < chunk.token_count);
    }

    #[test]
    fn test_priority_ordering() {
        let assembler = ContextAssembler::new(1000);

        let chunk1 = make_test_chunk("fn low() { }", 100);
        let chunk2 = make_test_chunk("fn high() { }", 100);

        let results = vec![
            make_test_result(chunk1, 0.3), // Low priority
            make_test_result(chunk2, 0.9), // High priority
        ];

        let context = assembler.assemble("fix the bug", results, None);

        // High priority should come first
        assert_eq!(context.entries[0].chunk.symbol_path, "test::function");
        assert!(context.entries[0].score > 0.8);
    }
}

