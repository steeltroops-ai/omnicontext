//! Event deduplication for IDE events.
//!
//! Tracks in-flight re-indexing tasks and skips duplicate events to prevent
//! redundant work. This is particularly important for rapid file edits where
//! multiple `text_edited` events may arrive before the first re-index completes.
//!
//! ## Strategy
//!
//! - Track in-flight tasks by file path
//! - Skip duplicate events for files already being processed
//! - Clean up completed tasks automatically
//! - Provide statistics for monitoring
//!
//! ## Example
//!
//! ```rust
//! use omni_daemon::event_dedup::EventDeduplicator;
//!
//! let dedup = EventDeduplicator::new();
//!
//! // Try to start processing a file
//! if dedup.try_start_processing("src/main.rs") {
//!     // Process the file...
//!     dedup.finish_processing("src/main.rs");
//! } else {
//!     // Skip duplicate event
//!     println!("Already processing src/main.rs");
//! }
//! ```

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::RwLock;

/// Event deduplicator for tracking in-flight tasks.
#[derive(Clone)]
pub struct EventDeduplicator {
    /// In-flight tasks by file path.
    in_flight: Arc<RwLock<HashSet<PathBuf>>>,
    /// Statistics for monitoring.
    stats: Arc<RwLock<DeduplicationStats>>,
    /// Task start times for timeout detection.
    task_times: Arc<RwLock<HashMap<PathBuf, Instant>>>,
}

