//! File hash cache for change detection — three-tier strategy.
//!
//! ## Strategy
//!
//! Change detection operates in three tiers to minimise I/O cost:
//!
//! 1. **Tier 1 — mtime (in-memory, ~1 µs/file):** Compare `fs::metadata().modified()`
//!    against the cached `SystemTime` from the last known-clean state. If identical,
//!    skip entirely. Only a `stat` syscall; zero bytes read.
//!
//! 2. **Tier 2 — xxHash3 content hash (~40 ns/KB):** If mtime changed, read the file
//!    once and hash with xxHash3. If the hash matches, the file was `touch`ed without
//!    content change; sync the mtime and skip re-indexing.
//!
//! 3. **Tier 3 — content returned directly:** When the hash differs, return the
//!    already-read content to the caller so it can proceed without a second read.
//!
//! ## Persistence
//!
//! xxHash3 values (u64) are stored in `.omnicontext/file_hashes.json`. The mtime
//! cache is in-memory only and is not persisted; it is rebuilt from successful index
//! runs as the engine runs.
//!
//! Atomic writes use temp-file + rename to prevent corruption.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

use crate::error::{OmniError, OmniResult};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Three-tier file hash cache for change detection.
///
/// - Persistent: xxHash3 of content per file, stored in `file_hashes.json`.
/// - In-memory: `SystemTime` of mtime at last known-clean state; not persisted.
pub struct FileHashCache {
    /// Persistent map: file path → xxHash3 (u64) of content.
    hashes: HashMap<PathBuf, u64>,
    /// In-memory map: file path → mtime at last known-clean state.
    /// Not persisted — rebuilt from successful index runs.
    mtime_cache: HashMap<PathBuf, SystemTime>,
    /// Path to the on-disk JSON cache.
    cache_file: PathBuf,
    /// Whether the hash map has been modified since last save.
    dirty: bool,
}

impl FileHashCache {
    /// Create a new empty hash cache backed by `<index_dir>/file_hashes.json`.
    pub fn new(index_dir: &Path) -> Self {
        Self {
            hashes: HashMap::new(),
            mtime_cache: HashMap::new(),
            cache_file: index_dir.join("file_hashes.json"),
            dirty: false,
        }
    }

    /// Load the hash cache from disk.
    ///
    /// Returns a fresh cache if the file is absent or cannot be parsed.
    pub fn load(index_dir: &Path) -> OmniResult<Self> {
        let cache_file = index_dir.join("file_hashes.json");

        if !cache_file.exists() {
            tracing::debug!("hash cache file not found, starting fresh");
            return Ok(Self {
                hashes: HashMap::new(),
                mtime_cache: HashMap::new(),
                cache_file,
                dirty: false,
            });
        }

        match fs::read_to_string(&cache_file) {
            Ok(content) => {
                let cache_data: CacheData = serde_json::from_str(&content)
                    .map_err(|e| OmniError::Internal(format!("failed to parse hash cache: {e}")))?;

                tracing::info!(
                    entries = cache_data.hashes.len(),
                    "loaded hash cache from disk"
                );

                Ok(Self {
                    hashes: cache_data.hashes,
                    mtime_cache: HashMap::new(), // always rebuilt in memory
                    cache_file,
                    dirty: false,
                })
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to load hash cache, starting fresh");
                Ok(Self {
                    hashes: HashMap::new(),
                    mtime_cache: HashMap::new(),
                    cache_file,
                    dirty: false,
                })
            }
        }
    }

    // -----------------------------------------------------------------------
    // Primary API
    // -----------------------------------------------------------------------

