//! SQLite connection pooling for concurrent read operations.
//!
//! This module provides a connection pool wrapper around SQLite to enable
//! concurrent read operations while maintaining a single writer connection.
//!
//! ## Architecture
//!
//! - Single writer connection (SQLite constraint)
//! - Pool of read-only connections for concurrent queries (via r2d2)
//! - WAL mode enables readers to proceed without blocking writers
//!
//! ## Expected Impact
//!
//! - 2-3x read throughput improvement (per tech stack research)
//! - Reduced contention on single connection
//! - Better utilization of multi-core systems

use std::path::{Path, PathBuf};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::error::{OmniError, OmniResult};

/// Connection pool configuration.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of read connections in the pool.
    pub max_read_connections: u32,
    /// Minimum number of idle connections to maintain.
    pub min_idle_connections: u32,
    /// Connection timeout in seconds.
    pub connection_timeout_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_read_connections: 8,
            min_idle_connections: 2,
            connection_timeout_secs: 5,
        }
    }
}

/// SQLite connection pool with separate writer and reader connections.
///
/// ## Design
///
/// - Writer: Single connection for all write operations (SQLite constraint)
/// - Readers: r2d2 pool of read-only connections for concurrent queries
/// - WAL mode: Enables readers to proceed without blocking writers
///
/// ## Thread Safety Note
///
/// The writer connection is NOT thread-safe (rusqlite::Connection is !Send/!Sync).
/// The owner of ConnectionPool must ensure exclusive access to writer operations.
/// Reader connections are managed by r2d2 and can be safely shared across threads.
pub struct ConnectionPool {
    /// Single writer connection (NOT thread-safe - caller must ensure exclusive access).
    writer: Connection,
    /// Pool of read-only connections (managed by r2d2, thread-safe).
    reader_pool: Pool<SqliteConnectionManager>,
    /// Database path for diagnostics.
    db_path: PathBuf,
}

impl ConnectionPool {
    /// Create a new connection pool.
    pub fn new(db_path: &Path, config: PoolConfig) -> OmniResult<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create writer connection
        let writer = Connection::open(db_path)?;
        Self::configure_writer_connection(&writer)?;

        // Create reader pool with r2d2
        let manager = SqliteConnectionManager::file(db_path).with_init(|conn| {
            // Configure each reader connection
            Self::configure_reader_connection(conn)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(())
        });

        let reader_pool = Pool::builder()
            .max_size(config.max_read_connections)
            .min_idle(Some(config.min_idle_connections))
            .connection_timeout(std::time::Duration::from_secs(
                config.connection_timeout_secs,
            ))
            .build(manager)
            .map_err(|e| OmniError::Internal(format!("failed to create connection pool: {e}")))?;

        Ok(Self {
            writer,
            reader_pool,
            db_path: db_path.to_path_buf(),
        })
    }

    /// Configure a writer SQLite connection with optimal settings.
    fn configure_writer_connection(conn: &Connection) -> OmniResult<()> {
        // WAL mode for concurrent reads during writes
        conn.pragma_update(None, "journal_mode", "WAL")?;

        // Task 6.2: WAL checkpoint tuning to prevent unbounded growth
        // Checkpoint after 1000 pages (~4MB with 4KB page size)
        conn.pragma_update(None, "wal_autocheckpoint", "1000")?;

        // NORMAL synchronous mode (balance between safety and performance)
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        // 64MB cache for writer
        conn.pragma_update(None, "cache_size", "-64000")?;

        // Enable foreign keys
        conn.pragma_update(None, "foreign_keys", "ON")?;

        // 5s retry on SQLITE_BUSY
        conn.pragma_update(None, "busy_timeout", "5000")?;

        // 256MB memory-mapped I/O for writer
        conn.pragma_update(None, "mmap_size", "268435456")?;

        // Use memory for temporary tables
        conn.pragma_update(None, "temp_store", "MEMORY")?;

        Ok(())
    }

    /// Configure a reader SQLite connection.
    fn configure_reader_connection(conn: &Connection) -> OmniResult<()> {
        // Read-only mode
        conn.pragma_update(None, "query_only", "ON")?;

        // 32MB cache per reader (smaller than writer)
        conn.pragma_update(None, "cache_size", "-32000")?;

        // Use memory for temporary tables
        conn.pragma_update(None, "temp_store", "MEMORY")?;

        // 128MB memory-mapped I/O per reader
        conn.pragma_update(None, "mmap_size", "134217728")?;

        // 5s retry on SQLITE_BUSY
        conn.pragma_update(None, "busy_timeout", "5000")?;

        Ok(())
    }

    /// Get the writer connection (mutable access).
    ///
    /// This returns a mutable reference to the writer connection.
    /// Only one writer can be active at a time (enforced by Rust's borrow checker).
    pub fn writer(&mut self) -> &mut Connection {
        &mut self.writer
    }

    /// Get a read-only connection from the pool.
    ///
    /// This returns a pooled connection from r2d2 that can be used for read
    /// operations. Multiple readers can be active concurrently.
    pub fn reader(&self) -> OmniResult<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.reader_pool
            .get()
            .map_err(|e| OmniError::Internal(format!("failed to get reader connection: {e}")))
    }

    /// Get pool statistics for monitoring.
    pub fn stats(&self) -> PoolStats {
        let state = self.reader_pool.state();
        PoolStats {
            active_connections: state.connections - state.idle_connections,
            idle_connections: state.idle_connections,
            total_connections: state.connections,
            max_connections: self.reader_pool.max_size(),
        }
    }

    /// Checkpoint the WAL file manually.
    ///
    /// This forces a checkpoint of the WAL file, which can help prevent
    /// unbounded WAL growth. Normally handled automatically by wal_autocheckpoint.
    pub fn checkpoint(&mut self) -> OmniResult<()> {
        self.writer
            .pragma_update(None, "wal_checkpoint", "PASSIVE")?;
        Ok(())
    }

    /// Get the database path.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

