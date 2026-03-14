//! Per-repo persistent memory store.
//!
//! Stores key-value pairs in `<repo>/.omnicontext/memory.json`.
//!
//! ## Persistence model
//!
//! Writes are atomic: the serialized JSON is written to a sibling `.memory.json.tmp`
//! file, then `std::fs::rename` moves it over `memory.json` in a single syscall.
//! On all supported platforms (Linux, macOS, Windows) this rename is atomic when
//! both paths are on the same filesystem — which they always are since `.tmp` lives
//! in the same `.omnicontext/` directory.
//!
//! ## Context injection
//!
//! [`MemoryStore::format_prefix`] renders the store as an XML-comment-delimited
//! block that can be prepended to any `context_window` response.  Consumers see:
//!
//! ```text
//! <!-- memory -->
//! key1: value1
//! key2: value2
//! <!-- /memory -->
//!
//! ```
//!
//! ## Limits
//!
//! | Constraint         | Value     |
//! |--------------------|-----------|
//! | Max key length     | 256 bytes |
//! | Max value length   | 64 KiB    |
//! | Max entries        | 1,000     |

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{OmniError, OmniResult};

/// Relative path (from repo root) to the memory file.
pub const MEMORY_FILE: &str = ".omnicontext/memory.json";

/// Temporary file written before rename.
const MEMORY_FILE_TMP: &str = ".omnicontext/.memory.json.tmp";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Persistent key-value memory store for a single repository.
///
/// All keys are sorted (via [`BTreeMap`]) to guarantee stable JSON serialization
/// across platforms and Rust versions — byte-for-byte identical output as long
/// as the entries are unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryStore {
    /// Key-value entries, sorted by key for stable serialization.
    pub entries: BTreeMap<String, MemoryEntry>,
}

