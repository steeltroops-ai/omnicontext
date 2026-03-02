//! Speculative pre-fetch module for improved UX.
//!
//! This module implements a cache that pre-fetches likely contexts based on
//! IDE events (file open, cursor move, text edit). The goal is to have context
//! ready before the user explicitly requests it, reducing perceived latency.

#![allow(dead_code)] // TODO: Remove when fully implemented
#![allow(clippy::unwrap_used)] // TODO: Proper error handling
#![allow(clippy::expect_used)] // TODO: Proper error handling
#![allow(clippy::cast_precision_loss)] // Acceptable for hit rate calculation

use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A pre-fetched context entry with TTL.
#[derive(Debug, Clone)]
struct CachedContext {
    /// The pre-fetched context string.
    context: String,
    /// When this entry was created.
    created_at: Instant,
    /// Time-to-live for this entry.
    ttl: Duration,
}

impl CachedContext {
    /// Check if this entry has expired.
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// Cache key for pre-fetched contexts.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum CacheKey {
    /// File-level context (entire file).
    File(PathBuf),
    /// Symbol-level context (specific symbol and its dependencies).
    Symbol { file: PathBuf, symbol: String },
    /// Function-level context (function and related tests).
    Function { file: PathBuf, function: String },
}

/// Pre-fetch cache with LRU eviction and TTL.
pub struct PrefetchCache {
    /// LRU cache with TTL entries.
    cache: Arc<Mutex<LruCache<CacheKey, CachedContext>>>,
    /// Default TTL for cached entries.
    default_ttl: Arc<Mutex<Duration>>,
    /// Cache hit counter.
    hits: Arc<Mutex<u64>>,
    /// Cache miss counter.
    misses: Arc<Mutex<u64>>,
}

impl PrefetchCache {
    /// Create a new pre-fetch cache.
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of entries (default: 100)
    /// * `ttl` - Time-to-live for entries (default: 5 minutes)
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("capacity must be non-zero"),
            ))),
            default_ttl: Arc::new(Mutex::new(ttl)),
            hits: Arc::new(Mutex::new(0)),
            misses: Arc::new(Mutex::new(0)),
        }
    }

    /// Get cached context for a file.
    #[allow(clippy::ptr_arg)] // PathBuf is more ergonomic for cache keys
    pub fn get_file_context(&self, file: &PathBuf) -> Option<String> {
        let key = CacheKey::File(file.clone());
        self.get(&key)
    }

    /// Get cached context for a symbol.
    #[allow(clippy::ptr_arg)] // PathBuf is more ergonomic for cache keys
    pub fn get_symbol_context(&self, file: &PathBuf, symbol: &str) -> Option<String> {
        let key = CacheKey::Symbol {
            file: file.clone(),
            symbol: symbol.to_string(),
        };
        self.get(&key)
    }

    /// Get cached context for a function.
    #[allow(clippy::ptr_arg)] // PathBuf is more ergonomic for cache keys
    pub fn get_function_context(&self, file: &PathBuf, function: &str) -> Option<String> {
        let key = CacheKey::Function {
            file: file.clone(),
            function: function.to_string(),
        };
        self.get(&key)
    }

    /// Store file-level context.
    pub fn put_file_context(&self, file: PathBuf, context: String) {
        let key = CacheKey::File(file);
        self.put(key, context);
    }

    /// Store symbol-level context.
    pub fn put_symbol_context(&self, file: PathBuf, symbol: String, context: String) {
        let key = CacheKey::Symbol { file, symbol };
        self.put(key, context);
    }

    /// Store function-level context.
    pub fn put_function_context(&self, file: PathBuf, function: String, context: String) {
        let key = CacheKey::Function { file, function };
        self.put(key, context);
    }

    /// Get cache hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let hits = *self.hits.lock().unwrap();
        let misses = *self.misses.lock().unwrap();
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let hits = *self.hits.lock().unwrap();
        let misses = *self.misses.lock().unwrap();
        let size = self.cache.lock().unwrap().len();
        CacheStats {
            hits,
            misses,
            size,
            hit_rate: self.hit_rate(),
        }
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        self.cache.lock().unwrap().clear();
        *self.hits.lock().unwrap() = 0;
        *self.misses.lock().unwrap() = 0;
    }

    /// Update cache configuration dynamically.
    ///
    /// # Arguments
    /// * `capacity` - New cache capacity (optional)
    /// * `ttl_seconds` - New TTL in seconds (optional)
    ///
    /// # Returns
    /// `true` if any configuration was updated, `false` otherwise
    pub fn update_config(&self, capacity: Option<usize>, ttl_seconds: Option<u64>) -> bool {
        let mut updated = false;

        // Update capacity if provided
        if let Some(new_capacity) = capacity {
            if new_capacity > 0 {
                let mut cache = self.cache.lock().unwrap();
                let new_cache = LruCache::new(
                    NonZeroUsize::new(new_capacity).expect("capacity must be non-zero"),
                );

                // Transfer existing entries to new cache (up to new capacity)
                let old_cache = std::mem::replace(&mut *cache, new_cache);
                for (key, value) in old_cache.iter().take(new_capacity) {
                    cache.put(key.clone(), value.clone());
                }

                updated = true;
                tracing::debug!(new_capacity = new_capacity, "cache capacity updated");
            }
        }

        // Update TTL if provided
        if let Some(new_ttl_seconds) = ttl_seconds {
            if new_ttl_seconds > 0 {
                let new_ttl = Duration::from_secs(new_ttl_seconds);
                *self.default_ttl.lock().unwrap() = new_ttl;
                updated = true;
                tracing::debug!(
                    new_ttl_seconds = new_ttl_seconds,
                    "cache TTL updated (affects new entries)"
                );
            }
        }

        updated
    }

    // Internal methods

    fn get(&self, key: &CacheKey) -> Option<String> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get(key) {
            if entry.is_expired() {
                // Entry expired, remove it
                let _ = cache.pop(key);
                *self.misses.lock().unwrap() += 1;
                None
            } else {
                // Cache hit
                *self.hits.lock().unwrap() += 1;
                Some(entry.context.clone())
            }
        } else {
            // Cache miss
            *self.misses.lock().unwrap() += 1;
            None
        }
    }

    fn put(&self, key: CacheKey, context: String) {
        let ttl = *self.default_ttl.lock().unwrap();
        let entry = CachedContext {
            context,
            created_at: Instant::now(),
            ttl,
        };
        self.cache.lock().unwrap().put(key, entry);
    }
}

