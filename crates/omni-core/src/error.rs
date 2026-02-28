//! Error types for omni-core.
//!
//! Uses a hierarchical error enum so callers can pattern-match on
//! the subsystem that failed. Each subsystem also has its own error
//! type internally, which gets converted to `OmniError` at the boundary.

use std::path::PathBuf;

use thiserror::Error;

/// Top-level error type for all omni-core operations.
#[derive(Debug, Error)]
pub enum OmniError {
    // ---- Recoverable (operation failed, system healthy) ----
    /// A single file failed to parse. The rest of the index is fine.
    #[error("parse error for {path}: {message}")]
    Parse {
        /// Path to the file that failed to parse.
        path: PathBuf,
        /// Human-readable error description.
        message: String,
    },

    /// Embedding inference failed for a chunk. Keyword search still works.
    #[error("embedding error for chunk {chunk_id}: {message}")]
    Embed {
        /// Database ID of the chunk that failed to embed.
        chunk_id: i64,
        /// Human-readable error description.
        message: String,
    },

    /// Requested file or symbol was not found in the index.
    #[error("not found: {entity}")]
    NotFound {
        /// Description of what was not found.
        entity: String,
    },

    // ---- Degraded (system works with reduced capability) ----
    /// Embedding model is unavailable. System falls back to keyword-only search.
    #[error("embedding model unavailable: {reason}")]
    ModelUnavailable {
        /// Why the model couldn't be loaded.
        reason: String,
    },

    /// Vector index is unavailable. System falls back to keyword-only search.
    #[error("vector index unavailable: {reason}")]
    VectorUnavailable {
        /// Why the vector index couldn't be loaded.
        reason: String,
    },

    // ---- Fatal (system cannot operate) ----
    /// Database corruption detected. Requires reindex.
    #[error("database corruption: {details}")]
    DatabaseCorruption {
        /// Diagnostic details.
        details: String,
    },

    /// Not enough disk space to continue indexing.
    #[error("insufficient disk space: {available_mb}MB available, {required_mb}MB required")]
    InsufficientDisk {
        /// Available space in megabytes.
        available_mb: u64,
        /// Required space in megabytes.
        required_mb: u64,
    },

    /// Configuration is invalid or missing required fields.
    #[error("configuration error: {details}")]
    Config {
        /// What's wrong with the config.
        details: String,
    },

    // ---- Wrapped external errors ----
    /// SQLite error.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Generic internal error for unexpected conditions.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Convenience type alias for Results in omni-core.
pub type OmniResult<T> = Result<T, OmniError>;
