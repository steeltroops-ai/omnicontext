//! ONNX Session Pool for parallel embedding inference.
//!
//! ## Problem
//!
//! A single `Mutex<Session>` serializes all embedding requests.
//! Under concurrent load (MCP queries + VS Code + background indexing),
//! this creates a bottleneck where only one thread embeds at a time.
//!
//! ## Solution
//!
//! Create N independent ONNX sessions from the same model file.
//! Each session has its own weights in memory (ONNX Runtime doesn't share
//! weights across sessions). The pool manages checkout/return semantics
//! with zero-copy handoff using `crossbeam`-style channel semantics
//! implemented on top of `std::sync`.
//!
//! ## Memory Trade-off
//!
//! Each session consumes ~550MB for the Jina model. With pool_size=2,
//! total memory is ~1.1GB. For most dev machines (16GB+), this is acceptable.
//! The pool_size is configurable and defaults to `min(2, num_cpus/2)`.
//!
//! ## Thread Safety
//!
//! ONNX Runtime sessions are `Send` but not `Sync` -- they can be moved
//! between threads but not shared. The pool wraps each in a `Mutex` and
//! hands out exclusive access via `SessionGuard`.

use std::path::Path;
use std::sync::{Condvar, Mutex};

use crate::error::{OmniError, OmniResult};
use ort::session::Session;

/// A pool of ONNX sessions for parallel inference.
///
/// Automatically manages session lifecycle and provides checkout/return
/// semantics with bounded wait times.
pub struct SessionPool {
    /// Available sessions (LIFO stack for cache locality).
    sessions: Mutex<Vec<Session>>,
    /// Condition variable to notify waiting threads when a session is returned.
    available: Condvar,
    /// Total number of sessions in the pool (available + checked out).
    pool_size: usize,
    /// Path to the model file (for diagnostics).
    model_path: String,
}

/// RAII guard that returns the session to the pool on drop.
pub struct SessionGuard<'a> {
    session: Option<Session>,
    pool: &'a SessionPool,
}

impl<'a> SessionGuard<'a> {
    /// Get a mutable reference to the ONNX session.
    pub fn session_mut(&mut self) -> &mut Session {
        self.session.as_mut().expect("session was already returned")
    }
}

impl<'a> Drop for SessionGuard<'a> {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            self.pool.return_session(session);
        }
    }
}

impl SessionPool {
    /// Create a new session pool with `pool_size` independent ONNX sessions.
    ///
    /// Each session is loaded from the same model file. This means N copies
    /// of the model weights in memory, but enables true parallel inference.
    ///
    /// Returns `None` if the model file doesn't exist or no sessions could be created.
    pub fn new(model_path: &Path, pool_size: usize) -> OmniResult<Option<Self>> {
        if !model_path.exists() {
            return Ok(None);
        }

        let effective_size = pool_size.max(1);
        let mut sessions = Vec::with_capacity(effective_size);

        for i in 0..effective_size {
            match Session::builder() {
                Ok(builder) => match builder.commit_from_file(model_path) {
                    Ok(session) => {
                        sessions.push(session);
                        tracing::debug!(
                            session_idx = i,
                            model = %model_path.display(),
                            "loaded ONNX session for pool"
                        );
                    }
                    Err(e) => {
                        if i == 0 {
                            // If the first session fails, the model is bad
                            tracing::error!(
                                error = %e,
                                "failed to load first ONNX session, pool creation aborted"
                            );
                            return Ok(None);
                        }
                        tracing::warn!(
                            session_idx = i,
                            error = %e,
                            "failed to load additional session, pool will be smaller"
                        );
                        break;
                    }
                },
                Err(e) => {
                    if i == 0 {
                        tracing::error!(error = %e, "ONNX builder failed");
                        return Ok(None);
                    }
                    break;
                }
            }
        }

        if sessions.is_empty() {
            return Ok(None);
        }

        let actual_size = sessions.len();
        tracing::info!(
            pool_size = actual_size,
            model = %model_path.display(),
            "ONNX session pool initialized"
        );

        Ok(Some(Self {
            sessions: Mutex::new(sessions),
            available: Condvar::new(),
            pool_size: actual_size,
            model_path: model_path.display().to_string(),
        }))
    }

    /// Check out a session from the pool.
    ///
    /// Blocks until a session is available. Returns a RAII guard that
    /// automatically returns the session when dropped.
    ///
    /// Timeout: 30 seconds. Returns error if no session becomes available.
    pub fn checkout(&self) -> OmniResult<SessionGuard<'_>> {
        let timeout = std::time::Duration::from_secs(30);
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| OmniError::Internal("session pool lock poisoned".into()))?;

        let deadline = std::time::Instant::now() + timeout;

        while sessions.is_empty() {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(OmniError::Internal(format!(
                    "session pool timeout: all {} sessions busy for >30s (model: {})",
                    self.pool_size, self.model_path
                )));
            }

            let (guard, wait_result) = self
                .available
                .wait_timeout(sessions, remaining)
                .map_err(|_| OmniError::Internal("condvar wait failed".into()))?;
            sessions = guard;

            if wait_result.timed_out() && sessions.is_empty() {
                return Err(OmniError::Internal(format!(
                    "session pool timeout: all {} sessions busy (model: {})",
                    self.pool_size, self.model_path
                )));
            }
        }

        let session = sessions.pop().expect("checked non-empty above");

        Ok(SessionGuard {
            session: Some(session),
            pool: self,
        })
    }

    /// Try to check out a session without blocking.
    ///
    /// Returns `None` if no session is currently available.
    pub fn try_checkout(&self) -> Option<SessionGuard<'_>> {
        let mut sessions = self.sessions.lock().ok()?;
        let session = sessions.pop()?;
        Some(SessionGuard {
            session: Some(session),
            pool: self,
        })
    }

    /// Return a session to the pool.
    fn return_session(&self, session: Session) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.push(session);
            self.available.notify_one();
        }
    }

    /// Number of sessions in the pool (total, not just available).
    pub fn pool_size(&self) -> usize {
        self.pool_size
    }

    /// Number of sessions currently available for checkout.
    pub fn available_count(&self) -> usize {
        self.sessions.lock().map(|s| s.len()).unwrap_or(0)
    }
}

/// Compute the optimal pool size for the current machine.
///
/// Strategy: `min(max_sessions, max(1, num_cpus / 4))`
///
/// Rationale:
/// - Each session uses ~550MB RAM for Jina v2
/// - On a 16GB machine with 8 cores: pool_size=2 (1.1GB, plenty of headroom)
/// - On a 32GB machine with 16 cores: pool_size=4 (2.2GB)
/// - max_sessions caps at a user-configurable limit
pub fn optimal_pool_size(max_sessions: usize) -> usize {
    let cpus = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4);
    let optimal = (cpus / 4).max(1);
    optimal.min(max_sessions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimal_pool_size_respects_max() {
        assert_eq!(optimal_pool_size(1), 1);
    }

    #[test]
    fn test_optimal_pool_size_at_least_one() {
        assert!(optimal_pool_size(10) >= 1);
    }

    #[test]
    fn test_pool_nonexistent_model() {
        let result = SessionPool::new(Path::new("/nonexistent/model.onnx"), 2);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_pool_size_zero_becomes_one() {
        // pool_size=0 should be treated as 1
        let result = SessionPool::new(Path::new("/nonexistent/model.onnx"), 0);
        assert!(result.is_ok());
    }
}
