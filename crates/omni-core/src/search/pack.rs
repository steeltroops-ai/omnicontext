//! Packed context window assembly for agent-oriented consumption.
//!
//! `pack_context_window()` (see `crate::pipeline::Engine`) produces a flat
//! ordered JSON-serialisable list of [`PackedContextEntry`] values that fills
//! a caller-specified token budget with minimum redundancy.
//!
//! ## Algorithm
//!
//! 1. Search with rerank to obtain `Vec<SearchResult>`.
//! 2. Sort results by `(file_path, line_start)`.
//! 3. Merge adjacent same-file chunks where `chunk[i].line_end + 1 >= chunk[i+1].line_start`.
//!    Merged entry: `score = max(scores)`, `token_count = sum(token_counts)`,
//!    content = joined with `\n`.
//! 4. Re-sort merged list by `score` descending.
//! 5. Greedy pack: accumulate entries while `running_total + entry.token_count <= token_budget`.
//! 6. Return packed list.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::types::ChunkKind;

/// A single entry in a packed context window.
///
/// When adjacent same-file chunks are coalesced, `content` contains both
/// chunks joined with `\n`, `token_count` is the sum, and `line_end` extends
/// to the last merged chunk's boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackedContextEntry {
    /// File containing this chunk.
    pub file_path: PathBuf,
    /// Fully-qualified symbol path of the primary (or first merged) chunk.
    pub symbol_path: String,
    /// First line of the entry (1-based).
    pub line_start: u32,
    /// Last line of the entry (1-based, inclusive).
    pub line_end: u32,
    /// Source content.  Adjacent same-file chunks are joined with `\n`.
    pub content: String,
    /// Sum of token counts across all merged chunks.
    pub token_count: u32,
    /// Maximum relevance score across all merged chunks.
    pub score: f64,
    /// Kind of the primary chunk.
    pub kind: ChunkKind,
}

/// Merge adjacent same-file entries in `entries` (already sorted by
/// `(file_path, line_start)`).
///
/// Two entries are considered adjacent when:
///   `current.file_path == next.file_path && current.line_end + 1 >= next.line_start`
///
/// This collapses runs of close-together chunks (e.g. a function followed
/// immediately by its doc-test) into a single entry, reducing context
/// fragmentation for downstream agents.
pub fn merge_adjacent(entries: Vec<PackedContextEntry>) -> Vec<PackedContextEntry> {
    let mut merged: Vec<PackedContextEntry> = Vec::with_capacity(entries.len());

    for entry in entries {
        match merged.last_mut() {
            Some(prev)
                if prev.file_path == entry.file_path
                    && prev.line_end.saturating_add(1) >= entry.line_start =>
            {
                // Merge into previous entry.
                prev.line_end = prev.line_end.max(entry.line_end);
                prev.score = prev.score.max(entry.score);
                prev.token_count += entry.token_count;
                prev.content.push('\n');
                prev.content.push_str(&entry.content);
            }
            _ => merged.push(entry),
        }
    }

    merged
}

/// Greedy token-budget packing.
///
/// Iterates `entries` (assumed sorted by score descending) and accumulates
/// entries while the running token total remains within `token_budget`.
/// Returns the packed subset together with the final token total.
pub fn greedy_pack(
    entries: Vec<PackedContextEntry>,
    token_budget: u32,
) -> (Vec<PackedContextEntry>, u32) {
    let mut packed: Vec<PackedContextEntry> = Vec::new();
    let mut total: u32 = 0;

    for entry in entries {
        let next = total.saturating_add(entry.token_count);
        if next <= token_budget {
            total = next;
            packed.push(entry);
        }
    }

    (packed, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(
        file: &str,
        line_start: u32,
        line_end: u32,
        tokens: u32,
        score: f64,
    ) -> PackedContextEntry {
        PackedContextEntry {
            file_path: PathBuf::from(file),
            symbol_path: format!("{}::{}", file, line_start),
            line_start,
            line_end,
            content: format!("// lines {line_start}-{line_end}"),
            token_count: tokens,
            score,
            kind: ChunkKind::Function,
        }
    }

    #[test]
    fn test_merge_adjacent_same_file_consecutive() {
        // Lines 10-20 followed by 21-30 in the same file should merge.
        let entries = vec![
            entry("src/a.rs", 10, 20, 30, 0.9),
            entry("src/a.rs", 21, 30, 25, 0.7),
        ];
        let merged = merge_adjacent(entries);
        assert_eq!(merged.len(), 1, "consecutive chunks should merge");
        assert_eq!(merged[0].line_start, 10);
        assert_eq!(merged[0].line_end, 30);
        assert_eq!(merged[0].token_count, 55, "token_count = sum");
        assert!((merged[0].score - 0.9).abs() < 1e-9, "score = max");
    }

    #[test]
    fn test_merge_adjacent_different_files_not_merged() {
        let entries = vec![
            entry("src/a.rs", 1, 10, 20, 0.8),
            entry("src/b.rs", 1, 10, 20, 0.6),
        ];
        let merged = merge_adjacent(entries);
        assert_eq!(merged.len(), 2, "different files must not merge");
    }

    #[test]
    fn test_merge_adjacent_gap_not_merged() {
        // Lines 1-10 and 15-20 have a gap > 1 — should not merge.
        let entries = vec![
            entry("src/a.rs", 1, 10, 15, 0.8),
            entry("src/a.rs", 15, 20, 10, 0.5),
        ];
        let merged = merge_adjacent(entries);
        // line_end=10, line_start=15 → 10+1=11 < 15, so no merge
        assert_eq!(merged.len(), 2, "non-adjacent chunks should stay separate");
    }

    #[test]
    fn test_greedy_pack_respects_budget() {
        let entries = vec![
            entry("src/a.rs", 1, 50, 200, 0.9),
            entry("src/b.rs", 1, 30, 150, 0.8),
            entry("src/c.rs", 1, 20, 300, 0.5),
        ];
        let (packed, total) = greedy_pack(entries, 400);
        // 200 + 150 = 350 ≤ 400; next would be 350+300=650 > 400 → excluded
        assert_eq!(packed.len(), 2);
        assert_eq!(total, 350);
    }

    #[test]
    fn test_greedy_pack_empty() {
        let (packed, total) = greedy_pack(vec![], 1000);
        assert!(packed.is_empty());
        assert_eq!(total, 0);
    }
}
