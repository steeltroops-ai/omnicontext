# OmniContext Error Recovery & Resilience

## Design Principle

OmniContext must **never crash** due to bad input. A single malformed file, corrupted index, or OOM condition must not bring down the entire service. The system degrades gracefully.

## Failure Modes and Recovery

### 1. Tree-sitter Parse Failure

**Cause**: Malformed syntax, unsupported language construct, grammar bug
**Impact**: Single file not indexed
**Recovery**:

```
1. Catch the panic (tree-sitter is C, can segfault on edge cases)
   - Use std::panic::catch_unwind around parse calls
2. Log warning with file path and error details
3. Mark file as "parse_failed" in index.db
4. Still attempt keyword-only indexing (raw content, no AST structure)
5. Retry on next file modification event
```

**User visibility**: Status command shows "N files failed to parse"

### 2. ONNX Runtime OOM / Timeout

**Cause**: Extremely large chunk, corrupted model, insufficient RAM
**Impact**: Chunk(s) not embedded
**Recovery**:

```
1. Set per-inference timeout: 5 seconds max
2. If timeout: split chunk in half and retry
3. If OOM: log error, fall back to keyword-only index for this chunk
4. Set vector_id = NULL for unembedded chunks
5. Search engine treats NULL vector_id chunks as keyword-only candidates
```

### 3. SQLite Corruption

**Cause**: Process killed during write, disk full, filesystem corruption
**Impact**: Index unavailable
**Recovery**:

```
1. On startup: run "PRAGMA integrity_check"
2. If corrupted:
   a. Attempt WAL replay (sqlite3_recover)
   b. If recovery fails: delete index.db, trigger full reindex
   c. Notify user: "Index corrupted, rebuilding..."
3. Maintain state.json separately (not in SQLite) as recovery checkpoint
```

### 4. usearch Index Corruption

**Cause**: mmap crash, incomplete write, disk full
**Impact**: Semantic search unavailable
**Recovery**:

```
1. On startup: attempt to load vectors.usearch
2. If load fails:
   a. Delete vectors.usearch
   b. Rebuild from chunk embeddings in SQLite
   c. If no embeddings in SQLite: trigger full reindex
3. During rebuild: fall back to keyword-only search
```

### 5. File Watcher Event Loss

**Cause**: Known issue on all platforms under heavy I/O, overflow buffer
**Impact**: Index becomes stale
**Recovery**:

```
1. Periodic full scan every 5 minutes (configurable)
2. Compare file hashes (stored in state.json) against disk
3. Re-index any files with changed hashes
4. On startup: always do a full scan to catch missed events
```

### 6. Cold Boot (No Index, Immediate Query)

**Cause**: First run, or after index deletion
**Impact**: No search results
**Recovery**:

```
1. Return empty results with a message: "Index is building. Try again shortly."
2. Start indexing immediately in background
3. Expose indexing progress via MCP resource: "indexing_status"
4. As chunks become available, they become searchable immediately
   (don't wait for full index completion)
```

### 7. Disk Full

**Cause**: User's disk is full
**Impact**: Can't write index
**Recovery**:

```
1. Check available space before starting index operations
2. If < 100MB available: refuse to index, warn user
3. If write fails mid-operation: rollback SQLite transaction
4. Don't corrupt the existing index -- fail the new write, keep the old
```

### 8. Embedding Model Missing

**Cause**: Model file deleted, wrong path, incompatible ONNX version
**Impact**: No semantic search
**Recovery**:

```
1. On startup: verify model file exists and is valid ONNX
2. If missing: warn user, operate in keyword-only mode
3. Provide CLI command: "omnicontext download-model" to re-fetch
4. Support fallback model list: try primary, then secondary, then keyword-only
```

## Error Taxonomy

```rust
#[derive(Debug, Error)]
pub enum OmniError {
    // Recoverable -- operation failed but system is healthy
    #[error("parse failed for {path}: {source}")]
    ParseError { path: PathBuf, source: Box<dyn Error> },

    #[error("embedding failed for chunk {chunk_id}: {source}")]
    EmbedError { chunk_id: i64, source: Box<dyn Error> },

    #[error("file not found: {path}")]
    FileNotFound { path: PathBuf },

    // Degraded -- system works with reduced capability
    #[error("model not available, using keyword-only search")]
    ModelUnavailable,

    #[error("vector index unavailable, using keyword-only search")]
    VectorIndexUnavailable,

    // Fatal -- system cannot operate
    #[error("database corruption detected: {details}")]
    DatabaseCorruption { details: String },

    #[error("insufficient disk space: {available_mb}MB available, {required_mb}MB required")]
    InsufficientDisk { available_mb: u64, required_mb: u64 },

    #[error("configuration error: {details}")]
    ConfigError { details: String },
}
```

## Monitoring

Every error increments a counter exposed via the status command:

```
omnicontext status

Index: healthy
Files indexed: 12,847
Files failed:  3 (run `omnicontext status --failed` for details)
Search mode:   hybrid (semantic + keyword)
Last indexed:  2 seconds ago
Uptime:        14h 23m
Errors (last hour): parse=2, embed=0, store=0
```