impl EventDeduplicator {
    /// Create a new event deduplicator.
    pub fn new() -> Self {
        Self {
            in_flight: Arc::new(RwLock::new(HashSet::new())),
            stats: Arc::new(RwLock::new(DeduplicationStats::default())),
            task_times: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Try to start processing a file.
    ///
    /// Returns `true` if processing should proceed, `false` if the file is
    /// already being processed (duplicate event).
    pub fn try_start_processing(&self, file_path: impl Into<PathBuf>) -> bool {
        let path = file_path.into();
        let mut in_flight = self.in_flight.write();

        if in_flight.contains(&path) {
            // Duplicate event
            self.stats.write().duplicates_skipped += 1;
            tracing::debug!(
                file = %path.display(),
                "skipping duplicate event (already in-flight)"
            );
            false
        } else {
            // New event
            in_flight.insert(path.clone());
            self.task_times.write().insert(path, Instant::now());
            self.stats.write().events_processed += 1;
            true
        }
    }

    /// Finish processing a file.
    ///
    /// Removes the file from the in-flight set and records processing time.
    pub fn finish_processing(&self, file_path: impl Into<PathBuf>) {
        let path = file_path.into();
        let mut in_flight = self.in_flight.write();
        in_flight.remove(&path);

        // Record processing time
        if let Some(start_time) = self.task_times.write().remove(&path) {
            let elapsed = start_time.elapsed();
            let mut stats = self.stats.write();
            stats.total_processing_time_ms += elapsed.as_millis() as u64;
            stats.tasks_completed += 1;
        }
    }

    /// Check if a file is currently being processed.
    pub fn is_processing(&self, file_path: impl Into<PathBuf>) -> bool {
        let path = file_path.into();
        self.in_flight.read().contains(&path)
    }

    /// Get the number of in-flight tasks.
    pub fn in_flight_count(&self) -> usize {
        self.in_flight.read().len()
    }

    /// Get deduplication statistics.
    pub fn stats(&self) -> DeduplicationStats {
        self.stats.read().clone()
    }

    /// Reset statistics.
    pub fn reset_stats(&self) {
        *self.stats.write() = DeduplicationStats::default();
    }

    /// Clean up stale tasks (tasks that have been in-flight for too long).
    ///
    /// This is a safety mechanism to prevent tasks from getting stuck in the
    /// in-flight set if they fail without calling `finish_processing`.
    pub fn cleanup_stale_tasks(&self, timeout_secs: u64) {
        let now = Instant::now();
        let mut in_flight = self.in_flight.write();
        let mut task_times = self.task_times.write();
        let mut stats = self.stats.write();

        let stale_tasks: Vec<PathBuf> = task_times
            .iter()
            .filter(|(_, start_time)| now.duration_since(**start_time).as_secs() >= timeout_secs)
            .map(|(path, _)| path.clone())
            .collect();

        for path in stale_tasks {
            in_flight.remove(&path);
            task_times.remove(&path);
            stats.stale_tasks_cleaned += 1;

            tracing::warn!(
                file = %path.display(),
                timeout_secs = timeout_secs,
                "cleaned up stale task (exceeded timeout)"
            );
        }
    }

    /// Get all in-flight file paths.
    pub fn in_flight_files(&self) -> Vec<PathBuf> {
        self.in_flight.read().iter().cloned().collect()
    }
}

impl Default for EventDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

/// Deduplication statistics.
#[derive(Debug, Clone, Default)]
pub struct DeduplicationStats {
    /// Total events processed (not skipped).
    pub events_processed: u64,
    /// Duplicate events skipped.
    pub duplicates_skipped: u64,
    /// Tasks completed successfully.
    pub tasks_completed: u64,
    /// Total processing time in milliseconds.
    pub total_processing_time_ms: u64,
    /// Stale tasks cleaned up.
    pub stale_tasks_cleaned: u64,
}

impl DeduplicationStats {
    /// Get the deduplication rate (0.0 to 1.0).
    pub fn deduplication_rate(&self) -> f64 {
        let total = self.events_processed + self.duplicates_skipped;
        if total == 0 {
            0.0
        } else {
            self.duplicates_skipped as f64 / total as f64
        }
    }

    /// Get the average processing time in milliseconds.
    pub fn avg_processing_time_ms(&self) -> f64 {
        if self.tasks_completed == 0 {
            0.0
        } else {
            self.total_processing_time_ms as f64 / self.tasks_completed as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_start_processing() {
        let dedup = EventDeduplicator::new();

        // First event should succeed
        assert!(dedup.try_start_processing("src/main.rs"));

        // Duplicate event should be skipped
        assert!(!dedup.try_start_processing("src/main.rs"));

        // Different file should succeed
        assert!(dedup.try_start_processing("src/lib.rs"));
    }

    #[test]
    fn test_finish_processing() {
        let dedup = EventDeduplicator::new();

        dedup.try_start_processing("src/main.rs");
        assert_eq!(dedup.in_flight_count(), 1);

        dedup.finish_processing("src/main.rs");
        assert_eq!(dedup.in_flight_count(), 0);

        // Should be able to process again
        assert!(dedup.try_start_processing("src/main.rs"));
    }

    #[test]
    fn test_is_processing() {
        let dedup = EventDeduplicator::new();

        assert!(!dedup.is_processing("src/main.rs"));

        dedup.try_start_processing("src/main.rs");
        assert!(dedup.is_processing("src/main.rs"));

        dedup.finish_processing("src/main.rs");
        assert!(!dedup.is_processing("src/main.rs"));
    }

    #[test]
    fn test_stats() {
        let dedup = EventDeduplicator::new();

        // Process 3 events, 2 duplicates
        dedup.try_start_processing("src/main.rs");
        dedup.try_start_processing("src/main.rs"); // duplicate
        dedup.try_start_processing("src/lib.rs");
        dedup.try_start_processing("src/lib.rs"); // duplicate

        let stats = dedup.stats();
        assert_eq!(stats.events_processed, 2);
        assert_eq!(stats.duplicates_skipped, 2);
        assert_eq!(stats.deduplication_rate(), 0.5);
    }

    #[test]
    fn test_cleanup_stale_tasks() {
        let dedup = EventDeduplicator::new();

        dedup.try_start_processing("src/main.rs");
        assert_eq!(dedup.in_flight_count(), 1);

        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(150));

        // Cleanup with 0.1 second timeout (task should be stale)
        dedup.cleanup_stale_tasks(0); // 0 seconds = everything is stale immediately

        // After cleanup, in-flight should be 0
        assert_eq!(dedup.in_flight_count(), 0);

        let stats = dedup.stats();
        assert_eq!(stats.stale_tasks_cleaned, 1);
    }

    #[test]
    fn test_in_flight_files() {
        let dedup = EventDeduplicator::new();

        dedup.try_start_processing("src/main.rs");
        dedup.try_start_processing("src/lib.rs");

        let files = dedup.in_flight_files();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&PathBuf::from("src/main.rs")));
        assert!(files.contains(&PathBuf::from("src/lib.rs")));
    }

    #[test]
    fn test_reset_stats() {
        let dedup = EventDeduplicator::new();

        dedup.try_start_processing("src/main.rs");
        dedup.try_start_processing("src/main.rs"); // duplicate

        let stats = dedup.stats();
        assert_eq!(stats.events_processed, 1);
        assert_eq!(stats.duplicates_skipped, 1);

        dedup.reset_stats();

        let stats = dedup.stats();
        assert_eq!(stats.events_processed, 0);
        assert_eq!(stats.duplicates_skipped, 0);
    }
}
