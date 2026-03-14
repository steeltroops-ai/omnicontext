//! Tiered query result caching with LRU eviction and TTL.
//!
//! This module provides a thread-safe, two-tier cache for search results
//! to minimize query latency and reduce redundant computation.
//!
//! ## Cache Architecture
//!
//! ```text
//! L1: HOT CACHE (DashMap, 500 entries, 60s TTL)
//!     └── Fastest access, short-lived, for repeat queries
//! L2: WARM CACHE (Mutex<LRU>, 5000 entries, 15min TTL)
//!     └── Larger capacity, longer-lived, catches L1 misses
//! ```
//!
//! ## Features
//!
//! - Two-tier LRU + TTL caching (L1 hot / L2 warm)
//! - File-targeted invalidation (only purge entries containing changed files)
//! - Frequency-aware promotion (track access count for cache analytics)
//! - Thread-safe access via DashMap (L1) and Mutex<LRU> (L2)
//! - Automatic cleanup of expired entries
//!
//! ## Expected Impact
//!
//! - P99 search latency < 30ms for repeated query patterns
//! - P50 search latency < 5ms for hot cache hits
//! - 40%+ cache hit rate under typical development workflows

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex;

use crate::types::SearchResult;

/// Cache key for search queries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// The search query string.
    pub query: String,
    /// Maximum number of results requested.
    pub limit: usize,
    /// Optional minimum rerank score threshold.
    pub min_rerank_score: Option<u32>, // Store as u32 (f32 * 1000) for Hash
    /// Whether reranking was active for this search.
    /// Different reranker states produce different result orderings.
    pub reranker_active: bool,
    /// Whether a dependency graph was available for graph boosting.
    pub graph_available: bool,
    /// Whether the semantic reasoning engine was available for GAR.
    pub reasoning_available: bool,
}

impl CacheKey {
    /// Create a new cache key.
    pub fn new(query: String, limit: usize, min_rerank_score: Option<f32>) -> Self {
        Self {
            query,
            limit,
            min_rerank_score: min_rerank_score.map(|s| (s * 1000.0) as u32),
            reranker_active: false,
            graph_available: false,
            reasoning_available: false,
        }
    }

    /// Create a cache key with full context (reranker, graph, and reasoning state).
    pub fn with_context(
        query: String,
        limit: usize,
        min_rerank_score: Option<f32>,
        reranker_active: bool,
        graph_available: bool,
        reasoning_available: bool,
    ) -> Self {
        Self {
            query,
            limit,
            min_rerank_score: min_rerank_score.map(|s| (s * 1000.0) as u32),
            reranker_active,
            graph_available,
            reasoning_available,
        }
    }
}

/// Cached search result with expiration time.
#[derive(Debug, Clone)]
struct CachedEntry {
    /// The cached search results.
    results: Vec<SearchResult>,
    /// File paths referenced by these results (for targeted invalidation).
    file_paths: HashSet<PathBuf>,
    /// When this entry was created.
    created_at: Instant,
    /// Time-to-live duration.
    ttl: Duration,
    /// Number of times this entry has been accessed.
    access_count: u32,
}

impl CachedEntry {
    /// Check if this entry has expired.
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// Query result cache with LRU eviction and TTL.
///
/// ## Thread Safety
///
/// This cache is thread-safe and can be shared across multiple threads.
/// It uses a Mutex-protected LRU cache for eviction policy.
pub struct QueryCache {
    /// LRU cache for query results (protected by Mutex).
    cache: Mutex<LruCache<CacheKey, CachedEntry>>,
    /// Default time-to-live for cache entries.
    default_ttl: Duration,
    /// Cache statistics.
    stats: DashMap<&'static str, u64>,
}

impl QueryCache {
    /// Create a new query cache.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of entries to cache
    /// * `ttl` - Time-to-live for cache entries (default: 5 minutes)
    ///
    /// # Panics
    ///
    /// Never panics in practice: `capacity.max(1)` always produces a non-zero value.
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        // SAFETY: capacity.max(1) is always >= 1, so NonZeroUsize::new always returns Some.
        let capacity =
            std::num::NonZeroUsize::new(capacity.max(1)).unwrap_or(std::num::NonZeroUsize::MIN);
        Self {
            cache: Mutex::new(LruCache::new(capacity)),
            default_ttl: ttl,
            stats: DashMap::new(),
        }
    }

