//! Circuit breaker pattern for preventing cascading failures.
//!
//! A circuit breaker monitors the failure rate of operations and temporarily
//! blocks requests when failures exceed a threshold. This prevents cascading
//! failures and gives failing subsystems time to recover.
//!
//! ## States
//!
//! - **Closed**: Normal operation, requests pass through
//! - **Open**: Too many failures, requests are blocked
//! - **HalfOpen**: Testing if subsystem has recovered
//!
//! ## State Transitions
//!
//! ```text
//! Closed --[failures > threshold]--> Open
//! Open --[timeout expired]--> HalfOpen
//! HalfOpen --[success]--> Closed
//! HalfOpen --[failure]--> Open
//! ```
//!
//! ## Example
//!
//! ```rust
//! # use omni_core::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerError};
//! # use std::time::Duration;
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! let breaker = CircuitBreaker::new("embedder", 5, Duration::from_secs(60));
//!
//! // Protected call
//! match breaker.call(async { Ok::<_, String>("result") }).await {
//!     Ok(result) => println!("Success: {:?}", result),
//!     Err(CircuitBreakerError::Open) => println!("Circuit open, skipping"),
//!     Err(e) => println!("Operation failed: {:?}", e),
//! }
//! # }
//! ```

use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CircuitState {
    /// Normal operation, requests pass through.
    Closed = 0,
    /// Too many failures, requests are blocked.
    Open = 1,
    /// Testing if subsystem has recovered.
    HalfOpen = 2,
}

impl From<u8> for CircuitState {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Closed,
            1 => Self::Open,
            2 => Self::HalfOpen,
            _ => Self::Closed, // Default to closed for invalid values
        }
    }
}

/// Circuit breaker for preventing cascading failures.
///
/// Monitors operation failures and temporarily blocks requests when the failure
/// threshold is exceeded. Automatically attempts recovery after a timeout.
pub struct CircuitBreaker {
    /// Name of the protected subsystem (for logging).
    name: String,
    /// Current state (Closed, Open, HalfOpen).
    state: AtomicU8,
    /// Number of consecutive failures.
    failure_count: AtomicUsize,
    /// Timestamp of last failure (seconds since UNIX epoch).
    last_failure_time: AtomicU64,
    /// Number of failures before opening the circuit.
    failure_threshold: usize,
    /// Time to wait before attempting recovery (seconds).
    timeout_secs: u64,
    /// Total number of successful calls.
    success_count: AtomicUsize,
    /// Total number of failed calls.
    total_failures: AtomicUsize,
    /// Total number of rejected calls (circuit open).
    rejected_count: AtomicUsize,
}

impl CircuitBreaker {
    /// Create a new circuit breaker.
    ///
    /// # Arguments
    ///
    /// - `name`: Name of the protected subsystem (for logging)
    /// - `failure_threshold`: Number of failures before opening the circuit
    /// - `timeout`: Time to wait before attempting recovery
    ///
    /// # Example
    ///
    /// ```rust
    /// use omni_core::resilience::circuit_breaker::CircuitBreaker;
    /// use std::time::Duration;
    ///
    /// let breaker = CircuitBreaker::new("embedder", 5, Duration::from_secs(60));
    /// ```
    pub fn new(name: impl Into<String>, failure_threshold: usize, timeout: Duration) -> Self {
        Self {
            name: name.into(),
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicUsize::new(0),
            last_failure_time: AtomicU64::new(0),
            failure_threshold,
            timeout_secs: timeout.as_secs(),
            success_count: AtomicUsize::new(0),
            total_failures: AtomicUsize::new(0),
            rejected_count: AtomicUsize::new(0),
        }
    }

    /// Get the current state.
    pub fn state(&self) -> CircuitState {
        CircuitState::from(self.state.load(Ordering::Relaxed))
    }

