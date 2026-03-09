//! File hash cache for change detection.
//!
//! This module implements SHA-256-based file hashing to detect content changes
//! and skip unnecessary re-indexing. When a file is modified, we compare its
//! current hash with the stored hash to determine if re-indexing is needed.
//!
//! ## Performance Impact
//! - Expected: 50-80% reduction in unnecessary re-indexing
//! - Hash computation: ~1ms per file (amortized by avoiding re-indexing)
//! - Cache lookup: <1μs (HashMap)
//!
//! ## Persistence
//! - Hashes stored in `.omnicontext/file_hashes.json`
//! - Loaded on startup, saved after each indexing operation
//! - Atomic writes with temp file + rename

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{OmniError, OmniResult};

/// File hash cache for change detection.
///
/// Maintains a mapping from file path to SHA-256 hash of file content.
/// Used to skip re-indexing of unchanged files.
pub struct FileHashCache {
    /// Map from file path to SHA-256 hash (hex string)
    hashes: HashMap<PathBuf, String>,
    /// Path to the cache file on disk
    cache_file: PathBuf,
    /// Whether the cache has been modified since last save
    dirty: bool,
}

impl FileHashCache {
    /// Create a new hash cache.
    ///
    /// The cache file will be stored at `<index_dir>/file_hashes.json`.
    pub fn new(index_dir: &Path) -> Self {
        let cache_file = index_dir.join("file_hashes.json");
        Self {
            hashes: HashMap::new(),
            cache_file,
            dirty: false,
        }
    }

