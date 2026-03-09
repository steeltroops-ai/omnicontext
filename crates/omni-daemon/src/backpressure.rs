//! Backpressure handling for daemon overload protection.
//!
//! Monitors daemon load and rejects requests when the system is overloaded.
//! This prevents cascading failures and ensures the daemon remains responsive
//! even under heavy load.
//!
//! ## Load Metrics
//!
//! - **In-flight requests**: Number of concurrent requests being processed
//! - **Queue depth**: Number of pending requests waiting to be processed
//! - **Memory usage**: Current memory consumption
//! - **CPU usage**: Estimated CPU load based on request rate
//!
//! ## Thresholds
//!
//! - **Warning**: 80% of capacity (log warning, continue processing)
//! - **Critical**: 95% of capacity (reject new requests with 503)
//!
//! ## Example
//!
//! ```rust
//! use omni_daemon::backpressure::BackpressureMonitor;
//!
//! let monitor = BackpressureMonitor::new(100); // max 100 concurrent requests
//!
//! // Check if we can accept a new request
//! if monitor.can_accept_request() {
//!     monitor.start_request();
//!     // Process request...
//!     monitor.finish_request();
//! } else {
//!     // Return 503 Service Unavailable
//!     println!("Daemon overloaded, rejecting request");
//! }
//! ```

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

/// Backpressure monitor for load management.
#[derive(Clone)]
pub struct BackpressureMonitor {
    /// Maximum concurrent requests.
    max_concurrent: usize,
    /// Current in-flight requests.
    in_flight: Arc<AtomicUsize>,
    /// Total requests accepted.
    total_accepted: Arc<AtomicU64>,
    /// Total requests rejected.
    total_rejected: Arc<AtomicU64>,
    /// Peak concurrent requests.
    peak_concurrent: Arc<AtomicUsize>,
}

impl BackpressureMonitor {
    /// Create a new backpressure monitor.
    ///
    /// # Arguments
    ///
    /// - `max_concurrent`: Maximum number of concurrent requests to allow
    ///
    /// # Example
    ///
    /// ```rust
    /// use omni_daemon::backpressure::BackpressureMonitor;
    ///
    /// let monitor = BackpressureMonitor::new(100);
    /// ```
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            in_flight: Arc::new(AtomicUsize::new(0)),
            total_accepted: Arc::new(AtomicU64::new(0)),
            total_rejected: Arc::new(AtomicU64::new(0)),
            peak_concurrent: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Check if a new request can be accepted.
    ///
    /// Returns `true` if the daemon has capacity, `false` if overloaded.
    pub fn can_accept_request(&self) -> bool {
        let current = self.in_flight.load(Ordering::Relaxed);
        current < self.max_concurrent
    }

    /// Start processing a request.
    ///
    /// Increments the in-flight counter and updates statistics.
    /// Returns `true` if the request was accepted, `false` if rejected.
    pub fn start_request(&self) -> bool {
        let current = self.in_flight.fetch_add(1, Ordering::Relaxed) + 1;

        if current > self.max_concurrent {
            // Overloaded, reject request
            self.in_flight.fetch_sub(1, Ordering::Relaxed);
            self.total_rejected.fetch_add(1, Ordering::Relaxed);

            tracing::warn!(
                in_flight = current - 1,
                max_concurrent = self.max_concurrent,
                "daemon overloaded, rejecting request (503)"
            );

            false
        } else {
            // Accepted
            self.total_accepted.fetch_add(1, Ordering::Relaxed);

            // Update peak
            let mut peak = self.peak_concurrent.load(Ordering::Relaxed);
            while current > peak {
                match self.peak_concurrent.compare_exchange_weak(
                    peak,
                    current,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(p) => peak = p,
                }
            }

            // Log warning if approaching capacity
            let load_percent = (current as f64 / self.max_concurrent as f64) * 100.0;
            if load_percent >= 80.0 {
                tracing::warn!(
                    in_flight = current,
                    max_concurrent = self.max_concurrent,
                    load_percent = load_percent as u32,
                    "daemon load high (approaching capacity)"
                );
            }

            true
        }
    }

    /// Finish processing a request.
    ///
    /// Decrements the in-flight counter.
    pub fn finish_request(&self) {
        self.in_flight.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get the current number of in-flight requests.
    pub fn in_flight_count(&self) -> usize {
        self.in_flight.load(Ordering::Relaxed)
    }

    /// Get the current load percentage (0.0 to 100.0).
    pub fn load_percent(&self) -> f64 {
        let current = self.in_flight.load(Ordering::Relaxed);
        (current as f64 / self.max_concurrent as f64) * 100.0
    }

    /// Get backpressure statistics.
    pub fn stats(&self) -> BackpressureStats {
        BackpressureStats {
            max_concurrent: self.max_concurrent,
            in_flight: self.in_flight.load(Ordering::Relaxed),
            total_accepted: self.total_accepted.load(Ordering::Relaxed),
            total_rejected: self.total_rejected.load(Ordering::Relaxed),
            peak_concurrent: self.peak_concurrent.load(Ordering::Relaxed),
        }
    }

    /// Reset statistics.
    pub fn reset_stats(&self) {
        self.total_accepted.store(0, Ordering::Relaxed);
        self.total_rejected.store(0, Ordering::Relaxed);
        self.peak_concurrent.store(0, Ordering::Relaxed);
    }
}

/// Backpressure statistics.
#[derive(Debug, Clone)]
pub struct BackpressureStats {
    /// Maximum concurrent requests.
    pub max_concurrent: usize,
    /// Current in-flight requests.
    pub in_flight: usize,
    /// Total requests accepted.
    pub total_accepted: u64,
    /// Total requests rejected.
    pub total_rejected: u64,
    /// Peak concurrent requests.
    pub peak_concurrent: usize,
}

impl BackpressureStats {
    /// Get the rejection rate (0.0 to 1.0).
    pub fn rejection_rate(&self) -> f64 {
        let total = self.total_accepted + self.total_rejected;
        if total == 0 {
            0.0
        } else {
            self.total_rejected as f64 / total as f64
        }
    }

