//! Health monitoring for automatic recovery.
//!
//! Continuously monitors subsystem health and triggers automatic recovery when
//! degradation is detected. Each subsystem reports its health status, and the
//! monitor aggregates this into an overall system health assessment.
//!
//! ## Health States
//!
//! - **Healthy**: All subsystems operating normally
//! - **Degraded**: Some subsystems experiencing issues but still functional
//! - **Critical**: One or more subsystems have failed
//!
//! ## Monitored Subsystems
//!
//! - **Parser**: AST parsing and language detection
//! - **Embedder**: ONNX model inference
//! - **Index**: SQLite database operations
//! - **Vector**: HNSW vector search
//! - **Graph**: Dependency graph queries
//!
//! ## Example
//!
//! ```rust
//! use omni_core::resilience::health_monitor::{HealthMonitor, SubsystemHealth};
//!
//! let monitor = HealthMonitor::new();
//!
//! // Report subsystem health
//! monitor.report_health("embedder", SubsystemHealth::Healthy);
//!
//! // Check overall health
//! let status = monitor.overall_health();
//! println!("System health: {:?}", status);
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

/// Health status of a subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubsystemHealth {
    /// Operating normally.
    Healthy = 0,
    /// Experiencing issues but still functional.
    Degraded = 1,
    /// Failed and non-functional.
    Critical = 2,
}

impl SubsystemHealth {
    /// Check if the subsystem is healthy.
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy)
    }

    /// Check if the subsystem is degraded.
    pub fn is_degraded(&self) -> bool {
        matches!(self, Self::Degraded)
    }

    /// Check if the subsystem is critical.
    pub fn is_critical(&self) -> bool {
        matches!(self, Self::Critical)
    }

    /// Get a human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Degraded => "Degraded",
            Self::Critical => "Critical",
        }
    }
}

/// Health report for a subsystem.
#[derive(Debug, Clone)]
pub struct HealthReport {
    /// Subsystem name.
    pub subsystem: String,
    /// Current health status.
    pub health: SubsystemHealth,
    /// Optional message describing the issue.
    pub message: Option<String>,
    /// Timestamp of the report.
    pub timestamp: Instant,
}

impl HealthReport {
    /// Create a new health report.
    pub fn new(subsystem: impl Into<String>, health: SubsystemHealth) -> Self {
        Self {
            subsystem: subsystem.into(),
            health,
            message: None,
            timestamp: Instant::now(),
        }
    }

    /// Create a health report with a message.
    pub fn with_message(
        subsystem: impl Into<String>,
        health: SubsystemHealth,
        message: impl Into<String>,
    ) -> Self {
        Self {
            subsystem: subsystem.into(),
            health,
            message: Some(message.into()),
            timestamp: Instant::now(),
        }
    }

    /// Check if the report is stale (older than threshold).
    pub fn is_stale(&self, threshold: Duration) -> bool {
        self.timestamp.elapsed() > threshold
    }
}

/// Overall system health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemHealth {
    /// All subsystems healthy.
    Healthy,
    /// Some subsystems degraded.
    Degraded,
    /// One or more subsystems critical.
    Critical,
}

impl SystemHealth {
    /// Get a human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Degraded => "Degraded",
            Self::Critical => "Critical",
        }
    }
}

/// Health monitor for tracking subsystem health.
pub struct HealthMonitor {
    /// Health reports for each subsystem.
    reports: Arc<RwLock<HashMap<String, HealthReport>>>,
    /// Threshold for considering a report stale.
    staleness_threshold: Duration,
}

