//! Rules injection for repository-local agent instructions.
//!
//! Loads `.omnicontext/rules.md` from the repository root and surfaces its
//! contents as a formatted prefix block that can be prepended to any context
//! window response.  The file is optional — its absence is not an error.
//!
//! ## Caching
//!
//! [`RulesLoader`] stores the last-seen modification time alongside the cached
//! content.  Subsequent calls to [`RulesLoader::load_cached`] stat the file and
//! skip the read when the `mtime` is unchanged, keeping hot-path latency near
//! zero.
//!
//! ## Truncation
//!
//! Files larger than [`RULES_MAX_BYTES`] are truncated at the byte boundary and
//! a marker comment is appended so consumers know the content is incomplete.

use std::path::Path;
use std::time::SystemTime;

use crate::error::{OmniError, OmniResult};

/// Relative path (from repo root) to the rules file.
pub const RULES_FILE: &str = ".omnicontext/rules.md";

/// Maximum byte size accepted before truncation.
pub const RULES_MAX_BYTES: u64 = 65_536;

/// Truncation marker appended when the file exceeds [`RULES_MAX_BYTES`].
const TRUNCATION_MARKER: &str = "<!-- rules truncated at 65536 bytes -->";

/// Loads and caches `.omnicontext/rules.md` for a repository.
///
/// Create one instance per engine/session and call [`load_cached`] to get a
/// rules string that is refreshed only when the file changes on disk.
///
/// [`load_cached`]: RulesLoader::load_cached
pub struct RulesLoader {
    cached_content: Option<String>,
    cached_mtime: Option<SystemTime>,
}

impl RulesLoader {
    /// Create a new loader with an empty cache.
    pub fn new() -> Self {
        Self {
            cached_content: None,
            cached_mtime: None,
        }
    }

    /// Load rules from disk, bypassing the cache entirely.
    ///
    /// Returns `Ok(None)` when the file does not exist.  Returns an error only
    /// for genuine I/O failures (permission denied, unreadable filesystem, …).
    ///
    /// Content larger than [`RULES_MAX_BYTES`] is truncated at that byte
    /// boundary and the [`TRUNCATION_MARKER`] is appended.  A leading UTF-8 BOM
    /// (`\u{FEFF}`) is stripped when present.
    pub fn load(repo_path: &Path) -> OmniResult<Option<String>> {
        let rules_path = repo_path.join(RULES_FILE);

        // Stat first — absence is fine, any other OS error is surfaced.
        let metadata = match std::fs::metadata(&rules_path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(e) => return Err(OmniError::Io(e)),
        };

        let file_len = metadata.len();

        // Read the content, truncating if necessary.
        let content = if file_len > RULES_MAX_BYTES {
            // Read only the first RULES_MAX_BYTES bytes.
            use std::io::Read;
            let mut buf = vec![0u8; RULES_MAX_BYTES as usize];
            let mut file = std::fs::File::open(&rules_path)?;
            file.read_exact(&mut buf)?;

            // Decode as UTF-8, replacing ill-formed sequences that result from
            // truncating in the middle of a multi-byte character.
            let mut s = String::from_utf8_lossy(&buf).into_owned();
            s.push('\n');
            s.push_str(TRUNCATION_MARKER);
            s
        } else {
            std::fs::read_to_string(&rules_path)?
        };

        // Strip BOM when present.
        let content = content
            .strip_prefix('\u{FEFF}')
            .map_or(content.as_str(), |s| s)
            .to_owned();

        Ok(Some(content))
    }