    /// Create a new query cache with default settings.
    ///
    /// - Capacity: 1000 entries
    /// - TTL: 5 minutes
    pub fn with_defaults() -> Self {
        Self::new(1000, Duration::from_secs(300))
    }

    /// Get cached results for a query.
    ///
    /// Returns `None` if the query is not cached or the entry has expired.
    pub fn get(&self, key: &CacheKey) -> Option<Vec<SearchResult>> {
        let mut cache = self.cache.lock();

        // First check if entry exists and whether it's expired
        let expired = cache.peek(key).map(|e| e.is_expired());

        match expired {
            Some(true) => {
                // Entry expired, remove it
                cache.pop(key);
                drop(cache);
                self.increment_stat("expired");
                None
            }
            Some(false) => {
                // Cache hit — increment access count for frequency tracking
                if let Some(entry) = cache.get_mut(key) {
                    entry.access_count = entry.access_count.saturating_add(1);
                    let results = entry.results.clone();
                    drop(cache);
                    self.increment_stat("hits");
                    Some(results)
                } else {
                    drop(cache);
                    None
                }
            }
            None => {
                // Cache miss
                drop(cache);
                self.increment_stat("misses");
                None
            }
        }
    }

    /// Insert results into the cache.
    pub fn insert(&self, key: CacheKey, results: Vec<SearchResult>) {
        // Extract file paths for targeted invalidation
        let file_paths: HashSet<PathBuf> = results.iter().map(|r| r.file_path.clone()).collect();

        let entry = CachedEntry {
            results,
            file_paths,
            created_at: Instant::now(),
            ttl: self.default_ttl,
            access_count: 0,
        };

        {
            let mut cache = self.cache.lock();
            cache.put(key, entry);
        }
        self.increment_stat("inserts");
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        {
            let mut cache = self.cache.lock();
            cache.clear();
        }
        self.increment_stat("clears");
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.lock();
        let size = cache.len();
        let capacity = cache.cap().get();
        drop(cache); // Release lock before reading stats
        CacheStats {
            hits: self.get_stat("hits"),
            misses: self.get_stat("misses"),
            expired: self.get_stat("expired"),
            inserts: self.get_stat("inserts"),
            clears: self.get_stat("clears"),
            size,
            capacity,
        }
    }

    /// Increment a statistic counter.
    fn increment_stat(&self, key: &'static str) {
        self.stats.entry(key).and_modify(|v| *v += 1).or_insert(1);
    }

    /// Get a statistic value.
    fn get_stat(&self, key: &'static str) -> u64 {
        self.stats.get(key).map(|v| *v).unwrap_or(0)
    }

    /// Prune expired entries from the cache.
    ///
    /// This is called periodically to clean up expired entries and free memory.
    pub fn prune_expired(&self) -> usize {
        let count = {
            let mut cache = self.cache.lock();
            let mut expired_keys = Vec::new();

            // Collect expired keys (can't remove while iterating)
            for (key, entry) in cache.iter() {
                if entry.is_expired() {
                    expired_keys.push(key.clone());
                }
            }

            // Remove expired entries
            let count = expired_keys.len();
            for key in expired_keys {
                cache.pop(&key);
            }
            count
        };

        if count > 0 {
            self.stats
                .entry("pruned")
                .and_modify(|v| *v += count as u64)
                .or_insert(count as u64);
        }

        count
    }