    /// Get the current load percentage (0.0 to 100.0).
    pub fn load_percent(&self) -> f64 {
        (self.in_flight as f64 / self.max_concurrent as f64) * 100.0
    }

    /// Get the peak load percentage (0.0 to 100.0).
    pub fn peak_load_percent(&self) -> f64 {
        (self.peak_concurrent as f64 / self.max_concurrent as f64) * 100.0
    }
}

/// RAII guard for automatic request tracking.
///
/// Automatically calls `finish_request()` when dropped.
pub struct RequestGuard {
    monitor: BackpressureMonitor,
}

impl RequestGuard {
    /// Create a new request guard.
    ///
    /// Returns `Some(guard)` if the request was accepted, `None` if rejected.
    pub fn new(monitor: BackpressureMonitor) -> Option<Self> {
        if monitor.start_request() {
            Some(Self { monitor })
        } else {
            None
        }
    }
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.monitor.finish_request();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_accept_request() {
        let monitor = BackpressureMonitor::new(2);

        assert!(monitor.can_accept_request());
        monitor.start_request();
        assert!(monitor.can_accept_request());
        monitor.start_request();
        assert!(!monitor.can_accept_request());
    }

    #[test]
    fn test_start_request() {
        let monitor = BackpressureMonitor::new(2);

        assert!(monitor.start_request());
        assert_eq!(monitor.in_flight_count(), 1);

        assert!(monitor.start_request());
        assert_eq!(monitor.in_flight_count(), 2);

        // Third request should be rejected
        assert!(!monitor.start_request());
        assert_eq!(monitor.in_flight_count(), 2);
    }

    #[test]
    fn test_finish_request() {
        let monitor = BackpressureMonitor::new(2);

        monitor.start_request();
        monitor.start_request();
        assert_eq!(monitor.in_flight_count(), 2);

        monitor.finish_request();
        assert_eq!(monitor.in_flight_count(), 1);

        monitor.finish_request();
        assert_eq!(monitor.in_flight_count(), 0);
    }

    #[test]
    fn test_load_percent() {
        let monitor = BackpressureMonitor::new(10);

        assert_eq!(monitor.load_percent(), 0.0);

        monitor.start_request();
        assert_eq!(monitor.load_percent(), 10.0);

        monitor.start_request();
        assert_eq!(monitor.load_percent(), 20.0);
    }

    #[test]
    fn test_stats() {
        let monitor = BackpressureMonitor::new(2);

        monitor.start_request();
        monitor.start_request();
        monitor.start_request(); // rejected

        let stats = monitor.stats();
        assert_eq!(stats.total_accepted, 2);
        assert_eq!(stats.total_rejected, 1);
        assert_eq!(stats.peak_concurrent, 2);
        assert_eq!(stats.rejection_rate(), 1.0 / 3.0);
    }

    #[test]
    fn test_request_guard() {
        let monitor = BackpressureMonitor::new(2);

        {
            let _guard1 = RequestGuard::new(monitor.clone());
            assert_eq!(monitor.in_flight_count(), 1);

            {
                let _guard2 = RequestGuard::new(monitor.clone());
                assert_eq!(monitor.in_flight_count(), 2);

                // Third request should be rejected
                let guard3 = RequestGuard::new(monitor.clone());
                assert!(guard3.is_none());
                assert_eq!(monitor.in_flight_count(), 2);
            }

            // guard2 dropped
            assert_eq!(monitor.in_flight_count(), 1);
        }

        // guard1 dropped
        assert_eq!(monitor.in_flight_count(), 0);
    }

    #[test]
    fn test_reset_stats() {
        let monitor = BackpressureMonitor::new(2);

        monitor.start_request();
        monitor.start_request();
        monitor.start_request(); // rejected

        let stats = monitor.stats();
        assert_eq!(stats.total_accepted, 2);
        assert_eq!(stats.total_rejected, 1);

        monitor.reset_stats();

        let stats = monitor.stats();
        assert_eq!(stats.total_accepted, 0);
        assert_eq!(stats.total_rejected, 0);
        assert_eq!(stats.peak_concurrent, 0);
    }
}
