//! SQLite metadata store and FTS5 full-text search index.
//!
//! This module manages the persistent storage of file metadata, chunks,
//! symbols, and dependencies. It also provides full-text search via FTS5.
//!
//! ## Concurrency
//!
//! SQLite is configured in WAL mode for concurrent reads during writes.
//! Only one writer is allowed at a time (SQLite constraint).
//!
//! ## Design
//!
//! All CRUD operations are atomic per-file. When re-indexing a file,
//! we delete all its chunks/symbols first, then insert new ones within
//! a single transaction. This avoids orphaned records.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::redundant_closure_for_method_calls
)]

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::OmniResult;
use crate::types::{
    Chunk, ChunkKind, DependencyEdge, DependencyKind, FileInfo, Language, Symbol, Visibility,
};

/// Current database schema version. Increment when schema changes.
const SCHEMA_VERSION: i64 = 6;

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

        // Configure for performance and concurrency
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", "-64000")?; // 64MB cache
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", "5000")?; // 5s retry on SQLITE_BUSY
        conn.pragma_update(None, "mmap_size", "268435456")?; // 256MB memory-mapped I/O
        conn.pragma_update(None, "temp_store", "MEMORY")?;

        let index = Self { conn };
        index.ensure_schema()?;
        index.ensure_schema_version()?;

        Ok(index)
    }

    /// Create all tables and indexes if they don't exist.
    fn ensure_schema(&self) -> OmniResult<()> {
        self.conn.execute_batch(include_str!("schema.sql"))?;
        Ok(())
    }

    /// Ensure schema version is tracked and compatible.
    fn ensure_schema_version(&self) -> OmniResult<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL,
                migrated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;

        let current: Option<i64> = self
            .conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .optional()?
            .flatten();

        match current {
            None => {
                // First run -- set initial version
                self.conn.execute(
                    "INSERT INTO schema_version (version) VALUES (?1)",
                    params![SCHEMA_VERSION],
                )?;
            }
            Some(v) if v < SCHEMA_VERSION => {
                tracing::info!(from = v, to = SCHEMA_VERSION, "schema migration required");
                // v1 → v2: add content_hash column to chunks table.
                // ALTER TABLE … ADD COLUMN is safe on SQLite (no data loss).
                if v < 2 {
                    self.conn.execute_batch(
                        "ALTER TABLE chunks ADD COLUMN content_hash INTEGER NOT NULL DEFAULT 0;",
                    )?;
                    tracing::info!("migrated schema: added content_hash column to chunks");
                }
                // v2 → v3: add commits_fts virtual table + external_docs table.
                // CREATE VIRTUAL TABLE IF NOT EXISTS is safe to run on existing dbs.
                if v < 3 {
                    self.conn.execute_batch(
                        "CREATE VIRTUAL TABLE IF NOT EXISTS commits_fts USING fts5(
                            message,
                            summary,
                            author,
                            content='commits',
                            content_rowid='rowid',
                            tokenize='porter unicode61 remove_diacritics 2'
                        );
                        CREATE TABLE IF NOT EXISTS external_docs (
                            id           INTEGER PRIMARY KEY,
                            source_url   TEXT    NOT NULL UNIQUE,
                            title        TEXT    NOT NULL,
                            content      TEXT    NOT NULL,
                            chunk_ids    TEXT    NOT NULL DEFAULT '[]',
                            ingested_at  TEXT    NOT NULL DEFAULT (datetime('now'))
                        );
                        CREATE INDEX IF NOT EXISTS idx_external_docs_url ON external_docs(source_url);",
                    )?;
                    tracing::info!("migrated schema v3: commits_fts + external_docs");
                }
                // v3 → v4: add commit_files junction table for O(1) path lookup.
                if v < 4 {
                    self.conn.execute_batch(
                        "CREATE TABLE IF NOT EXISTS commit_files (
                            commit_hash  TEXT NOT NULL REFERENCES commits(hash) ON DELETE CASCADE,
                            file_path    TEXT NOT NULL,
                            PRIMARY KEY  (commit_hash, file_path)
                        );
                        CREATE INDEX IF NOT EXISTS idx_commit_files_path ON commit_files(file_path);
                        CREATE INDEX IF NOT EXISTS idx_commit_files_hash ON commit_files(commit_hash);",
                    )?;
                    tracing::info!("migrated schema v4: commit_files junction table");
                }
                // v4 → v5: add file_graph_edges table for persistent FileDependencyGraph.
                if v < 5 {
                    self.conn.execute_batch(
                        "CREATE TABLE IF NOT EXISTS file_graph_edges (
                            source_path  TEXT NOT NULL,
                            target_path  TEXT NOT NULL,
                            edge_type    TEXT NOT NULL,
                            weight       REAL NOT NULL DEFAULT 1.0,
                            PRIMARY KEY  (source_path, target_path, edge_type)
                        );
                        CREATE INDEX IF NOT EXISTS idx_file_graph_source ON file_graph_edges(source_path);
                        CREATE INDEX IF NOT EXISTS idx_file_graph_target ON file_graph_edges(target_path);",
                    )?;
                    tracing::info!("migrated schema v5: file_graph_edges table");
                }
                // v5 → v6: add sparse_vectors table for BGE-M3 SPLADE output.
                if v < 6 {
                    self.conn.execute_batch(
                        "CREATE TABLE IF NOT EXISTS sparse_vectors (
                            chunk_id   INTEGER NOT NULL PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
                            tokens     TEXT    NOT NULL
                        );
                        CREATE INDEX IF NOT EXISTS idx_sparse_vectors_chunk ON sparse_vectors(chunk_id);",
                    )?;
                    tracing::info!("migrated schema v6: sparse_vectors table");
                }
                self.conn.execute(
                    "INSERT INTO schema_version (version) VALUES (?1)",
                    params![SCHEMA_VERSION],
                )?;
            }
            Some(v) if v > SCHEMA_VERSION => {
                return Err(crate::error::OmniError::Config {
                    details: format!(
                        "database schema version ({v}) is newer than this binary ({SCHEMA_VERSION}). Upgrade OmniContext."
                    ),
                });
            }
            _ => {
                // Schema is current
            }
        }

        Ok(())
    }

    /// Clear all indexed repository data while keeping schema and indexes intact.
    pub fn clear_all(&self) -> OmniResult<()> {
        let tx = self.conn.unchecked_transaction()?;

        // Clear in dependency-safe order.
        tx.execute("DELETE FROM dependencies", [])?;
        tx.execute("DELETE FROM symbols", [])?;
        tx.execute("DELETE FROM chunks", [])?;
        tx.execute("DELETE FROM files", [])?;
        tx.execute("DELETE FROM commits", [])?;

        // Ensure FTS content is emptied as well.
        tx.execute("DELETE FROM chunks_fts", [])?;
        // Tolerate missing commits_fts (may not exist on older dbs before v3 migration)
        let _ = tx.execute("DELETE FROM commits_fts", []);

        tx.commit()?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // File operations
    // -----------------------------------------------------------------------

    /// Insert or update a file record. Returns the file ID.
    pub fn upsert_file(&self, file: &FileInfo) -> OmniResult<i64> {
        self.conn.execute(
            "INSERT INTO files (path, language, hash, size_bytes, last_modified)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(path) DO UPDATE SET
                language = excluded.language,
                hash = excluded.hash,
                size_bytes = excluded.size_bytes,
                indexed_at = datetime('now'),
                last_modified = excluded.last_modified",
            params![
                file.path.to_string_lossy().as_ref(),
                file.language.as_str(),
                file.content_hash,
                file.size_bytes,
            ],
        )?;

        let id = self.conn.last_insert_rowid();
        // If the row was updated (not inserted), last_insert_rowid returns 0
        // In that case, query for the existing ID
        if id == 0 {
            let existing_id: i64 = self.conn.query_row(
                "SELECT id FROM files WHERE path = ?1",
                params![file.path.to_string_lossy().as_ref()],
                |row| row.get(0),
            )?;
            Ok(existing_id)
        } else {
            Ok(id)
        }
    }

    /// Get a file record by path.
    pub fn get_file_by_path(&self, path: &Path) -> OmniResult<Option<FileInfo>> {
        let result = self
            .conn
            .query_row(
                "SELECT id, path, language, hash, size_bytes FROM files WHERE path = ?1",
                params![path.to_string_lossy().as_ref()],
                |row| {
                    Ok(FileInfo {
                        id: row.get(0)?,
                        path: std::path::PathBuf::from(row.get::<_, String>(1)?),
                        language: Language::from_extension(&row.get::<_, String>(2)?),
                        content_hash: row.get(3)?,
                        size_bytes: row.get(4)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Get a file record by its database ID.
    pub fn get_file_by_id(&self, id: i64) -> OmniResult<Option<FileInfo>> {
        let result = self
            .conn
            .query_row(
                "SELECT id, path, language, hash, size_bytes FROM files WHERE id = ?1",
                params![id],
                |row| {
                    Ok(FileInfo {
                        id: row.get(0)?,
                        path: std::path::PathBuf::from(row.get::<_, String>(1)?),
                        language: Language::from_extension(&row.get::<_, String>(2)?),
                        content_hash: row.get(3)?,
                        size_bytes: row.get(4)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Get the hash of an indexed file (for change detection).
    pub fn get_file_hash(&self, path: &Path) -> OmniResult<Option<String>> {
        let hash = self
            .conn
            .query_row(
                "SELECT hash FROM files WHERE path = ?1",
                params![path.to_string_lossy().as_ref()],
                |row| row.get(0),
            )
            .optional()?;

        Ok(hash)
    }

    /// Delete a file and all its associated chunks and symbols.
    pub fn delete_file(&self, path: &Path) -> OmniResult<bool> {
        let changes = self.conn.execute(
            "DELETE FROM files WHERE path = ?1",
            params![path.to_string_lossy().as_ref()],
        )?;
        Ok(changes > 0)
    }

    /// Get all indexed files.
    pub fn get_all_files(&self) -> OmniResult<Vec<FileInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, language, hash, size_bytes FROM files ORDER BY path")?;

        let files = stmt.query_map([], |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                path: std::path::PathBuf::from(row.get::<_, String>(1)?),
                language: Language::from_extension(&row.get::<_, String>(2)?),
                content_hash: row.get(3)?,
                size_bytes: row.get(4)?,
            })
        })?;

        let mut result = Vec::new();
        for file in files {
            result.push(file?);
        }
        Ok(result)
    }

    /// Count total indexed files.
    pub fn file_count(&self) -> OmniResult<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get file freshness timestamps as (file_id, indexed_at_iso8601).
    ///
    /// Returns the `indexed_at` timestamp for every file, which indicates when
    /// the file was last re-indexed (and therefore last modified).
    pub fn get_file_freshness(&self) -> OmniResult<Vec<(i64, String)>> {
        let mut stmt = self.conn.prepare("SELECT id, indexed_at FROM files")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Chunk operations
    // -----------------------------------------------------------------------

    /// Insert a chunk record. Returns the chunk ID.
    pub fn insert_chunk(&self, chunk: &Chunk) -> OmniResult<i64> {
        self.conn.execute(
            "INSERT INTO chunks (file_id, symbol_path, kind, visibility, line_start,
             line_end, content, doc_comment, token_count, weight, vector_id, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                chunk.file_id,
                chunk.symbol_path,
                format!("{:?}", chunk.kind).to_lowercase(),
                format!("{:?}", chunk.visibility).to_lowercase(),
                chunk.line_start,
                chunk.line_end,
                chunk.content,
                chunk.doc_comment,
                chunk.token_count,
                chunk.weight,
                chunk.vector_id.map(|v| v as i64),
                chunk.content_hash as i64,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Insert multiple chunks in a single transaction for better performance.
    /// Returns the chunk IDs in the same order as the input.
    pub fn insert_chunks_batch(&self, chunks: &[Chunk]) -> OmniResult<Vec<i64>> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        let tx = self.conn.unchecked_transaction()?;
        let mut chunk_ids = Vec::with_capacity(chunks.len());

        for chunk in chunks {
            tx.execute(
                "INSERT INTO chunks (file_id, symbol_path, kind, visibility, line_start,
                 line_end, content, doc_comment, token_count, weight, vector_id, content_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    chunk.file_id,
                    chunk.symbol_path,
                    chunk.kind.as_str(),
                    chunk.visibility.as_str(),
                    chunk.line_start,
                    chunk.line_end,
                    chunk.content,
                    chunk.doc_comment,
                    chunk.token_count,
                    chunk.weight,
                    chunk.vector_id.map(|v| v as i64),
                    chunk.content_hash as i64,
                ],
            )?;
            chunk_ids.push(tx.last_insert_rowid());
        }

        tx.commit()?;
        Ok(chunk_ids)
    }

    /// Delete all chunks belonging to a file.
    pub fn delete_chunks_for_file(&self, file_id: i64) -> OmniResult<usize> {
        let changes = self
            .conn
            .execute("DELETE FROM chunks WHERE file_id = ?1", params![file_id])?;
        Ok(changes)
    }

    /// Get all chunks for a file.
    pub fn get_chunks_for_file(&self, file_id: i64) -> OmniResult<Vec<Chunk>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_id, symbol_path, kind, visibility, line_start,
             line_end, content, doc_comment, token_count, weight, vector_id, content_hash
             FROM chunks WHERE file_id = ?1 ORDER BY line_start",
        )?;

        let chunks = stmt.query_map(params![file_id], |row| {
            Ok(Chunk {
                id: row.get(0)?,
                file_id: row.get(1)?,
                symbol_path: row.get(2)?,
                kind: parse_chunk_kind(&row.get::<_, String>(3)?),
                visibility: parse_visibility(&row.get::<_, String>(4)?),
                line_start: row.get(5)?,
                line_end: row.get(6)?,
                content: row.get(7)?,
                doc_comment: row.get(8)?,
                token_count: row.get(9)?,
                weight: row.get(10)?,
                vector_id: row.get::<_, Option<i64>>(11)?.map(|v| v as u64),
                is_summary: false,
                content_hash: row.get::<_, i64>(12)? as u64,
            })
        })?;

        let mut result = Vec::new();
        for chunk in chunks {
            result.push(chunk?);
        }
        Ok(result)
    }

    /// Update the vector_id for a chunk (after embedding).
    pub fn set_chunk_vector_id(&self, chunk_id: i64, vector_id: u64) -> OmniResult<()> {
        self.conn.execute(
            "UPDATE chunks SET vector_id = ?1 WHERE id = ?2",
            params![vector_id as i64, chunk_id],
        )?;
        Ok(())
    }

    /// Get (symbol_path → content_hash) pairs for all chunks of a file.
    ///
    /// Used for chunk-level delta detection: if the stored hash matches the
    /// freshly-computed xxHash3 of the chunk content, the chunk is unchanged
    /// and can skip re-embedding. A stored value of 0 means "not computed"
    /// and always triggers re-embedding.
    pub fn get_chunk_content_hashes_for_file(
        &self,
        file_id: i64,
    ) -> OmniResult<std::collections::HashMap<String, u64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT symbol_path, content_hash FROM chunks WHERE file_id = ?1")?;

        let rows = stmt.query_map(params![file_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (symbol_path, hash_i64) = row?;
            map.insert(symbol_path, hash_i64 as u64);
        }
        Ok(map)
    }

    /// Begin a batch transaction that spans multiple `reindex_file` calls.
    ///
    /// When a batch transaction is active, individual `reindex_file` calls
    /// should not open their own transactions. The caller is responsible for
    /// committing via `commit_batch_transaction`.
    ///
    /// # Note
    /// SQLite only supports one writer at a time. This batch transaction is
    /// useful for bulk index runs where sequential per-file transactions would
    /// each incur fsync overhead.
    pub fn begin_batch_transaction(&self) -> OmniResult<()> {
        self.conn.execute_batch("BEGIN DEFERRED TRANSACTION")?;
        Ok(())
    }

    /// Commit the active batch transaction.
    pub fn commit_batch_transaction(&self) -> OmniResult<()> {
        self.conn.execute_batch("COMMIT")?;
        Ok(())
    }

    /// Roll back the active batch transaction.
    pub fn rollback_batch_transaction(&self) -> OmniResult<()> {
        self.conn.execute_batch("ROLLBACK")?;
        Ok(())
    }

    /// Count total chunks across all files.
    pub fn chunk_count(&self) -> OmniResult<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Count chunks that have embeddings (vector_id is not NULL).
    pub fn embedded_chunk_count(&self) -> OmniResult<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE vector_id IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Get embedding coverage percentage (0.0 to 100.0).
    pub fn embedding_coverage(&self) -> OmniResult<f64> {
        let total = self.chunk_count()? as f64;
        if total == 0.0 {
            return Ok(0.0);
        }
        let embedded = self.embedded_chunk_count()? as f64;
        Ok((embedded / total) * 100.0)
    }

    /// Get all chunks that don't have embeddings (vector_id IS NULL).
    ///
    /// This is useful for retrying failed embeddings.
    pub fn get_chunks_without_vectors(&self) -> OmniResult<Vec<Chunk>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_id, symbol_path, kind, visibility, line_start,
             line_end, content, doc_comment, token_count, weight, vector_id, content_hash
             FROM chunks WHERE vector_id IS NULL ORDER BY file_id, line_start",
        )?;

        let chunks = stmt.query_map([], |row| {
            Ok(Chunk {
                id: row.get(0)?,
                file_id: row.get(1)?,
                symbol_path: row.get(2)?,
                kind: parse_chunk_kind(&row.get::<_, String>(3)?),
                visibility: parse_visibility(&row.get::<_, String>(4)?),
                line_start: row.get(5)?,
                line_end: row.get(6)?,
                content: row.get(7)?,
                doc_comment: row.get(8)?,
                token_count: row.get(9)?,
                weight: row.get(10)?,
                vector_id: row.get::<_, Option<i64>>(11)?.map(|v| v as u64),
                is_summary: false,
                content_hash: row.get::<_, i64>(12)? as u64,
            })
        })?;

        let mut result = Vec::new();
        for chunk in chunks {
            result.push(chunk?);
        }
        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Symbol operations
    // -----------------------------------------------------------------------

    /// Insert a symbol record. Returns the symbol ID.
    pub fn insert_symbol(&self, symbol: &Symbol) -> OmniResult<i64> {
        self.conn.execute(
            "INSERT OR REPLACE INTO symbols (name, fqn, kind, file_id, line, chunk_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                symbol.name,
                symbol.fqn,
                format!("{:?}", symbol.kind).to_lowercase(),
                symbol.file_id,
                symbol.line,
                symbol.chunk_id,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Insert multiple symbols in a single transaction for better performance.
    /// Returns the symbol IDs in the same order as the input.
    pub fn insert_symbols_batch(&self, symbols: &[Symbol]) -> OmniResult<Vec<i64>> {
        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        let tx = self.conn.unchecked_transaction()?;
        let mut symbol_ids = Vec::with_capacity(symbols.len());

        for symbol in symbols {
            tx.execute(
                "INSERT OR REPLACE INTO symbols (name, fqn, kind, file_id, line, chunk_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    symbol.name,
                    symbol.fqn,
                    symbol.kind.as_str(),
                    symbol.file_id,
                    symbol.line,
                    symbol.chunk_id,
                ],
            )?;
            symbol_ids.push(tx.last_insert_rowid());
        }

        tx.commit()?;
        Ok(symbol_ids)
    }

    /// Look up a symbol by its fully qualified name.
    pub fn get_symbol_by_fqn(&self, fqn: &str) -> OmniResult<Option<Symbol>> {
        let result = self
            .conn
            .query_row(
                "SELECT id, name, fqn, kind, file_id, line, chunk_id
             FROM symbols WHERE fqn = ?1",
                params![fqn],
                |row| {
                    Ok(Symbol {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        fqn: row.get(2)?,
                        kind: parse_chunk_kind(&row.get::<_, String>(3)?),
                        file_id: row.get(4)?,
                        line: row.get(5)?,
                        chunk_id: row.get(6)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Look up a symbol by its database ID.
    pub fn get_symbol_by_id(&self, id: i64) -> OmniResult<Option<Symbol>> {
        let result = self
            .conn
            .query_row(
                "SELECT id, name, fqn, kind, file_id, line, chunk_id
             FROM symbols WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Symbol {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        fqn: row.get(2)?,
                        kind: parse_chunk_kind(&row.get::<_, String>(3)?),
                        file_id: row.get(4)?,
                        line: row.get(5)?,
                        chunk_id: row.get(6)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Search symbols by name prefix (for autocomplete).
    pub fn search_symbols_by_name(&self, prefix: &str, limit: usize) -> OmniResult<Vec<Symbol>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, fqn, kind, file_id, line, chunk_id
             FROM symbols WHERE name LIKE ?1 ORDER BY name LIMIT ?2",
        )?;

        let pattern = format!("{prefix}%");
        let symbols = stmt.query_map(params![pattern, limit as i64], |row| {
            Ok(Symbol {
                id: row.get(0)?,
                name: row.get(1)?,
                fqn: row.get(2)?,
                kind: parse_chunk_kind(&row.get::<_, String>(3)?),
                file_id: row.get(4)?,
                line: row.get(5)?,
                chunk_id: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for s in symbols {
            result.push(s?);
        }
        Ok(result)
    }

    /// Delete all symbols belonging to a file.
    pub fn delete_symbols_for_file(&self, file_id: i64) -> OmniResult<usize> {
        let changes = self
            .conn
            .execute("DELETE FROM symbols WHERE file_id = ?1", params![file_id])?;
        Ok(changes)
    }

    /// Count total symbols.
    pub fn symbol_count(&self) -> OmniResult<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM symbols", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get the first symbol defined in a file (by line order).
    ///
    /// Used as the source node for import-based dependency edges.
    pub fn get_first_symbol_for_file(&self, file_id: i64) -> OmniResult<Option<Symbol>> {
        let result = self
            .conn
            .query_row(
                "SELECT id, name, fqn, kind, file_id, line, chunk_id
             FROM symbols WHERE file_id = ?1 ORDER BY line LIMIT 1",
                params![file_id],
                |row| {
                    Ok(Symbol {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        fqn: row.get(2)?,
                        kind: parse_chunk_kind(&row.get::<_, String>(3)?),
                        file_id: row.get(4)?,
                        line: row.get(5)?,
                        chunk_id: row.get(6)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Search symbols whose FQN ends with the given suffix.
    ///
    /// This is the core of import resolution: `config::Config` should match
    /// `crate::config::Config` or `my_module.config.Config`.
    pub fn search_symbols_by_fqn_suffix(
        &self,
        suffix: &str,
        limit: usize,
    ) -> OmniResult<Vec<Symbol>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, fqn, kind, file_id, line, chunk_id
             FROM symbols WHERE fqn LIKE ?1 ORDER BY length(fqn) ASC LIMIT ?2",
        )?;

        // Match FQNs ending with the suffix (preceded by :: or . or at start)
        let pattern = format!("%{suffix}");
        let symbols = stmt.query_map(params![pattern, limit as i64], |row| {
            Ok(Symbol {
                id: row.get(0)?,
                name: row.get(1)?,
                fqn: row.get(2)?,
                kind: parse_chunk_kind(&row.get::<_, String>(3)?),
                file_id: row.get(4)?,
                line: row.get(5)?,
                chunk_id: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for s in symbols {
            result.push(s?);
        }
        Ok(result)
    }

    /// Get ALL symbols defined in a file (ordered by line).
    ///
    /// Used for call graph construction -- we need to iterate all symbols
    /// in a file to resolve their references.
    pub fn get_all_symbols_for_file(&self, file_id: i64) -> OmniResult<Vec<Symbol>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, fqn, kind, file_id, line, chunk_id
             FROM symbols WHERE file_id = ?1 ORDER BY line",
        )?;

        let symbols = stmt.query_map(params![file_id], |row| {
            Ok(Symbol {
                id: row.get(0)?,
                name: row.get(1)?,
                fqn: row.get(2)?,
                kind: parse_chunk_kind(&row.get::<_, String>(3)?),
                file_id: row.get(4)?,
                line: row.get(5)?,
                chunk_id: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for s in symbols {
            result.push(s?);
        }
        Ok(result)
    }

    /// Get ALL symbols in the index.
    ///
    /// Used for loading the dependency graph on startup.
    pub fn get_all_symbols(&self) -> OmniResult<Vec<Symbol>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, fqn, kind, file_id, line, chunk_id
             FROM symbols ORDER BY id",
        )?;

        let symbols = stmt.query_map([], |row| {
            Ok(Symbol {
                id: row.get(0)?,
                name: row.get(1)?,
                fqn: row.get(2)?,
                kind: parse_chunk_kind(&row.get::<_, String>(3)?),
                file_id: row.get(4)?,
                line: row.get(5)?,
                chunk_id: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for s in symbols {
            result.push(s?);
        }
        Ok(result)
    }

    // -----------------------------------------------------------------------
    // FTS5 keyword search
    // -----------------------------------------------------------------------

    /// Search chunks using FTS5 full-text search.
    ///
    /// Returns (chunk_id, bm25_score) pairs, ordered by relevance.
    ///
    /// ## Query Building
    ///
    /// FTS5 phrase quoting (`"foo bar"`) requires exact consecutive token matching,
    /// which produces zero results for multi-word queries when tokens appear in different
    /// parts of a file (the common case). Instead we:
    ///
    /// 1. Split the query into individual tokens.
    /// 2. Quote each token individually to prevent FTS5 syntax errors on
    ///    special chars (hyphens, colons, `::`, `->`, etc.).
    /// 3. Join with AND — all tokens must appear somewhere in the document.
    /// 4. If the AND query returns zero results, fall back to OR so partial
    ///    matches are still surfaced (R16: graceful degradation).
    pub fn keyword_search(&self, query: &str, limit: usize) -> OmniResult<Vec<(i64, f64)>> {
        // Design: per-token quoting with AND → OR fallback.
        // This eliminates zero-result multi-word queries while retaining
        // FTS5 special-character safety from individual token quoting.
        let tokens: Vec<String> = query
            .split_whitespace()
            .filter(|t| !t.is_empty())
            .map(|t| format!("\"{}\"", t.replace('"', "")))
            .collect();

        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        // AND query: all tokens must appear (high precision)
        let and_query = tokens.join(" AND ");

        let sql = "SELECT rowid, bm25(chunks_fts, 1.0, 0.5, 2.0) as score
                   FROM chunks_fts
                   WHERE chunks_fts MATCH ?1
                   ORDER BY score
                   LIMIT ?2";

        let mut stmt = self.conn.prepare(sql)?;

        let and_results = stmt.query_map(params![and_query, limit as i64], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })?;

        let mut out = Vec::new();
        for r in and_results {
            out.push(r?);
        }

        // R16 — Graceful degradation: if AND produced nothing, retry with OR
        // so at least one token match is returned. Useful when embedding
        // coverage is 0% and keyword is the sole retrieval signal.
        if out.is_empty() && tokens.len() > 1 {
            let or_query = tokens.join(" OR ");
            let mut stmt2 = self.conn.prepare(sql)?;
            let or_results = stmt2.query_map(params![or_query, limit as i64], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
            })?;
            for r in or_results {
                out.push(r?);
            }
        }

        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Transaction helpers
    // -----------------------------------------------------------------------

    /// Re-index a file atomically: delete old data, insert new chunks and symbols.
    ///
    /// This is the primary write operation. It ensures consistency by
    /// wrapping delete+insert in a single transaction. Stale dependency
    /// edges are cleaned up before symbols are deleted.
    pub fn reindex_file(
        &self,
        file: &FileInfo,
        chunks: &[Chunk],
        symbols: &[Symbol],
    ) -> OmniResult<(i64, Vec<i64>)> {
        // Use a named SAVEPOINT rather than BEGIN TRANSACTION so this call is
        // safe both standalone and when nested inside a batch transaction
        // opened by `begin_batch_transaction()`.  SQLite SAVEPOINTs are
        // reentrant — they work correctly at any nesting depth.
        let savepoint_name = "reindex_file_sp";
        self.conn
            .execute_batch(&format!("SAVEPOINT {savepoint_name}"))?;

        // All writes go through a macro-local closure so we can ROLLBACK TO
        // the savepoint on any error without propagating a half-written state.
        let result: rusqlite::Result<(i64, Vec<i64>)> = (|| {
            let conn = &self.conn;

            // Upsert the file
            conn.execute(
                "INSERT INTO files (path, language, hash, size_bytes, last_modified)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(path) DO UPDATE SET
                language = excluded.language,
                hash = excluded.hash,
                size_bytes = excluded.size_bytes,
                indexed_at = datetime('now'),
                last_modified = excluded.last_modified",
                params![
                    file.path.to_string_lossy().as_ref(),
                    file.language.as_str(),
                    file.content_hash,
                    file.size_bytes,
                ],
            )?;

            let file_id: i64 = conn.query_row(
                "SELECT id FROM files WHERE path = ?1",
                params![file.path.to_string_lossy().as_ref()],
                |row| row.get(0),
            )?;

            // Delete stale dependency edges for symbols in this file BEFORE
            // deleting the symbols themselves. This prevents ghost edges.
            conn.execute(
            "DELETE FROM dependencies WHERE source_id IN (SELECT id FROM symbols WHERE file_id = ?1)
             OR target_id IN (SELECT id FROM symbols WHERE file_id = ?1)",
            params![file_id],
        )?;

            // Delete old chunks and symbols for this file
            conn.execute("DELETE FROM symbols WHERE file_id = ?1", params![file_id])?;
            conn.execute("DELETE FROM chunks WHERE file_id = ?1", params![file_id])?;

            // Insert new chunks using a prepared, cached statement for SOTA speed
            let mut chunk_ids = Vec::with_capacity(chunks.len());
            {
                let mut chunk_stmt = conn.prepare_cached(
                    "INSERT INTO chunks (file_id, symbol_path, kind, visibility, line_start,
             line_end, content, doc_comment, token_count, weight, vector_id, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                )?;

                for chunk in chunks {
                    chunk_stmt.execute(params![
                        file_id,
                        chunk.symbol_path,
                        chunk.kind.as_str(),
                        chunk.visibility.as_str(),
                        chunk.line_start,
                        chunk.line_end,
                        chunk.content,
                        chunk.doc_comment,
                        chunk.token_count,
                        chunk.weight,
                        chunk.vector_id.map(|v| v as i64),
                        chunk.content_hash as i64,
                    ])?;
                    chunk_ids.push(conn.last_insert_rowid());
                }
            }

            // Insert new symbols using a prepared, cached statement
            {
                let mut symbol_stmt = conn.prepare_cached(
                    "INSERT OR REPLACE INTO symbols (name, fqn, kind, file_id, line, chunk_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                )?;

                for symbol in symbols {
                    symbol_stmt.execute(params![
                        symbol.name,
                        symbol.fqn,
                        symbol.kind.as_str(),
                        file_id,
                        symbol.line,
                        symbol.chunk_id,
                    ])?;
                }
            }

            Ok((file_id, chunk_ids))
        })();

        match result {
            Ok(val) => {
                self.conn
                    .execute_batch(&format!("RELEASE {savepoint_name}"))?;
                Ok(val)
            }
            Err(e) => {
                // Roll back to the savepoint to leave the DB in a clean state,
                // then release it (required to free the savepoint even after rollback).
                let _ = self
                    .conn
                    .execute_batch(&format!("ROLLBACK TO {savepoint_name}"));
                let _ = self
                    .conn
                    .execute_batch(&format!("RELEASE {savepoint_name}"));
                Err(crate::error::OmniError::Database(e))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Status / diagnostics
    // -----------------------------------------------------------------------

    /// Run an integrity check on the database.
    pub fn check_integrity(&self) -> OmniResult<bool> {
        let result: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        Ok(result == "ok")
    }

    /// Get aggregate statistics about the index.
    pub fn statistics(&self) -> OmniResult<IndexStats> {
        Ok(IndexStats {
            file_count: self.file_count()?,
            chunk_count: self.chunk_count()?,
            symbol_count: self.symbol_count()?,
        })
    }

    /// Get file counts grouped by language.
    pub fn language_distribution(&self) -> OmniResult<Vec<(String, usize)>> {
        let mut stmt = self.conn.prepare(
            "SELECT language, COUNT(*) FROM files GROUP BY language ORDER BY COUNT(*) DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })?;
        let mut dist = Vec::new();
        for r in rows {
            dist.push(r?);
        }
        Ok(dist)
    }

    /// Search for a file by path suffix (fuzzy match).
    ///
    /// Useful when the caller provides a partial path or uses different
    /// separators (backslash vs forward slash). Returns the first match.
    pub fn search_file_by_path_suffix(&self, suffix: &str) -> OmniResult<Option<FileInfo>> {
        let normalized = suffix.replace('\\', "/");
        let like_pattern = format!("%{normalized}");
        let result = self
            .conn
            .query_row(
                "SELECT id, path, language, hash, size_bytes FROM files WHERE path LIKE ?1 LIMIT 1",
                params![like_pattern],
                |row| {
                    Ok(FileInfo {
                        id: row.get(0)?,
                        path: std::path::PathBuf::from(row.get::<_, String>(1)?),
                        language: Language::from_extension(&row.get::<_, String>(2)?),
                        content_hash: row.get(3)?,
                        size_bytes: row.get(4)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Get the raw connection for advanced queries.
    /// Use sparingly -- prefer adding methods to this struct.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    // -----------------------------------------------------------------------
    // Dependency operations
    // -----------------------------------------------------------------------

    /// Insert a dependency edge. Idempotent (ignores duplicates).
    pub fn insert_dependency(&self, edge: &DependencyEdge) -> OmniResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO dependencies (source_id, target_id, kind) VALUES (?1, ?2, ?3)",
            params![edge.source_id, edge.target_id, edge.kind.as_str()],
        )?;
        Ok(())
    }

    /// Get all dependencies FROM a given symbol (outgoing edges = what it depends on).
    pub fn get_upstream_dependencies(&self, symbol_id: i64) -> OmniResult<Vec<DependencyEdge>> {
        let mut stmt = self
            .conn
            .prepare("SELECT source_id, target_id, kind FROM dependencies WHERE source_id = ?1")?;
        let edges = stmt.query_map(params![symbol_id], |row| {
            let kind_str: String = row.get(2)?;
            Ok(DependencyEdge {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                kind: DependencyKind::from_str_lossy(&kind_str),
            })
        })?;
        Ok(edges.filter_map(|e| e.ok()).collect())
    }

    /// Get all dependencies TO a given symbol (incoming edges = what depends on it).
    pub fn get_downstream_dependencies(&self, symbol_id: i64) -> OmniResult<Vec<DependencyEdge>> {
        let mut stmt = self
            .conn
            .prepare("SELECT source_id, target_id, kind FROM dependencies WHERE target_id = ?1")?;
        let edges = stmt.query_map(params![symbol_id], |row| {
            let kind_str: String = row.get(2)?;
            Ok(DependencyEdge {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                kind: DependencyKind::from_str_lossy(&kind_str),
            })
        })?;
        Ok(edges.filter_map(|e| e.ok()).collect())
    }

    /// Delete dependencies involving a symbol (both as source and target).
    pub fn delete_dependencies_for_symbol(&self, symbol_id: i64) -> OmniResult<usize> {
        let count1 = self.conn.execute(
            "DELETE FROM dependencies WHERE source_id = ?1",
            params![symbol_id],
        )?;
        let count2 = self.conn.execute(
            "DELETE FROM dependencies WHERE target_id = ?1",
            params![symbol_id],
        )?;
        Ok(count1 + count2)
    }

    /// Count total dependency edges.
    pub fn dependency_count(&self) -> OmniResult<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM dependencies", [], |row| row.get(0))?;
        Ok(count as usize)
    }
    /// Get ALL dependency edges from the database.
    ///
    /// Used to populate the in-memory dependency graph on engine startup.
    pub fn get_all_dependencies(&self) -> OmniResult<Vec<DependencyEdge>> {
        let mut stmt = self
            .conn
            .prepare("SELECT source_id, target_id, kind FROM dependencies")?;
        let edges = stmt.query_map([], |row| {
            let kind_str: String = row.get(2)?;
            Ok(DependencyEdge {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                kind: DependencyKind::from_str_lossy(&kind_str),
            })
        })?;
        Ok(edges.filter_map(|e| e.ok()).collect())
    }
}

/// Aggregate index statistics.
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// Number of indexed files.
    pub file_count: usize,
    /// Number of chunks.
    pub chunk_count: usize,
    /// Number of symbols.
    pub symbol_count: usize,
}

// ---------------------------------------------------------------------------
// Commit search
// ---------------------------------------------------------------------------

impl MetadataIndex {
    // Design: per-token AND search with OR fallback, same strategy as keyword_search.
    // Searches message + summary + author fields via commits_fts virtual table.

    /// Search commits using FTS5 full-text search over message and summary.
    ///
    /// Returns rowids sorted by BM25 relevance. The caller resolves each
    /// rowid to a `CommitInfo` via the `commits` table.
    pub fn search_commits(&self, query: &str, limit: usize) -> OmniResult<Vec<i64>> {
        let tokens: Vec<String> = query
            .split_whitespace()
            .filter(|t| !t.is_empty())
            .map(|t| format!("\"{}\"", t.replace('"', "")))
            .collect();

        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        let sql = "SELECT rowid FROM commits_fts
                   WHERE commits_fts MATCH ?1
                   ORDER BY bm25(commits_fts)
                   LIMIT ?2";

        let and_query = tokens.join(" AND ");
        let mut stmt = self.conn.prepare(sql)?;
        let and_ids: Vec<i64> = stmt
            .query_map(params![and_query, limit as i64], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        if !and_ids.is_empty() || tokens.len() == 1 {
            return Ok(and_ids);
        }

        // OR fallback
        let or_query = tokens.join(" OR ");
        let mut stmt2 = self.conn.prepare(sql)?;
        let or_ids: Vec<i64> = stmt2
            .query_map(params![or_query, limit as i64], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(or_ids)
    }

    /// Fetch commit records by their database rowids.
    pub fn get_commits_by_rowids(
        &self,
        rowids: &[i64],
    ) -> OmniResult<Vec<crate::commits::CommitInfo>> {
        if rowids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: String = rowids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT hash, message, author, timestamp, summary, files_changed
             FROM commits WHERE rowid IN ({placeholders})"
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let params_vec: Vec<&dyn rusqlite::ToSql> =
            rowids.iter().map(|r| r as &dyn rusqlite::ToSql).collect();
        let commits = stmt
            .query_map(params_vec.as_slice(), |row| {
                let files_json: String = row.get(5)?;
                let files: Vec<String> = serde_json::from_str(&files_json).unwrap_or_default();
                Ok(crate::commits::CommitInfo {
                    hash: row.get(0)?,
                    message: row.get(1)?,
                    author: row.get(2)?,
                    timestamp: row.get(3)?,
                    summary: row.get(4)?,
                    files_changed: files,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(commits)
    }
}

// ---------------------------------------------------------------------------
// External doc ingestion
// ---------------------------------------------------------------------------

/// A record for an externally ingested document.
#[derive(Debug, Clone)]
pub struct ExternalDoc {
    /// Canonical source URL or file path used as the unique key.
    pub source_url: String,
    /// Title of the document (page title, filename, or inferred).
    pub title: String,
    /// Full text content (Markdown or plain text after extraction).
    pub content: String,
    /// IDs of chunks created from this document.
    pub chunk_ids: Vec<i64>,
}

impl MetadataIndex {
    /// Upsert an external document record.
    ///
    /// If `source_url` already exists, updates the title and content
    /// but preserves `ingested_at` for the original ingestion time.
    pub fn upsert_external_doc(
        &self,
        source_url: &str,
        title: &str,
        content: &str,
        chunk_ids: &[i64],
    ) -> OmniResult<i64> {
        let ids_json = serde_json::to_string(chunk_ids).unwrap_or_else(|_| "[]".into());
        self.conn.execute(
            "INSERT INTO external_docs (source_url, title, content, chunk_ids)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(source_url) DO UPDATE SET
                title = excluded.title,
                content = excluded.content,
                chunk_ids = excluded.chunk_ids",
            params![source_url, title, content, ids_json],
        )?;
        let id: i64 = self.conn.query_row(
            "SELECT id FROM external_docs WHERE source_url = ?1",
            params![source_url],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// List all ingested external documents.
    pub fn list_external_docs(&self) -> OmniResult<Vec<ExternalDoc>> {
        let mut stmt = self.conn.prepare(
            "SELECT source_url, title, content, chunk_ids FROM external_docs
             ORDER BY ingested_at DESC",
        )?;
        let docs = stmt
            .query_map([], |row| {
                let ids_json: String = row.get(3)?;
                let chunk_ids: Vec<i64> = serde_json::from_str(&ids_json).unwrap_or_default();
                Ok(ExternalDoc {
                    source_url: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    chunk_ids,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(docs)
    }

    /// Check if a URL has already been ingested.
    pub fn external_doc_exists(&self, source_url: &str) -> bool {
        self.conn
            .query_row(
                "SELECT 1 FROM external_docs WHERE source_url = ?1",
                params![source_url],
                |_| Ok(true),
            )
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// Commit–file junction table (schema v4)
// ---------------------------------------------------------------------------

impl MetadataIndex {
    /// Populate `commit_files` for one commit.
    ///
    /// Uses `INSERT OR IGNORE` so repeated calls are safe.
    /// Call this immediately after `INSERT OR REPLACE INTO commits`.
    pub fn insert_commit_files(&self, commit_hash: &str, files: &[String]) -> OmniResult<()> {
        if files.is_empty() {
            return Ok(());
        }
        let mut stmt = self.conn.prepare_cached(
            "INSERT OR IGNORE INTO commit_files (commit_hash, file_path) VALUES (?1, ?2)",
        )?;
        for file_path in files {
            stmt.execute(params![commit_hash, file_path])?;
        }
        Ok(())
    }

    /// Fetch commits that touched `file_path` using the indexed junction table.
    ///
    /// Falls back to the JSON `LIKE` scan when the `commit_files` table does
    /// not exist (databases created before schema v4).
    pub fn commits_for_file_fast(
        &self,
        file_path: &str,
        limit: usize,
    ) -> OmniResult<Vec<crate::commits::CommitInfo>> {
        // Capability check: use junction table only when it exists.
        let table_exists: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='commit_files'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if !table_exists {
            // Graceful fallback: legacy LIKE scan.
            return self.commits_for_file_like(file_path, limit);
        }

        let mut stmt = self.conn.prepare(
            "SELECT c.hash, c.message, c.author, c.timestamp, c.summary, c.files_changed
             FROM commits c
             JOIN commit_files cf ON c.hash = cf.commit_hash
             WHERE cf.file_path = ?1
             ORDER BY c.timestamp DESC
             LIMIT ?2",
        )?;

        let commits = stmt
            .query_map(params![file_path, limit as i64], |row| {
                let files_json: String = row.get(5)?;
                let files: Vec<String> = serde_json::from_str(&files_json).unwrap_or_default();
                Ok(crate::commits::CommitInfo {
                    hash: row.get(0)?,
                    message: row.get(1)?,
                    author: row.get(2)?,
                    timestamp: row.get(3)?,
                    summary: row.get(4)?,
                    files_changed: files,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(commits)
    }

    /// Legacy LIKE-based scan (pre-v4 fallback).
    fn commits_for_file_like(
        &self,
        file_path: &str,
        limit: usize,
    ) -> OmniResult<Vec<crate::commits::CommitInfo>> {
        let pattern = format!("%\"{file_path}\"%");
        let mut stmt = self.conn.prepare(
            "SELECT hash, message, author, timestamp, summary, files_changed
             FROM commits
             WHERE files_changed LIKE ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;
        let commits = stmt
            .query_map(params![pattern, limit as i64], |row| {
                let files_json: String = row.get(5)?;
                let files: Vec<String> = serde_json::from_str(&files_json).unwrap_or_default();
                Ok(crate::commits::CommitInfo {
                    hash: row.get(0)?,
                    message: row.get(1)?,
                    author: row.get(2)?,
                    timestamp: row.get(3)?,
                    summary: row.get(4)?,
                    files_changed: files,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(commits)
    }
}

// ---------------------------------------------------------------------------
// File graph edge persistence (schema v5)
// ---------------------------------------------------------------------------

impl MetadataIndex {
    /// Persist a batch of file-level dependency edges.
    ///
    /// Uses `INSERT OR REPLACE` inside a transaction.
    pub fn save_file_graph_edges(
        &self,
        edges: &[crate::graph::dependencies::DependencyEdge],
    ) -> OmniResult<()> {
        if edges.is_empty() {
            return Ok(());
        }

        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO file_graph_edges
                 (source_path, target_path, edge_type, weight)
                 VALUES (?1, ?2, ?3, ?4)",
            )?;
            for edge in edges {
                stmt.execute(params![
                    edge.source.to_string_lossy().as_ref(),
                    edge.target.to_string_lossy().as_ref(),
                    edge.edge_type.as_str(),
                    edge.weight,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Load all persisted file-level dependency edges.
    ///
    /// Called once on engine startup to restore the in-memory graph.
    pub fn load_file_graph_edges(
        &self,
    ) -> OmniResult<Vec<crate::graph::dependencies::DependencyEdge>> {
        use crate::graph::dependencies::{DependencyEdge, EdgeType};
        use std::path::PathBuf;

        // Table may not exist on databases created before schema v5.
        let table_exists: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='file_graph_edges'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if !table_exists {
            return Ok(Vec::new());
        }

        let mut stmt = self
            .conn
            .prepare("SELECT source_path, target_path, edge_type, weight FROM file_graph_edges")?;

        let edges = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(src, tgt, et, w)| {
                EdgeType::parse(&et).map(|edge_type| DependencyEdge {
                    source: PathBuf::from(src),
                    target: PathBuf::from(tgt),
                    edge_type,
                    weight: w as f32,
                })
            })
            .collect();

        Ok(edges)
    }

    /// Delete all persisted edges where `source_path` matches.
    ///
    /// Called before re-persisting edges for a re-indexed file.
    pub fn delete_file_graph_edges_for_file(&self, path: &std::path::Path) -> OmniResult<()> {
        let path_str = path.to_string_lossy();
        self.conn.execute(
            "DELETE FROM file_graph_edges WHERE source_path = ?1",
            params![path_str.as_ref()],
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Sparse vector store (schema v6)
// ---------------------------------------------------------------------------

impl MetadataIndex {
    /// Persist sparse token weights for one chunk.
    ///
    /// Stores as a JSON array `[[token_id, weight], ...]`.
    pub fn save_sparse_vector(&self, chunk_id: i64, tokens: &[(u32, f32)]) -> OmniResult<()> {
        let json = serde_json::to_string(tokens).map_err(|e| {
            crate::error::OmniError::Internal(format!("sparse vector serialize: {e}"))
        })?;
        self.conn.execute(
            "INSERT OR REPLACE INTO sparse_vectors (chunk_id, tokens) VALUES (?1, ?2)",
            params![chunk_id, json],
        )?;
        Ok(())
    }

    /// Retrieve sparse token weights for one chunk.
    pub fn get_sparse_vector(&self, chunk_id: i64) -> OmniResult<Option<Vec<(u32, f32)>>> {
        let result: Option<String> = self
            .conn
            .query_row(
                "SELECT tokens FROM sparse_vectors WHERE chunk_id = ?1",
                params![chunk_id],
                |row| row.get(0),
            )
            .optional()?;

        match result {
            None => Ok(None),
            Some(json) => {
                let tokens: Vec<(u32, f32)> = serde_json::from_str(&json).map_err(|e| {
                    crate::error::OmniError::Internal(format!("sparse vector deserialize: {e}"))
                })?;
                Ok(Some(tokens))
            }
        }
    }

    /// Compute dot-product similarity of `query_tokens` against all stored
    /// sparse vectors and return the top-`limit` results as `(chunk_id, score)`.
    ///
    /// For indexes ≤100k chunks this loads all rows into Rust memory for
    /// dot-product computation. Callers with >100k chunks should use the
    /// in-memory `SparseInvertedIndex` instead.
    pub fn search_sparse(
        &self,
        query_tokens: &[(u32, f32)],
        limit: usize,
    ) -> OmniResult<Vec<(i64, f32)>> {
        if query_tokens.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        // Build a lookup map for the query tokens: token_id → weight.
        let query_map: std::collections::HashMap<u32, f32> = query_tokens.iter().copied().collect();

        let mut stmt = self
            .conn
            .prepare("SELECT chunk_id, tokens FROM sparse_vectors")?;

        let mut scores: Vec<(i64, f32)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(chunk_id, json)| {
                let tokens: Vec<(u32, f32)> = serde_json::from_str(&json).ok()?;
                let score: f32 = tokens
                    .iter()
                    .filter_map(|(tid, w)| query_map.get(tid).map(|qw| qw * w))
                    .sum();
                if score > 0.0 {
                    Some((chunk_id, score))
                } else {
                    None
                }
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(limit);
        Ok(scores)
    }

    /// Load all (chunk_id, tokens) rows from `sparse_vectors` for in-memory index construction.
    ///
    /// Used by `Engine` to rebuild the `SparseInvertedIndex` after each `run_index()` run.
    /// Returns an empty Vec (not an error) when the table is empty.
    #[allow(clippy::type_complexity)]
    pub fn get_all_sparse_vectors(&self) -> OmniResult<Vec<(i64, Vec<(u32, f32)>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT chunk_id, tokens FROM sparse_vectors")?;

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(chunk_id, json)| {
                let tokens: Vec<(u32, f32)> = serde_json::from_str(&json).ok()?;
                Some((chunk_id, tokens))
            })
            .collect();

        Ok(rows)
    }
}

// ---------------------------------------------------------------------------
// Parse helpers (delegates to centralized methods on types)
// ---------------------------------------------------------------------------

fn parse_chunk_kind(s: &str) -> ChunkKind {
    ChunkKind::from_str_lossy(s)
}

fn parse_visibility(s: &str) -> Visibility {
    Visibility::from_str_lossy(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn open_test_db() -> MetadataIndex {
        let dir = tempfile::tempdir().expect("create temp dir");
        let db_path = dir.path().join("test.db");
        // Keep dir alive by leaking (fine for tests)
        let index = MetadataIndex::open(&db_path).expect("open database");
        std::mem::forget(dir);
        index
    }

    fn test_file_info() -> FileInfo {
        FileInfo {
            id: 0,
            path: PathBuf::from("src/main.py"),
            language: Language::Python,
            content_hash: "abc123def456".to_string(),
            size_bytes: 1024,
        }
    }

    fn test_chunk(file_id: i64) -> Chunk {
        Chunk {
            id: 0,
            file_id,
            symbol_path: "main.hello".to_string(),
            kind: ChunkKind::Function,
            visibility: Visibility::Public,
            line_start: 1,
            line_end: 5,
            content: "def hello():\n    print('hello')".to_string(),
            doc_comment: Some("A greeting function.".to_string()),
            token_count: 10,
            weight: 0.85,
            vector_id: None,
            is_summary: false,
            content_hash: 0,
        }
    }

    fn test_symbol(file_id: i64) -> Symbol {
        Symbol {
            id: 0,
            name: "hello".to_string(),
            fqn: "main.hello".to_string(),
            kind: ChunkKind::Function,
            file_id,
            line: 1,
            chunk_id: None,
        }
    }

    #[test]
    fn test_open_creates_database() {
        let index = open_test_db();
        assert!(index.check_integrity().expect("check integrity"));
    }

    #[test]
    fn test_upsert_and_get_file() {
        let index = open_test_db();
        let file = test_file_info();

        let id = index.upsert_file(&file).expect("upsert file");
        assert!(id > 0);

        let retrieved = index
            .get_file_by_path(&file.path)
            .expect("get file")
            .expect("should exist");

        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.content_hash, "abc123def456");
        assert_eq!(retrieved.size_bytes, 1024);
    }

    #[test]
    fn test_upsert_file_updates_existing() {
        let index = open_test_db();
        let mut file = test_file_info();

        let id1 = index.upsert_file(&file).expect("first upsert");

        file.content_hash = "newhashnewha".to_string();
        file.size_bytes = 2048;
        let id2 = index.upsert_file(&file).expect("second upsert");

        assert_eq!(id1, id2, "should update, not insert");

        let retrieved = index
            .get_file_by_path(&file.path)
            .expect("get file")
            .expect("should exist");
        assert_eq!(retrieved.content_hash, "newhashnewha");
        assert_eq!(retrieved.size_bytes, 2048);
    }

    #[test]
    fn test_file_hash_lookup() {
        let index = open_test_db();
        let file = test_file_info();
        index.upsert_file(&file).expect("upsert");

        let hash = index.get_file_hash(&file.path).expect("get hash");
        assert_eq!(hash, Some("abc123def456".to_string()));

        let missing = index
            .get_file_hash(Path::new("nonexistent.py"))
            .expect("get hash");
        assert_eq!(missing, None);
    }

    #[test]
    fn test_delete_file() {
        let index = open_test_db();
        let file = test_file_info();
        index.upsert_file(&file).expect("upsert");

        let deleted = index.delete_file(&file.path).expect("delete");
        assert!(deleted);

        let retrieved = index.get_file_by_path(&file.path).expect("get");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_insert_and_get_chunks() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert file");

        let chunk = test_chunk(file_id);
        let chunk_id = index.insert_chunk(&chunk).expect("insert chunk");
        assert!(chunk_id > 0);

        let chunks = index.get_chunks_for_file(file_id).expect("get chunks");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].symbol_path, "main.hello");
        assert_eq!(chunks[0].kind, ChunkKind::Function);
        assert_eq!(
            chunks[0].doc_comment.as_deref(),
            Some("A greeting function.")
        );
    }

    #[test]
    fn test_delete_chunks_for_file() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert file");

        index.insert_chunk(&test_chunk(file_id)).expect("insert");
        index.insert_chunk(&test_chunk(file_id)).expect("insert");

        assert_eq!(index.chunk_count().expect("count"), 2);

        let deleted = index.delete_chunks_for_file(file_id).expect("delete");
        assert_eq!(deleted, 2);
        assert_eq!(index.chunk_count().expect("count"), 0);
    }

    #[test]
    fn test_set_chunk_vector_id() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert file");

        let chunk_id = index.insert_chunk(&test_chunk(file_id)).expect("insert");
        index.set_chunk_vector_id(chunk_id, 42).expect("set vector");

        let chunks = index.get_chunks_for_file(file_id).expect("get");
        assert_eq!(chunks[0].vector_id, Some(42));
    }

    #[test]
    fn test_insert_and_lookup_symbol() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert file");

        let symbol = test_symbol(file_id);
        let sym_id = index.insert_symbol(&symbol).expect("insert symbol");
        assert!(sym_id > 0);

        let found = index
            .get_symbol_by_fqn("main.hello")
            .expect("lookup")
            .expect("should exist");
        assert_eq!(found.name, "hello");
        assert_eq!(found.kind, ChunkKind::Function);
    }

    #[test]
    fn test_search_symbols_by_name() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert file");

        let symbols = vec![
            Symbol {
                id: 0,
                name: "hello".into(),
                fqn: "main.hello".into(),
                kind: ChunkKind::Function,
                file_id,
                line: 1,
                chunk_id: None,
            },
            Symbol {
                id: 0,
                name: "help_me".into(),
                fqn: "main.help_me".into(),
                kind: ChunkKind::Function,
                file_id,
                line: 10,
                chunk_id: None,
            },
            Symbol {
                id: 0,
                name: "goodbye".into(),
                fqn: "main.goodbye".into(),
                kind: ChunkKind::Function,
                file_id,
                line: 20,
                chunk_id: None,
            },
        ];

        for s in &symbols {
            index.insert_symbol(s).expect("insert");
        }

        let results = index.search_symbols_by_name("hel", 10).expect("search");
        assert_eq!(results.len(), 2); // hello, help_me
        assert!(results.iter().all(|s| s.name.starts_with("hel")));
    }

    #[test]
    fn test_keyword_search() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert file");

        let mut chunk1 = test_chunk(file_id);
        chunk1.content =
            "def authenticate_user(username, password):\n    return check_db(username, password)"
                .to_string();
        chunk1.symbol_path = "auth.authenticate_user".to_string();
        index.insert_chunk(&chunk1).expect("insert");

        let mut chunk2 = test_chunk(file_id);
        chunk2.content =
            "def list_users():\n    return db.query('SELECT * FROM users')".to_string();
        chunk2.symbol_path = "users.list_users".to_string();
        index.insert_chunk(&chunk2).expect("insert");

        let results = index.keyword_search("authenticate", 10).expect("search");
        assert!(
            !results.is_empty(),
            "should find results for 'authenticate'"
        );

        // Multi-word AND query — both tokens appear in chunk1; must return results.
        let multi_results = index
            .keyword_search("authenticate_user username", 10)
            .expect("multi-word search");
        assert!(
            !multi_results.is_empty(),
            "multi-word AND query should return results when all tokens present"
        );

        // OR fallback — "authenticate_user" is present, "xyznonexistent" is not.
        // Should still return a result via OR fallback.
        let fallback_results = index
            .keyword_search("authenticate_user xyznonexistent", 10)
            .expect("or fallback search");
        assert!(
            !fallback_results.is_empty(),
            "OR fallback should surface partial matches when AND returns nothing"
        );
    }

    #[test]
    fn test_reindex_file_atomic() {
        let index = open_test_db();
        let file = test_file_info();

        // First indexing
        let chunks = vec![test_chunk(0)];
        let symbols = vec![test_symbol(0)];
        let (file_id, chunk_ids) = index
            .reindex_file(&file, &chunks, &symbols)
            .expect("reindex");

        assert!(file_id > 0);
        assert_eq!(chunk_ids.len(), 1);
        assert_eq!(index.chunk_count().expect("count"), 1);
        assert_eq!(index.symbol_count().expect("count"), 1);

        // Re-index with different data
        let new_chunks = vec![test_chunk(0), test_chunk(0)];
        let new_symbols = vec![
            Symbol {
                id: 0,
                name: "a".into(),
                fqn: "main.a".into(),
                kind: ChunkKind::Function,
                file_id: 0,
                line: 1,
                chunk_id: None,
            },
            Symbol {
                id: 0,
                name: "b".into(),
                fqn: "main.b".into(),
                kind: ChunkKind::Function,
                file_id: 0,
                line: 10,
                chunk_id: None,
            },
        ];
        let (file_id2, chunk_ids2) = index
            .reindex_file(&file, &new_chunks, &new_symbols)
            .expect("reindex");

        assert_eq!(file_id, file_id2, "same file should get same ID");
        assert_eq!(chunk_ids2.len(), 2);
        assert_eq!(
            index.chunk_count().expect("count"),
            2,
            "old chunks should be replaced"
        );
        assert_eq!(
            index.symbol_count().expect("count"),
            2,
            "old symbols should be replaced"
        );
    }

    #[test]
    fn test_cascade_delete() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert");

        index
            .insert_chunk(&test_chunk(file_id))
            .expect("insert chunk");
        index
            .insert_symbol(&test_symbol(file_id))
            .expect("insert symbol");

        assert_eq!(index.chunk_count().expect("count"), 1);
        assert_eq!(index.symbol_count().expect("count"), 1);

        index.delete_file(&file.path).expect("delete");

        assert_eq!(
            index.chunk_count().expect("count"),
            0,
            "chunks should cascade"
        );
        assert_eq!(
            index.symbol_count().expect("count"),
            0,
            "symbols should cascade"
        );
    }

    #[test]
    fn test_statistics() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert");
        index.insert_chunk(&test_chunk(file_id)).expect("insert");
        index.insert_symbol(&test_symbol(file_id)).expect("insert");

        let stats = index.statistics().expect("stats");
        assert_eq!(stats.file_count, 1);
        assert_eq!(stats.chunk_count, 1);
        assert_eq!(stats.symbol_count, 1);
    }

    #[test]
    fn test_insert_chunk_stores_content_hash() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert");

        let mut chunk = test_chunk(file_id);
        chunk.content_hash = 0xDEAD_BEEF_1234_5678_u64;
        let _id = index.insert_chunk(&chunk).expect("insert chunk");

        let chunks = index.get_chunks_for_file(file_id).expect("get chunks");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_hash, 0xDEAD_BEEF_1234_5678_u64);
    }

    #[test]
    fn test_get_chunk_content_hashes_returns_map() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert");

        let mut chunk_a = test_chunk(file_id);
        chunk_a.symbol_path = "main.func_a".to_string();
        chunk_a.content_hash = 111_u64;

        let mut chunk_b = test_chunk(file_id);
        chunk_b.symbol_path = "main.func_b".to_string();
        chunk_b.line_start = 10;
        chunk_b.line_end = 20;
        chunk_b.content_hash = 222_u64;

        index.insert_chunk(&chunk_a).expect("insert a");
        index.insert_chunk(&chunk_b).expect("insert b");

        let hashes = index
            .get_chunk_content_hashes_for_file(file_id)
            .expect("get hashes");

        assert_eq!(hashes.len(), 2);
        assert_eq!(hashes.get("main.func_a").copied(), Some(111_u64));
        assert_eq!(hashes.get("main.func_b").copied(), Some(222_u64));
    }

    #[test]
    fn test_schema_migration_content_hash_default_zero() {
        // Verify that chunks inserted without explicit content_hash default to 0.
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert");

        // Insert a chunk with content_hash = 0 (the default)
        let chunk = test_chunk(file_id); // test_chunk sets content_hash = 0
        index.insert_chunk(&chunk).expect("insert");

        let chunks = index.get_chunks_for_file(file_id).expect("get");
        assert_eq!(chunks[0].content_hash, 0, "default content_hash must be 0");
    }

    // -----------------------------------------------------------------------
    // Sparse vector round-trip tests (Item 8 — BGE-M3 sparse track)
    // -----------------------------------------------------------------------

    #[test]
    fn test_save_and_get_sparse_vector_round_trip() {
        let index = open_test_db();

        // Insert a parent file + chunk so the FK constraint on sparse_vectors is satisfied.
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert file");
        let mut chunk = test_chunk(file_id);
        chunk.symbol_path = "main.sparse_fn".to_string();
        let chunk_id = index.insert_chunk(&chunk).expect("insert chunk");

        // Store a sparse vector for that chunk.
        let tokens: Vec<(u32, f32)> = vec![(42, 0.9), (7, 0.5), (100, 0.1)];
        index
            .save_sparse_vector(chunk_id, &tokens)
            .expect("save sparse vector");

        // Round-trip: retrieve and compare.
        let retrieved = index
            .get_sparse_vector(chunk_id)
            .expect("get sparse vector")
            .expect("sparse vector should be present");

        assert_eq!(retrieved.len(), tokens.len(), "token count must match");
        for ((tid, weight), (rtid, rweight)) in tokens.iter().zip(retrieved.iter()) {
            assert_eq!(tid, rtid, "token_id must match");
            assert!(
                (weight - rweight).abs() < 1e-6,
                "weight must survive JSON round-trip: {weight} vs {rweight}"
            );
        }
    }

    #[test]
    fn test_save_sparse_vector_overwrite() {
        // INSERT OR REPLACE semantics: a second save for the same chunk_id replaces the first.
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert file");
        let mut chunk = test_chunk(file_id);
        chunk.symbol_path = "main.overwrite_fn".to_string();
        let chunk_id = index.insert_chunk(&chunk).expect("insert chunk");

        index
            .save_sparse_vector(chunk_id, &[(1, 0.8)])
            .expect("first save");
        index
            .save_sparse_vector(chunk_id, &[(2, 0.3), (3, 0.7)])
            .expect("second save overwrites");

        let retrieved = index
            .get_sparse_vector(chunk_id)
            .expect("get")
            .expect("present");
        assert_eq!(retrieved.len(), 2, "second write must replace first");
        assert_eq!(retrieved[0].0, 2);
        assert_eq!(retrieved[1].0, 3);
    }

    #[test]
    fn test_get_sparse_vector_missing_returns_none() {
        let index = open_test_db();
        // chunk_id 999 never inserted — must return None, not an error.
        let result = index
            .get_sparse_vector(999)
            .expect("query must not error for missing row");
        assert!(result.is_none(), "missing sparse vector must be None");
    }

    #[test]
    fn test_search_sparse_dot_product_scoring() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert file");

        // chunk A: tokens (1→1.0, 2→1.0)
        let mut chunk_a = test_chunk(file_id);
        chunk_a.symbol_path = "main.fn_a".to_string();
        let id_a = index.insert_chunk(&chunk_a).expect("insert a");
        index
            .save_sparse_vector(id_a, &[(1, 1.0_f32), (2, 1.0_f32)])
            .expect("save a");

        // chunk B: token (1→0.5) only — lower overlap with query
        let mut chunk_b = test_chunk(file_id);
        chunk_b.symbol_path = "main.fn_b".to_string();
        let id_b = index.insert_chunk(&chunk_b).expect("insert b");
        index
            .save_sparse_vector(id_b, &[(1, 0.5_f32)])
            .expect("save b");

        // chunk C: no overlapping tokens — should not appear in results
        let mut chunk_c = test_chunk(file_id);
        chunk_c.symbol_path = "main.fn_c".to_string();
        let id_c = index.insert_chunk(&chunk_c).expect("insert c");
        index
            .save_sparse_vector(id_c, &[(99, 1.0_f32)])
            .expect("save c");

        // Query: token (1→1.0, 2→0.5) — chunk A scores 1.5, chunk B scores 0.5
        let results = index
            .search_sparse(&[(1, 1.0_f32), (2, 0.5_f32)], 10)
            .expect("search_sparse");

        // chunk C has no overlap and must be absent.
        let ids: Vec<i64> = results.iter().map(|(id, _)| *id).collect();
        assert!(!ids.contains(&id_c), "zero-overlap chunk must not appear");

        // chunk A must outrank chunk B.
        assert_eq!(
            ids[0], id_a,
            "chunk A has higher dot-product, must rank first"
        );
        assert_eq!(ids[1], id_b, "chunk B must rank second");

        // Score for A = 1*1.0 + 0.5*1.0 = 1.5
        let score_a = results.iter().find(|(id, _)| *id == id_a).unwrap().1;
        assert!(
            (score_a - 1.5_f32).abs() < 1e-5,
            "dot-product score must be 1.5, got {score_a}"
        );
    }

    #[test]
    fn test_get_all_sparse_vectors_returns_all_rows() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert file");

        for i in 0..5_i64 {
            let mut chunk = test_chunk(file_id);
            chunk.symbol_path = format!("main.fn_{i}");
            let chunk_id = index.insert_chunk(&chunk).expect("insert chunk");
            index
                .save_sparse_vector(chunk_id, &[(i as u32, 1.0)])
                .expect("save");
        }

        let all = index.get_all_sparse_vectors().expect("get all");
        assert_eq!(
            all.len(),
            5,
            "get_all_sparse_vectors must return all 5 rows"
        );
        // Every row must have exactly one token entry.
        for (_, tokens) in &all {
            assert_eq!(tokens.len(), 1);
        }
    }
}
