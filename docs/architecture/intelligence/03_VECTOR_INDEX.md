# Subsystem 3: Vector Index

**Status**: Flat brute-force O(n) scan. Correct but will not scale.
**Priority**: Medium. Not a crisis at current scale. Becomes critical at 50K+ chunks.

---

## Current Implementation Audit

### What Exists

`crates/omni-core/src/vector/mod.rs` (442 lines)

```rust
pub struct VectorIndex {
    dimensions: usize,
    vectors: HashMap<u64, Vec<f32>>,  // entire index in memory
    index_path: Option<PathBuf>,
}

pub fn search(&self, query: &[f32], k: usize) -> OmniResult<Vec<(u64, f32)>> {
    // O(n) exhaustive scan over all vectors
    let mut scores: Vec<(u64, f32)> = self.vectors.iter()
        .map(|(&id, vec)| (id, dot_product(query, vec)))
        .collect();
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1)...);
    scores.truncate(k);
    Ok(scores)
}
```

The code comments even acknowledge this:

> "HNSW (usearch) integration is planned for Phase 2c when larger indexes are needed."

### Performance Profile

| Vector Count | Query Latency (768 dim, CPU) | Memory |
| ------------ | ---------------------------- | ------ |
| 1,000        | ~0.2ms                       | 3MB    |
| 5,000        | ~1ms                         | 15MB   |
| 50,000       | ~10ms                        | 150MB  |
| 200,000      | ~40ms                        | 600MB  |
| 1,000,000    | ~200ms                       | 3GB    |

At current scale (705 active, target ~5000), latency is fine. At enterprise scale (50K-500K chunks across a large monorepo), the flat scan becomes unacceptable.

### Persistence Issues

The vector index is persisted as a bincode-serialized `Vec<(u64, Vec<f32>)>`. On save:

```rust
let encoded = bincode::serialize(&data)?;
std::fs::write(&tmp_path, encoded)?;
std::fs::rename(&tmp_path, path)?;  // atomic
```

For 50K vectors at 768 dims: `50000 * 768 * 4 bytes = ~150MB` binary blob. This is loaded entirely into memory on startup. At 1M vectors, this is 3GB and startup takes 10-30 seconds while deserializing.

---

## State-of-the-Art Research

### Option A: HNSW via `usearch` (Recommended for Phase 2)

**Algorithm**: Hierarchical Navigable Small World (Malkov & Yashunin, 2018)

**Performance at 1M vectors, 768 dim, CPU**:

- Query latency: 1-3ms at 95% recall
- Build time: ~10 minutes
- Memory: ~3GB (same as flat, vectors still in memory)
- Index size on disk: ~3GB binary

**Rust crate**: `usearch` (official Rust bindings from Unum Cloud)

```toml
[dependencies]
usearch = "2.1"
```

```rust
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

let options = IndexOptions {
    dimensions: 768,
    metric: MetricKind::Cos,
    quantization: ScalarKind::F32,
    connectivity: 16,   // M parameter: higher = better recall, more memory
    expansion_add: 128, // ef_construction: higher = better index quality, slower build
    expansion_search: 64, // ef_search: recall/speed tradeoff at query time
    ..Default::default()
};

let index = Index::new(&options)?;
index.reserve(100_000)?;  // pre-allocate
index.add(id, &vector)?;
let results = index.search(&query, 20)?; // returns (keys, distances)
```

**Why HNSW over DiskANN**:

