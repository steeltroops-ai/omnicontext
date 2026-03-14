-- OmniContext SQLite Schema
-- Version: 2
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
    id           INTEGER PRIMARY KEY,
    file_id      INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    symbol_path  TEXT    NOT NULL,
    kind         TEXT    NOT NULL,
    visibility   TEXT    NOT NULL DEFAULT 'private',
    line_start   INTEGER NOT NULL,
    line_end     INTEGER NOT NULL,
    content      TEXT    NOT NULL,
    doc_comment  TEXT,
    metadata     TEXT,
    vector_id    INTEGER,
    token_count  INTEGER NOT NULL,
    weight       REAL    NOT NULL DEFAULT 1.0,
    content_hash INTEGER NOT NULL DEFAULT 0
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

-- FTS5 virtual table for commit search (message + summary)
CREATE VIRTUAL TABLE IF NOT EXISTS commits_fts USING fts5(
    message,
    summary,
    author,
    content='commits',
    content_rowid='rowid',
    tokenize='porter unicode61 remove_diacritics 2'
);

-- FTS sync triggers for commits
CREATE TRIGGER IF NOT EXISTS commits_ai AFTER INSERT ON commits BEGIN
    INSERT INTO commits_fts(rowid, message, summary, author)
    VALUES (new.rowid, new.message, new.summary, new.author);
END;

CREATE TRIGGER IF NOT EXISTS commits_ad AFTER DELETE ON commits BEGIN
    INSERT INTO commits_fts(commits_fts, rowid, message, summary, author)
    VALUES ('delete', old.rowid, old.message, old.summary, old.author);
END;

CREATE TRIGGER IF NOT EXISTS commits_au AFTER UPDATE ON commits BEGIN
    INSERT INTO commits_fts(commits_fts, rowid, message, summary, author)
    VALUES ('delete', old.rowid, old.message, old.summary, old.author);
    INSERT INTO commits_fts(rowid, message, summary, author)
    VALUES (new.rowid, new.message, new.summary, new.author);
END;

-- External documents ingested for enriched context (API docs, RFCs, wikis)
CREATE TABLE IF NOT EXISTS external_docs (
    id           INTEGER PRIMARY KEY,
    source_url   TEXT    NOT NULL UNIQUE,
    title        TEXT    NOT NULL,
    content      TEXT    NOT NULL,
    chunk_ids    TEXT    NOT NULL DEFAULT '[]',
    ingested_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_external_docs_url ON external_docs(source_url);

-- Junction table for commit→file associations (schema v4).
-- Replaces the JSON LIKE full-table scan in commits_for_file().
CREATE TABLE IF NOT EXISTS commit_files (
    commit_hash  TEXT NOT NULL REFERENCES commits(hash) ON DELETE CASCADE,
    file_path    TEXT NOT NULL,
    PRIMARY KEY  (commit_hash, file_path)
);
CREATE INDEX IF NOT EXISTS idx_commit_files_path ON commit_files(file_path);
CREATE INDEX IF NOT EXISTS idx_commit_files_hash ON commit_files(commit_hash);

-- Persisted file-level dependency graph (schema v5).
-- Mirrors FileDependencyGraph outgoing adjacency list.
CREATE TABLE IF NOT EXISTS file_graph_edges (
    source_path  TEXT NOT NULL,
    target_path  TEXT NOT NULL,
    edge_type    TEXT NOT NULL,  -- 'imports'|'inherits'|'calls'|'instantiates'|'historical_co_change'
    weight       REAL NOT NULL DEFAULT 1.0,
    PRIMARY KEY  (source_path, target_path, edge_type)
);
CREATE INDEX IF NOT EXISTS idx_file_graph_source ON file_graph_edges(source_path);
CREATE INDEX IF NOT EXISTS idx_file_graph_target ON file_graph_edges(target_path);

-- Learned sparse vector store for BGE-M3 SPLADE output (schema v6).
-- Stores top-K (token_id, weight) pairs per chunk as a JSON array.
-- Only populated when config.embedding.enable_sparse_retrieval = true.
CREATE TABLE IF NOT EXISTS sparse_vectors (
    chunk_id   INTEGER NOT NULL PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
    tokens     TEXT    NOT NULL  -- JSON: [[token_id, weight], ...]
);
CREATE INDEX IF NOT EXISTS idx_sparse_vectors_chunk ON sparse_vectors(chunk_id);

-- Indexes for query performance
CREATE INDEX IF NOT EXISTS idx_chunks_file       ON chunks(file_id);
CREATE INDEX IF NOT EXISTS idx_chunks_kind       ON chunks(kind);
CREATE INDEX IF NOT EXISTS idx_chunks_visibility ON chunks(visibility);
CREATE INDEX IF NOT EXISTS idx_symbols_name      ON symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_fqn       ON symbols(fqn);
CREATE INDEX IF NOT EXISTS idx_deps_source       ON dependencies(source_id);
CREATE INDEX IF NOT EXISTS idx_deps_target       ON dependencies(target_id);
