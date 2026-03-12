//! Intelligent chunk deduplication and canonicalization.
//!
//! Goes beyond simple line-overlap detection to identify and merge:
//! - **Content-identical chunks** across different files (copy-paste detection)
//! - **Subsumed chunks** where one fully contains another's line range
//! - **Near-duplicate chunks** with high content similarity (n-gram Jaccard)
//! - **Symbol canonical forms** — merges definition + implementation chunks
//!
//! Designed to maximize information density within a fixed token budget.

#![allow(
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::must_use_candidate
)]

use std::collections::{HashMap, HashSet};

/// Configuration for chunk deduplication.
#[derive(Debug, Clone)]
pub struct DeduplicationConfig {
    /// Minimum Jaccard similarity (0.0–1.0) to consider two chunks near-duplicates.
    /// Default: 0.7 (70% n-gram overlap).
    pub similarity_threshold: f64,
    /// N-gram size for content similarity hashing.
    /// Default: 3 (trigrams).
    pub ngram_size: usize,
    /// Whether to detect cross-file content duplicates.
    /// Default: true.
    pub cross_file_dedup: bool,
    /// Whether to merge subsumed (fully-contained) chunks.
    /// Default: true.
    pub merge_subsumed: bool,
}

impl Default for DeduplicationConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.7,
            ngram_size: 3,
            cross_file_dedup: true,
            merge_subsumed: true,
        }
    }
}

/// A chunk reference for deduplication analysis.
/// Lightweight — does not own the content, just references it.
#[derive(Debug, Clone)]
pub struct ChunkRef {
    /// Unique chunk ID.
    pub id: i64,
    /// File ID this chunk belongs to.
    pub file_id: i64,
    /// Start line (1-based).
    pub line_start: u32,
    /// End line (1-based, inclusive).
    pub line_end: u32,
    /// The chunk content (borrowed for analysis).
    pub content: String,
    /// Relevance score (higher = more relevant).
    pub score: f64,
    /// Token count.
    pub token_count: u32,
    /// Symbol path (empty if not a symbol chunk).
    pub symbol_path: String,
}

/// Result of deduplication: which chunks to keep and which were removed.
#[derive(Debug, Clone)]
pub struct DeduplicationResult {
    /// Indices of chunks to keep (in original order, preserving rank).
    pub kept_indices: Vec<usize>,
    /// Indices of chunks that were removed as duplicates.
    pub removed_indices: Vec<usize>,
    /// Tokens saved by deduplication.
    pub tokens_saved: u32,
    /// Number of content-identical duplicates found.
    pub content_dupes: usize,
    /// Number of subsumed (fully-contained) chunks removed.
    pub subsumed_count: usize,
    /// Number of near-duplicate chunks removed.
    pub near_dupes: usize,
}