    /// Load rules with mtime-based cache.
    ///
    /// Returns the cached string when the file's modification time is identical
    /// to the last-seen value.  Falls through to a full disk read whenever the
    /// mtime advances or when no cached value exists yet.
    ///
    /// Returns `Ok(None)` when the file is absent.
    pub fn load_cached(&mut self, repo_path: &Path) -> OmniResult<Option<String>> {
        let rules_path = repo_path.join(RULES_FILE);

        let current_mtime = match std::fs::metadata(&rules_path) {
            // `.modified()` is infallible on Windows/Linux/macOS; `ok()` handles
            // the rare embedded platform that doesn't track mtime.
            Ok(m) => m.modified().ok(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File was removed; clear cache and return None.
                self.cached_content = None;
                self.cached_mtime = None;
                return Ok(None);
            }
            Err(e) => return Err(OmniError::Io(e)),
        };

        // Cache hit: mtime is known and unchanged.
        if let (Some(cached), Some(new)) = (self.cached_mtime, current_mtime) {
            if cached == new && self.cached_content.is_some() {
                return Ok(self.cached_content.clone());
            }
        }

        // Cache miss or platform has no mtime — reload from disk.
        let content = Self::load(repo_path)?;
        self.cached_content.clone_from(&content);
        self.cached_mtime = current_mtime;
        Ok(content)
    }

    /// Wrap raw rules content in XML-style comment delimiters.
    ///
    /// The resulting string is suitable for prepending to any context window
    /// output so that consuming LLMs see the rules as a structured prefix block.
    ///
    /// ```text
    /// <!-- rules -->
    /// <rules content>
    /// <!-- /rules -->
    ///
    /// <context content>
    /// ```
    pub fn format_prefix(rules: &str) -> String {
        format!("<!-- rules -->\n{rules}\n<!-- /rules -->\n\n")
    }
}

