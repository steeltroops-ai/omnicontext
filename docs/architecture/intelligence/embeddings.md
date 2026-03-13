# Embedding Engine

**Location**: `crates/omni-core/src/embedder/mod.rs`, `crates/omni-core/src/embedder/model_manager.rs`

---

## Overview

The embedding engine converts code chunks into dense vector representations used for semantic retrieval. All inference runs locally via ONNX Runtime — no external API calls are made at any point in the pipeline.

---

## Model

**`jinaai/jina-embeddings-v2-base-code`**

| Property | Value |
|----------|-------|
| Architecture | Jina BERT (ALiBi positional encoding) |
| Output dimensions | 768 |
| Context window | 8192 tokens |
| Runtime format | ONNX |
| Model size on disk | ~550 MB |
| Inference backend | `ort` crate (ONNX Runtime) |
| Hardware | CPU (primary); GPU via ONNX EP if available |

This model is trained on a code-specific corpus covering 30+ programming languages. It produces embeddings that capture semantic intent — two functions implementing the same algorithm in different languages will produce similar vectors.

---

## Model Download and Storage

The model is auto-downloaded on first use to `~/.omnicontext/models/jina-embeddings-v2-base-code/`. The download process:

1. Fetches `model.onnx` and `tokenizer.json` from the configured model registry URL
2. Computes SHA-256 checksum on the downloaded file and compares against the expected value in `ModelSpec`
3. Rejects and re-downloads if the checksum does not match
4. Verifies the file size is within expected bounds before accepting

The `--no-model` install flag skips this download. When the model is absent, the engine operates in degraded mode (keyword-only search).

---

## Session Pooling

The embedder maintains a pool of ONNX Runtime sessions to support concurrent embedding requests without contention. Sessions are created lazily at startup and returned to the pool after each use. Pool size is configurable; default is 2 sessions.

This allows the indexing pipeline and the search pipeline (for query embedding) to run concurrently without serializing on a single session lock.

---

## Batch Scheduling

Chunks are embedded in batches of up to 80 chunks per inference call. The batch scheduler:

- Accumulates chunks from the pipeline until the batch window is full or a configurable timeout elapses
- Pads batches that are smaller than the window with attention mask zeroing
- Flushes immediately when the indexing pipeline signals end-of-file

Batch size is tunable via `config.embedder.batch_size`. Larger batches improve throughput; smaller batches reduce latency for incremental re-indexing.

---

## INT8 Quantization

INT8 quantization infrastructure is implemented in `crates/omni-core/src/embedder/quantization.rs`. When quantization is enabled:

- Float32 weights are quantized to INT8 at model load time
- Vector storage uses INT8 representation (4x memory reduction vs F32)
- Cosine similarity is computed with dequantization on the fly

The quantization path is selectable via `config.embedder.quantization = "int8"`. Quality impact is less than 2% recall loss at 95% recall threshold for 768-dimensional vectors.

---

## Degraded Mode Fallback

When the embedding model is unavailable (missing, failed checksum, ONNX Runtime initialization failure), the embedder transitions to degraded mode:

- All new chunks are stored without vector embeddings (`vector_id = NULL`)
- Search continues using FTS5 keyword search and symbol lookup only
- The degraded state is surfaced in the daemon status IPC response and in the VS Code sidebar
- `omnicontext doctor` provides diagnosis and repair instructions
- `omnicontext embed --retry-failed` re-attempts embedding for all chunks with `vector_id = NULL`

Degraded mode is a safety net, not a silent failure. The system logs a structured error at `error` level with the reason and recovery path.

---

## Asymmetric Query Encoding

Query vectors and document vectors use different encoding strategies. The model supports instruction-following prefixes:

- **Passage side** (indexed chunks): plain text (no prefix)
- **Query side** (search queries): prefixed with `"Represent this sentence for searching relevant passages: "` before embedding

This asymmetry improves retrieval quality by aligning the query vector space closer to the passage vector space.

---

## Performance

| Metric | Value |
|--------|-------|
| Throughput (CPU, batch=80) | > 800 chunks/sec |
| Single chunk latency | ~1.2ms |
| Memory during inference | ~2GB RSS |
| Model load time (cold) | ~3s |
| Model load time (warm, mmap) | ~500ms |

---

## See Also

- [Chunking](./chunking.md) — upstream chunk production
- [Vector Index](./vector-index.md) — downstream vector storage
- [Hybrid Search](./hybrid-search.md) — how embeddings are used in retrieval
