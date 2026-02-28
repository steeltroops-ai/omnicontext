# OmniContext Concurrency Architecture

## Overview

OmniContext is a long-running process that must handle:

1. **Continuous file watching** (async I/O)
2. **Batch indexing** (CPU-bound parsing + embedding)
3. **Concurrent queries** (mixed CPU + I/O)
4. **Background maintenance** (index compaction, cache eviction)

All of these happen simultaneously. This document defines the concurrency model.

## Thread Pool Architecture

```
+------------------------------------------------------+
|                    tokio runtime                      |
|  (default thread pool: num_cpus async worker threads) |
+------------------------------------------------------+
       |              |               |            |
       v              v               v            v
  File Watcher   MCP Server     Query Engine   Maintenance
  (async I/O)    (async I/O)    (async I/O)    (async timer)
       |              |               |
       v              v               v
+------------------------------------------------------+
|              spawn_blocking pool                      |
|         (bounded: max 4 concurrent tasks)             |
+------------------------------------------------------+
       |              |               |
       v              v               v
  Tree-sitter     ONNX Embed    Graph Rebuild
  (CPU-bound)     (CPU-bound)   (CPU-bound)
```

## Channel Architecture

```
FileWatcher ----[watch_tx: bounded(256)]----> Indexing Pipeline

Indexing Pipeline:
  file_events ----[parse_tx: bounded(64)]-----> Parser Workers (2 threads)
  parsed_chunks --[embed_tx: bounded(128)]----> Embedder Worker (1 thread, batched)
  embedded ------[store_tx: bounded(64)]------> Store Worker (1 thread, transactional)

MCP Server ----[query_tx: bounded(32)]----> Query Engine
Query Engine --[result_tx: oneshot]-------> MCP Server (per-request)
```

### Channel Capacity Rationale

| Channel    | Capacity | Rationale                                            |
| ---------- | -------- | ---------------------------------------------------- |
| `watch_tx` | 256      | File events come in bursts; buffer to avoid dropping |
| `parse_tx` | 64       | Parsing is fast; small buffer sufficient             |
| `embed_tx` | 128      | Embedding is slower; larger buffer for batching      |
| `store_tx` | 64       | SQLite writes are fast in WAL mode                   |
| `query_tx` | 32       | Concurrent queries shouldn't exceed this             |

### Backpressure Strategy

When channels are full:

1. **watch_tx full**: File watcher blocks (OS buffers filesystem events)
2. **parse_tx full**: Parser waits (backpressure propagates to watcher)
3. **embed_tx full**: Parser waits (this is the bottleneck -- embedding is slowest)
4. **store_tx full**: Embedder waits
5. **query_tx full**: MCP server returns a "server busy" error to the agent

## Concurrency Rules

### Rule 1: CPU-bound Work Uses spawn_blocking

```rust
// CORRECT
let ast = tokio::task::spawn_blocking(move || {
    parser.parse(&source_code)
}).await?;

// WRONG -- blocks the async runtime
let ast = parser.parse(&source_code);
```

### Rule 2: Never Hold Mutex Across .await

```rust
// CORRECT
let data = {
    let guard = state.lock().unwrap();
    guard.clone() // release lock before await
};
let result = async_operation(data).await;

// WRONG -- deadlock risk
let guard = state.lock().unwrap();
let result = async_operation(&guard).await; // holds lock across await!
```

### Rule 3: DashMap for Shared Read-Heavy State

```rust
// Symbol table: read-heavy, write-rare
let symbols: DashMap<String, Symbol> = DashMap::new();

// Insert is rare (during indexing)
symbols.insert(fqn, symbol);

// Read is frequent (during search)
if let Some(sym) = symbols.get(&name) {
    // ...
}
```

### Rule 4: Bounded Concurrency for spawn_blocking

```rust
use tokio::sync::Semaphore;

static BLOCKING_PERMITS: Semaphore = Semaphore::const_new(4);

async fn cpu_bound_task() {
    let _permit = BLOCKING_PERMITS.acquire().await.unwrap();
    tokio::task::spawn_blocking(move || {
        // heavy CPU work
    }).await?;
    // permit auto-released
}
```

## Indexing Pipeline State Machine

```
               +---------+
               |  Idle   |
               +----+----+
                    |
            File event received
                    |
               +----v----+
               | Parsing |  (spawn_blocking, tree-sitter)
               +----+----+
                    |
            AST extracted, chunks created
                    |
               +----v-----+
               | Embedding |  (spawn_blocking, ONNX batch)
               +----+-----+
                    |
            Vectors computed
                    |
               +----v----+
               | Storing |  (async, SQLite transaction)
               +----+----+
                    |
            Index updated, FTS synced, vector inserted
                    |
               +----v----+
               |  Idle   |
               +---------+
```

## Read-Write Isolation

- **SQLite**: WAL mode ensures readers never block writers
- **usearch**: Read-only queries use a snapshot; writes append to journal
- **DashMap**: Lock-free reads, sharded writes
- **petgraph**: Protected by `RwLock` -- rebuild takes write lock, queries take read lock

## Shutdown Sequence

1. Stop file watcher (no new events)
2. Drain all channels (process remaining items)
3. Flush SQLite WAL to main database
4. Persist usearch index to disk
5. Serialize dependency graph to bincode
6. Exit
