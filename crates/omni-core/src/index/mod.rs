//! SQLite metadata store and FTS5 full-text search index.
//!
//! This module manages the persistent storage of file metadata, chunks,
//! symbols, and dependencies. It also provides full-text search via FTS5.
//!
//! ## Concurrency
//!
//! SQLite is configured in WAL mode for concurrent reads during writes.
//! Only one writer is allowed at a time (SQLite constraint).

use std::path::Path;

use rusqlite::Connection;

use crate::error::OmniResult;

/// SQLite-backed metadata and full-text search index.
pub struct MetadataIndex {
    conn: Connection,
}

impl MetadataIndex {
    /// Open or create an index database at the given path.
    pub fn open(db_path: &Path) -> OmniResult<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;

        // Configure for performance
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", "-64000")?; // 64MB cache
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let index = Self { conn };
        index.ensure_schema()?;

        Ok(index)
    }

    /// Create all tables and indexes if they don't exist.
    fn ensure_schema(&self) -> OmniResult<()> {
        self.conn.execute_batch(include_str!("schema.sql"))?;
        Ok(())
    }

    /// Run an integrity check on the database.
    pub fn check_integrity(&self) -> OmniResult<bool> {
        let result: String = self.conn.query_row(
            "PRAGMA integrity_check",
            [],
            |row| row.get(0),
        )?;
        Ok(result == "ok")
    }

    /// Get the raw connection for advanced queries.
    /// Use sparingly -- prefer adding methods to this struct.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_creates_database() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let db_path = dir.path().join("test.db");
        let index = MetadataIndex::open(&db_path).expect("open database");
        assert!(index.check_integrity().expect("check integrity"));
    }
}
