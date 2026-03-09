//! Query result caching with LRU eviction and TTL.
//!
//! This module provides a thread-safe cache for search results to reduce
//! redundant queries and improve response times for frequent searches.
//!
//! ## Features
//!
//! - LRU eviction policy (least recently used items are evicted first)
//! - Time-to-live (TTL) expiration (default: 5 minutes)
//! - Thread-safe access via DashMap
//! - Automatic cleanup of expired entries
//!
//! ## Expected Impact
//!
//! - Reduced query latency for repeated searches
//! - Lower database load
//! - Better user experience for common queries

use std::hash::{Hash, Hasher};
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
}

impl CacheKey {
    /// Create a new cache key.
    pub fn new(query: String, limit: usize, min_rerank_score: Option<f32>) -> Self {
        Self {
            query,
            limit,
            min_rerank_score: min_rerank_score.map(|s| (s * 1000.0) as u32),
        }
    }
}

/// Cached search result with expiration time.
#[derive(Debug, Clone)]
struct CachedEntry {
    /// The cached search results.
    results: Vec<SearchResult>,
    /// When this entry was created.
    created_at: Instant,
    /// Time-to-live duration.
    ttl: Duration,
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
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(capacity.try_into().unwrap())),
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

        if let Some(entry) = cache.get(key) {
            if entry.is_expired() {
                // Entry expired, remove it
                cache.pop(key);
                self.increment_stat("expired");
                None
            } else {
                // Cache hit
                self.increment_stat("hits");
                Some(entry.results.clone())
            }
        } else {
            // Cache miss
            self.increment_stat("misses");
            None
        }
    }

    /// Insert results into the cache.
    pub fn insert(&self, key: CacheKey, results: Vec<SearchResult>) {
        let entry = CachedEntry {
            results,
            created_at: Instant::now(),
            ttl: self.default_ttl,
        };

        let mut cache = self.cache.lock();
        cache.put(key, entry);
        self.increment_stat("inserts");
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        let mut cache = self.cache.lock();
        cache.clear();
        self.increment_stat("clears");
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.get_stat("hits"),
            misses: self.get_stat("misses"),
            expired: self.get_stat("expired"),
            inserts: self.get_stat("inserts"),
            clears: self.get_stat("clears"),
            size: self.cache.lock().len(),
            capacity: self.cache.lock().cap().get(),
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

        if count > 0 {
            self.stats
                .entry("pruned")
                .and_modify(|v| *v += count as u64)
                .or_insert(count as u64);
        }

        count
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
    use crate::types::{Chunk, ChunkKind};
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
            },
            file_path: PathBuf::from("test.rs"),
            score,
            score_breakdown: Default::default(),
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
}