/// Perform intelligent deduplication on a ranked list of chunks.
///
/// Chunks are processed in score order (highest first). When a duplicate or
/// near-duplicate is found, the lower-scored chunk is removed.
pub fn deduplicate(chunks: &[ChunkRef], config: &DeduplicationConfig) -> DeduplicationResult {
    let n = chunks.len();
    if n == 0 {
        return DeduplicationResult {
            kept_indices: Vec::new(),
            removed_indices: Vec::new(),
            tokens_saved: 0,
            content_dupes: 0,
            subsumed_count: 0,
            near_dupes: 0,
        };
    }

    let mut removed: HashSet<usize> = HashSet::new();
    let mut tokens_saved: u32 = 0;
    let mut content_dupes: usize = 0;
    let mut subsumed_count: usize = 0;
    let mut near_dupes: usize = 0;

    // Phase 1: Content-hash deduplication (exact matches)
    if config.cross_file_dedup {
        let mut content_hashes: HashMap<u64, usize> = HashMap::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let hash = content_hash(&chunk.content);
            if let Some(&existing_idx) = content_hashes.get(&hash) {
                if !removed.contains(&existing_idx) {
                    // Keep the higher-scored one (which is `existing_idx` since we process in order)
                    removed.insert(i);
                    tokens_saved += chunk.token_count;
                    content_dupes += 1;
                }
            } else {
                content_hashes.insert(hash, i);
            }
        }
    }

    // Phase 2: Subsumption detection (same file, one range contains the other)
    if config.merge_subsumed {
        for i in 0..n {
            if removed.contains(&i) {
                continue;
            }
            for j in (i + 1)..n {
                if removed.contains(&j) {
                    continue;
                }
                if chunks[i].file_id != chunks[j].file_id {
                    continue;
                }

                // Check if i fully contains j
                if chunks[i].line_start <= chunks[j].line_start
                    && chunks[i].line_end >= chunks[j].line_end
                {
                    removed.insert(j);
                    tokens_saved += chunks[j].token_count;
                    subsumed_count += 1;
                }
                // Check if j fully contains i
                else if chunks[j].line_start <= chunks[i].line_start
                    && chunks[j].line_end >= chunks[i].line_end
                {
                    removed.insert(i);
                    tokens_saved += chunks[i].token_count;
                    subsumed_count += 1;
                    break; // i is removed, stop checking against other j's
                }
            }
        }
    }

    // Phase 3: Near-duplicate detection via n-gram Jaccard similarity
    if config.similarity_threshold > 0.0 && config.similarity_threshold < 1.0 {
        // Precompute n-gram sets
        let ngram_sets: Vec<HashSet<u64>> = chunks
            .iter()
            .map(|c| compute_ngram_hashes(&c.content, config.ngram_size))
            .collect();

        for i in 0..n {
            if removed.contains(&i) {
                continue;
            }
            for j in (i + 1)..n {
                if removed.contains(&j) {
                    continue;
                }
                // Skip if different files and cross-file dedup is disabled
                if !config.cross_file_dedup && chunks[i].file_id != chunks[j].file_id {
                    continue;
                }

                let similarity = jaccard_similarity(&ngram_sets[i], &ngram_sets[j]);
                if similarity >= config.similarity_threshold {
                    // Remove the lower-scored chunk (j, since chunks are sorted by score desc)
                    removed.insert(j);
                    tokens_saved += chunks[j].token_count;
                    near_dupes += 1;
                }
            }
        }
    }

    // Build result
    let mut kept_indices: Vec<usize> = Vec::with_capacity(n - removed.len());
    let mut removed_indices: Vec<usize> = Vec::with_capacity(removed.len());

    for i in 0..n {
        if removed.contains(&i) {
            removed_indices.push(i);
        } else {
            kept_indices.push(i);
        }
    }

    DeduplicationResult {
        kept_indices,
        removed_indices,
        tokens_saved,
        content_dupes,
        subsumed_count,
        near_dupes,
    }
}

/// Canonicalize symbol paths: if multiple chunks share the same symbol
/// (e.g., definition + implementation), keep only the highest-scored one.
///
/// Returns indices to remove from the chunk list.
///
/// **Precondition**: chunks should be ordered by score descending for
/// consistent "keep highest scored" behavior.
pub fn canonicalize_symbols(chunks: &[ChunkRef]) -> Vec<usize> {
    let mut symbol_map: HashMap<&str, usize> = HashMap::new();
    let mut removed: HashSet<usize> = HashSet::new();

    for (i, chunk) in chunks.iter().enumerate() {
        if chunk.symbol_path.is_empty() {
            continue;
        }

        if let Some(&existing_idx) = symbol_map.get(chunk.symbol_path.as_str()) {
            // Skip if the existing entry was already removed by a previous swap
            if removed.contains(&existing_idx) {
                // The previous "winner" was itself removed — this chunk becomes the new owner
                symbol_map.insert(&chunk.symbol_path, i);
                continue;
            }

            // Default: keep the existing (higher-scored) chunk, remove this one
            let existing_lines =
                chunks[existing_idx].line_end - chunks[existing_idx].line_start + 1;
            let new_lines = chunk.line_end - chunk.line_start + 1;

            if new_lines > existing_lines * 2 {
                // The new chunk is significantly more comprehensive — swap
                removed.insert(existing_idx);
                symbol_map.insert(&chunk.symbol_path, i);
            } else {
                removed.insert(i);
            }
        } else {
            symbol_map.insert(&chunk.symbol_path, i);
        }
    }

    let mut result: Vec<usize> = removed.into_iter().collect();
    result.sort_unstable();
    result
}

