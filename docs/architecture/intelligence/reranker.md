# Reranking Engine

**Location**: `crates/omni-core/src/reranker/mod.rs`

---

## Overview

The reranking engine refines the initial RRF-fused result set using a dedicated cross-encoder model. It runs after the three-signal retrieval stage (keyword, semantic, symbol) and before context assembly, consuming the top-K candidates and producing a relevance-ordered final ranking.

---

## Model

**`cross-encoder/ms-marco-MiniLM-L-6-v2`**

| Property | Value |
|----------|-------|
| Architecture | MiniLM-L6 (22M parameters) |
| Type | Cross-encoder (sequence-pair classifier) |
| Training data | MS MARCO passage ranking |
| Output | `[batch, 1]` relevance logit |
| ONNX size | ~90 MB |
| Inference latency | ~1ms per query-document pair on CPU |
| Runtime | `ort` crate (ONNX Runtime) |

A cross-encoder jointly processes the query and document with full cross-attention between all token pairs. This produces true relevance scores, not cosine similarity proxies. The model is distinct from the embedding model — it is loaded from a separate ONNX file and uses a separate ONNX session.

---

## Score Normalization

Raw logits from the cross-encoder are converted to relevance probabilities using sigmoid normalization:

```
score = 1 / (1 + exp(-logit))
```

This maps the output to `[0, 1]` where values above 0.5 indicate positive relevance. Platt calibration is applied over the sigmoid output to align the probability scale with observed precision on the internal evaluation set.

---

## RRF Blending

The final result score combines the upstream RRF fusion score and the cross-encoder score:

```
final_score = 0.3 * rrf_score + 0.7 * reranker_score
```

The 0.7 weight on the cross-encoder reflects its higher precision at true relevance judgments. The 0.3 RRF component preserves recall from the retrieval stage for cases where the cross-encoder underweights keyword-dominant results.

The RRF formula is `1/(k + rank)` where `k` is configurable (default 60, matching the original Cormack 2009 paper). Lower values increase the advantage of top-ranked candidates; higher values flatten the distribution.

---

## `rerank_with_priority()`

The primary entry point for the reranking pipeline is `rerank_with_priority()`, which:

1. Receives the top-K candidates from RRF fusion (default K=20)
2. Filters candidates below `min_threshold` score without running inference (early termination)
3. Batches remaining candidates for cross-encoder inference
4. Applies sigmoid + Platt calibration to raw logits
5. Blends with RRF scores
6. Returns candidates sorted by `final_score` descending, truncated to the requested result count

The `min_threshold` parameter (configurable, default 0.05) skips cross-encoder inference for candidates that are clearly irrelevant based on their RRF score alone. This reduces inference calls by 40–60% on typical query distributions.

---

## Graceful Degradation

If the reranker model is unavailable:

- The reranking step is skipped transparently
- RRF fusion scores are used directly for final ranking
- The degraded state is reported in the daemon status response
- No error is surfaced to the MCP caller — result quality degrades gracefully

---

## Performance

| Metric | Value |
|--------|-------|
| Latency per candidate pair | ~1ms |
| Latency for top-20 rerank | ~5–15ms |
| With early termination | ~2–8ms (40–60% skip rate) |
| Model load time | ~500ms (cold) |
| Model size | ~90MB |

---

## See Also

- [Hybrid Search](./hybrid-search.md) — upstream RRF fusion
- [Query Engine](./query-engine.md) — query classification that determines candidate set
- [Vector Index](./vector-index.md) — semantic retrieval signal