impl Default for RulesLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;
    use tempfile::TempDir;

    /// Create a temporary repo directory with an optional `.omnicontext/rules.md`.
    fn make_repo(content: Option<&str>) -> TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        if let Some(text) = content {
            let rules_dir = dir.path().join(".omnicontext");
            std::fs::create_dir_all(&rules_dir).expect("create .omnicontext");
            let mut f = std::fs::File::create(rules_dir.join("rules.md")).expect("create rules.md");
            f.write_all(text.as_bytes()).expect("write rules");
        }
        dir
    }

    // -----------------------------------------------------------------------

    #[test]
    fn test_load_missing_file_returns_none() {
        let dir = make_repo(None);
        let result = RulesLoader::load(dir.path()).expect("no error");
        assert!(result.is_none(), "expected None for absent rules file");
    }

    #[test]
    fn test_load_existing_file() {
        let content = "# Project Rules\n\nAlways write tests.\n";
        let dir = make_repo(Some(content));
        let result = RulesLoader::load(dir.path()).expect("no error");
        assert_eq!(result.as_deref(), Some(content));
    }

    #[test]
    fn test_load_truncates_at_max_bytes() {
        // Build a string that exceeds the limit by writing `n` ASCII 'x' chars.
        let oversized: String = "x".repeat(RULES_MAX_BYTES as usize + 100);
        let dir = make_repo(Some(&oversized));
        let result = RulesLoader::load(dir.path())
            .expect("no error")
            .expect("Some");

        // Must not exceed max + truncation marker + newline overhead.
        assert!(result.len() <= RULES_MAX_BYTES as usize + TRUNCATION_MARKER.len() + 2);
        assert!(
            result.contains(TRUNCATION_MARKER),
            "truncation marker must be present"
        );
        // Must NOT contain any of the overflow characters.
        assert!(
            result.len() < oversized.len(),
            "truncated content must be shorter than original"
        );
    }

    #[test]
    fn test_load_strips_bom() {
        let with_bom = "\u{FEFF}# Rules\n\nDo the right thing.\n".to_string();
        let dir = make_repo(Some(&with_bom));
        let result = RulesLoader::load(dir.path())
            .expect("no error")
            .expect("Some");
        assert!(
            !result.starts_with('\u{FEFF}'),
            "BOM must be stripped from result"
        );
        assert!(
            result.starts_with("# Rules"),
            "content after BOM must be intact"
        );
    }

    #[test]
    fn test_format_prefix_wraps_in_comments() {
        let rules = "Be concise.";
        let formatted = RulesLoader::format_prefix(rules);
        assert!(
            formatted.starts_with("<!-- rules -->"),
            "must open with comment"
        );
        assert!(formatted.contains(rules), "must contain rules content");
        assert!(
            formatted.contains("<!-- /rules -->"),
            "must close with comment"
        );
        // Trailing blank line separates prefix from context body.
        assert!(formatted.ends_with("\n\n"), "must end with blank line");
    }

    #[test]
    fn test_cached_load_returns_same_on_unchanged_mtime() {
        let content = "# Cached Rules\n";
        let dir = make_repo(Some(content));
        let mut loader = RulesLoader::new();

        let first = loader
            .load_cached(dir.path())
            .expect("no error")
            .expect("Some");
        let second = loader
            .load_cached(dir.path())
            .expect("no error")
            .expect("Some");

        assert_eq!(first, second, "second call must return identical content");
        // Confirm cache is populated.
        assert!(loader.cached_content.is_some());
        assert!(loader.cached_mtime.is_some());
    }

    #[test]
    fn test_cached_load_reloads_on_mtime_change() {
        let dir = make_repo(Some("v1\n"));
        let mut loader = RulesLoader::new();

        let first = loader
            .load_cached(dir.path())
            .expect("no error")
            .expect("Some");
        assert_eq!(first.trim(), "v1");

        // Overwrite the file and advance mtime by at least 1 second so the
        // comparison is reliable on filesystems with 1-second mtime granularity.
        let rules_path = dir.path().join(".omnicontext/rules.md");
        std::fs::write(&rules_path, "v2\n").expect("overwrite");

        // Force mtime forward by setting it explicitly via filetime crate if
        // available, or by sleeping 1s. We use a retry loop for CI robustness.
        //
        // Since we can't depend on `filetime` here we advance via repeated
        // writes until the OS reflects a different mtime.
        let original_mtime = loader.cached_mtime;
        let mut attempts = 0u32;
        loop {
            let meta = std::fs::metadata(&rules_path).expect("stat");
            let new_mtime = meta.modified().ok();
            if new_mtime != original_mtime {
                break;
            }
            if attempts >= 20 {
                // The filesystem may have 1s granularity; sleep briefly.
                std::thread::sleep(std::time::Duration::from_millis(1100));
                std::fs::write(&rules_path, "v2\n").expect("re-overwrite");
                break;
            }
            std::fs::write(&rules_path, "v2\n").expect("re-overwrite");
            attempts += 1;
        }

        let second = loader
            .load_cached(dir.path())
            .expect("no error")
            .expect("Some");
        assert_eq!(
            second.trim(),
            "v2",
            "reloaded content must reflect the update"
        );
    }

    #[test]
    fn test_default_new_has_empty_cache() {
        let loader = RulesLoader::default();
        assert!(loader.cached_content.is_none(), "cache must start empty");
        assert!(loader.cached_mtime.is_none(), "mtime must start empty");
    }

    #[test]
    fn test_cached_load_absent_file_clears_cache_on_removal() {
        let content = "# Rules\n";
        let dir = make_repo(Some(content));
        let mut loader = RulesLoader::new();

        // Prime the cache.
        loader
            .load_cached(dir.path())
            .expect("prime cache")
            .expect("Some");
        assert!(loader.cached_content.is_some());

        // Remove the file.
        std::fs::remove_file(dir.path().join(".omnicontext/rules.md")).expect("remove");

        let result = loader.load_cached(dir.path()).expect("no error");
        assert!(result.is_none(), "must return None after removal");
        assert!(loader.cached_content.is_none(), "cache must be cleared");
        assert!(loader.cached_mtime.is_none(), "mtime must be cleared");
    }

    #[test]
    fn test_load_exact_max_bytes_not_truncated() {
        // A file at exactly the limit should not be truncated.
        let exact: String = "a".repeat(RULES_MAX_BYTES as usize);
        let dir = make_repo(Some(&exact));
        let result = RulesLoader::load(dir.path())
            .expect("no error")
            .expect("Some");
        assert!(
            !result.contains(TRUNCATION_MARKER),
            "exact-size file must not carry truncation marker"
        );
    }
}
