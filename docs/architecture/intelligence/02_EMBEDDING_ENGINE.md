# Subsystem 2: Embedding Engine

**Status**: CRITICAL BUG -- 15% coverage (705/4711 chunks). Root cause must be identified and eliminated.
**Priority**: P0. The entire semantic search layer is degraded to near-useless.

---

## Current Implementation Audit

### Architecture

`crates/omni-core/src/embedder/mod.rs`  
`crates/omni-core/src/embedder/model_manager.rs`

- Model: `jinaai/jina-embeddings-v2-base-code` (ONNX, ~550MB)
- Dimensions: 768
- Context window: 8192 tokens
- Inference: `ort` crate (ONNX Runtime), CPU only
- Batch size: 32 chunks per inference call
- Fallback: Degraded mode when model unavailable -- keyword-only search

### The 15% Coverage Problem

**705 embeddings out of 4711 chunks = 14.95% coverage.**

This means 84% of the codebase has NO vector representation. Every search query against this index is running keyword-only BM25 for 84% of results. The "hybrid" search is effectively not hybrid.

### Root Cause Analysis

The most likely causes, in order of probability:

**Cause 1: Model download timeout or partial download (most likely)**

The embedder checks `is_model_ready()` which verifies the file is > 1MB. If the download was partially interrupted, the model file may exist at 1.1MB but be corrupt / truncated. The session will fail to load, the embedder falls to degraded mode, and indexing proceeds without any embeddings.

```rust
// In is_model_ready():
if meta.len() < 1_000_000 { return false; }  // 1MB check
// But no hash check -- a 50MB corrupt file passes this check silently!
```

**Cause 2: ONNX Runtime DLL not found at runtime**

On Windows, `onnxruntime.dll` must be in the same directory as the binary or in PATH. If installation moved the binaries but not the DLLs, every ONNX session creation fails silently:

```rust
Err(e) => {
    tracing::warn!(error = %e, "failed to create ONNX session builder...");
    None  // <- falls to degraded, no error propagated
}
```

**Cause 3: Batch processing silently drops failed batches**

In `embed_batch()`, if a single batch fails inference, that batch's chunks get no vector but indexing continues:

```rust
for batch in chunks.chunks(self.config.batch_size) {
    let batch_embeddings = self.run_inference(&mut session, batch)?;
    // The `?` here propagates the error -- so this is actually correct
    // But upstream callers may be catching and swallowing this error
}
```

**Cause 4: Pipeline calls embedder asynchronously and race condition silences errors**

Need to audit `pipeline/mod.rs` for how embedding results are connected to chunk storage.

### Diagnosis Commands

```powershell
# Check if model file exists and its size
Test-Path $HOME\.omnicontext\models\jina-embeddings-v2-base-code\model.onnx
(Get-Item $HOME\.omnicontext\models\jina-embeddings-v2-base-code\model.onnx).length

# Check if ONNX Runtime DLL is present
Get-ChildItem $HOME\.omnicontext\bin\onnxruntime*.dll

# Run with verbose logging to see where embedding fails
$env:RUST_LOG="omni_core=debug,ort=debug"
omnicontext index --path . 2>&1 | Select-String "embed|onnx|model|degraded"
```

---

## The Fix: Make Embedder Fail Hard, Not Silently

The philosophy "fall to keyword-only if embedding fails" is correct for resilience. But the **detection** of failure must be explicit, not silent. A system that fails silently produces misleading results.

**Fix 1: SHA-256 hash verification on model file**

```rust
// In is_model_ready():
pub fn is_model_ready(spec: &ModelSpec) -> bool {
    let model = model_path(spec);
    if !model.exists() { return false; }

    let size = std::fs::metadata(&model).map(|m| m.len()).unwrap_or(0);
    if size < spec.approx_size_bytes / 2 {
        // File is less than half expected size -- corrupt
        tracing::warn!(
            path = %model.display(),
            actual_bytes = size,
            expected_bytes = spec.approx_size_bytes,
            "model file appears truncated, will re-download"
        );
        let _ = std::fs::remove_file(&model); // force re-download
        return false;
    }

    true
}
```

**Fix 2: Surface embedding coverage as a diagnostic metric**

After indexing, compute and log:

```
Indexed: 4711 chunks
Embedded: 705 (14.95%) -- WARNING: below 90% threshold
Keyword-only: 4006 (85.05%)
Reason: ONNX session failed to initialize
```

This should also be surfaced in the VS Code sidebar status panel.

**Fix 3: Re-embedding pass command**

Add `omnicontext embed --retry-failed` that reads all chunks with `vector_id = NULL` from the database and attempts to embed them. This allows recovery without a full re-index.

**Fix 4: Explicit error when embedder is degraded at startup**

```rust
// Instead of:
tracing::warn!("model auto-download failed, will operate in keyword-only mode");

// Use:
tracing::error!(
    model = spec.name,
    "CRITICAL: embedding model failed to load. Semantic search is DISABLED. \
     Run `omnicontext doctor` to diagnose. Indexing will continue in keyword-only mode \
     but search quality will be severely degraded."
);
```