/// A single memory entry — the value and a last-write wall-clock timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// The stored value.
    pub value: String,
    /// Unix timestamp (seconds) of last write.
    pub updated_at: u64,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl MemoryStore {
    /// Maximum byte length of a key.
    pub const MAX_KEY_LEN: usize = 256;
    /// Maximum byte length of a value (64 KiB).
    pub const MAX_VALUE_LEN: usize = 65_536;
    /// Maximum number of distinct keys in a single store.
    pub const MAX_ENTRIES: usize = 1_000;

    // -----------------------------------------------------------------------
    // Load / Save
    // -----------------------------------------------------------------------

    /// Load from `<repo>/.omnicontext/memory.json`.
    ///
    /// Returns an empty [`MemoryStore`] when the file does not exist.  Returns
    /// an error only for genuine I/O failures (permission denied, unreadable
    /// filesystem, …) or malformed JSON.
    pub fn load(repo_path: &Path) -> OmniResult<Self> {
        let path = Self::memory_path(repo_path);

        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(e) => return Err(OmniError::Io(e)),
        };

        serde_json::from_str(&raw).map_err(|e| OmniError::Serialization(e.to_string()))
    }

    /// Persist the store to `<repo>/.omnicontext/memory.json` atomically.
    ///
    /// The write sequence is:
    /// 1. Create `.omnicontext/` if it does not exist.
    /// 2. Serialize to pretty-printed JSON.
    /// 3. Write to `<repo>/.omnicontext/.memory.json.tmp`.
    /// 4. `std::fs::rename(tmp, memory.json)` — atomic on same filesystem.
    pub fn save(&self, repo_path: &Path) -> OmniResult<()> {
        let dir = repo_path.join(".omnicontext");
        std::fs::create_dir_all(&dir)?;

        let final_path = Self::memory_path(repo_path);
        let tmp_path = repo_path.join(MEMORY_FILE_TMP);

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| OmniError::Serialization(e.to_string()))?;

        std::fs::write(&tmp_path, json.as_bytes())?;
        std::fs::rename(&tmp_path, &final_path)?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Mutation
    // -----------------------------------------------------------------------

    /// Insert or update a key-value pair.
    ///
    /// Returns `Err` if:
    /// - The key exceeds [`Self::MAX_KEY_LEN`] bytes.
    /// - The value exceeds [`Self::MAX_VALUE_LEN`] bytes.
    /// - The key is new and inserting it would exceed [`Self::MAX_ENTRIES`].
    pub fn set(&mut self, key: String, value: String) -> OmniResult<()> {
        if key.len() > Self::MAX_KEY_LEN {
            return Err(OmniError::Config {
                details: format!(
                    "memory key '{}...' exceeds {} bytes",
                    &key[..Self::MAX_KEY_LEN.min(key.len())],
                    Self::MAX_KEY_LEN
                ),
            });
        }
        if value.len() > Self::MAX_VALUE_LEN {
            return Err(OmniError::Config {
                details: format!(
                    "memory value for key '{}' exceeds {} bytes",
                    key,
                    Self::MAX_VALUE_LEN
                ),
            });
        }

        // Guard against exceeding the entry limit only for genuinely new keys.
        let is_new_key = !self.entries.contains_key(&key);
        if is_new_key && self.entries.len() >= Self::MAX_ENTRIES {
            return Err(OmniError::Config {
                details: format!(
                    "memory store is full ({} entries); remove a key before adding new ones",
                    Self::MAX_ENTRIES
                ),
            });
        }

        self.entries.insert(
            key,
            MemoryEntry {
                value,
                updated_at: Self::unix_now(),
            },
        );

        Ok(())
    }

    /// Return a reference to the value stored under `key`, or `None` if absent.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(|e| e.value.as_str())
    }

    /// Remove a key from the store.
    ///
    /// Returns `true` if the key was present, `false` if it was not.
    pub fn remove(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    // -----------------------------------------------------------------------
    // Query
    // -----------------------------------------------------------------------

    /// List all keys with their last-updated Unix timestamps, in key-sorted order.
    pub fn list_keys(&self) -> Vec<(&str, u64)> {
        self.entries
            .iter()
            .map(|(k, e)| (k.as_str(), e.updated_at))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Context injection
    // -----------------------------------------------------------------------

    /// Format the store as a context prefix block.
    ///
    /// Produces:
    /// ```text
    /// <!-- memory -->
    /// key1: value1
    /// key2: value2
    /// <!-- /memory -->
    ///
    /// ```
    ///
    /// Returns an empty string when the store is empty so callers can safely
    /// concatenate without producing spurious blank blocks.
    pub fn format_prefix(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        let mut out = String::from("<!-- memory -->\n");
        for (key, entry) in &self.entries {
            // Values may contain newlines; emit them verbatim so that multi-line
            // notes are preserved.  The closing delimiter on its own line
            // unambiguously terminates the block for any parser.
            out.push_str(key);
            out.push_str(": ");
            out.push_str(&entry.value);
            out.push('\n');
        }
        out.push_str("<!-- /memory -->\n\n");
        out
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn memory_path(repo_path: &Path) -> PathBuf {
        repo_path.join(MEMORY_FILE)
    }

    fn unix_now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Create a temporary directory for use as a repo root.
    fn make_repo() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    // -----------------------------------------------------------------------
    // Load / Save
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_missing_returns_empty() {
        let dir = make_repo();
        let store = MemoryStore::load(dir.path()).expect("load should not fail");
        assert!(
            store.entries.is_empty(),
            "absent file must yield empty store"
        );
    }

    #[test]
    fn test_save_and_reload() {
        let dir = make_repo();
        let mut store = MemoryStore::default();
        store
            .set("arch".to_string(), "hexagonal".to_string())
            .unwrap();
        store
            .set("team".to_string(), "platform".to_string())
            .unwrap();

        store.save(dir.path()).expect("save");

        let loaded = MemoryStore::load(dir.path()).expect("reload");
        assert_eq!(loaded.get("arch"), Some("hexagonal"));
        assert_eq!(loaded.get("team"), Some("platform"));
    }

    #[test]
    fn test_atomic_save_creates_omnicontext_dir() {
        let dir = make_repo();
        // The .omnicontext directory does not exist yet.
        assert!(!dir.path().join(".omnicontext").exists());

        let store = MemoryStore::default();
        store.save(dir.path()).expect("save");

        assert!(
            dir.path().join(".omnicontext").is_dir(),
            "save must create .omnicontext/"
        );
        assert!(
            dir.path().join(MEMORY_FILE).exists(),
            "memory.json must exist after save"
        );
        // Temporary file must have been cleaned up by the rename.
        assert!(
            !dir.path().join(MEMORY_FILE_TMP).exists(),
            ".memory.json.tmp must not persist after save"
        );
    }

    #[test]
    fn test_save_roundtrip_preserves_timestamps() {
        let dir = make_repo();
        let mut store = MemoryStore::default();
        store.set("k".to_string(), "v".to_string()).unwrap();

        let ts_before = store.entries["k"].updated_at;
        store.save(dir.path()).unwrap();

        let loaded = MemoryStore::load(dir.path()).unwrap();
        let ts_after = loaded.entries["k"].updated_at;

        assert_eq!(
            ts_before, ts_after,
            "timestamp must survive serialization round-trip"
        );
    }

    // -----------------------------------------------------------------------
    // Mutation
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_and_get() {
        let mut store = MemoryStore::default();
        store.set("lang".to_string(), "rust".to_string()).unwrap();
        assert_eq!(store.get("lang"), Some("rust"));
        assert_eq!(store.get("missing"), None);
    }

    #[test]
    fn test_overwrite_existing_key() {
        let mut store = MemoryStore::default();
        store.set("x".to_string(), "old".to_string()).unwrap();
        store.set("x".to_string(), "new".to_string()).unwrap();
        assert_eq!(store.get("x"), Some("new"));
        assert_eq!(
            store.entries.len(),
            1,
            "overwrite must not create a duplicate entry"
        );
    }

    #[test]
    fn test_remove_key() {
        let mut store = MemoryStore::default();
        store.set("k".to_string(), "v".to_string()).unwrap();
        let removed = store.remove("k");
        assert!(removed, "remove must return true for existing key");
        assert!(store.get("k").is_none(), "key must be gone after remove");
    }

    #[test]
    fn test_remove_nonexistent_returns_false() {
        let mut store = MemoryStore::default();
        assert!(
            !store.remove("ghost"),
            "remove of absent key must return false"
        );
    }

    // -----------------------------------------------------------------------
    // Constraint enforcement
    // -----------------------------------------------------------------------

    #[test]
    fn test_max_key_len_enforced() {
        let mut store = MemoryStore::default();
        let long_key = "k".repeat(MemoryStore::MAX_KEY_LEN + 1);
        let result = store.set(long_key, "v".to_string());
        assert!(
            result.is_err(),
            "key exceeding MAX_KEY_LEN must be rejected"
        );
    }

    #[test]
    fn test_max_value_len_enforced() {
        let mut store = MemoryStore::default();
        let long_value = "v".repeat(MemoryStore::MAX_VALUE_LEN + 1);
        let result = store.set("k".to_string(), long_value);
        assert!(
            result.is_err(),
            "value exceeding MAX_VALUE_LEN must be rejected"
        );
    }

    #[test]
    fn test_max_entries_enforced() {
        let mut store = MemoryStore::default();
        // Fill to the limit.
        for i in 0..MemoryStore::MAX_ENTRIES {
            store
                .set(format!("key_{i}"), "v".to_string())
                .expect("should fit");
        }
        assert_eq!(store.entries.len(), MemoryStore::MAX_ENTRIES);

        // One more new key must fail.
        let result = store.set("overflow_key".to_string(), "v".to_string());
        assert!(result.is_err(), "inserting beyond MAX_ENTRIES must fail");

        // But updating an existing key must still succeed.
        store
            .set("key_0".to_string(), "updated".to_string())
            .expect("updating existing key at capacity must succeed");
        assert_eq!(store.get("key_0"), Some("updated"));
    }

    // -----------------------------------------------------------------------
    // Query
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_keys_returns_sorted() {
        let mut store = MemoryStore::default();
        store.set("zebra".to_string(), "1".to_string()).unwrap();
        store.set("apple".to_string(), "2".to_string()).unwrap();
        store.set("mango".to_string(), "3".to_string()).unwrap();

        let keys: Vec<&str> = store.list_keys().into_iter().map(|(k, _)| k).collect();
        assert_eq!(
            keys,
            vec!["apple", "mango", "zebra"],
            "list_keys must return keys in lexicographic order"
        );
    }

    // -----------------------------------------------------------------------
    // Context injection
    // -----------------------------------------------------------------------

    #[test]
    fn test_format_prefix_empty_returns_empty_string() {
        let store = MemoryStore::default();
        assert_eq!(
            store.format_prefix(),
            "",
            "empty store must produce empty string"
        );
    }

    #[test]
    fn test_format_prefix_nonempty() {
        let mut store = MemoryStore::default();
        store.set("db".to_string(), "postgres".to_string()).unwrap();
        store
            .set("framework".to_string(), "axum".to_string())
            .unwrap();

        let prefix = store.format_prefix();

        assert!(
            prefix.starts_with("<!-- memory -->\n"),
            "must open with <!-- memory --> tag"
        );
        assert!(
            prefix.contains("<!-- /memory -->"),
            "must close with <!-- /memory --> tag"
        );
        assert!(
            prefix.ends_with("<!-- /memory -->\n\n"),
            "must end with blank line after closing tag"
        );
        assert!(prefix.contains("db: postgres\n"), "must contain db entry");
        assert!(
            prefix.contains("framework: axum\n"),
            "must contain framework entry"
        );

        // Keys must appear in sorted order.
        let db_pos = prefix.find("db: postgres").unwrap();
        let fw_pos = prefix.find("framework: axum").unwrap();
        assert!(
            db_pos < fw_pos,
            "'db' must appear before 'framework' (lexicographic order)"
        );
    }
}