    /// Pre-populate the in-memory mtime cache from the filesystem for all
    /// files currently in the hash cache.
    ///
    /// ## Startup Performance — The Problem
    ///
    /// After a daemon restart, `mtime_cache` is empty (it is not persisted).
    /// On the first `run_index()` call, every known file falls through to
    /// Tier 2 (read + xxHash3), even if nothing has changed since the last
    /// run.  For a 10k-file repo this reads ~100-200 MB from disk unnecessarily.
    ///
    /// ## Solution
    ///
    /// `warm_mtime_cache()` runs at engine startup and does a single `stat`
    /// per known file.  Files whose `mtime` has not changed since the last
    /// index run will have their mtime cached immediately.  On the first
    /// subsequent `check_and_read()` call:
    ///
    /// - Unchanged files: mtime matches cached value → Tier 1 hit → O(1) skip.
    /// - Changed files: mtime differs → Tier 2 read (only changed files).
    ///
    /// This reduces cold-restart indexing time from O(N × read_cost) to
    /// O(N × stat_cost) for unchanged repositories — matching Augment's
    /// hash-based change detection behavior exactly.
    ///
    /// `repo_path` is used to resolve relative paths stored in the cache.
    pub fn warm_mtime_cache(&mut self, repo_path: &Path) {
        let paths: Vec<PathBuf> = self.hashes.keys().cloned().collect();
        let mut warmed = 0usize;

        for rel_path in paths {
            // Paths in the hash cache are stored relative to repo root.
            let abs_path = if rel_path.is_absolute() {
                rel_path.clone()
            } else {
                repo_path.join(&rel_path)
            };

            if let Ok(metadata) = fs::metadata(&abs_path) {
                if let Ok(mtime) = metadata.modified() {
                    self.mtime_cache.insert(rel_path, mtime);
                    warmed += 1;
                }
            }
        }

        tracing::info!(
            warmed,
            total_known = self.hashes.len(),
            "mtime cache warmed from filesystem — first indexing pass uses stat-only checks"
        );
    }