/// Connection pool statistics.
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Number of active (in-use) connections.
    pub active_connections: u32,
    /// Number of idle connections.
    pub idle_connections: u32,
    /// Total number of connections.
    pub total_connections: u32,
    /// Maximum allowed connections.
    pub max_connections: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_pool_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = ConnectionPool::new(&db_path, PoolConfig::default()).unwrap();

        // Verify pool was created
        let stats = pool.stats();
        assert!(stats.max_connections > 0);
    }

    #[test]
    fn test_concurrent_reads() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let mut pool = ConnectionPool::new(&db_path, PoolConfig::default()).unwrap();

        // Create a test table
        {
            let writer = pool.writer();
            writer
                .execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", [])
                .unwrap();
            writer
                .execute("INSERT INTO test (value) VALUES ('test')", [])
                .unwrap();
        }

        // Get reader pool for sharing (only readers are thread-safe)
        let reader_pool = pool.reader_pool.clone();

        // Spawn multiple readers
        let mut handles = vec![];
        for i in 0..4 {
            let reader_pool_clone = reader_pool.clone();
            let handle = thread::spawn(move || {
                let reader = reader_pool_clone.get().unwrap();
                let count: i64 = reader
                    .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
                    .unwrap();
                assert_eq!(count, 1, "thread {} failed", i);
            });
            handles.push(handle);
        }

        // Wait for all readers to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_pool_stats() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = ConnectionPool::new(&db_path, PoolConfig::default()).unwrap();

        let stats = pool.stats();
        assert_eq!(
            stats.total_connections,
            stats.active_connections + stats.idle_connections
        );
        assert!(stats.max_connections >= stats.total_connections);
    }

    #[test]
    fn test_wal_checkpoint() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let mut pool = ConnectionPool::new(&db_path, PoolConfig::default()).unwrap();

        // Create some data
        {
            let writer = pool.writer();
            writer
                .execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", [])
                .unwrap();
            for i in 0..100 {
                writer
                    .execute("INSERT INTO test (id) VALUES (?1)", [i])
                    .unwrap();
            }
        }

        // Checkpoint should succeed
        pool.checkpoint().unwrap();
    }

    #[test]
    fn test_writer_exclusive_access() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let mut pool = ConnectionPool::new(&db_path, PoolConfig::default()).unwrap();

        // Create table
        {
            let writer = pool.writer();
            writer
                .execute(
                    "CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)",
                    [],
                )
                .unwrap();
        }

        // Insert data
        {
            let writer = pool.writer();
            for i in 0..10 {
                writer
                    .execute("INSERT INTO test (value) VALUES (?1)", [i])
                    .unwrap();
            }
        }

        // Verify all writes succeeded
        let reader = pool.reader().unwrap();
        let count: i64 = reader
            .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 10);
    }
}
