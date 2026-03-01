//! File system watcher with debouncing.
//!
//! Uses the `notify` crate for platform-native filesystem monitoring.
//! Events are debounced and sent through a channel to the indexing pipeline.
#![allow(clippy::doc_markdown, clippy::missing_errors_doc)]
//!
//! ## Design
//!
//! - `full_scan` walks the directory tree synchronously and emits events
//! - `watch` uses notify's debounced watcher for live FS monitoring
//! - Exclude patterns are checked against path components (not full globs)
//! - Language detection uses file extension via `Language::from_extension`

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use tokio::sync::mpsc;

use crate::config::{IndexingConfig, WatcherConfig};
use crate::error::{OmniError, OmniResult};
use crate::types::{Language, PipelineEvent};

/// File system watcher that emits pipeline events.
pub struct FileWatcher {
    watcher_config: WatcherConfig,
    indexing_config: IndexingConfig,
    root: PathBuf,
}

impl FileWatcher {
    /// Create a new file watcher for the given root directory.
    #[must_use]
    pub fn new(
        root: &Path,
        watcher_config: &WatcherConfig,
        indexing_config: &IndexingConfig,
    ) -> Self {
        Self {
            watcher_config: watcher_config.clone(),
            indexing_config: indexing_config.clone(),
            root: root.to_path_buf(),
        }
    }

    /// Perform a full directory scan and emit FileChanged for all source files.
    ///
    /// Returns the number of files discovered.
    pub fn full_scan(&self, tx: &mpsc::Sender<PipelineEvent>) -> OmniResult<usize> {
        let mut count = 0usize;
        self.walk_dir(&self.root, tx, &mut count)?;
        tracing::info!(files = count, root = %self.root.display(), "full scan complete");
        Ok(count)
    }