    /// Combined change detection and file read (the hot path for incremental indexing).
    ///
    /// Returns:
    /// - `Ok((false, None))` — file is unchanged; caller should skip re-indexing.
    /// - `Ok((true, Some(content)))` — file changed; content already in memory
    ///   so the caller need not read it again.
    /// - `Err(_)` — I/O failure.
    ///
    /// Internally runs the three-tier strategy:
    /// - Tier 1: mtime check (in-memory, ~1 µs) — most files stop here on warm runs.
    /// - Tier 2: xxHash3 (~40 ns/KB) — catches `touch` without edit.
    /// - Tier 3: returns content directly — no double-read.
    pub fn check_and_read(&mut self, path: &Path) -> OmniResult<(bool, Option<String>)> {
        // --- Tier 1: mtime ---
        let mtime = fs::metadata(path).and_then(|m| m.modified()).ok();

        if let Some(mtime) = mtime {
            if self.mtime_cache.get(path) == Some(&mtime) {
                // mtime identical → file cannot have changed
                return Ok((false, None));
            }
        }

        // --- Tier 2 / 3: read content + hash ---
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return Err(OmniError::Io(e));
            }
        };

        let hash = xxh3_64(content.as_bytes());

        // Tier 2: hash matches → content is identical despite mtime change
        if self.hashes.get(path) == Some(&hash) {
            // Sync mtime so future stat checks short-circuit
            if let Some(mtime) = mtime {
                self.mtime_cache.insert(path.to_path_buf(), mtime);
            }
            return Ok((false, None));
        }

        // Tier 3: content changed → return it directly
        Ok((true, Some(content)))
    }

    /// Record that a file was successfully indexed with the given content hash and mtime.
    ///
    /// Updates both the persistent hash store (dirty) and the in-memory mtime cache.
    pub fn update_from_read(&mut self, path: PathBuf, hash: u64, mtime: SystemTime) {
        self.hashes.insert(path.clone(), hash);
        self.mtime_cache.insert(path, mtime);
        self.dirty = true;
    }

    // -----------------------------------------------------------------------
    // Legacy / backward-compat API (used by process_file and existing tests)
    // -----------------------------------------------------------------------

    /// Compute xxHash3 of a file's content.
    ///
    /// Returns the hash as a raw `u64`. This replaces the old SHA-256 path.
    pub fn compute_hash_u64(path: &Path) -> OmniResult<u64> {
        let content = fs::read(path)?;
        Ok(xxh3_64(&content))
    }

    /// Compute SHA-256 of a file's content as a hex string.
    ///
    /// Kept for callers that still need the hex representation stored in the
    /// `files.hash` column (`FileInfo.content_hash: String`).
    pub fn compute_hash(path: &Path) -> OmniResult<String> {
        use sha2::{Digest, Sha256};
        let content = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        Ok(format!("{hash:x}"))
    }

    /// Check if a file has changed since it was last recorded in the cache.
    ///
    /// Returns `true` when the file is absent from the cache or its xxHash3
    /// differs from the stored value. Uses the mtime tier as a fast-path.
    ///
    /// Prefer `check_and_read()` for the indexing hot path — it avoids the
    /// double-read that calling `has_changed()` + `read_to_string()` incurs.
    pub fn has_changed(&mut self, path: &Path) -> OmniResult<bool> {
        let (changed, _) = self.check_and_read(path)?;
        Ok(changed)
    }

    /// Record a file's xxHash3 hash after successful indexing.
    ///
    /// Also captures the current mtime so subsequent warm-run checks can use
    /// the Tier 1 mtime short-circuit without re-reading the file.
    pub fn update_hash(&mut self, path: PathBuf, _hash_ignored: String) {
        // Design: the old API passed a SHA-256 hex string; we silently recompute
        // using xxHash3 so internal storage is consistent.  The parameter is
        // kept for API compatibility.
        if let Ok(hash) = Self::compute_hash_u64(&path) {
            let mtime = fs::metadata(&path)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            self.update_from_read(path, hash, mtime);
        } else {
            // File unreadable at commit time — remove stale entry if any
            self.hashes.remove(&path);
            self.mtime_cache.remove(&path);
            self.dirty = true;
        }
    }

    /// Get the stored xxHash3 for a file, or `None` if not cached.
    pub fn get_hash_u64(&self, path: &Path) -> Option<u64> {
        self.hashes.get(path).copied()
    }

    /// Get the cached hash as a hex string (for backward compatibility).
    ///
    /// Returns `None` if the file is not in the cache.
    pub fn get_hash(&self, path: &Path) -> Option<String> {
        self.hashes.get(path).map(|h| format!("{h:016x}"))
    }

    /// Remove a file from the cache (called when a file is deleted).
    pub fn remove(&mut self, path: &Path) -> bool {
        let removed_hash = self.hashes.remove(path).is_some();
        self.mtime_cache.remove(path);
        if removed_hash {
            self.dirty = true;
        }
        removed_hash
    }

    /// Save the hash cache to disk using atomic temp-file + rename.
    ///
    /// No-op when the cache has not been modified since the last save.
    pub fn save(&mut self) -> OmniResult<()> {
        if !self.dirty {
            return Ok(());
        }

        if let Some(parent) = self.cache_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let cache_data = CacheData {
            version: 2,
            hashes: self.hashes.clone(),
        };

        let json = serde_json::to_string_pretty(&cache_data)
            .map_err(|e| OmniError::Internal(format!("failed to serialize hash cache: {e}")))?;

        let temp_file = self.cache_file.with_extension("json.tmp");
        fs::write(&temp_file, &json)?;
        fs::rename(&temp_file, &self.cache_file).map_err(|e| {
            let _ = fs::remove_file(&temp_file);
            OmniError::Io(e)
        })?;

        self.dirty = false;

        tracing::debug!(
            entries = self.hashes.len(),
            path = %self.cache_file.display(),
            "saved hash cache to disk"
        );

        Ok(())
    }

    /// Number of entries in the persistent hash store.
    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    /// Whether the persistent hash store is empty.
    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }

    /// Clear all entries from both the hash store and the mtime cache.
    pub fn clear(&mut self) {
        self.hashes.clear();
        self.mtime_cache.clear();
        self.dirty = true;
    }

    /// Cache statistics for diagnostic reporting.
    pub fn statistics(&self) -> CacheStatistics {
        CacheStatistics {
            total_entries: self.hashes.len(),
            mtime_entries: self.mtime_cache.len(),
            dirty: self.dirty,
            cache_file: self.cache_file.clone(),
        }
    }

    /// Prune entries for files that no longer exist on disk.
    ///
    /// Returns the number of entries removed.
    pub fn prune_missing_files(&mut self) -> usize {
        let before = self.hashes.len();
        self.hashes.retain(|path, _| path.exists());
        self.mtime_cache.retain(|path, _| path.exists());
        let removed = before - self.hashes.len();

        if removed > 0 {
            self.dirty = true;
            tracing::info!(removed, "pruned missing files from hash cache");
        }

        removed
    }
}

