# Vector Index

**Location**: `crates/omni-core/src/vector/hnsw.rs`, `crates/omni-core/src/vector/mod.rs`

---

## Overview

The vector index stores 768-dimensional float32 embeddings produced by the embedding engine and provides approximate nearest neighbor (ANN) search for the semantic retrieval signal in the hybrid search pipeline.

---

## Implementation

The production vector index is an HNSW (Hierarchical Navigable Small World) graph backed by the `usearch` crate. The implementation is in `crates/omni-core/src/vector/hnsw.rs`.

```rust
pub struct HNSWIndex {
    inner: usearch::Index,
    tombstones: HashSet<u64>,
    version: u64,
}
```

### Index Parameters

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Dimensions | 768 | jina-embeddings-v2-base-code output size |
| Metric | Cosine | Normalized vectors; cosine outperforms dot product on code |
| M (connectivity) | 24 | Higher than default 16 for better recall on clustered code vectors |
| ef_construction | 200 | Better index quality; build is a one-time cost |
| ef_search | 100 | High recall at query time; latency remains < 5ms |
| Quantization | F16 | 2x storage reduction, < 0.5% quality loss vs F32 |

---

## mmap-Based Storage

For indexes exceeding the in-memory threshold, vectors are stored in a memory-mapped file (`~/.omnicontext/index/<repo_hash>/vectors.usearch`). The operating system manages page eviction — vectors not recently accessed do not consume RAM. This allows indexes up to several hundred thousand chunks to operate within a developer machine's memory budget.

Smaller indexes (below the threshold configured in `config.vector.flat_threshold`, default 5,000 vectors) use an in-memory flat representation for simplicity and determinism in tests.

---

## Flat Fallback

For repositories under `flat_threshold` vectors, the flat index is used:

```rust
pub struct FlatIndex {
    dimensions: usize,
    vectors: HashMap<u64, Vec<f32>>,
}
```

The flat index performs O(n) exhaustive cosine scan and is only appropriate at small scale (< 5,000 chunks). It switches to HNSW automatically when the threshold is crossed on the next full index rebuild.

---

## Incremental Updates

HNSW does not support in-place deletion. Incremental updates use a tombstone strategy:

1. On file change: new chunk vectors are added normally via `index.add(id, &vector)`
2. Old chunk IDs for the changed file are added to the tombstone set
3. Tombstoned vectors are excluded from search results by post-filtering
4. A background compaction task (daemon-managed) rebuilds the HNSW index periodically from non-tombstoned vectors and atomically swaps the new index file in

The compaction interval is configurable via `config.daemon.compaction_interval_hours` (default: 6 hours).

---

## Performance

| Vector Count | HNSW Query (768 dim, CPU) | Flat Query (768 dim) | Memory |
|--------------|--------------------------|----------------------|--------|
| 1,000 | < 0.5ms | < 0.2ms | 6MB (flat) |
| 5,000 | < 1ms | < 1ms | 30MB (flat) |
| 50,000 | < 2ms | ~10ms | 150MB (mmap) |
| 200,000 | < 3ms | ~40ms | 600MB (mmap) |
| 1,000,000 | < 5ms | ~200ms | 3GB (mmap) |

---

## Storage Layout

```
~/.omnicontext/index/<repo_hash>/
├── metadata.db          # SQLite: chunks, files, symbols
├── vectors.usearch      # HNSW index (usearch binary format)
└── vectors.usearch.tmp  # Atomic swap target during compaction
```

---

## See Also

- [Embeddings](./embeddings.md) — upstream vector production
- [Hybrid Search](./hybrid-search.md) — how the vector index is queried
- [ADR-003](../ADR.md#adr-003-usearch-for-vector-index) — rationale for usearch selection