    /// Execute a protected operation.
    ///
    /// Returns `Err(CircuitBreakerError::Open)` if the circuit is open.
    /// Otherwise, executes the operation and updates the circuit state based
    /// on the result.
    pub async fn call<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        match self.state() {
            CircuitState::Open => {
                if self.should_attempt_recovery() {
                    self.transition_to_half_open();
                    self.test_recovery(f).await
                } else {
                    self.rejected_count.fetch_add(1, Ordering::Relaxed);
                    tracing::debug!(
                        circuit = %self.name,
                        state = "open",
                        "circuit breaker rejected request"
                    );
                    Err(CircuitBreakerError::Open)
                }
            }
            CircuitState::HalfOpen => self.test_recovery(f).await,
            CircuitState::Closed => self.execute_with_monitoring(f).await,
        }
    }

    /// Execute operation with monitoring.
    async fn execute_with_monitoring<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        match f.await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(e) => {
                self.on_failure();
                Err(CircuitBreakerError::OperationFailed(e))
            }
        }
    }

    /// Test if subsystem has recovered (HalfOpen state).
    async fn test_recovery<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        match f.await {
            Ok(result) => {
                self.on_recovery_success();
                Ok(result)
            }
            Err(e) => {
                self.on_recovery_failure();
                Err(CircuitBreakerError::OperationFailed(e))
            }
        }
    }

    /// Check if enough time has passed to attempt recovery.
    fn should_attempt_recovery(&self) -> bool {
        let last_failure = self.last_failure_time.load(Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now >= last_failure + self.timeout_secs
    }

    /// Transition to HalfOpen state.
    fn transition_to_half_open(&self) {
        self.state
            .store(CircuitState::HalfOpen as u8, Ordering::Relaxed);
        tracing::info!(
            circuit = %self.name,
            state = "half_open",
            "circuit breaker attempting recovery"
        );
    }

    /// Handle successful operation.
    fn on_success(&self) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
        self.failure_count.store(0, Ordering::Relaxed);
    }

    /// Handle failed operation.
    fn on_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        self.total_failures.fetch_add(1, Ordering::Relaxed);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_failure_time.store(now, Ordering::Relaxed);

        if failures >= self.failure_threshold && self.state() == CircuitState::Closed {
            self.transition_to_open();
        }
    }

    /// Handle successful recovery test.
    fn on_recovery_success(&self) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
        self.failure_count.store(0, Ordering::Relaxed);
        self.state
            .store(CircuitState::Closed as u8, Ordering::Relaxed);

        tracing::info!(
            circuit = %self.name,
            state = "closed",
            "circuit breaker recovered successfully"
        );
    }

    /// Handle failed recovery test.
    fn on_recovery_failure(&self) {
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        self.transition_to_open();
    }

    /// Transition to Open state.
    fn transition_to_open(&self) {
        self.state
            .store(CircuitState::Open as u8, Ordering::Relaxed);

        tracing::warn!(
            circuit = %self.name,
            state = "open",
            failures = self.failure_count.load(Ordering::Relaxed),
            threshold = self.failure_threshold,
            timeout_secs = self.timeout_secs,
            "circuit breaker opened due to repeated failures"
        );
    }

    /// Get circuit breaker statistics.
    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            name: self.name.clone(),
            state: self.state(),
            success_count: self.success_count.load(Ordering::Relaxed),
            failure_count: self.failure_count.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            rejected_count: self.rejected_count.load(Ordering::Relaxed),
            failure_threshold: self.failure_threshold,
            timeout_secs: self.timeout_secs,
        }
    }

    /// Reset the circuit breaker to Closed state.
    pub fn reset(&self) {
        self.state
            .store(CircuitState::Closed as u8, Ordering::Relaxed);
        self.failure_count.store(0, Ordering::Relaxed);
        self.last_failure_time.store(0, Ordering::Relaxed);

        tracing::info!(
            circuit = %self.name,
            "circuit breaker manually reset"
        );
    }

    /// Execute a **synchronous** protected operation.
    ///
    /// Same semantics as [`call`] but for non-async code paths.
    pub fn call_sync<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Result<T, E>,
    {
        match self.state() {
            CircuitState::Open => {
                if self.should_attempt_recovery() {
                    self.transition_to_half_open();
                    match f() {
                        Ok(v) => {
                            self.on_recovery_success();
                            Ok(v)
                        }
                        Err(e) => {
                            self.on_recovery_failure();
                            Err(CircuitBreakerError::OperationFailed(e))
                        }
                    }
                } else {
                    self.rejected_count.fetch_add(1, Ordering::Relaxed);
                    Err(CircuitBreakerError::Open)
                }
            }
            CircuitState::HalfOpen => match f() {
                Ok(v) => {
                    self.on_recovery_success();
                    Ok(v)
                }
                Err(e) => {
                    self.on_recovery_failure();
                    Err(CircuitBreakerError::OperationFailed(e))
                }
            },
            CircuitState::Closed => match f() {
                Ok(v) => {
                    self.on_success();
                    Ok(v)
                }
                Err(e) => {
                    self.on_failure();
                    Err(CircuitBreakerError::OperationFailed(e))
                }
            },
        }
    }
}