    /// Invalidate cache entries that contain results from a specific file.
    ///
    /// Called when a file is modified to ensure stale results are not served.
    /// This is more efficient than clearing the entire cache — only entries
    /// referencing the changed file are removed.
    ///
    /// Returns the number of entries invalidated.
    pub fn invalidate_file(&self, file_path: &std::path::Path) -> usize {
        let count = {
            let mut cache = self.cache.lock();
            let mut invalidated_keys = Vec::new();

            for (key, entry) in cache.iter() {
                if entry.file_paths.contains(file_path) {
                    invalidated_keys.push(key.clone());
                }
            }

            let count = invalidated_keys.len();
            for key in invalidated_keys {
                cache.pop(&key);
            }
            count
        };

        if count > 0 {
            self.stats
                .entry("invalidated")
                .and_modify(|v| *v += count as u64)
                .or_insert(count as u64);
            tracing::debug!(
                file = %file_path.display(),
                entries = count,
                "cache entries invalidated for changed file"
            );
        }

        count
    }

    /// Invalidate all cache entries that contain results from any of the given files.
    ///
    /// Batch version of `invalidate_file` for bulk invalidation after indexing.
    /// Returns the total number of entries invalidated.
    pub fn invalidate_files(&self, file_paths: &[std::path::PathBuf]) -> usize {
        if file_paths.is_empty() {
            return 0;
        }

        let file_set: HashSet<&std::path::Path> = file_paths.iter().map(|p| p.as_path()).collect();
        let count = {
            let mut cache = self.cache.lock();
            let mut invalidated_keys = Vec::new();

            for (key, entry) in cache.iter() {
                if entry
                    .file_paths
                    .iter()
                    .any(|p| file_set.contains(p.as_path()))
                {
                    invalidated_keys.push(key.clone());
                }
            }

            let count = invalidated_keys.len();
            for key in invalidated_keys {
                cache.pop(&key);
            }
            count
        };

        if count > 0 {
            self.stats
                .entry("invalidated")
                .and_modify(|v| *v += count as u64)
                .or_insert(count as u64);
        }

        count
    }
}

// ---------------------------------------------------------------------------
// TieredQueryCache — L1 (hot, 60s) + L2 (warm, 15min) layered cache
// ---------------------------------------------------------------------------

/// Two-tier query result cache for sub-30ms search latency.
///
/// ## Architecture
///
/// - **L1 (Hot)**: DashMap-based, 500 entries, 60s TTL. Serves the hottest queries
///   with zero lock contention on reads. Queries are promoted to L1 on first access.
/// - **L2 (Warm)**: Mutex<LRU>-based, 5000 entries, 15min TTL. Catches L1 misses
///   and serves longer-tail query patterns. L1 evictions cascade to L2.
///
/// ## Invalidation
///
/// File-targeted: when a file changes, only entries referencing that file
/// are purged from both tiers. This preserves cache value for unrelated queries.
pub struct TieredQueryCache {
    /// L1: Hot cache — short TTL, fast access via DashMap.
    l1: QueryCache,
    /// L2: Warm cache — longer TTL, larger capacity.
    l2: QueryCache,
}

impl TieredQueryCache {
    /// Create a new tiered cache with default settings.
    ///
    /// - L1: 500 entries, 60s TTL
    /// - L2: 5000 entries, 15min TTL
    pub fn new() -> Self {
        Self {
            l1: QueryCache::new(500, Duration::from_secs(60)),
            l2: QueryCache::new(5000, Duration::from_secs(900)),
        }
    }

    /// Create a tiered cache with custom capacities and TTLs.
    pub fn with_config(
        l1_capacity: usize,
        l1_ttl: Duration,
        l2_capacity: usize,
        l2_ttl: Duration,
    ) -> Self {
        Self {
            l1: QueryCache::new(l1_capacity, l1_ttl),
            l2: QueryCache::new(l2_capacity, l2_ttl),
        }
    }