- OmniContext targets codebases up to ~1M chunks maximum (even large monorepos don't exceed this)
- At 1M scale, HNSW with 768 dims fits in 3GB RAM -- acceptable on modern developer machines
- HNSW is 3-5x faster than DiskANN at the same recall for in-memory datasets
- DiskANN's advantage only materializes at 10M+ vectors where RAM is insufficient

**Why HNSW over ScaNN**:

- ScaNN requires a GPU for significant speedup; on CPU HNSW is comparable
- HNSW has a mature Rust crate; ScaNN is C++ only

### Option B: DiskANN for future scale (Phase 4+)

**When**: When the enterprise server-mode index grows past 5M chunks (multi-repo indexing).

**Rust crate**: No official Rust crate. Would require FFI to the C++ library or a reimplementation.

**Practical recommendation**: Spike DiskANN only if HNSW proves insufficient in Phase 4.

### Option C: INT8 Quantization (Implement alongside HNSW)

Reduce each float32 (4 bytes) to int8 (1 byte). 4x reduction in storage and memory bandwidth.

Quality impact on 768-dim vectors: <2% recall loss at 95% recall threshold.

```rust
// Quantize at index time:
fn quantize_f32_to_i8(vec: &[f32]) -> Vec<i8> {
    let max = vec.iter().cloned().fold(0.0f32, f32::max).abs();
    let scale = 127.0 / max;
    vec.iter().map(|&x| (x * scale).round() as i8).collect()
}
```

**Result**: A 1M vector index shrinks from 3GB to 750MB. Practical for developer machines.

---

## HNSW Parameter Tuning for Code Search

Code search has different characteristics than document search:

1. **Query distribution is narrow**: Most queries cluster around function names and identifiers. The search space is not uniformly distributed.
2. **High precision over recall**: For code, returning 5 highly relevant results is better than 20 mediocre ones.
3. **Update frequency**: Developer codebases change constantly. HNSW supports incremental adds but not efficient deletes.

**Recommended parameters**:

| Parameter        | Value | Reason                                                             |
| ---------------- | ----- | ------------------------------------------------------------------ |
| M (connectivity) | 24    | Higher than default 16 for better recall on clustered code vectors |
| ef_construction  | 200   | Better index quality, build time is one-time cost                  |
| ef_search        | 100   | High recall at query time, latency remains <5ms                    |
| Quantization     | F16   | 2x compression, <0.5% quality loss vs F32                          |

### Handling Incremental Updates

HNSW does not support efficient deletion. For a code index that changes continuously:

**Strategy**: Tombstone + periodic rebuild

1. On file change: add new chunk vectors normally, mark old vectors as tombstoned
2. Tombstoned vectors are excluded from search results (filter by validity map)
3. Every N hours (configurable): rebuild the full HNSW index from non-tombstoned vectors
4. Rebuild is async, runs in daemon background thread, swapped atomically on completion

```rust
pub struct HNSWIndex {
    inner: usearch::Index,
    tombstones: HashSet<u64>,  // deleted vector IDs
    version: u64,
}
```

---

## Implementation Plan

### Phase A (current -- flat scan): No change needed yet

At <10K chunks, the flat scan is fine. Latency is <2ms. Fix the embedding coverage first (Subsystem 2).

### Phase B (when consistently >10K chunks): HNSW upgrade

1. Add `usearch` dependency to `omni-core/Cargo.toml`
2. Implement `HNSWIndex` struct wrapping `usearch::Index`
3. Add tombstone map for incremental deletes
4. Implement atomic index swap on rebuild
5. Keep `VectorIndex` (flat) as the in-test code path for unit tests (fast, deterministic)
6. Behind a feature flag initially: `OMNI_VECTOR_BACKEND=hnsw`

### Phase C (enterprise server mode): Full HNSW + INT8

1. Add INT8 quantization to the HNSW index
2. Implement background rebuild thread in the daemon
3. Expose index stats via the REST API

---

## Flows with Problems

```
Current Flow:
embed_batch() -> [768-dim f32 vectors]
    -> VectorIndex::add() -> HashMap<u64, Vec<f32>>  [in memory]
    -> VectorIndex::save() -> bincode serialize -> disk  [full file rewrite every time]
    -> VectorIndex::search() -> O(n) linear scan

Problems:
1. Full file rewrite on every save is O(n) I/O
2. No delete support -- removed files leave orphan vectors
3. All vectors must fit in RAM (no disk-resident index)

Target Flow (HNSW):
embed_batch() -> [768-dim f32 vectors]
    -> HNSWIndex::add() -> usearch graph update  [in memory, O(log n)]
    -> HNSWIndex::save() -> usearch serialize  [incremental, fast]
    -> HNSWIndex::search() -> graph traversal O(log n), actual: ~2ms
    -> Result filtered for tombstone IDs
```