impl HealthMonitor {
    /// Create a new health monitor.
    ///
    /// Reports older than 5 minutes are considered stale.
    pub fn new() -> Self {
        Self {
            reports: Arc::new(RwLock::new(HashMap::new())),
            staleness_threshold: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Create a health monitor with a custom staleness threshold.
    pub fn with_staleness_threshold(threshold: Duration) -> Self {
        Self {
            reports: Arc::new(RwLock::new(HashMap::new())),
            staleness_threshold: threshold,
        }
    }

    /// Report health for a subsystem.
    pub fn report_health(&self, subsystem: impl Into<String>, health: SubsystemHealth) {
        let report = HealthReport::new(subsystem, health);
        self.report(report);
    }

    /// Report health with a message.
    pub fn report_health_with_message(
        &self,
        subsystem: impl Into<String>,
        health: SubsystemHealth,
        message: impl Into<String>,
    ) {
        let report = HealthReport::with_message(subsystem, health, message);
        self.report(report);
    }

    /// Report a health report.
    fn report(&self, report: HealthReport) {
        let subsystem = report.subsystem.clone();
        let health = report.health;

        tracing::debug!(
            subsystem = %subsystem,
            health = health.name(),
            message = ?report.message,
            "health report received"
        );

        self.reports.write().insert(subsystem, report);
    }

    /// Get the overall system health.
    pub fn overall_health(&self) -> SystemHealth {
        let reports = self.reports.read();

        if reports.is_empty() {
            return SystemHealth::Healthy;
        }

        let mut has_critical = false;
        let mut has_degraded = false;

        for report in reports.values() {
            // Skip stale reports
            if report.is_stale(self.staleness_threshold) {
                continue;
            }

            match report.health {
                SubsystemHealth::Critical => has_critical = true,
                SubsystemHealth::Degraded => has_degraded = true,
                SubsystemHealth::Healthy => {}
            }
        }

        if has_critical {
            SystemHealth::Critical
        } else if has_degraded {
            SystemHealth::Degraded
        } else {
            SystemHealth::Healthy
        }
    }

    /// Get health reports for all subsystems.
    pub fn all_reports(&self) -> Vec<HealthReport> {
        self.reports.read().values().cloned().collect()
    }

    /// Get health report for a specific subsystem.
    pub fn get_report(&self, subsystem: &str) -> Option<HealthReport> {
        self.reports.read().get(subsystem).cloned()
    }

    /// Get unhealthy subsystems.
    pub fn unhealthy_subsystems(&self) -> Vec<HealthReport> {
        self.reports
            .read()
            .values()
            .filter(|r| !r.health.is_healthy() && !r.is_stale(self.staleness_threshold))
            .cloned()
            .collect()
    }

    /// Clear all health reports.
    pub fn clear(&self) {
        self.reports.write().clear();
    }

    /// Remove stale reports.
    pub fn prune_stale(&self) {
        let mut reports = self.reports.write();
        reports.retain(|_, report| !report.is_stale(self.staleness_threshold));
    }

    /// Get health statistics.
    pub fn stats(&self) -> HealthStats {
        let reports = self.reports.read();
        let total = reports.len();

        let mut healthy = 0;
        let mut degraded = 0;
        let mut critical = 0;
        let mut stale = 0;

        for report in reports.values() {
            if report.is_stale(self.staleness_threshold) {
                stale += 1;
                continue;
            }

            match report.health {
                SubsystemHealth::Healthy => healthy += 1,
                SubsystemHealth::Degraded => degraded += 1,
                SubsystemHealth::Critical => critical += 1,
            }
        }

        HealthStats {
            total,
            healthy,
            degraded,
            critical,
            stale,
        }
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Health statistics.
#[derive(Debug, Clone)]
pub struct HealthStats {
    /// Total number of subsystems.
    pub total: usize,
    /// Number of healthy subsystems.
    pub healthy: usize,
    /// Number of degraded subsystems.
    pub degraded: usize,
    /// Number of critical subsystems.
    pub critical: usize,
    /// Number of stale reports.
    pub stale: usize,
}

impl HealthStats {
    /// Get the health rate (0.0 to 1.0).
    pub fn health_rate(&self) -> f64 {
        let active = self.total - self.stale;
        if active == 0 {
            1.0
        } else {
            self.healthy as f64 / active as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subsystem_health_ordering() {
        assert!(SubsystemHealth::Healthy < SubsystemHealth::Degraded);
        assert!(SubsystemHealth::Degraded < SubsystemHealth::Critical);
    }

    #[test]
    fn test_health_monitor_report() {
        let monitor = HealthMonitor::new();

        monitor.report_health("embedder", SubsystemHealth::Healthy);
        monitor.report_health("parser", SubsystemHealth::Degraded);

        let report = monitor.get_report("embedder").unwrap();
        assert_eq!(report.health, SubsystemHealth::Healthy);

        let report = monitor.get_report("parser").unwrap();
        assert_eq!(report.health, SubsystemHealth::Degraded);
    }

    #[test]
    fn test_overall_health_healthy() {
        let monitor = HealthMonitor::new();

        monitor.report_health("embedder", SubsystemHealth::Healthy);
        monitor.report_health("parser", SubsystemHealth::Healthy);

        assert_eq!(monitor.overall_health(), SystemHealth::Healthy);
    }

    #[test]
    fn test_overall_health_degraded() {
        let monitor = HealthMonitor::new();

        monitor.report_health("embedder", SubsystemHealth::Healthy);
        monitor.report_health("parser", SubsystemHealth::Degraded);

        assert_eq!(monitor.overall_health(), SystemHealth::Degraded);
    }

    #[test]
    fn test_overall_health_critical() {
        let monitor = HealthMonitor::new();

        monitor.report_health("embedder", SubsystemHealth::Healthy);
        monitor.report_health("parser", SubsystemHealth::Critical);

        assert_eq!(monitor.overall_health(), SystemHealth::Critical);
    }

    #[test]
    fn test_unhealthy_subsystems() {
        let monitor = HealthMonitor::new();

        monitor.report_health("embedder", SubsystemHealth::Healthy);
        monitor.report_health("parser", SubsystemHealth::Degraded);
        monitor.report_health("index", SubsystemHealth::Critical);

        let unhealthy = monitor.unhealthy_subsystems();
        assert_eq!(unhealthy.len(), 2);
    }

    #[test]
    fn test_health_stats() {
        let monitor = HealthMonitor::new();

        monitor.report_health("embedder", SubsystemHealth::Healthy);
        monitor.report_health("parser", SubsystemHealth::Degraded);
        monitor.report_health("index", SubsystemHealth::Critical);

        let stats = monitor.stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.healthy, 1);
        assert_eq!(stats.degraded, 1);
        assert_eq!(stats.critical, 1);
        assert!((stats.health_rate() - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_stale_reports() {
        let monitor = HealthMonitor::with_staleness_threshold(Duration::from_millis(100));

        monitor.report_health("embedder", SubsystemHealth::Healthy);

        // Wait for report to become stale
        std::thread::sleep(Duration::from_millis(150));

        let stats = monitor.stats();
        assert_eq!(stats.stale, 1);
    }

    #[test]
    fn test_prune_stale() {
        let monitor = HealthMonitor::with_staleness_threshold(Duration::from_millis(100));

        monitor.report_health("embedder", SubsystemHealth::Healthy);
        monitor.report_health("parser", SubsystemHealth::Healthy);

        // Wait for reports to become stale
        std::thread::sleep(Duration::from_millis(150));

        monitor.prune_stale();

        let stats = monitor.stats();
        assert_eq!(stats.total, 0);
    }

    #[test]
    fn test_clear() {
        let monitor = HealthMonitor::new();

        monitor.report_health("embedder", SubsystemHealth::Healthy);
        monitor.report_health("parser", SubsystemHealth::Healthy);

        monitor.clear();

        let stats = monitor.stats();
        assert_eq!(stats.total, 0);
    }
}