/// Circuit breaker statistics.
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// Name of the protected subsystem.
    pub name: String,
    /// Current state.
    pub state: CircuitState,
    /// Total successful calls.
    pub success_count: usize,
    /// Current consecutive failures.
    pub failure_count: usize,
    /// Total failures (all time).
    pub total_failures: usize,
    /// Total rejected calls (circuit open).
    pub rejected_count: usize,
    /// Failure threshold.
    pub failure_threshold: usize,
    /// Timeout in seconds.
    pub timeout_secs: u64,
}

impl CircuitBreakerStats {
    /// Get success rate (0.0 to 1.0).
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.total_failures;
        if total == 0 {
            1.0
        } else {
            self.success_count as f64 / total as f64
        }
    }

    /// Get failure rate (0.0 to 1.0).
    pub fn failure_rate(&self) -> f64 {
        1.0 - self.success_rate()
    }
}

/// Circuit breaker error.
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open, request rejected.
    Open,
    /// Operation failed.
    OperationFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "circuit breaker is open"),
            Self::OperationFailed(e) => write!(f, "operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error> std::error::Error for CircuitBreakerError<E> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let breaker = CircuitBreaker::new("test", 3, Duration::from_secs(60));
        assert_eq!(breaker.state(), CircuitState::Closed);

        // Successful call
        let result = breaker.call(async { Ok::<_, String>("success") }).await;
        assert!(result.is_ok());
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_threshold() {
        let breaker = CircuitBreaker::new("test", 3, Duration::from_secs(60));

        // Fail 3 times
        for _ in 0..3 {
            let _ = breaker.call(async { Err::<(), _>("error") }).await;
        }

        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_rejects_when_open() {
        let breaker = CircuitBreaker::new("test", 2, Duration::from_secs(60));

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker.call(async { Err::<(), _>("error") }).await;
        }

        // Next call should be rejected
        let result = breaker.call(async { Ok::<_, String>("success") }).await;
        assert!(matches!(result, Err(CircuitBreakerError::Open)));

        let stats = breaker.stats();
        assert_eq!(stats.rejected_count, 1);
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let breaker = CircuitBreaker::new("test", 2, Duration::from_secs(1));

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker.call(async { Err::<(), _>("error") }).await;
        }
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Verify circuit is still open before recovery attempt
        assert_eq!(breaker.state(), CircuitState::Open);

        // Next call should transition to HalfOpen, succeed, and close the circuit
        let result = breaker.call(async { Ok::<_, String>("success") }).await;
        assert!(result.is_ok());

        // Give a tiny bit of time for state transition to complete
        tokio::time::sleep(Duration::from_millis(10)).await;

        // After successful recovery, circuit should be closed
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_stats() {
        let breaker = CircuitBreaker::new("test", 5, Duration::from_secs(60));

        // 3 successes
        for _ in 0..3 {
            let _ = breaker.call(async { Ok::<_, String>("success") }).await;
        }

        // 2 failures
        for _ in 0..2 {
            let _ = breaker.call(async { Err::<(), _>("error") }).await;
        }

        let stats = breaker.stats();
        assert_eq!(stats.success_count, 3);
        assert_eq!(stats.total_failures, 2);
        assert_eq!(stats.failure_count, 2);
        assert!((stats.success_rate() - 0.6).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let breaker = CircuitBreaker::new("test", 2, Duration::from_secs(60));

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker.call(async { Err::<(), _>("error") }).await;
        }
        assert_eq!(breaker.state(), CircuitState::Open);

        // Reset
        breaker.reset();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.stats().failure_count, 0);
    }
}
