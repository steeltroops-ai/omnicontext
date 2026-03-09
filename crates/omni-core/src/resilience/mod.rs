//! Resilience patterns for fail-safe architecture.
//!
//! This module implements system design patterns for automatic recovery and
//! fault tolerance. All critical paths are protected by circuit breakers and
//! health monitors to ensure 99.9%+ uptime.
//!
//! ## Key Patterns
//!
//! 1. **Circuit Breaker**: Prevents cascading failures by detecting repeated
//!    failures and temporarily blocking requests to failing subsystems.
//!
//! 2. **Health Monitoring**: Continuously monitors subsystem health and triggers
//!    automatic recovery when degradation is detected.
//!
//! 3. **Self-Healing**: Systems automatically repair themselves and restore full
//!    functionality without human intervention.
//!
//! ## Usage
//!
//! ```rust
//! use omni_core::resilience::circuit_breaker::CircuitBreaker;
//!
//! let breaker = CircuitBreaker::new("embedder", 5, Duration::from_secs(60));
//!
//! // Protected call
//! let result = breaker.call(|| {
//!     embedder.embed(chunk)
//! }).await?;
//! ```

pub mod circuit_breaker;
pub mod health_monitor;
