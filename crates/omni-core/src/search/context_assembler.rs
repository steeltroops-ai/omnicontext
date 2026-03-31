//! Context assembly with priority-based packing and compression.
//!
//! Assembles token-budget-aware context windows from search results,
//! prioritizing critical chunks and compressing low-priority ones to
//! fit maximum relevant context within the budget.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::graph::dependencies::FileDependencyGraph;
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
    /// and packs chunks within token budget. After selection, causal
    /// ordering is applied: within-file by line number, across files
    /// by dependency topology when `dep_graph` is provided.
    ///
    /// The effective token budget is scaled by the query intent:
    /// - Debug/Edit: 60% (fewer, high-precision results)
    /// - Refactor/Dependency: 80% (moderate context depth)
    /// - Explain/DataFlow/Generate: 100% (full budget for broad context)
    /// - TestCoverage: 70% (focused on test code)
    /// - Unknown: 100% (no restriction)
    pub fn assemble(
        &self,
        query: &str,
        search_results: Vec<SearchResult>,
        active_file: Option<&PathBuf>,
        dep_graph: Option<&FileDependencyGraph>,
    ) -> ContextWindow {
        // Classify intent and get strategy
        let intent = QueryIntent::classify(query);
        let strategy = intent.context_strategy();

        // Scale token budget by intent
        let budget_fraction = Self::intent_budget_fraction(intent);
        let effective_budget = (self.token_budget as f32 * budget_fraction) as u32;

        // Convert search results to prioritized entries
        let mut entries = self.prioritize_entries(search_results, active_file, &strategy);

        // Sort by priority (highest first), then by score
        entries.sort_by(|a, b| {
            let a_priority = a.priority.unwrap_or(ChunkPriority::Low);
            let b_priority = b.priority.unwrap_or(ChunkPriority::Low);

            b_priority.cmp(&a_priority).then_with(|| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        // Pack within the intent-scaled token budget
        let mut packed = self.pack_with_budget_limit(entries, &strategy, effective_budget);

        // Apply causal ordering: within-file by line number, across files by dependency depth
        apply_causal_ordering(&mut packed.entries, dep_graph);

        ContextWindow {
            entries: packed.entries,
            total_tokens: packed.total_tokens,
            token_budget: self.token_budget,
        }
    }

    /// Compute the budget fraction for a given query intent.
    ///
    /// Focused intents (Debug, Edit) should produce fewer but more precise
    /// results, so they use a smaller fraction of the total budget.
    /// Broad intents (Explain, DataFlow) need full context.
    fn intent_budget_fraction(intent: QueryIntent) -> f32 {
        match intent {
            QueryIntent::Debug => 0.60,
            QueryIntent::Edit => 0.60,
            QueryIntent::TestCoverage => 0.70,
            QueryIntent::Refactor | QueryIntent::Dependency => 0.80,
            QueryIntent::Explain | QueryIntent::DataFlow | QueryIntent::Generate => 1.0,
            QueryIntent::Unknown => 1.0,
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

            let is_test = matches!(result.chunk.kind, crate::types::ChunkKind::Test);

            let priority = ChunkPriority::from_score_and_context(
                result.score, is_active_file, is_test,
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
                    shadow_header: None,
                });
            }
        }

        entries
    }

    /// Pack entries within the default token budget, applying compression as needed.
    #[allow(dead_code)]
    fn pack_with_budget(
        &self,
        entries: Vec<ContextEntry>,
        strategy: &ContextStrategy,
    ) -> ContextWindow {
        self.pack_with_budget_limit(entries, strategy, self.token_budget)
    }

    /// Pack pre-built context entries within a token budget using knapsack DP.
    ///
    /// This is the public entry point used by `assemble_context_window()` in the
    /// search pipeline, which handles file-grouping and GAR injection before
    /// delegating the final packing step here.
    ///
    /// Entries should already have priorities assigned. The strategy controls
    /// whether high-level filtering is applied and other packing behavior.
    ///
    /// After packing, causal ordering is applied: within each file chunks are
    /// sorted by line number ascending; across files by dependency topology when
    /// `dep_graph` is provided.
    pub fn pack_entries_with_strategy(
        &self,
        entries: Vec<ContextEntry>,
        strategy: &ContextStrategy,
        budget: u32,
        dep_graph: Option<&FileDependencyGraph>,
    ) -> ContextWindow {
        let mut window = self.pack_with_budget_limit(entries, strategy, budget);
        apply_causal_ordering(&mut window.entries, dep_graph);
        window
    }

    /// Pack entries within an explicit token budget using 0/1 knapsack optimization.
    ///
    /// The budget may be less than `self.token_budget` when the query intent
    /// requests a tighter context window (e.g., Debug queries use 60%).
    ///
    /// For small-to-medium inputs (N <= 300 items, budget <= 200k tokens), uses
    /// dynamic programming to find the mathematically optimal subset. For larger
    /// inputs, falls back to the greedy priority-sorted approach.
    fn pack_with_budget_limit(
        &self,
        entries: Vec<ContextEntry>,
        strategy: &ContextStrategy,
        budget: u32,
    ) -> ContextWindow {
        if entries.is_empty() {
            return ContextWindow {
                entries: Vec::new(),
                total_tokens: 0,
                token_budget: self.token_budget,
            };
        }

        // Prepare items: apply compression to non-critical entries that exceed budget
        let mut items: Vec<ContextEntry> = Vec::with_capacity(entries.len());
        for mut entry in entries {
            let priority = entry.priority.unwrap_or(ChunkPriority::Low);

            // Try compressed form if chunk is too large
            if entry.chunk.token_count > budget / 2 && priority != ChunkPriority::Critical {
                entry.chunk = self.compress_chunk(&entry.chunk, priority);
            }

            // Skip items that alone exceed the entire budget
            if entry.chunk.token_count <= budget {
                items.push(entry);
            }
        }

        // For high-level strategies, filter out low-priority items early
        if strategy.prioritize_high_level {
            items.retain(|e| e.priority.unwrap_or(ChunkPriority::Low) != ChunkPriority::Low);
        }

        let n = items.len();

        // Decide: knapsack vs greedy fallback
        // Knapsack DP is O(N * B). For N=300, B=200000 -> 60M cells at 1 byte = 60MB.
        // We use a scaled budget (tokens / 4) to reduce table size.
        let scale_factor: u32 = if budget > 50_000 { 4 } else { 1 };
        let scaled_budget = (budget / scale_factor) as usize;

        if n > 300 || scaled_budget > 200_000 {
            // Greedy fallback for very large inputs
            return self.pack_greedy(items, budget);
        }

        // --- 0/1 Knapsack Dynamic Programming ---
        // value[i] = score * priority_weight (higher priority = higher value multiplier)
        // weight[i] = token_count (scaled)
        let values: Vec<f64> = items
            .iter()
            .map(|e| {
                let priority_mult = match e.priority.unwrap_or(ChunkPriority::Low) {
                    ChunkPriority::Critical => 4.0,
                    ChunkPriority::High => 2.0,
                    ChunkPriority::Medium => 1.0,
                    ChunkPriority::Low => 0.5,
                };
                e.score * priority_mult
            })
            .collect();

        let weights: Vec<usize> = items
            .iter()
            .map(|e| (e.chunk.token_count / scale_factor).max(1) as usize)
            .collect();

        // DP table: dp[j] = max value achievable with capacity j
        let cap = scaled_budget;
        let mut dp = vec![0.0_f64; cap + 1];
        // Track which items are selected: keep[i][j] = true if item i is included at capacity j
        let mut keep = vec![vec![false; cap + 1]; n];

        for i in 0..n {
            let w = weights[i];
            let v = values[i];
            // Iterate capacity backwards to avoid using item i twice
            for j in (w..=cap).rev() {
                let with_item = dp[j - w] + v;
                if with_item > dp[j] {
                    dp[j] = with_item;
                    keep[i][j] = true;
                }
            }
        }

        // Trace back to find selected items
        let mut selected = vec![false; n];
        let mut remaining = cap;
        for i in (0..n).rev() {
            if keep[i][remaining] {
                selected[i] = true;
                remaining -= weights[i];
            }
        }

        // Build result preserving original priority order (highest first)
        let mut packed_entries = Vec::new();
        let mut total_tokens: u32 = 0;
        for (i, entry) in items.into_iter().enumerate() {
            if selected[i] {
                total_tokens += entry.chunk.token_count;
                packed_entries.push(entry);
            }
        }

        ContextWindow {
            entries: packed_entries,
            total_tokens,
            token_budget: self.token_budget,
        }
    }

    /// Greedy fallback packer for large inputs where DP is too expensive.
    fn pack_greedy(&self, entries: Vec<ContextEntry>, budget: u32) -> ContextWindow {
        let mut packed_entries = Vec::new();
        let mut total_tokens: u32 = 0;

        for mut entry in entries {
            let priority = entry.priority.unwrap_or(ChunkPriority::Low);
            let chunk_tokens = entry.chunk.token_count;

            if total_tokens + chunk_tokens <= budget {
                total_tokens += chunk_tokens;
                packed_entries.push(entry);
                continue;
            }

            // Try compression
            let compression_factor = priority.compression_factor();
            if compression_factor > 0.0 {
                let compressed = self.compress_chunk(&entry.chunk, priority);
                if total_tokens + compressed.token_count <= budget {
                    total_tokens += compressed.token_count;
                    entry.chunk = compressed;
                    packed_entries.push(entry);
                    continue;
                }
            }

            // Out of budget
            if total_tokens > budget / 2 {
                break;
            }
        }

        ContextWindow {
            entries: packed_entries,
            total_tokens,
            token_budget: budget,
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
            is_summary: chunk.is_summary,
            content_hash: chunk.content_hash,
        }
    }
}