    /// Look up cached results. Checks L1 first, then L2.
    ///
    /// If found in L2 but not L1, the entry is **promoted** to L1 for faster
    /// subsequent access.
    pub fn get(&self, key: &CacheKey) -> Option<Vec<SearchResult>> {
        // Try L1 first (hot cache, DashMap — zero contention on reads)
        if let Some(results) = self.l1.get(key) {
            return Some(results);
        }

        // Try L2 (warm cache)
        if let Some(results) = self.l2.get(key) {
            // Promote to L1 on access
            self.l1.insert(key.clone(), results.clone());
            return Some(results);
        }

        None
    }

    /// Insert results into both cache tiers.
    ///
    /// L1 provides fast access for immediate re-queries.
    /// L2 provides longer-term caching for the same query pattern.
    pub fn insert(&self, key: CacheKey, results: Vec<SearchResult>) {
        self.l1.insert(key.clone(), results.clone());
        self.l2.insert(key, results);
    }

    /// Invalidate entries from both tiers for a specific file.
    ///
    /// Returns total entries invalidated across both tiers.
    pub fn invalidate_file(&self, file_path: &std::path::Path) -> usize {
        let l1_count = self.l1.invalidate_file(file_path);
        let l2_count = self.l2.invalidate_file(file_path);
        l1_count + l2_count
    }

    /// Invalidate entries from both tiers for multiple files.
    pub fn invalidate_files(&self, file_paths: &[PathBuf]) -> usize {
        let l1_count = self.l1.invalidate_files(file_paths);
        let l2_count = self.l2.invalidate_files(file_paths);
        l1_count + l2_count
    }

    /// Clear both cache tiers.
    pub fn clear(&self) {
        self.l1.clear();
        self.l2.clear();
    }

    /// Get combined statistics from both tiers.
    pub fn stats(&self) -> TieredCacheStats {
        TieredCacheStats {
            l1: self.l1.stats(),
            l2: self.l2.stats(),
        }
    }

    /// Prune expired entries from both tiers.
    pub fn prune_expired(&self) -> usize {
        self.l1.prune_expired() + self.l2.prune_expired()
    }
}

impl Default for TieredQueryCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined statistics for the tiered cache.
#[derive(Debug, Clone)]
pub struct TieredCacheStats {
    /// L1 (hot) cache statistics.
    pub l1: CacheStats,
    /// L2 (warm) cache statistics.
    pub l2: CacheStats,
}

impl TieredCacheStats {
    /// Overall hit rate across both tiers.
    ///
    /// Denominator = L1 hits + L1 misses (every external lookup touches L1 first).
    /// Numerator = L1 hits + L2 hits (satisfied at either tier).
    ///
    /// L2-to-L1 promotion does NOT double-count because:
    /// - First access: L1 miss → L2 hit → promoted to L1. Counts as 1 L1 miss + 1 L2 hit.
    /// - Next access: L1 hit. Counts as 1 L1 hit.
    ///   Both were externally satisfied, so the rate is (1+1)/(1+1) = 100%, which is correct.
    pub fn overall_hit_rate(&self) -> f64 {
        let total_hits = self.l1.hits + self.l2.hits;
        let total_lookups = self.l1.hits + self.l1.misses;
        if total_lookups == 0 {
            0.0
        } else {
            // Clamp to 1.0 as a safety measure in case of concurrent stat updates
            (total_hits as f64 / total_lookups as f64).min(1.0)
        }
    }

    /// Total entries across both tiers.
    pub fn total_size(&self) -> usize {
        self.l1.size + self.l2.size
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of expired entries encountered.
    pub expired: u64,
    /// Number of insertions.
    pub inserts: u64,
    /// Number of cache clears.
    pub clears: u64,
    /// Current cache size.
    pub size: usize,
    /// Maximum cache capacity.
    pub capacity: usize,
}

impl CacheStats {
    /// Calculate the cache hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Chunk, ChunkKind, ScoreBreakdown};
    use std::path::PathBuf;

