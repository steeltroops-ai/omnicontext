//! Performance metrics tracking for the daemon.
//!
//! Tracks search latencies, memory usage, and other performance indicators.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

/// Performance metrics tracker.
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    inner: Arc<Mutex<MetricsInner>>,
}

#[derive(Debug)]
struct MetricsInner {
    /// Search latency samples (in milliseconds).
    search_latencies: Vec<u64>,
    /// Maximum number of latency samples to keep.
    max_samples: usize,
    /// Peak memory usage in bytes.
    peak_memory_bytes: u64,
    /// Total number of searches performed.
    total_searches: u64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new(1000) // Keep last 1000 samples
    }
}

impl PerformanceMetrics {
    /// Create a new performance metrics tracker.
    pub fn new(max_samples: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MetricsInner {
                search_latencies: Vec::with_capacity(max_samples),
                max_samples,
                peak_memory_bytes: 0,
                total_searches: 0,
            })),
        }
    }

    /// Record a search latency.
    pub fn record_search_latency(&self, duration: Duration) {
        #[allow(clippy::cast_possible_truncation)]
        let latency_ms = duration.as_millis().min(u128::from(u64::MAX)) as u64;

        if let Ok(mut inner) = self.inner.lock() {
            inner.total_searches += 1;

            // Add sample
            if inner.search_latencies.len() >= inner.max_samples {
                // Remove oldest sample (FIFO)
                inner.search_latencies.remove(0);
            }
            inner.search_latencies.push(latency_ms);
        }
    }

    /// Update peak memory usage.
    pub fn update_memory_usage(&self, current_bytes: u64) {
        if let Ok(mut inner) = self.inner.lock() {
            if current_bytes > inner.peak_memory_bytes {
                inner.peak_memory_bytes = current_bytes;
            }
        }
    }

    /// Get search latency percentile (P50, P95, P99).
    pub fn get_latency_percentile(&self, percentile: f64) -> f64 {
        let Ok(inner) = self.inner.lock() else {
            return 0.0;
        };

        if inner.search_latencies.is_empty() {
            return 0.0;
        }

        let mut sorted = inner.search_latencies.clone();
        sorted.sort_unstable();

        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let index = ((sorted.len() as f64 - 1.0) * percentile).round() as usize;
        let index = index.min(sorted.len() - 1);

        #[allow(clippy::cast_precision_loss)]
        let latency = sorted[index] as f64;
        latency
    }

    /// Get current memory usage in bytes.
    ///
    /// Simple estimation: returns 0 for now.
    /// In production, this could use platform-specific APIs
    /// or be updated from external monitoring.
    #[must_use]
    pub const fn get_current_memory_bytes() -> u64 {
        0
    }

    /// Get peak memory usage in bytes.
    pub fn get_peak_memory_bytes(&self) -> u64 {
        self.inner
            .lock()
            .map(|inner| inner.peak_memory_bytes)
            .unwrap_or(0)
    }

    /// Get total number of searches performed.
    pub fn get_total_searches(&self) -> u64 {
        self.inner
            .lock()
            .map(|inner| inner.total_searches)
            .unwrap_or(0)
    }

    /// Reset all metrics.
    #[allow(dead_code)]
    pub fn reset(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.search_latencies.clear();
            inner.peak_memory_bytes = 0;
            inner.total_searches = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_tracking() {
        let metrics = PerformanceMetrics::new(100);

        // Record some latencies
        metrics.record_search_latency(Duration::from_millis(10));
        metrics.record_search_latency(Duration::from_millis(20));
        metrics.record_search_latency(Duration::from_millis(30));
        metrics.record_search_latency(Duration::from_millis(40));
        metrics.record_search_latency(Duration::from_millis(50));

        // Check percentiles
        let p50 = metrics.get_latency_percentile(0.5);
        assert!((p50 - 30.0).abs() < 1.0); // P50 should be ~30ms

        let p95 = metrics.get_latency_percentile(0.95);
        assert!((p95 - 50.0).abs() < 1.0); // P95 should be ~50ms

        assert_eq!(metrics.get_total_searches(), 5);
    }

    #[test]
    fn test_max_samples() {
        let metrics = PerformanceMetrics::new(3);

        // Record more samples than max
        metrics.record_search_latency(Duration::from_millis(10));
        metrics.record_search_latency(Duration::from_millis(20));
        metrics.record_search_latency(Duration::from_millis(30));
        metrics.record_search_latency(Duration::from_millis(40)); // Should evict 10ms

        // P50 should be based on [20, 30, 40]
        let p50 = metrics.get_latency_percentile(0.5);
        assert!((p50 - 30.0).abs() < 1.0);

        assert_eq!(metrics.get_total_searches(), 4);
    }

    #[test]
    fn test_memory_tracking() {
        let metrics = PerformanceMetrics::new(100);

        metrics.update_memory_usage(1000);
        assert_eq!(metrics.get_peak_memory_bytes(), 1000);

        metrics.update_memory_usage(500); // Lower, shouldn't update peak
        assert_eq!(metrics.get_peak_memory_bytes(), 1000);

        metrics.update_memory_usage(2000); // Higher, should update peak
        assert_eq!(metrics.get_peak_memory_bytes(), 2000);
    }
}