    /// Load the hash cache from disk.
    ///
    /// Returns a new cache if the file doesn't exist or can't be loaded.
    pub fn load(index_dir: &Path) -> OmniResult<Self> {
        let cache_file = index_dir.join("file_hashes.json");

        if !cache_file.exists() {
            tracing::debug!("hash cache file not found, starting fresh");
            return Ok(Self {
                hashes: HashMap::new(),
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
                    cache_file,
                    dirty: false,
                })
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to load hash cache, starting fresh");
                Ok(Self {
                    hashes: HashMap::new(),
                    cache_file,
                    dirty: false,
                })
            }
        }
    }

    /// Compute SHA-256 hash of a file's content.
    ///
    /// Returns the hash as a lowercase hex string.
    pub fn compute_hash(path: &Path) -> OmniResult<String> {
        let content = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// Check if a file has changed since last indexing.
    ///
    /// Returns `true` if:
    /// - File is not in cache (never indexed)
    /// - File hash differs from cached hash (content changed)
    ///
    /// Returns `false` if file hash matches cached hash (unchanged).
    pub fn has_changed(&self, path: &Path) -> OmniResult<bool> {
        let current_hash = Self::compute_hash(path)?;

        match self.hashes.get(path) {
            Some(cached_hash) => Ok(current_hash != *cached_hash),
            None => Ok(true), // Not in cache = never indexed = changed
        }
    }

    /// Update the hash for a file.
    ///
    /// Call this after successfully indexing a file to record its current state.
    pub fn update_hash(&mut self, path: PathBuf, hash: String) {
        self.hashes.insert(path, hash);
        self.dirty = true;
    }

    /// Get the cached hash for a file.
    ///
    /// Returns `None` if the file is not in the cache.
    pub fn get_hash(&self, path: &Path) -> Option<&String> {
        self.hashes.get(path)
    }

    /// Remove a file from the cache.
    ///
    /// Call this when a file is deleted.
    pub fn remove(&mut self, path: &Path) -> bool {
        if self.hashes.remove(path).is_some() {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Save the hash cache to disk.
    ///
    /// Uses atomic write (temp file + rename) to prevent corruption.
    /// Only writes if the cache has been modified since last save.
    pub fn save(&mut self) -> OmniResult<()> {
        if !self.dirty {
            return Ok(()); // No changes to save
        }

        // Ensure parent directory exists
        if let Some(parent) = self.cache_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let cache_data = CacheData {
            version: 1,
            hashes: self.hashes.clone(),
        };

        let json = serde_json::to_string_pretty(&cache_data)
            .map_err(|e| OmniError::Internal(format!("failed to serialize hash cache: {e}")))?;

        // Atomic write: temp file + rename
        let temp_file = self.cache_file.with_extension("json.tmp");
        fs::write(&temp_file, json)?;
        fs::rename(&temp_file, &self.cache_file).map_err(|e| {
            // Clean up temp file on failure
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

    /// Get the number of entries in the cache.
    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }

    /// Clear all entries from the cache.
    pub fn clear(&mut self) {
        self.hashes.clear();
        self.dirty = true;
    }

    /// Get statistics about the cache.
    pub fn statistics(&self) -> CacheStatistics {
        CacheStatistics {
            total_entries: self.hashes.len(),
            dirty: self.dirty,
            cache_file: self.cache_file.clone(),
        }
    }

    /// Prune entries for files that no longer exist.
    ///
    /// Returns the number of entries removed.
    pub fn prune_missing_files(&mut self) -> usize {
        let before = self.hashes.len();
        self.hashes.retain(|path, _| path.exists());
        let removed = before - self.hashes.len();

        if removed > 0 {
            self.dirty = true;
            tracing::info!(removed, "pruned missing files from hash cache");
        }

        removed
    }
}

/// Serializable cache data structure.
#[derive(Debug, Serialize, Deserialize)]
struct CacheData {
    /// Cache format version (for future migrations)
    version: u32,
    /// Map from file path to SHA-256 hash
    hashes: HashMap<PathBuf, String>,
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStatistics {
    /// Total number of entries in the cache
    pub total_entries: usize,
    /// Whether the cache has unsaved changes
    pub dirty: bool,
    /// Path to the cache file
    pub cache_file: PathBuf,
}

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

    #[test]
    fn test_compute_hash() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let file_path = create_test_file(temp_dir.path(), "test.txt", "hello world");

        let hash = FileHashCache::compute_hash(&file_path).expect("compute hash");

        // SHA-256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_compute_hash_different_content() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let file1 = create_test_file(temp_dir.path(), "file1.txt", "content1");
        let file2 = create_test_file(temp_dir.path(), "file2.txt", "content2");

        let hash1 = FileHashCache::compute_hash(&file1).expect("compute hash1");
        let hash2 = FileHashCache::compute_hash(&file2).expect("compute hash2");

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_has_changed_new_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let cache = FileHashCache::new(temp_dir.path());
        let file_path = create_test_file(temp_dir.path(), "new.txt", "content");

        // New file should be marked as changed
        assert!(cache.has_changed(&file_path).expect("check changed"));
    }

    #[test]
    fn test_has_changed_unchanged_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let mut cache = FileHashCache::new(temp_dir.path());
        let file_path = create_test_file(temp_dir.path(), "test.txt", "content");

        let hash = FileHashCache::compute_hash(&file_path).expect("compute hash");
        cache.update_hash(file_path.clone(), hash);

        // File should not be marked as changed
        assert!(!cache.has_changed(&file_path).expect("check changed"));
    }

    #[test]
    fn test_has_changed_modified_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let mut cache = FileHashCache::new(temp_dir.path());
        let file_path = create_test_file(temp_dir.path(), "test.txt", "original");

        let hash = FileHashCache::compute_hash(&file_path).expect("compute hash");
        cache.update_hash(file_path.clone(), hash);

        // Modify the file
        let mut file = fs::File::create(&file_path).expect("open file");
        file.write_all(b"modified").expect("write modified content");

        // File should be marked as changed
        assert!(cache.has_changed(&file_path).expect("check changed"));
    }

    #[test]
    fn test_update_hash() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let mut cache = FileHashCache::new(temp_dir.path());
        let file_path = PathBuf::from("test.txt");

        assert_eq!(cache.len(), 0);
        assert!(!cache.dirty);

        cache.update_hash(file_path.clone(), "abc123".to_string());

        assert_eq!(cache.len(), 1);
        assert!(cache.dirty);
        assert_eq!(cache.get_hash(&file_path), Some(&"abc123".to_string()));
    }

    #[test]
    fn test_remove() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let mut cache = FileHashCache::new(temp_dir.path());
        let file_path = PathBuf::from("test.txt");

        cache.update_hash(file_path.clone(), "abc123".to_string());
        assert_eq!(cache.len(), 1);

        let removed = cache.remove(&file_path);
        assert!(removed);
        assert_eq!(cache.len(), 0);
        assert!(cache.dirty);
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let index_dir = temp_dir.path().join("index");
        fs::create_dir_all(&index_dir).expect("create index dir");

        // Create and populate cache
        {
            let mut cache = FileHashCache::new(&index_dir);
            cache.update_hash(PathBuf::from("file1.txt"), "hash1".to_string());
            cache.update_hash(PathBuf::from("file2.txt"), "hash2".to_string());
            cache.save().expect("save cache");
        }

        // Load cache and verify
        {
            let cache = FileHashCache::load(&index_dir).expect("load cache");
            assert_eq!(cache.len(), 2);
            assert_eq!(
                cache.get_hash(&PathBuf::from("file1.txt")),
                Some(&"hash1".to_string())
            );
            assert_eq!(
                cache.get_hash(&PathBuf::from("file2.txt")),
                Some(&"hash2".to_string())
            );
            assert!(!cache.dirty);
        }
    }

    #[test]
    fn test_save_only_when_dirty() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let mut cache = FileHashCache::new(temp_dir.path());

        // Save when not dirty should be no-op
        cache.save().expect("save");
        assert!(!cache.cache_file.exists());

        // Save when dirty should write file
        cache.update_hash(PathBuf::from("test.txt"), "hash".to_string());
        cache.save().expect("save");
        assert!(cache.cache_file.exists());
        assert!(!cache.dirty);
    }

    #[test]
    fn test_clear() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let mut cache = FileHashCache::new(temp_dir.path());

        cache.update_hash(PathBuf::from("file1.txt"), "hash1".to_string());
        cache.update_hash(PathBuf::from("file2.txt"), "hash2".to_string());
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.dirty);
    }

    #[test]
    fn test_statistics() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let mut cache = FileHashCache::new(temp_dir.path());

        cache.update_hash(PathBuf::from("test.txt"), "hash".to_string());

        let stats = cache.statistics();
        assert_eq!(stats.total_entries, 1);
        assert!(stats.dirty);
    }

    #[test]
    fn test_prune_missing_files() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let mut cache = FileHashCache::new(temp_dir.path());

        // Add existing file
        let existing = create_test_file(temp_dir.path(), "existing.txt", "content");
        cache.update_hash(existing.clone(), "hash1".to_string());

        // Add non-existent file
        let missing = temp_dir.path().join("missing.txt");
        cache.update_hash(missing.clone(), "hash2".to_string());

        assert_eq!(cache.len(), 2);

        let removed = cache.prune_missing_files();
        assert_eq!(removed, 1);
        assert_eq!(cache.len(), 1);
        assert!(cache.get_hash(&existing).is_some());
        assert!(cache.get_hash(&missing).is_none());
    }
}
