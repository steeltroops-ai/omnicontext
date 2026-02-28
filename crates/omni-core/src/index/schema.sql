-- OmniContext SQLite Schema
-- Version: 1
-- WAL mode is set programmatically, not in schema.

CREATE TABLE IF NOT EXISTS files (
    id          INTEGER PRIMARY KEY,
    path        TEXT    NOT NULL UNIQUE,
    language    TEXT    NOT NULL,
    hash        TEXT    NOT NULL,
    size_bytes  INTEGER NOT NULL,
    indexed_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    last_modified TEXT  NOT NULL
);

CREATE TABLE IF NOT EXISTS chunks (
    id          INTEGER PRIMARY KEY,
    file_id     INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    symbol_path TEXT    NOT NULL,
    kind        TEXT    NOT NULL,
    visibility  TEXT    NOT NULL DEFAULT 'private',
    line_start  INTEGER NOT NULL,
    line_end    INTEGER NOT NULL,
    content     TEXT    NOT NULL,
    doc_comment TEXT,
    metadata    TEXT,
    vector_id   INTEGER,
    token_count INTEGER NOT NULL,
    weight      REAL    NOT NULL DEFAULT 1.0
);

-- FTS5 virtual table for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
    content,
    doc_comment,
    symbol_path,
    content='chunks',
    content_rowid='id',
    tokenize='porter unicode61 remove_diacritics 2'
);

-- FTS sync triggers
CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
    INSERT INTO chunks_fts(rowid, content, doc_comment, symbol_path)
    VALUES (new.id, new.content, new.doc_comment, new.symbol_path);
END;

CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
    INSERT INTO chunks_fts(chunks_fts, rowid, content, doc_comment, symbol_path)
    VALUES ('delete', old.id, old.content, old.doc_comment, old.symbol_path);
END;

CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks BEGIN
    INSERT INTO chunks_fts(chunks_fts, rowid, content, doc_comment, symbol_path)
    VALUES ('delete', old.id, old.content, old.doc_comment, old.symbol_path);
    INSERT INTO chunks_fts(rowid, content, doc_comment, symbol_path)
    VALUES (new.id, new.content, new.doc_comment, new.symbol_path);
END;

CREATE TABLE IF NOT EXISTS symbols (
    id      INTEGER PRIMARY KEY,
    name    TEXT    NOT NULL,
    fqn     TEXT    NOT NULL UNIQUE,
    kind    TEXT    NOT NULL,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    line    INTEGER NOT NULL,
    chunk_id INTEGER REFERENCES chunks(id)
);

CREATE TABLE IF NOT EXISTS dependencies (
    source_id INTEGER NOT NULL REFERENCES symbols(id),
    target_id INTEGER NOT NULL REFERENCES symbols(id),
    kind      TEXT    NOT NULL,
    PRIMARY KEY (source_id, target_id, kind)
);

CREATE TABLE IF NOT EXISTS commits (
    hash            TEXT PRIMARY KEY,
    message         TEXT NOT NULL,
    author          TEXT NOT NULL,
    timestamp       TEXT NOT NULL,
    summary         TEXT,
    files_changed   TEXT NOT NULL
);

-- Indexes for query performance
CREATE INDEX IF NOT EXISTS idx_chunks_file       ON chunks(file_id);
CREATE INDEX IF NOT EXISTS idx_chunks_kind       ON chunks(kind);
CREATE INDEX IF NOT EXISTS idx_chunks_visibility ON chunks(visibility);
CREATE INDEX IF NOT EXISTS idx_symbols_name      ON symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_fqn       ON symbols(fqn);
CREATE INDEX IF NOT EXISTS idx_deps_source       ON dependencies(source_id);
CREATE INDEX IF NOT EXISTS idx_deps_target       ON dependencies(target_id);