impl Default for PrefetchCache {
    fn default() -> Self {
        Self::new(100, Duration::from_secs(300)) // 100 entries, 5 minutes TTL
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Current cache size.
    pub size: usize,
    /// Hit rate (0.0 to 1.0).
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit() {
        let cache = PrefetchCache::default();
        let file = PathBuf::from("test.rs");
        let context = "fn main() {}".to_string();

        cache.put_file_context(file.clone(), context.clone());
        let result = cache.get_file_context(&file);

        assert_eq!(result, Some(context));
        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cache.stats().misses, 0);
    }

    #[test]
    fn test_cache_miss() {
        let cache = PrefetchCache::default();
        let file = PathBuf::from("test.rs");

        let result = cache.get_file_context(&file);

        assert_eq!(result, None);
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn test_cache_expiry() {
        let cache = PrefetchCache::new(100, Duration::from_millis(10));
        let file = PathBuf::from("test.rs");
        let context = "fn main() {}".to_string();

        cache.put_file_context(file.clone(), context);
        std::thread::sleep(Duration::from_millis(20));

        let result = cache.get_file_context(&file);
        assert_eq!(result, None);
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn test_symbol_context() {
        let cache = PrefetchCache::default();
        let file = PathBuf::from("test.rs");
        let symbol = "MyStruct".to_string();
        let context = "struct MyStruct {}".to_string();

        cache.put_symbol_context(file.clone(), symbol.clone(), context.clone());
        let result = cache.get_symbol_context(&file, &symbol);

        assert_eq!(result, Some(context));
    }

    #[test]
    fn test_hit_rate() {
        let cache = PrefetchCache::default();
        let file = PathBuf::from("test.rs");

        cache.put_file_context(file.clone(), "context".to_string());

        // 1 hit
        cache.get_file_context(&file);
        // 1 miss
        cache.get_file_context(&PathBuf::from("other.rs"));

        assert_eq!(cache.hit_rate(), 0.5);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = PrefetchCache::new(2, Duration::from_secs(300));

        cache.put_file_context(PathBuf::from("file1.rs"), "context1".to_string());
        cache.put_file_context(PathBuf::from("file2.rs"), "context2".to_string());
        cache.put_file_context(PathBuf::from("file3.rs"), "context3".to_string());

        // file1 should be evicted
        assert_eq!(cache.get_file_context(&PathBuf::from("file1.rs")), None);
        assert_eq!(
            cache.get_file_context(&PathBuf::from("file2.rs")),
            Some("context2".to_string())
        );
        assert_eq!(
            cache.get_file_context(&PathBuf::from("file3.rs")),
            Some("context3".to_string())
        );
    }

    #[test]
    fn test_clear_cache() {
        let cache = PrefetchCache::default();
        let file1 = PathBuf::from("test1.rs");
        let file2 = PathBuf::from("test2.rs");

        // Add some entries
        cache.put_file_context(file1.clone(), "context1".to_string());
        cache.put_file_context(file2.clone(), "context2".to_string());

        // Generate some hits and misses
        cache.get_file_context(&file1); // hit
        cache.get_file_context(&PathBuf::from("nonexistent.rs")); // miss

        // Verify cache has entries and stats
        let stats_before = cache.stats();
        assert_eq!(stats_before.size, 2);
        assert_eq!(stats_before.hits, 1);
        assert_eq!(stats_before.misses, 1);

        // Clear the cache
        cache.clear();

        // Verify cache is empty and stats are reset
        let stats_after = cache.stats();
        assert_eq!(stats_after.size, 0);
        assert_eq!(stats_after.hits, 0);
        assert_eq!(stats_after.misses, 0);
        assert_eq!(stats_after.hit_rate, 0.0);

        // Verify entries are gone
        assert_eq!(cache.get_file_context(&file1), None);
        assert_eq!(cache.get_file_context(&file2), None);
    }
}

#[test]
fn test_update_config_capacity() {
    let cache = PrefetchCache::new(5, Duration::from_secs(300));

    // Add 3 entries
    cache.put_file_context(PathBuf::from("file1.rs"), "context1".to_string());
    cache.put_file_context(PathBuf::from("file2.rs"), "context2".to_string());
    cache.put_file_context(PathBuf::from("file3.rs"), "context3".to_string());

    assert_eq!(cache.stats().size, 3);

    // Update capacity to 10
    let updated = cache.update_config(Some(10), None);
    assert!(updated);

    // Existing entries should still be present
    assert_eq!(cache.stats().size, 3);
    assert!(cache.get_file_context(&PathBuf::from("file1.rs")).is_some());
    assert!(cache.get_file_context(&PathBuf::from("file2.rs")).is_some());
    assert!(cache.get_file_context(&PathBuf::from("file3.rs")).is_some());
}

#[test]
fn test_update_config_ttl() {
    let cache = PrefetchCache::new(100, Duration::from_secs(300));

    // Add an entry with old TTL
    cache.put_file_context(PathBuf::from("old.rs"), "old_context".to_string());

    // Update TTL to 1ms (very short)
    let updated = cache.update_config(None, Some(1));
    assert!(updated);

    // Add a new entry with new TTL
    cache.put_file_context(PathBuf::from("new.rs"), "new_context".to_string());

    // Wait for new entry to expire (1 second + buffer)
    std::thread::sleep(Duration::from_millis(1100));

    // Old entry should still be valid (has 300s TTL)
    assert!(cache.get_file_context(&PathBuf::from("old.rs")).is_some());

    // New entry should be expired (has 1s TTL)
    assert!(cache.get_file_context(&PathBuf::from("new.rs")).is_none());
}

#[test]
fn test_update_config_both() {
    let cache = PrefetchCache::new(5, Duration::from_secs(300));

    // Update both capacity and TTL
    let updated = cache.update_config(Some(20), Some(600));
    assert!(updated);

    // Add entries to verify new configuration
    for i in 1..=10 {
        cache.put_file_context(
            PathBuf::from(format!("file{}.rs", i)),
            format!("context{}", i),
        );
    }

    // All 10 entries should fit in new capacity
    assert_eq!(cache.stats().size, 10);
}

#[test]
fn test_update_config_no_changes() {
    let cache = PrefetchCache::new(100, Duration::from_secs(300));

    // Update with no parameters
    let updated = cache.update_config(None, None);
    assert!(!updated);
}

#[test]
fn test_update_config_capacity_reduction() {
    let cache = PrefetchCache::new(10, Duration::from_secs(300));

    // Add 5 entries
    for i in 1..=5 {
        cache.put_file_context(
            PathBuf::from(format!("file{}.rs", i)),
            format!("context{}", i),
        );
    }

    assert_eq!(cache.stats().size, 5);

    // Reduce capacity to 3
    let updated = cache.update_config(Some(3), None);
    assert!(updated);

    // Only 3 entries should remain
    assert_eq!(cache.stats().size, 3);
}