// ---------------------------------------------------------------------------
// Hashing / similarity helpers
// ---------------------------------------------------------------------------

/// Simple content hash using FNV-1a for fast comparison.
fn content_hash(content: &str) -> u64 {
    // Normalize whitespace before hashing for whitespace-insensitive comparison
    let normalized: String = content.split_whitespace().collect::<Vec<_>>().join(" ");
    fnv1a_hash(normalized.as_bytes())
}

/// FNV-1a hash.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Compute n-gram hashes for a content string.
fn compute_ngram_hashes(content: &str, n: usize) -> HashSet<u64> {
    let words: Vec<&str> = content.split_whitespace().collect();
    let mut hashes = HashSet::new();

    if words.len() < n {
        // Content too short for n-grams — hash the whole thing
        hashes.insert(fnv1a_hash(content.as_bytes()));
        return hashes;
    }

    for window in words.windows(n) {
        let ngram = window.join(" ");
        hashes.insert(fnv1a_hash(ngram.as_bytes()));
    }

    hashes
}

/// Jaccard similarity between two hash sets.
fn jaccard_similarity(a: &HashSet<u64>, b: &HashSet<u64>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: i64, file_id: i64, start: u32, end: u32, content: &str, score: f64) -> ChunkRef {
        ChunkRef {
            id,
            file_id,
            line_start: start,
            line_end: end,
            content: content.to_string(),
            score,
            token_count: content.len() as u32 / 4,
            symbol_path: String::new(),
        }
    }

    fn chunk_with_symbol(
        id: i64,
        file_id: i64,
        start: u32,
        end: u32,
        content: &str,
        score: f64,
        symbol: &str,
    ) -> ChunkRef {
        ChunkRef {
            id,
            file_id,
            line_start: start,
            line_end: end,
            content: content.to_string(),
            score,
            token_count: content.len() as u32 / 4,
            symbol_path: symbol.to_string(),
        }
    }

    #[test]
    fn test_empty_input() {
        let result = deduplicate(&[], &DeduplicationConfig::default());
        assert!(result.kept_indices.is_empty());
        assert!(result.removed_indices.is_empty());
        assert_eq!(result.tokens_saved, 0);
    }

    #[test]
    fn test_no_duplicates() {
        let chunks = vec![
            chunk(1, 1, 1, 10, "fn hello() { println!(\"hello\"); }", 0.9),
            chunk(2, 2, 1, 10, "fn goodbye() { println!(\"goodbye\"); }", 0.8),
        ];
        let result = deduplicate(&chunks, &DeduplicationConfig::default());
        assert_eq!(result.kept_indices, vec![0, 1]);
        assert!(result.removed_indices.is_empty());
    }

    #[test]
    fn test_content_exact_duplicate() {
        let content = "fn duplicate() { x + y }";
        let chunks = vec![
            chunk(1, 1, 1, 5, content, 0.9),
            chunk(2, 2, 10, 15, content, 0.7), // same content, different file
        ];
        let result = deduplicate(&chunks, &DeduplicationConfig::default());
        assert_eq!(result.kept_indices, vec![0]);
        assert_eq!(result.removed_indices, vec![1]);
        assert_eq!(result.content_dupes, 1);
    }

    #[test]
    fn test_whitespace_normalized_duplicate() {
        let chunks = vec![
            chunk(1, 1, 1, 5, "fn  hello()  {  x + y  }", 0.9),
            chunk(2, 2, 10, 15, "fn hello() { x + y }", 0.7),
        ];
        let result = deduplicate(&chunks, &DeduplicationConfig::default());
        assert_eq!(
            result.content_dupes, 1,
            "whitespace-normalized should match"
        );
        assert_eq!(result.kept_indices, vec![0]);
    }

    #[test]
    fn test_subsumption_same_file() {
        let chunks = vec![
            chunk(1, 1, 1, 20, "fn outer() { fn inner() {} }", 0.9), // wide range
            chunk(2, 1, 5, 10, "fn inner() {}", 0.8),                // subsumed
        ];
        let result = deduplicate(&chunks, &DeduplicationConfig::default());
        assert_eq!(result.kept_indices, vec![0]);
        assert_eq!(result.subsumed_count, 1);
    }

    #[test]
    fn test_subsumption_different_files_not_merged() {
        let chunks = vec![
            chunk(1, 1, 1, 20, "fn outer() { fn inner() {} }", 0.9),
            chunk(2, 2, 5, 10, "fn inner() {}", 0.8), // different file_id
        ];
        let config = DeduplicationConfig {
            cross_file_dedup: false,
            ..Default::default()
        };
        let result = deduplicate(&chunks, &config);
        // Subsumption only applies within same file
        assert_eq!(result.subsumed_count, 0);
    }

    #[test]
    fn test_near_duplicate_detection() {
        let base = "fn process_data(input: &str) -> Result<Data, Error> { let parsed = parse(input)?; let validated = validate(parsed)?; Ok(transform(validated)) }";
        let variant = "fn process_data(input: &str) -> Result<Data, Error> { let parsed = parse(input)?; let validated = validate(parsed)?; Ok(transform_v2(validated)) }";

        let chunks = vec![
            chunk(1, 1, 1, 10, base, 0.9),
            chunk(2, 2, 1, 10, variant, 0.7),
        ];
        let result = deduplicate(&chunks, &DeduplicationConfig::default());
        // These should be near-duplicates (high Jaccard similarity)
        assert!(
            result.near_dupes > 0 || result.content_dupes > 0,
            "very similar content should be detected as near-duplicate"
        );
    }

    #[test]
    fn test_similarity_threshold() {
        let a = "fn alpha() { let x = 1; let y = 2; x + y }";
        let b = "fn beta() { let a = 10; let b = 20; a * b }";

        let chunks = vec![chunk(1, 1, 1, 5, a, 0.9), chunk(2, 2, 1, 5, b, 0.7)];

        // With very high threshold, these are NOT duplicates
        let strict_config = DeduplicationConfig {
            similarity_threshold: 0.95,
            cross_file_dedup: false, // disable content-hash to test only Jaccard
            ..Default::default()
        };
        let result = deduplicate(&chunks, &strict_config);
        assert_eq!(result.near_dupes, 0);
    }

    #[test]
    fn test_tokens_saved() {
        let content = "fn duplicate() { x + y }";
        let chunks = vec![
            chunk(1, 1, 1, 5, content, 0.9),
            chunk(2, 2, 10, 15, content, 0.7),
        ];
        let result = deduplicate(&chunks, &DeduplicationConfig::default());
        assert!(result.tokens_saved > 0);
        assert_eq!(result.tokens_saved, chunks[1].token_count);
    }

    #[test]
    fn test_canonicalize_symbols_keeps_first() {
        let chunks = vec![
            chunk_with_symbol(1, 1, 1, 10, "fn validate() {}", 0.9, "auth::validate"),
            chunk_with_symbol(
                2,
                1,
                20,
                30,
                "impl Auth { fn validate() {} }",
                0.7,
                "auth::validate",
            ),
        ];
        let to_remove = canonicalize_symbols(&chunks);
        assert_eq!(to_remove, vec![1]); // removes lower-scored duplicate symbol
    }

    #[test]
    fn test_canonicalize_symbols_prefers_comprehensive() {
        let chunks = vec![
            chunk_with_symbol(1, 1, 5, 6, "fn short() {}", 0.9, "mod::func"),
            chunk_with_symbol(
                2,
                1,
                1,
                20,
                "/// Full doc\nfn short() {\n  // lots of impl\n  // more code\n}",
                0.7,
                "mod::func",
            ),
        ];
        let to_remove = canonicalize_symbols(&chunks);
        // The second chunk spans 20 lines vs 2 lines — >2x larger, should be preferred
        assert_eq!(to_remove, vec![0], "should prefer more comprehensive chunk");
    }

    #[test]
    fn test_canonicalize_no_symbols() {
        let chunks = vec![
            chunk(1, 1, 1, 10, "fn a() {}", 0.9),
            chunk(2, 2, 1, 10, "fn b() {}", 0.8),
        ];
        let to_remove = canonicalize_symbols(&chunks);
        assert!(
            to_remove.is_empty(),
            "chunks without symbols should not be merged"
        );
    }

    #[test]
    fn test_fnv1a_hash_deterministic() {
        let hash1 = fnv1a_hash(b"hello world");
        let hash2 = fnv1a_hash(b"hello world");
        assert_eq!(hash1, hash2);

        let hash3 = fnv1a_hash(b"hello world!");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_jaccard_identical() {
        let a: HashSet<u64> = [1, 2, 3].into_iter().collect();
        assert_eq!(jaccard_similarity(&a, &a), 1.0);
    }

    #[test]
    fn test_jaccard_disjoint() {
        let a: HashSet<u64> = [1, 2, 3].into_iter().collect();
        let b: HashSet<u64> = [4, 5, 6].into_iter().collect();
        assert_eq!(jaccard_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_jaccard_partial() {
        let a: HashSet<u64> = [1, 2, 3, 4].into_iter().collect();
        let b: HashSet<u64> = [3, 4, 5, 6].into_iter().collect();
        // intersection={3,4}=2, union={1,2,3,4,5,6}=6, similarity=2/6≈0.333
        let sim = jaccard_similarity(&a, &b);
        assert!((sim - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_jaccard_empty() {
        let a: HashSet<u64> = HashSet::new();
        let b: HashSet<u64> = HashSet::new();
        assert_eq!(jaccard_similarity(&a, &b), 1.0);
    }

    #[test]
    fn test_content_hash_whitespace_insensitive() {
        let h1 = content_hash("fn  hello()  {  x + y  }");
        let h2 = content_hash("fn hello() { x + y }");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_ngram_hashes_small_input() {
        let hashes = compute_ngram_hashes("hello", 3);
        assert_eq!(hashes.len(), 1, "single word should produce 1 hash");
    }

    #[test]
    fn test_ngram_hashes_normal() {
        let hashes = compute_ngram_hashes("fn process data input result", 3);
        // 5 words, windows of 3 → 3 trigrams
        assert_eq!(hashes.len(), 3);
    }

    #[test]
    fn test_combined_dedup_all_phases() {
        let chunks = vec![
            // Original high-scored
            chunk(
                1,
                1,
                1,
                20,
                "fn important() { complex logic here with many tokens }",
                0.95,
            ),
            // Content duplicate of chunk 1 in different file
            chunk(
                2,
                2,
                1,
                20,
                "fn important() { complex logic here with many tokens }",
                0.85,
            ),
            // Subsumed by chunk 1 (same file, contained lines)
            chunk(3, 1, 5, 10, "fn inner() {}", 0.80),
            // Unique chunk
            chunk(
                4,
                3,
                1,
                10,
                "fn totally_different() { unique code path }",
                0.75,
            ),
        ];

        let result = deduplicate(&chunks, &DeduplicationConfig::default());
        assert!(result.content_dupes >= 1, "should detect content duplicate");
        assert!(result.subsumed_count >= 1, "should detect subsumed chunk");
        assert!(
            result.kept_indices.contains(&0),
            "highest scored should be kept"
        );
        assert!(
            result.kept_indices.contains(&3),
            "unique chunk should be kept"
        );
    }
}