/// Reorder already-selected context entries for natural reading.
///
/// Ordering rules (applied after knapsack selection — never affects which
/// chunks are chosen, only the order they appear in the output):
///
/// **Within a file**: chunks are sorted by `line_start` ascending so the
/// reader encounters them in the same top-to-bottom order as the source file.
///
/// **Across files**: files are sorted by dependency distance from the
/// highest-scored ("anchor") chunk's file, using the `FileDependencyGraph`.
/// Distance=0 (anchor file) comes first, then direct neighbors, then
/// transitive neighbors, then disconnected files. Within the same distance
/// bucket files are ordered by their highest chunk score descending.
///
/// When `dep_graph` is `None` only within-file line ordering is applied.
pub fn apply_causal_ordering(
    entries: &mut Vec<ContextEntry>,
    dep_graph: Option<&FileDependencyGraph>,
) {
    if entries.is_empty() {
        return;
    }

    // --- Step A: Group by file, sort within each group by line_start ---
    // Collect unique file paths in their current (score-based) order so that
    // the cross-file ordering below can preserve or override that sequence.
    let mut file_order: Vec<PathBuf> = Vec::new();
    let mut by_file: HashMap<PathBuf, Vec<usize>> = HashMap::new();

    for (idx, entry) in entries.iter().enumerate() {
        let key = entry.file_path.clone();
        let group = by_file.entry(key.clone()).or_default();
        if group.is_empty() {
            file_order.push(key);
        }
        group.push(idx);
    }

    // Sort each file's chunk indices by line_start ascending
    for indices in by_file.values_mut() {
        indices.sort_by_key(|&i| entries[i].chunk.line_start);
    }

    // --- Step B: Determine cross-file ordering ---
    // Anchor = file of the highest-scored entry (first entry after score sort)
    let anchor_file = entries[0].file_path.clone();

    // Build distance map: file_path → hop distance from anchor
    let distance_map: HashMap<PathBuf, usize> = if let Some(graph) = dep_graph {
        match graph.get_neighbors(&anchor_file, 3) {
            Ok(neighbors) => {
                let mut map: HashMap<PathBuf, usize> = HashMap::new();
                map.insert(anchor_file.clone(), 0);
                for n in neighbors {
                    map.insert(n.path, n.distance);
                }
                map
            }
            Err(e) => {
                tracing::warn!(error = %e, "causal ordering: dep graph query failed, falling back to line-only order");
                let mut map = HashMap::new();
                map.insert(anchor_file.clone(), 0);
                map
            }
        }
    } else {
        // No graph — anchor gets 0, everything else is "unreachable"
        let mut map = HashMap::new();
        map.insert(anchor_file.clone(), 0);
        map
    };

    // For each file, compute (distance, neg_max_score) for sorting.
    // Unreachable files sort last (distance = usize::MAX).
    let file_sort_key = |path: &PathBuf| -> (usize, i64) {
        let dist = distance_map.get(path).copied().unwrap_or(usize::MAX);
        // Highest chunk score in this file (use negative for ascending sort trick)
        let max_score_neg = by_file
            .get(path)
            .and_then(|idxs| idxs.first())
            // After inner sort by line_start, we need the max score across the group
            .map(|_| {
                by_file[path]
                    .iter()
                    .map(|&i| (entries[i].score * -1e9) as i64)
                    .min()
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        (dist, max_score_neg)
    };

    file_order.sort_by_key(|p| file_sort_key(p));

    // --- Reconstruct entries in causal order ---
    let mut result: Vec<ContextEntry> = Vec::with_capacity(entries.len());
    for file_path in &file_order {
        if let Some(indices) = by_file.get(file_path) {
            for &idx in indices {
                // We'll drain from entries via swap; use a sentinel approach instead:
                // collect owned entries after rebuilding the indices vec.
                let _ = idx; // processed below
            }
        }
    }

    // Build owned result by taking entries out in order
    // Safety: we consume `entries` fully via indexed access — all indices
    // in `by_file` cover exactly [0, entries.len()) with no duplicates.
    let mut taken: Vec<Option<ContextEntry>> = entries.drain(..).map(Some).collect();
    for file_path in &file_order {
        if let Some(indices) = by_file.get(file_path) {
            for &idx in indices {
                if let Some(entry) = taken[idx].take() {
                    result.push(entry);
                }
            }
        }
    }

    *entries = result;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChunkKind, ScoreBreakdown, Visibility};

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
            is_summary: false,
            content_hash: 0,
        }
    }

    fn make_test_result(chunk: Chunk, score: f64) -> SearchResult {
        SearchResult {
            chunk,
            file_path: PathBuf::from("test.rs"),
            score,
            score_breakdown: ScoreBreakdown::default(),
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

        let results = vec![make_test_result(chunk1, 0.9), make_test_result(chunk2, 0.8)];

        let context = assembler.assemble("fix the bug", results, None, None);

        assert_eq!(context.entries.len(), 2);
        assert!(context.total_tokens <= 1000);
    }

    #[test]
    fn test_assemble_exceeds_budget() {
        let assembler = ContextAssembler::new(150);

        let chunk1 = make_test_chunk("fn test1() { }", 100);
        let chunk2 = make_test_chunk("fn test2() { }", 100);

        let results = vec![make_test_result(chunk1, 0.9), make_test_result(chunk2, 0.8)];

        let context = assembler.assemble("fix the bug", results, None, None);

        // With compression, we might fit both chunks (second one compressed)
        // The important thing is we don't exceed the budget
        assert!(!context.entries.is_empty());
        assert!(context.entries.len() <= 2);
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

        let context = assembler.assemble("fix the bug", results, None, None);

        // High priority should come first
        assert_eq!(context.entries[0].chunk.symbol_path, "test::function");
        assert!(context.entries[0].score > 0.8);
    }

    #[test]
    fn test_intent_budget_debug_is_reduced() {
        let fraction = ContextAssembler::intent_budget_fraction(QueryIntent::Debug);
        assert!(
            (fraction - 0.60).abs() < 1e-6,
            "debug should use 60% budget"
        );
    }

    #[test]
    fn test_intent_budget_explain_is_full() {
        let fraction = ContextAssembler::intent_budget_fraction(QueryIntent::Explain);
        assert!(
            (fraction - 1.0).abs() < 1e-6,
            "explain should use 100% budget"
        );
    }

    #[test]
    fn test_intent_budget_refactor_is_moderate() {
        let fraction = ContextAssembler::intent_budget_fraction(QueryIntent::Refactor);
        assert!(
            (fraction - 0.80).abs() < 1e-6,
            "refactor should use 80% budget"
        );
    }

    #[test]
    fn test_debug_query_gets_fewer_tokens() {
        let assembler = ContextAssembler::new(1000);

        // Create enough results to exceed budget (high scores -> High/Critical priority)
        let results: Vec<SearchResult> = (0..20)
            .map(|i| make_test_result(make_test_chunk(&format!("fn f_{i}() {{ }}",), 80), 0.9))
            .collect();

        // Debug query -> 60% budget = 600 tokens
        let debug_ctx =
            assembler.assemble("why is this function failing?", results.clone(), None, None);
        // "Unknown" intent -> 100% budget = 1000 tokens, no high_level filter
        let full_ctx = assembler.assemble("f_0 f_1 f_2", results, None, None);

        assert!(
            debug_ctx.total_tokens <= full_ctx.total_tokens,
            "debug ({}) should use <= tokens than full ({})",
            debug_ctx.total_tokens,
            full_ctx.total_tokens
        );
    }

    #[test]
    fn test_knapsack_beats_greedy() {
        // Scenario: budget = 250 tokens
        // Option A (greedy picks first): 1 chunk with 200 tokens, score 0.9
        // Option B (knapsack optimal):   3 chunks with 80 tokens each, score 0.85
        // Greedy picks A (200 tokens) then cannot fit any B (200+80=280 > 250).
        // Knapsack picks all 3 B items (240 tokens, total value = 3*0.85*2.0 = 5.1)
        // which beats A alone (1*0.9*2.0 = 1.8).
        let assembler = ContextAssembler::new(250);

        let big_chunk = make_test_chunk(
            "fn big() {\n  // lots of code\n  a();\n  b();\n  c();\n}",
            200,
        );
        let small1 = Chunk {
            id: 2,
            symbol_path: "test::small1".to_string(),
            ..make_test_chunk("fn small1() { }", 80)
        };
        let small2 = Chunk {
            id: 3,
            symbol_path: "test::small2".to_string(),
            ..make_test_chunk("fn small2() { }", 80)
        };
        let small3 = Chunk {
            id: 4,
            symbol_path: "test::small3".to_string(),
            ..make_test_chunk("fn small3() { }", 80)
        };

        let results = vec![
            make_test_result(big_chunk, 0.9), // High priority (score > 0.8)
            make_test_result(small1, 0.85),
            make_test_result(small2, 0.85),
            make_test_result(small3, 0.85),
        ];

        // "f_0 f_1" is Unknown intent -> 100% budget = 250
        let context = assembler.assemble("f_0 f_1", results, None, None);

        // Knapsack should pick 3 small chunks (240 tokens) over 1 big (200 tokens)
        // because total value is higher (3 * 0.85 * priority_mult > 1 * 0.9 * priority_mult)
        assert!(
            context.entries.len() >= 3,
            "knapsack should pick 3 small chunks, got {}",
            context.entries.len()
        );
        assert!(
            context.total_tokens <= 250,
            "should stay within budget, got {}",
            context.total_tokens
        );
    }

    // Helper: build a ContextEntry with explicit file_path, line_start, and score
    fn make_entry(file: &str, line_start: u32, score: f64) -> ContextEntry {
        let chunk = Chunk {
            id: 0,
            file_id: 0,
            symbol_path: format!("{}::fn", file),
            kind: ChunkKind::Function,
            visibility: Visibility::Public,
            line_start,
            line_end: line_start + 5,
            content: format!("fn at_line_{line_start}() {{}}"),
            doc_comment: None,
            token_count: 20,
            weight: 1.0,
            vector_id: None,
            is_summary: false,
            content_hash: 0,
        };
        ContextEntry {
            file_path: PathBuf::from(file),
            chunk,
            score,
            is_graph_neighbor: false,
            priority: Some(ChunkPriority::High),
            shadow_header: None,
        }
    }

    #[test]
    fn test_within_file_ordering_by_line() {
        // Two chunks from the same file delivered in reverse line order.
        // apply_causal_ordering must sort them ascending by line_start.
        let mut entries = vec![
            make_entry("src/lib.rs", 50, 0.9), // higher score, later line
            make_entry("src/lib.rs", 10, 0.7), // lower score, earlier line
        ];

        apply_causal_ordering(&mut entries, None);

        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].chunk.line_start, 10,
            "first chunk should be at line 10"
        );
        assert_eq!(
            entries[1].chunk.line_start, 50,
            "second chunk should be at line 50"
        );
    }

    #[test]
    fn test_cross_file_import_before_implementation() {
        // File A imports file B.  Anchor is A (highest score).
        // After causal ordering: A chunks before B chunks.
        use crate::graph::dependencies::{
            DependencyEdge as FileDepEdge, EdgeType, FileDependencyGraph,
        };

        let graph = FileDependencyGraph::new();
        let file_a = PathBuf::from("src/a.rs");
        let file_b = PathBuf::from("src/b.rs");

        graph
            .add_edge(&FileDepEdge {
                source: file_a.clone(),
                target: file_b.clone(),
                edge_type: EdgeType::Imports,
                weight: 1.0,
            })
            .expect("add edge a->b");

        // entries: b chunk arrives first (e.g. original heap order), then a chunk
        let mut entries = vec![
            {
                let mut e = make_entry("src/b.rs", 1, 0.7);
                e.file_path = file_b.clone();
                e
            },
            {
                let mut e = make_entry("src/a.rs", 1, 0.9); // anchor: highest score
                e.file_path = file_a.clone();
                e
            },
        ];

        apply_causal_ordering(&mut entries, Some(&graph));

        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].file_path, file_a,
            "anchor file a should appear first"
        );
        assert_eq!(
            entries[1].file_path, file_b,
            "dependency file b should appear second"
        );
    }

    #[test]
    fn test_causal_ordering_no_graph() {
        // With dep_graph=None, within-file line ordering still applies.
        // Two files: each with two chunks in reverse order.
        let mut entries = vec![
            make_entry("src/x.rs", 80, 0.9), // anchor file, higher line
            make_entry("src/x.rs", 20, 0.8), // anchor file, lower line
            make_entry("src/y.rs", 60, 0.7),
            make_entry("src/y.rs", 5, 0.6),
        ];

        apply_causal_ordering(&mut entries, None);

        assert_eq!(entries.len(), 4);

        // x.rs chunks should be sorted by line within the file group
        let x_entries: Vec<_> = entries
            .iter()
            .filter(|e| e.file_path == PathBuf::from("src/x.rs"))
            .collect();
        assert_eq!(x_entries[0].chunk.line_start, 20);
        assert_eq!(x_entries[1].chunk.line_start, 80);

        // y.rs chunks should be sorted by line within the file group
        let y_entries: Vec<_> = entries
            .iter()
            .filter(|e| e.file_path == PathBuf::from("src/y.rs"))
            .collect();
        assert_eq!(y_entries[0].chunk.line_start, 5);
        assert_eq!(y_entries[1].chunk.line_start, 60);
    }
}
