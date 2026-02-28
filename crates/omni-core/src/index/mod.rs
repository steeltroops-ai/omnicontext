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

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::OmniResult;
use crate::types::{Chunk, ChunkKind, FileInfo, Language, Symbol, Visibility};

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
        let result = self.conn.query_row(
            "SELECT id, path, language, hash, size_bytes FROM files WHERE path = ?1",
            params![path.to_string_lossy().as_ref()],
            |row| {
                Ok(FileInfo {
                    id: row.get(0)?,
                    path: std::path::PathBuf::from(row.get::<_, String>(1)?),
                    language: Language::from_extension(
                        &row.get::<_, String>(2)?
                    ),
                    content_hash: row.get(3)?,
                    size_bytes: row.get(4)?,
                })
            },
        ).optional()?;

        Ok(result)
    }

    /// Get the hash of an indexed file (for change detection).
    pub fn get_file_hash(&self, path: &Path) -> OmniResult<Option<String>> {
        let hash = self.conn.query_row(
            "SELECT hash FROM files WHERE path = ?1",
            params![path.to_string_lossy().as_ref()],
            |row| row.get(0),
        ).optional()?;

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

    /// Count total indexed files.
    pub fn file_count(&self) -> OmniResult<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM files",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    // -----------------------------------------------------------------------
    // Chunk operations
    // -----------------------------------------------------------------------

    /// Insert a chunk record. Returns the chunk ID.
    pub fn insert_chunk(&self, chunk: &Chunk) -> OmniResult<i64> {
        self.conn.execute(
            "INSERT INTO chunks (file_id, symbol_path, kind, visibility, line_start,
             line_end, content, doc_comment, token_count, weight, vector_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Delete all chunks belonging to a file.
    pub fn delete_chunks_for_file(&self, file_id: i64) -> OmniResult<usize> {
        let changes = self.conn.execute(
            "DELETE FROM chunks WHERE file_id = ?1",
            params![file_id],
        )?;
        Ok(changes)
    }

    /// Get all chunks for a file.
    pub fn get_chunks_for_file(&self, file_id: i64) -> OmniResult<Vec<Chunk>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_id, symbol_path, kind, visibility, line_start,
             line_end, content, doc_comment, token_count, weight, vector_id
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

    /// Count total chunks across all files.
    pub fn chunk_count(&self) -> OmniResult<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM chunks",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
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

    /// Look up a symbol by its fully qualified name.
    pub fn get_symbol_by_fqn(&self, fqn: &str) -> OmniResult<Option<Symbol>> {
        let result = self.conn.query_row(
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
        ).optional()?;

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
        let changes = self.conn.execute(
            "DELETE FROM symbols WHERE file_id = ?1",
            params![file_id],
        )?;
        Ok(changes)
    }

    /// Count total symbols.
    pub fn symbol_count(&self) -> OmniResult<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM symbols",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    // -----------------------------------------------------------------------
    // FTS5 keyword search
    // -----------------------------------------------------------------------

    /// Search chunks using FTS5 full-text search.
    ///
    /// Returns (chunk_id, bm25_score) pairs, ordered by relevance.
    pub fn keyword_search(
        &self,
        query: &str,
        limit: usize,
    ) -> OmniResult<Vec<(i64, f64)>> {
        // FTS5 uses BM25 for relevance ranking.
        // We search across content, doc_comment, and symbol_path.
        let mut stmt = self.conn.prepare(
            "SELECT rowid, bm25(chunks_fts, 1.0, 0.5, 2.0) as score
             FROM chunks_fts
             WHERE chunks_fts MATCH ?1
             ORDER BY score
             LIMIT ?2",
        )?;

        let results = stmt.query_map(params![query, limit as i64], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })?;

        let mut out = Vec::new();
        for r in results {
            out.push(r?);
        }
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Transaction helpers
    // -----------------------------------------------------------------------

    /// Re-index a file atomically: delete old data, insert new chunks and symbols.
    ///
    /// This is the primary write operation. It ensures consistency by
    /// wrapping delete+insert in a single transaction.
    pub fn reindex_file(
        &self,
        file: &FileInfo,
        chunks: &[Chunk],
        symbols: &[Symbol],
    ) -> OmniResult<(i64, Vec<i64>)> {
        let tx = self.conn.unchecked_transaction()?;

        // Upsert the file
        tx.execute(
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

        let file_id: i64 = tx.query_row(
            "SELECT id FROM files WHERE path = ?1",
            params![file.path.to_string_lossy().as_ref()],
            |row| row.get(0),
        )?;

        // Delete old chunks and symbols for this file
        tx.execute("DELETE FROM symbols WHERE file_id = ?1", params![file_id])?;
        tx.execute("DELETE FROM chunks WHERE file_id = ?1", params![file_id])?;

        // Insert new chunks
        let mut chunk_ids = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            tx.execute(
                "INSERT INTO chunks (file_id, symbol_path, kind, visibility, line_start,
                 line_end, content, doc_comment, token_count, weight, vector_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    file_id,
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
                ],
            )?;
            chunk_ids.push(tx.last_insert_rowid());
        }

        // Insert new symbols
        for symbol in symbols {
            tx.execute(
                "INSERT OR REPLACE INTO symbols (name, fqn, kind, file_id, line, chunk_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    symbol.name,
                    symbol.fqn,
                    format!("{:?}", symbol.kind).to_lowercase(),
                    file_id,
                    symbol.line,
                    symbol.chunk_id,
                ],
            )?;
        }

        tx.commit()?;
        Ok((file_id, chunk_ids))
    }

    // -----------------------------------------------------------------------
    // Status / diagnostics
    // -----------------------------------------------------------------------

    /// Run an integrity check on the database.
    pub fn check_integrity(&self) -> OmniResult<bool> {
        let result: String = self.conn.query_row(
            "PRAGMA integrity_check",
            [],
            |row| row.get(0),
        )?;
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

    /// Get the raw connection for advanced queries.
    /// Use sparingly -- prefer adding methods to this struct.
    pub fn connection(&self) -> &Connection {
        &self.conn
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
// Parse helpers
// ---------------------------------------------------------------------------

fn parse_chunk_kind(s: &str) -> ChunkKind {
    match s {
        "function" => ChunkKind::Function,
        "class" => ChunkKind::Class,
        "trait" => ChunkKind::Trait,
        "impl" => ChunkKind::Impl,
        "const" => ChunkKind::Const,
        "typedef" => ChunkKind::TypeDef,
        "module" => ChunkKind::Module,
        "test" => ChunkKind::Test,
        _ => ChunkKind::TopLevel,
    }
}

fn parse_visibility(s: &str) -> Visibility {
    match s {
        "public" => Visibility::Public,
        "crate" => Visibility::Crate,
        "protected" => Visibility::Protected,
        "private" => Visibility::Private,
        _ => Visibility::Private,
    }
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

        let missing = index.get_file_hash(Path::new("nonexistent.py")).expect("get hash");
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
        assert_eq!(chunks[0].doc_comment.as_deref(), Some("A greeting function."));
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
            Symbol { id: 0, name: "hello".into(), fqn: "main.hello".into(), kind: ChunkKind::Function, file_id, line: 1, chunk_id: None },
            Symbol { id: 0, name: "help_me".into(), fqn: "main.help_me".into(), kind: ChunkKind::Function, file_id, line: 10, chunk_id: None },
            Symbol { id: 0, name: "goodbye".into(), fqn: "main.goodbye".into(), kind: ChunkKind::Function, file_id, line: 20, chunk_id: None },
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
        chunk1.content = "def authenticate_user(username, password):\n    return check_db(username, password)".to_string();
        chunk1.symbol_path = "auth.authenticate_user".to_string();
        index.insert_chunk(&chunk1).expect("insert");

        let mut chunk2 = test_chunk(file_id);
        chunk2.content = "def list_users():\n    return db.query('SELECT * FROM users')".to_string();
        chunk2.symbol_path = "users.list_users".to_string();
        index.insert_chunk(&chunk2).expect("insert");

        let results = index.keyword_search("authenticate", 10).expect("search");
        assert!(!results.is_empty(), "should find results for 'authenticate'");
    }

    #[test]
    fn test_reindex_file_atomic() {
        let index = open_test_db();
        let file = test_file_info();

        // First indexing
        let chunks = vec![test_chunk(0)];
        let symbols = vec![test_symbol(0)];
        let (file_id, chunk_ids) = index.reindex_file(&file, &chunks, &symbols).expect("reindex");

        assert!(file_id > 0);
        assert_eq!(chunk_ids.len(), 1);
        assert_eq!(index.chunk_count().expect("count"), 1);
        assert_eq!(index.symbol_count().expect("count"), 1);

        // Re-index with different data
        let new_chunks = vec![test_chunk(0), test_chunk(0)];
        let new_symbols = vec![
            Symbol { id: 0, name: "a".into(), fqn: "main.a".into(), kind: ChunkKind::Function, file_id: 0, line: 1, chunk_id: None },
            Symbol { id: 0, name: "b".into(), fqn: "main.b".into(), kind: ChunkKind::Function, file_id: 0, line: 10, chunk_id: None },
        ];
        let (file_id2, chunk_ids2) = index.reindex_file(&file, &new_chunks, &new_symbols).expect("reindex");

        assert_eq!(file_id, file_id2, "same file should get same ID");
        assert_eq!(chunk_ids2.len(), 2);
        assert_eq!(index.chunk_count().expect("count"), 2, "old chunks should be replaced");
        assert_eq!(index.symbol_count().expect("count"), 2, "old symbols should be replaced");
    }

    #[test]
    fn test_cascade_delete() {
        let index = open_test_db();
        let file = test_file_info();
        let file_id = index.upsert_file(&file).expect("upsert");

        index.insert_chunk(&test_chunk(file_id)).expect("insert chunk");
        index.insert_symbol(&test_symbol(file_id)).expect("insert symbol");

        assert_eq!(index.chunk_count().expect("count"), 1);
        assert_eq!(index.symbol_count().expect("count"), 1);

        index.delete_file(&file.path).expect("delete");

        assert_eq!(index.chunk_count().expect("count"), 0, "chunks should cascade");
        assert_eq!(index.symbol_count().expect("count"), 0, "symbols should cascade");
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
}
