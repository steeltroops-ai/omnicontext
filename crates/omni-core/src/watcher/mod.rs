//! File system watcher with debouncing.
//!
//! Uses the `notify` crate for platform-native filesystem monitoring.
//! Events are debounced and sent through a channel to the indexing pipeline.

use std::path::{Path, PathBuf};

use tokio::sync::mpsc;

use crate::config::WatcherConfig;
use crate::types::PipelineEvent;
use crate::error::OmniResult;

/// File system watcher that emits pipeline events.
pub struct FileWatcher {
    config: WatcherConfig,
    root: PathBuf,
}

impl FileWatcher {
    /// Create a new file watcher for the given root directory.
    pub fn new(root: &Path, config: &WatcherConfig) -> Self {
        Self {
            config: config.clone(),
            root: root.to_path_buf(),
        }
    }

    /// Start watching for file changes.
    ///
    /// Sends `PipelineEvent` messages through the provided channel.
    /// This function blocks until shutdown is requested.
    pub async fn watch(&self, _tx: mpsc::Sender<PipelineEvent>) -> OmniResult<()> {
        // TODO: Implement using notify crate
        // 1. Create a debouncing watcher
        // 2. Recursively watch self.root
        // 3. Filter events against exclude patterns
        // 4. Map notify events to PipelineEvent
        // 5. Send through channel
        // 6. Return on PipelineEvent::Shutdown
        tracing::info!(root = %self.root.display(), "file watcher started");
        Ok(())
    }

    /// Perform a full directory scan and emit FileChanged for all source files.
    pub fn full_scan(&self, _tx: &mpsc::Sender<PipelineEvent>) -> OmniResult<usize> {
        // TODO: Walk directory, filter by language, emit events
        Ok(0)
    }
}