    /// Recursively walk a directory, emitting FileChanged events for source files.
    fn walk_dir(
        &self,
        dir: &Path,
        tx: &mpsc::Sender<PipelineEvent>,
        count: &mut usize,
    ) -> OmniResult<()> {
        let entries = std::fs::read_dir(dir).map_err(|e| {
            OmniError::Internal(format!("failed to read directory {}: {e}", dir.display()))
        })?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(error = %e, "failed to read directory entry");
                    continue;
                }
            };

            let path = entry.path();

            // Skip excluded paths
            if self.is_excluded(&path) {
                tracing::debug!(path = %path.display(), "excluded");
                continue;
            }

            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "cannot read file type");
                    continue;
                }
            };

            if file_type.is_dir() {
                self.walk_dir(&path, tx, count)?;
            } else if file_type.is_file() {
                // Check if this is a supported source file
                if !is_source_file_static(&path) {
                    continue;
                }

                // Check file size
                if let Ok(meta) = entry.metadata() {
                    if meta.len() > self.indexing_config.max_file_size {
                        tracing::debug!(
                            path = %path.display(),
                            size = meta.len(),
                            max = self.indexing_config.max_file_size,
                            "file too large, skipping"
                        );
                        continue;
                    }
                }

                // Emit event (best-effort, don't block on full channel)
                let event = PipelineEvent::FileChanged { path };
                if tx.try_send(event).is_err() {
                    tracing::warn!("pipeline channel full, dropping event");
                }
                *count += 1;
            } else if file_type.is_symlink() && self.indexing_config.follow_symlinks {
                // Follow symlinks if configured
                if let Ok(resolved) = std::fs::canonicalize(&path) {
                    if resolved.is_dir() {
                        self.walk_dir(&resolved, tx, count)?;
                    } else if resolved.is_file() && is_source_file_static(&resolved) {
                        let event = PipelineEvent::FileChanged { path: resolved };
                        if tx.try_send(event).is_err() {
                            tracing::warn!("pipeline channel full, dropping event");
                        }
                        *count += 1;
                    }
                }
            }
        }

        Ok(())
    }

    /// Start watching for file changes.
    ///
    /// Sends `PipelineEvent` messages through the provided channel.
    /// Blocks until a shutdown signal is received or an error occurs.
    pub async fn watch(&self, tx: mpsc::Sender<PipelineEvent>) -> OmniResult<()> {
        let debounce_ms = self.watcher_config.debounce_ms;
        let root = self.root.clone();

        tracing::info!(
            root = %root.display(),
            debounce_ms,
            "starting file watcher"
        );

        // Create a channel for notify events
        let (notify_tx, notify_rx) = std::sync::mpsc::channel();

        // Create debounced watcher
        let mut debouncer = new_debouncer(Duration::from_millis(debounce_ms), notify_tx)
            .map_err(|e| OmniError::Internal(format!("failed to create file watcher: {e}")))?;

        // Start watching
        debouncer
            .watcher()
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| OmniError::Internal(format!("failed to watch directory: {e}")))?;

        // Process events in a blocking task
        let indexing_config = self.indexing_config.clone();
        let max_file_size = self.indexing_config.max_file_size;

        tokio::task::spawn_blocking(move || {
            loop {
                match notify_rx.recv() {
                    Ok(Ok(events)) => {
                        for event in events {
                            let path = event.path;

                            // Filter by event kind
                            match event.kind {
                                DebouncedEventKind::Any => {}
                                _ => continue,
                            }

                            // Skip excluded and non-source files
                            if is_excluded_static(&path, &indexing_config.exclude_patterns) {
                                continue;
                            }

                            if path.is_file() {
                                // Check if it's a recognized source file
                                if !is_source_file_static(&path) {
                                    continue;
                                }

                                // Check file size
                                if let Ok(meta) = std::fs::metadata(&path) {
                                    if meta.len() > max_file_size {
                                        continue;
                                    }
                                }

                                let event = PipelineEvent::FileChanged { path };
                                if tx.try_send(event).is_err() {
                                    tracing::warn!("pipeline channel full");
                                }
                            } else if !path.exists() {
                                // File was deleted
                                let event = PipelineEvent::FileDeleted { path };
                                if tx.try_send(event).is_err() {
                                    tracing::warn!("pipeline channel full");
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::warn!(error = %e, "file watcher error");
                    }
                    Err(_) => {
                        // Channel closed -- watcher was dropped
                        tracing::info!("file watcher channel closed, stopping");
                        break;
                    }
                }
            }
        })
        .await
        .map_err(|e| OmniError::Internal(format!("watcher task panicked: {e}")))?;

        Ok(())
    }

    /// Check if a path should be excluded based on configured patterns.
    fn is_excluded(&self, path: &Path) -> bool {
        is_excluded_static(path, &self.indexing_config.exclude_patterns)
    }
}

/// Check if a path matches any exclude pattern.
///
/// Patterns are matched against individual path components:
/// - "node_modules" matches any path containing a "node_modules" directory
/// - "*.lock" matches files ending in .lock
/// - ".git" matches the .git directory
fn is_excluded_static(path: &Path, exclude_patterns: &[String]) -> bool {
    for component in path.components() {
        let name = component.as_os_str().to_string_lossy();
        for pattern in exclude_patterns {
            if let Some(suffix) = pattern.strip_prefix('*') {
                if name.ends_with(suffix) {
                    return true;
                }
            } else if name == pattern.as_str() {
                return true;
            }
        }
    }
    false
}

/// Check if a file has a recognized source file extension.
fn is_source_file_static(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    !matches!(Language::from_extension(ext), Language::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_excluded_directory() {
        let excludes = vec![".git".into(), "node_modules".into(), "target".into()];
        assert!(is_excluded_static(Path::new("/repo/.git/HEAD"), &excludes));
        assert!(is_excluded_static(
            Path::new("/repo/node_modules/foo/bar.js"),
            &excludes
        ));
        assert!(is_excluded_static(
            Path::new("/repo/target/debug/bin"),
            &excludes
        ));
        assert!(!is_excluded_static(
            Path::new("/repo/src/main.rs"),
            &excludes
        ));
    }

    #[test]
    fn test_is_excluded_glob() {
        let excludes = vec!["*.lock".into(), "*.min.js".into()];
        assert!(is_excluded_static(Path::new("/repo/Cargo.lock"), &excludes));
        assert!(is_excluded_static(Path::new("/repo/app.min.js"), &excludes));
        assert!(!is_excluded_static(Path::new("/repo/app.js"), &excludes));
    }

    #[test]
    fn test_is_source_file() {
        // Code files
        assert!(is_source_file_static(Path::new("main.rs")));
        assert!(is_source_file_static(Path::new("app.py")));
        assert!(is_source_file_static(Path::new("index.ts")));
        assert!(is_source_file_static(Path::new("app.js")));
        assert!(is_source_file_static(Path::new("main.go")));
        assert!(is_source_file_static(Path::new("App.java")));
        assert!(is_source_file_static(Path::new("main.c")));
        assert!(is_source_file_static(Path::new("main.cpp")));
        assert!(is_source_file_static(Path::new("Program.cs")));
        assert!(is_source_file_static(Path::new("styles.css")));
        // Document/config files
        assert!(is_source_file_static(Path::new("README.md")));
        assert!(is_source_file_static(Path::new("data.json")));
        assert!(is_source_file_static(Path::new("config.toml")));
        assert!(is_source_file_static(Path::new("config.yaml")));
        assert!(is_source_file_static(Path::new("index.html")));
        assert!(is_source_file_static(Path::new("script.sh")));
        // Truly unsupported
        assert!(!is_source_file_static(Path::new("Makefile")));
        assert!(!is_source_file_static(Path::new("image.png")));
    }

    #[test]
    fn test_full_scan_with_temp_dir() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();

        // Create some source files
        std::fs::write(root.join("main.rs"), "fn main() {}").expect("write");
        std::fs::write(root.join("lib.py"), "def foo(): pass").expect("write");
        std::fs::write(root.join("README.md"), "# Hello").expect("write");
        std::fs::create_dir(root.join("node_modules")).expect("create dir");
        std::fs::write(root.join("node_modules").join("dep.js"), "var x;").expect("write");

        let watcher_config = WatcherConfig::default();
        let indexing_config = IndexingConfig::default();
        let watcher = FileWatcher::new(root, &watcher_config, &indexing_config);

        let (tx, mut rx) = mpsc::channel(100);
        let count = watcher.full_scan(&tx).expect("scan");

        // Should find main.rs, lib.py, and README.md. Skip node_modules/dep.js.
        assert_eq!(count, 3, "should find exactly 3 indexable files");

        // Drain the events
        let mut events = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            events.push(evt);
        }
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_full_scan_empty_dir() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let watcher_config = WatcherConfig::default();
        let indexing_config = IndexingConfig::default();
        let watcher = FileWatcher::new(dir.path(), &watcher_config, &indexing_config);

        let (tx, _rx) = mpsc::channel(100);
        let count = watcher.full_scan(&tx).expect("scan");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_full_scan_large_file_skipped() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();

        // Create a file that's "too large"
        let mut config = IndexingConfig::default();
        config.max_file_size = 10; // 10 bytes max
        std::fs::write(
            root.join("big.rs"),
            "fn large_function() { /* lots of code */ }",
        )
        .expect("write");
        std::fs::write(root.join("small.rs"), "fn x(){}").expect("write");

        let watcher_config = WatcherConfig::default();
        let watcher = FileWatcher::new(root, &watcher_config, &config);

        let (tx, _rx) = mpsc::channel(100);
        let count = watcher.full_scan(&tx).expect("scan");
        assert_eq!(count, 1, "only the small file should be indexed");
    }

    #[test]
    fn test_full_scan_nested_directories() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("src/core")).expect("create dirs");
        std::fs::write(root.join("src/main.rs"), "fn main() {}").expect("write");
        std::fs::write(root.join("src/core/engine.rs"), "pub struct Engine;").expect("write");

        let watcher_config = WatcherConfig::default();
        let indexing_config = IndexingConfig::default();
        let watcher = FileWatcher::new(root, &watcher_config, &indexing_config);

        let (tx, _rx) = mpsc::channel(100);
        let count = watcher.full_scan(&tx).expect("scan");
        assert_eq!(count, 2);
    }
}