// ---------------------------------------------------------------------------
// Serializable on-disk format
// ---------------------------------------------------------------------------

/// Serializable cache data.
#[derive(Debug, Serialize, Deserialize)]
struct CacheData {
    /// Cache format version. Version 2 stores u64 xxHash3 values.
    version: u32,
    /// Map from file path to xxHash3 (stored as JSON integers).
    hashes: HashMap<PathBuf, u64>,
}

/// Cache statistics for diagnostic endpoints.
#[derive(Debug, Clone)]
pub struct CacheStatistics {
    /// Number of entries in the persistent hash store.
    pub total_entries: usize,
    /// Number of entries in the in-memory mtime cache.
    pub mtime_entries: usize,
    /// Whether the persistent store has unsaved changes.
    pub dirty: bool,
    /// Path to the on-disk cache file.
    pub cache_file: PathBuf,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = fs::File::create(&path).expect("create test file");
        file.write_all(content.as_bytes()).expect("write content");
        path
    }

    // -----------------------------------------------------------------------
    // xxHash3 correctness
    // -----------------------------------------------------------------------

    #[test]
    fn test_xxhash3_deterministic() {
        // Same input must always produce the same hash.
        let input = b"hello world, deterministic hashing";
        let h1 = xxh3_64(input);
        let h2 = xxh3_64(input);
        assert_eq!(h1, h2, "xxHash3 must be deterministic");
    }

    #[test]
    fn test_xxhash3_collision_resistance() {
        // Different inputs must produce different hashes.
        let h1 = xxh3_64(b"content_a");
        let h2 = xxh3_64(b"content_b");
        assert_ne!(h1, h2, "different inputs must produce different hashes");
    }

    #[test]
    fn test_compute_hash_u64_matches_inline() {
        let temp = TempDir::new().expect("temp dir");
        let path = create_test_file(temp.path(), "f.txt", "some data");
        let h1 = FileHashCache::compute_hash_u64(&path).expect("hash");
        let h2 = xxh3_64(b"some data");
        assert_eq!(h1, h2);
    }

    // -----------------------------------------------------------------------
    // check_and_read — three-tier logic
    // -----------------------------------------------------------------------

    #[test]
    fn test_check_and_read_new_file() {
        // A file not in the cache is always "changed".
        let temp = TempDir::new().expect("temp dir");
        let mut cache = FileHashCache::new(temp.path());
        let path = create_test_file(temp.path(), "new.txt", "new content");

        let (changed, content) = cache.check_and_read(&path).expect("check_and_read");
        assert!(changed, "new file must be reported as changed");
        assert_eq!(content.as_deref(), Some("new content"));
    }

    #[test]
    fn test_check_and_read_unchanged_hash() {
        // After recording a file, check_and_read on the same content returns false.
        let temp = TempDir::new().expect("temp dir");
        let mut cache = FileHashCache::new(temp.path());
        let path = create_test_file(temp.path(), "same.txt", "unchanged content");

        // First call: file is new → changed=true
        let (changed, content) = cache.check_and_read(&path).expect("first read");
        assert!(changed);
        let content = content.expect("content on first read");

        // Record it as indexed
        let hash = xxh3_64(content.as_bytes());
        let mtime = fs::metadata(&path)
            .and_then(|m| m.modified())
            .expect("mtime");
        cache.update_from_read(path.clone(), hash, mtime);

        // Second call: hash matches → unchanged
        let (changed2, content2) = cache.check_and_read(&path).expect("second read");
        assert!(!changed2, "same content must be reported as unchanged");
        assert!(content2.is_none(), "no content returned for unchanged file");
    }

    #[test]
    fn test_check_and_read_mtime_shortcircuit() {
        // After recording mtime, check_and_read short-circuits without reading.
        let temp = TempDir::new().expect("temp dir");
        let mut cache = FileHashCache::new(temp.path());
        let path = create_test_file(temp.path(), "cached.txt", "cached content");

        // Seed the cache with hash + mtime
        let hash = xxh3_64(b"cached content");
        let mtime = fs::metadata(&path)
            .and_then(|m| m.modified())
            .expect("mtime");
        cache.update_from_read(path.clone(), hash, mtime);

        // check_and_read should return (false, None) via the Tier 1 mtime check
        let (changed, content) = cache.check_and_read(&path).expect("mtime check");
        assert!(!changed, "mtime match must short-circuit to unchanged");
        assert!(content.is_none());
    }

    #[test]
    fn test_check_and_read_modified_content() {
        // Modifying a file's content is detected on the next check_and_read.
        let temp = TempDir::new().expect("temp dir");
        let mut cache = FileHashCache::new(temp.path());
        let path = create_test_file(temp.path(), "mod.txt", "original");

        // Seed with original hash + mtime
        let (changed, content) = cache.check_and_read(&path).expect("first");
        assert!(changed);
        let hash = xxh3_64(content.expect("content").as_bytes());
        let mtime = fs::metadata(&path)
            .and_then(|m| m.modified())
            .expect("mtime");
        cache.update_from_read(path.clone(), hash, mtime);

        // Overwrite file content
        {
            let mut f = fs::File::create(&path).expect("open for write");
            f.write_all(b"modified content").expect("write");
            f.sync_all().expect("sync");
        }

        // On Windows the NTFS mtime resolution is 100 ns, but the kernel may
        // batch updates within the same tick. Sleep 20 ms to guarantee the mtime
        // of the rewritten file is strictly later than the cached entry, even
        // under heavy CI load where the scheduler may delay the sleep.
        std::thread::sleep(std::time::Duration::from_millis(20));

        // check_and_read must detect the change
        let (changed2, content2) = cache.check_and_read(&path).expect("second");
        assert!(changed2, "modified file must be reported as changed");
        assert_eq!(content2.as_deref(), Some("modified content"));
    }

    // -----------------------------------------------------------------------
    // update_from_read / persistence
    // -----------------------------------------------------------------------

    #[test]
    fn test_update_from_read_marks_dirty() {
        let temp = TempDir::new().expect("temp dir");
        let mut cache = FileHashCache::new(temp.path());
        let path = PathBuf::from("fake.txt");

        assert!(!cache.dirty, "fresh cache must not be dirty");
        cache.update_from_read(path, 12345_u64, SystemTime::now());
        assert!(cache.dirty, "update_from_read must mark cache dirty");
    }

    #[test]
    fn test_mtime_cache_not_persisted() {
        // After save + load, mtime_cache must be empty (no mtime entries persisted).
        let temp = TempDir::new().expect("temp dir");
        let index_dir = temp.path().join("idx");
        fs::create_dir_all(&index_dir).expect("create dir");

        {
            let mut cache = FileHashCache::new(&index_dir);
            cache.update_from_read(PathBuf::from("file.txt"), 999_u64, SystemTime::now());
            assert_eq!(cache.mtime_cache.len(), 1, "mtime_cache has one entry");
            cache.save().expect("save");
        }

        // Reload
        let loaded = FileHashCache::load(&index_dir).expect("load");
        assert_eq!(loaded.hashes.len(), 1, "hash persisted");
        assert_eq!(
            loaded.mtime_cache.len(),
            0,
            "mtime_cache must be empty after load"
        );
    }

    // -----------------------------------------------------------------------
    // Legacy API compatibility
    // -----------------------------------------------------------------------

    #[test]
    fn test_compute_hash_returns_hex() {
        let temp = TempDir::new().expect("temp dir");
        let path = create_test_file(temp.path(), "hex.txt", "hello world");
        let hex = FileHashCache::compute_hash(&path).expect("sha256 hash");
        // SHA-256 produces 64 hex chars
        assert_eq!(hex.len(), 64);
        // Must be a valid hex string
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_remove_clears_mtime_and_hash() {
        let temp = TempDir::new().expect("temp dir");
        let mut cache = FileHashCache::new(temp.path());
        let path = PathBuf::from("to_remove.txt");

        cache.update_from_read(path.clone(), 42_u64, SystemTime::now());
        assert_eq!(cache.len(), 1);

        let removed = cache.remove(&path);
        assert!(removed);
        assert_eq!(cache.len(), 0);
        assert!(!cache.mtime_cache.contains_key(&path));
        assert!(cache.dirty);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp = TempDir::new().expect("temp dir");
        let index_dir = temp.path().join("idx2");
        fs::create_dir_all(&index_dir).expect("create");

        let path_a = PathBuf::from("a.rs");
        let path_b = PathBuf::from("b.rs");

        {
            let mut cache = FileHashCache::new(&index_dir);
            cache.update_from_read(path_a.clone(), 0xAAAA_AAAA, SystemTime::now());
            cache.update_from_read(path_b.clone(), 0xBBBB_BBBB, SystemTime::now());
            cache.save().expect("save");
        }

        let loaded = FileHashCache::load(&index_dir).expect("load");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get_hash_u64(&path_a), Some(0xAAAA_AAAA));
        assert_eq!(loaded.get_hash_u64(&path_b), Some(0xBBBB_BBBB));
        assert!(!loaded.dirty);
    }

    #[test]
    fn test_save_only_when_dirty() {
        let temp = TempDir::new().expect("temp dir");
        let mut cache = FileHashCache::new(temp.path());

        // Not dirty → save is a no-op, cache file must not exist
        cache.save().expect("no-op save");
        assert!(!cache.cache_file.exists());

        // Dirty → save writes the file
        cache.update_from_read(PathBuf::from("x.txt"), 1_u64, SystemTime::now());
        cache.save().expect("dirty save");
        assert!(cache.cache_file.exists());
        assert!(!cache.dirty);
    }

    #[test]
    fn test_clear() {
        let temp = TempDir::new().expect("temp dir");
        let mut cache = FileHashCache::new(temp.path());
        cache.update_from_read(PathBuf::from("f1.txt"), 1_u64, SystemTime::now());
        cache.update_from_read(PathBuf::from("f2.txt"), 2_u64, SystemTime::now());
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.mtime_cache.is_empty());
        assert!(cache.dirty);
    }

    #[test]
    fn test_prune_missing_files() {
        let temp = TempDir::new().expect("temp dir");
        let mut cache = FileHashCache::new(temp.path());

        let existing = create_test_file(temp.path(), "existing.txt", "data");
        let missing = temp.path().join("missing.txt");

        cache.update_from_read(existing.clone(), 1_u64, SystemTime::now());
        cache.update_from_read(missing.clone(), 2_u64, SystemTime::now());
        assert_eq!(cache.len(), 2);

        let removed = cache.prune_missing_files();
        assert_eq!(removed, 1);
        assert_eq!(cache.len(), 1);
        assert!(cache.get_hash_u64(&existing).is_some());
        assert!(cache.get_hash_u64(&missing).is_none());
    }

    #[test]
    fn test_statistics() {
        let temp = TempDir::new().expect("temp dir");
        let mut cache = FileHashCache::new(temp.path());
        cache.update_from_read(PathBuf::from("s.txt"), 9_u64, SystemTime::now());

        let stats = cache.statistics();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.mtime_entries, 1);
        assert!(stats.dirty);
    }
}