---

## Improving Embedding Quality (Beyond the Bug Fix)

### Technique 1: Instruction-Following Embeddings

`jina-embeddings-v2-base-code` is a bi-encoder trained contrastively. Like all bi-encoders, it benefits from task-specific prefixes on the query side:

```
Query side:  "Represent this code search query: {query}"
Passage side: "Represent this code snippet: {chunk content}"
```

The current implementation embeds chunks and queries with identical formatting. Adding asymmetric prompting can lift retrieval quality 8-15% without any model change. Jina-v2-code specifically recommends:

- Queries: `"Represent this sentence for searching relevant passages: {query}"`
- Passages: plain text (no prefix)

**Implementation**: Modify `embed_single()` for queries to prepend the instruction string.

### Technique 2: Multi-Vector Representations (ColBERT-style)

Instead of one 768-dim vector per chunk, produce one 768-dim vector per token (the full `[batch, seq_len, hidden]` output). Store the token matrix. At query time, compute MaxSim between query tokens and document tokens.

```
Query tokens:      [q1, q2, q3, q4]          (4 x 768)
Document tokens:   [d1, d2, d3, d4, d5, d6]  (6 x 768)
MaxSim = sum over qi of max_j(qi · dj)
```

**Benefit**: Token-level matching finds relevant chunks that share exact identifier names even if the overall function is semantically distant. Critical for code search where the exact function name matters.

**Cost**: Storage is 50-200x larger (N tokens per chunk vs 1 vector). For a 5,000 chunk index with average 128 tokens per chunk: 5000 _ 128 _ 768 floats = ~2.4GB. Feasible only with quantization.

**Practical path**: Implement as an optional mode via `OMNI_COLBERT_MODE=1`. Quantize token vectors to INT8 (4x compression). Target: < 600MB for 5000 chunks.

### Technique 3: Matryoshka Training / Dimension Reduction

For fast approximate search, reduce embedding dimensions from 768 to 256 or 128 using PCA trained on the existing index. A 768->128 reduction cuts vector storage and search time by 6x with minimal quality loss (5-8% on code retrieval benchmarks).

```rust
// Post-embed PCA projection (offline precomputed matrix):
fn project_to_compact(vec: &[f32], pca_matrix: &[[f32; 128]]) -> [f32; 128] {
    // Matrix multiply: [1x768] @ [768x128] -> [1x128]
}
```

**Recommendation**: Not needed until vector index exceeds 100K chunks in production.

---

## Implementation Plan

### Phase A (P0, This Week): Fix the coverage bug

1. Add SHA-256 hash verification to `is_model_ready()`
2. Add explicit error logging when embedder starts in degraded mode
3. Add coverage metric to post-indexing summary output
4. Add `omnicontext embed --retry-failed` CLI command
5. Re-run indexing, verify coverage reaches 90%+

### Phase B (2-3 weeks): Instruction-following queries

1. Add `QUERY_PREFIX` constant per model spec
2. Apply prefix in `SearchEngine::search()` before `embedder.embed_single(query)`
3. Benchmark retrieval quality before/after on test queries

### Phase C (6-8 weeks): Per-model spec validation

1. Add `expected_sha256` to `ModelSpec`
2. Compute SHA-256 on download completion and on every startup
3. Auto-re-download on hash mismatch

---

## Flows with Problems

```
Current (broken) flow:
model_manager::ensure_model()
    -> download() -> partial file -> is_model_ready() returns true (size > 1MB)
    -> Session::builder()::commit_from_file(corrupt_file)
    -> Err(_) -> degraded mode
    -> indexing runs: 4711 chunks processed
    -> 0 embeddings stored (embedder.is_available() = false)
    -> vector_id = NULL for all chunks
    -> 705 somehow get vectors (???) -- from a PREVIOUS index not cleared

Correct flow:
model_manager::ensure_model()
    -> download() -> hash verify -> pass
    -> Session loads OK -> embedder.is_available() = true
    -> indexing runs: 4711 chunks
    -> embed_batch() called for all chunks
    -> vector_id set for all chunks
    -> coverage: 100% (or 95%+ accounting for chunks that fail token budget)
```

---

## The 705 Mystery

The 705 embeddings that DO exist are likely from a previous successful index run before the model became corrupt/unavailable. The metadata database retains old `vector_id` values and the vector bin file retains old vectors. When a re-index runs with a degraded embedder, new chunks get NULL vector_id but old ones retain their vector_id. The vector bin file is also retained from the previous run -- so 705 old vectors are still in it.

This means the index is in a **split-brain state**: some chunks have stale vectors from an old version of the code, others have no vectors. The stale vectors are actively misleading search results because they point to code that may have changed.

**Fix**: On index invalidation (model change, or model re-download), delete the vector bin file and re-embed everything.