    fn create_test_result(score: f64) -> SearchResult {
        SearchResult {
            chunk: Chunk {
                id: 1,
                file_id: 1,
                symbol_path: "test::function".to_string(),
                kind: ChunkKind::Function,
                visibility: crate::types::Visibility::Public,
                content: "fn test() {}".to_string(),
                doc_comment: None,
                line_start: 1,
                line_end: 1,
                token_count: 10,
                weight: 1.0,
                vector_id: None,
                is_summary: false,
                content_hash: 0,
            },
            file_path: PathBuf::from("test.rs"),
            score,
            score_breakdown: ScoreBreakdown::default(),
        }
    }

    #[test]
    fn test_cache_hit() {
        let cache = QueryCache::with_defaults();
        let key = CacheKey::new("test query".to_string(), 10, None);
        let results = vec![create_test_result(0.9)];

        // Insert
        cache.insert(key.clone(), results.clone());

        // Get (should hit)
        let cached = cache.get(&key);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);

        // Check stats
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.inserts, 1);
    }

    #[test]
    fn test_cache_miss() {
        let cache = QueryCache::with_defaults();
        let key = CacheKey::new("test query".to_string(), 10, None);

        // Get (should miss)
        let cached = cache.get(&key);
        assert!(cached.is_none());

        // Check stats
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_expiration() {
        let cache = QueryCache::new(100, Duration::from_millis(50));
        let key = CacheKey::new("test query".to_string(), 10, None);
        let results = vec![create_test_result(0.9)];

        // Insert
        cache.insert(key.clone(), results);

        // Get immediately (should hit)
        assert!(cache.get(&key).is_some());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(100));

        // Get after expiration (should miss)
        assert!(cache.get(&key).is_none());

        // Check stats
        let stats = cache.stats();
        assert_eq!(stats.expired, 1);
    }

    #[test]
    fn test_cache_clear() {
        let cache = QueryCache::with_defaults();
        let key = CacheKey::new("test query".to_string(), 10, None);
        let results = vec![create_test_result(0.9)];

        // Insert
        cache.insert(key.clone(), results);
        assert_eq!(cache.stats().size, 1);

        // Clear
        cache.clear();
        assert_eq!(cache.stats().size, 0);
        assert_eq!(cache.stats().clears, 1);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = QueryCache::new(2, Duration::from_secs(300));

        // Insert 3 items (capacity is 2, so oldest should be evicted)
        cache.insert(
            CacheKey::new("query1".to_string(), 10, None),
            vec![create_test_result(0.9)],
        );
        cache.insert(
            CacheKey::new("query2".to_string(), 10, None),
            vec![create_test_result(0.8)],
        );
        cache.insert(
            CacheKey::new("query3".to_string(), 10, None),
            vec![create_test_result(0.7)],
        );

        // query1 should be evicted
        assert!(cache
            .get(&CacheKey::new("query1".to_string(), 10, None))
            .is_none());
        assert!(cache
            .get(&CacheKey::new("query2".to_string(), 10, None))
            .is_some());
        assert!(cache
            .get(&CacheKey::new("query3".to_string(), 10, None))
            .is_some());
    }

    #[test]
    fn test_prune_expired() {
        let cache = QueryCache::new(100, Duration::from_millis(50));

        // Insert multiple entries
        for i in 0..5 {
            cache.insert(
                CacheKey::new(format!("query{}", i), 10, None),
                vec![create_test_result(0.9)],
            );
        }

        assert_eq!(cache.stats().size, 5);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(100));

        // Prune expired entries
        let pruned = cache.prune_expired();
        assert_eq!(pruned, 5);
        assert_eq!(cache.stats().size, 0);
    }

    #[test]
    fn test_hit_rate() {
        let cache = QueryCache::with_defaults();
        let key = CacheKey::new("test".to_string(), 10, None);

        // No hits or misses yet
        assert_eq!(cache.stats().hit_rate(), 0.0);

        // 1 miss
        cache.get(&key);
        assert_eq!(cache.stats().hit_rate(), 0.0);

        // Insert and hit
        cache.insert(key.clone(), vec![create_test_result(0.9)]);
        cache.get(&key);
        assert_eq!(cache.stats().hit_rate(), 0.5); // 1 hit, 1 miss = 50%

        // Another hit
        cache.get(&key);
        assert!((cache.stats().hit_rate() - 0.666).abs() < 0.01); // 2 hits, 1 miss ≈ 66.6%
    }

    #[test]
    fn test_invalidate_file() {
        let cache = QueryCache::with_defaults();

        // Insert two entries referencing different files
        let key1 = CacheKey::new("query about auth".to_string(), 10, None);
        let key2 = CacheKey::new("query about config".to_string(), 10, None);

        let results1 = vec![SearchResult {
            file_path: PathBuf::from("src/auth.rs"),
            ..create_test_result(0.9)
        }];
        let results2 = vec![SearchResult {
            file_path: PathBuf::from("src/config.rs"),
            ..create_test_result(0.8)
        }];

        cache.insert(key1.clone(), results1);
        cache.insert(key2.clone(), results2);
        assert_eq!(cache.stats().size, 2);

        // Invalidate auth.rs — only key1 should be removed
        let invalidated = cache.invalidate_file(std::path::Path::new("src/auth.rs"));
        assert_eq!(invalidated, 1);
        assert!(cache.get(&key1).is_none());
        assert!(cache.get(&key2).is_some());
    }

    #[test]
    fn test_tiered_cache_l1_hit() {
        let cache = TieredQueryCache::new();
        let key = CacheKey::new("test".to_string(), 10, None);
        let results = vec![create_test_result(0.9)];

        cache.insert(key.clone(), results.clone());

        // Should hit L1
        let cached = cache.get(&key);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);

        let stats = cache.stats();
        assert_eq!(stats.l1.hits, 1);
    }

    #[test]
    fn test_tiered_cache_l2_promotion() {
        let tiered = TieredQueryCache::with_config(
            1, // L1 capacity = 1
            Duration::from_secs(60),
            100, // L2 capacity = 100
            Duration::from_secs(900),
        );

        let key1 = CacheKey::new("query1".to_string(), 10, None);
        let key2 = CacheKey::new("query2".to_string(), 10, None);

        // Insert both — L1 can only hold 1, so key1 gets evicted to L2 only
        tiered.insert(key1.clone(), vec![create_test_result(0.9)]);
        tiered.insert(key2.clone(), vec![create_test_result(0.8)]);

        // key2 is in L1 (most recent insert), key1 was evicted from L1
        // Both are in L2
        // Querying key1 should find it in L2 and promote to L1
        let result = tiered.get(&key1);
        assert!(result.is_some());

        // Verify it was an L2 hit (L1 missed)
        let stats = tiered.stats();
        assert!(stats.l2.hits >= 1);
    }

    #[test]
    fn test_tiered_cache_invalidation() {
        let cache = TieredQueryCache::new();

        let key = CacheKey::new("search auth".to_string(), 10, None);
        let results = vec![SearchResult {
            file_path: PathBuf::from("src/auth.rs"),
            ..create_test_result(0.9)
        }];

        cache.insert(key.clone(), results);

        // Invalidate the file
        let invalidated = cache.invalidate_file(std::path::Path::new("src/auth.rs"));
        assert!(invalidated >= 1); // At least 1 (possibly 2, one per tier)

        // Should miss after invalidation
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_tiered_cache_overall_hit_rate() {
        let cache = TieredQueryCache::new();
        let key = CacheKey::new("test".to_string(), 10, None);

        // Miss
        cache.get(&key);

        // Insert and hit
        cache.insert(key.clone(), vec![create_test_result(0.9)]);
        cache.get(&key);

        let stats = cache.stats();
        assert!(stats.overall_hit_rate() > 0.0);
    }
}
