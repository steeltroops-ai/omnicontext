# OmniContext Intelligence Architecture: Research Summary

**Generated**: March 2026  
**Scope**: Critical analysis of all 6 deployed subsystems. Research-backed upgrade paths.

---

## Architecture Map

```
User / AI Agent Query
        |
        v
[6] Query Engine     (intent classification, expansion, HyDE)
        |
        v (expanded query)
    ┌───┴────────────────────────────────────────┐
    |                                            |
    v                                            v
[4] BM25 FTS5                         [2] Embedding Engine
 keyword search                        (jina-v2-base-code ONNX)
    |                                            |
    v                                            v
keyword_results[]                      [3] Vector Index (flat → HNSW)
                                        semantic_results[]
    |                                            |
    └──────────────┬─────────────────────────────┘
                   |  + symbol_results[]
                   v
           [4] RRF Fusion
           (adaptive weights by intent)
                   |
                   v
           [5] Cross-Encoder Reranker
           (ms-marco-MiniLM or jina-reranker)
                   |
                   v
           Context Assembly
           (token budget, graph neighbors, dedup)
                   |
                   v
           [SearchResult[]]  → MCP / REST / VS Code
                   ^
                   |
              [1] Chunks in SQLite
         (tree-sitter AST + RAPTOR summaries)
```

---

## Critical Issues Matrix

| #      | Subsystem            | Issue                                                           | Severity     | Impact                             | Fix Effort   |
| ------ | -------------------- | --------------------------------------------------------------- | ------------ | ---------------------------------- | ------------ |
| **P0** | **Embedding Engine** | **15% coverage -- 84% of codebase unembedded**                  | **CRITICAL** | **No semantic search working**     | **1-3 days** |
| **P1** | **Reranker**         | **Using bi-encoder as cross-encoder -- scores are meaningless** | **HIGH**     | **Reranker hurts results**         | **3-5 days** |
| **P2** | Chunking             | Token estimation wrong (char/4 vs actual tokenizer)             | HIGH         | Chunks exceed model limit silently | 2 days       |
| **P3** | Query Engine         | No semantic expansion, no intent-based routing                  | MEDIUM       | NL query quality degraded          | 1-2 weeks    |
| **P4** | Hybrid Search        | Uniform RRF weights, no SPLADE signal                           | MEDIUM       | Suboptimal fusion                  | 2-4 weeks    |
| **P5** | Vector Index         | Flat O(n) scan scalability ceiling                              | LOW          | Not a problem yet (<10K chunks)    | 3-4 weeks    |

---

## Prioritized Fix Order

### Week 1: Emergency -- Stop the Bleeding

**Fix 1: Diagnose and fix embedding coverage (Subsystem 2)**

Root cause is almost certainly a corrupt/truncated model file or missing ONNX Runtime DLL.

```powershell
# Diagnose
$env:RUST_LOG="omni_core::embedder=debug,ort=debug"
omnicontext index --path . 2>&1 | Select-String "embed|onnx|degraded|load"

# If model corrupt: delete and re-download
Remove-Item -Recurse "$HOME\.omnicontext\models\jina-embeddings-v2-base-code"
omnicontext index --path .  # triggers fresh download

# Check coverage after
omnicontext stats  # should show >90% embedding coverage
```

**Fix 2: Disable the broken reranker (Subsystem 5)**

Until the cross-encoder model is integrated, disable the reranker entirely. The bi-encoder reranker is actively corrupting scores.

```powershell
# Set env var to disable reranker
$env:OMNI_DISABLE_RERANKER="1"
```

Or in config: `reranker.enabled = false`.

---

### Week 2-3: Architecture Correctness

**Fix 3: Real cross-encoder reranker (Subsystem 5)**

1. Add `RERANKER_MODEL` ModelSpec pointing to `cross-encoder/ms-marco-MiniLM-L-6-v2` (~90MB)
2. Fix `Reranker::new()` to load the reranker model independently
3. Fix `run_inference()` to extract the `[batch, 1]` logit with sigmoid activation
4. Verify reranker improves MRR on 20 test queries

**Fix 4: Accurate token counting in chunker (Subsystem 1)**

Replace `content.len() / 4` with actual tokenizer call. Requires passing a `&Tokenizer` into `chunk_elements()`. This ensures chunks never silently exceed model limits.

---

### Week 4-6: Quality Improvements

**Fix 5: Code vocabulary synonym expansion (Subsystem 6)**

Add 100-entry code synonym map. BM25 query expansion with synonyms. Expect 10-20% precision improvement on cross-domain NL queries.

**Fix 6: Query-type-adaptive RRF weights (Subsystem 4)**

Three weight sets (symbol/keyword/NL). Changes 4 constants, measurable improvement.

**Fix 7: Contextualize chunks with dep graph (Subsystem 1)**

Inject callers/callees from dep graph into chunk headers. Zero additional model cost, richer embedding signal.

---

### Month 2-3: Intelligence Upgrades

**Fix 8: RAPTOR hierarchical chunking (Subsystem 1)**

Add summary chunks per class/file. Architectural queries start working.

**Fix 9: SPLADE sparse retrieval signal (Subsystem 4)**

Add `splade-v3` ONNX as 4th retrieval signal. Expected 15-20% improvement on vocabulary-gap queries.

**Fix 10: Template-based HyDE for NL queries (Subsystem 6)**

Generate hypothetical code snippets for NL queries, embed and merge.

---

### Month 4+: Scale & Advanced

**Fix 11: HNSW vector index (Subsystem 3)**

Replace flat scan with `usearch` HNSW when chunk count consistently exceeds 20K.

**Fix 12: Learned fusion weights (Subsystem 4)**

Collect implicit feedback, train MLP for signal fusion.

---

## What Makes This a World-Class Context Engine

After all fixes, OmniContext's pipeline would be:

| Layer        | Technology                                                      | Status         |
| ------------ | --------------------------------------------------------------- | -------------- |
| Chunking     | Tree-sitter AST + RAPTOR summaries + dep-graph enriched headers | Research-grade |
| Embedding    | jina-v2-base-code ONNX + instruction-following query prefix     | Production     |
| Sparse       | SPLADE-v3 ONNX                                                  | Research-grade |
| Vector Index | HNSW (usearch) + INT8 quantization                              | Production     |
| Fusion       | Adaptive RRF with query-type weights                            | Production     |
| Reranker     | ms-marco cross-encoder + jina-reranker-v2                       | Production     |
| Query        | 9-type intent classifier + synonym expansion + template HyDE    | Research-grade |

**No AI company in 2026 runs all of these for code search locally in a single binary.**

The comparable architectures are:

- **Sourcegraph**: Cloud-based, similar signal set, no local option
- **GitHub Copilot Workspace**: Proprietary, cloud-only, no graph-aware reranking
- **Cursor**: No disclosed architecture, empirically weaker on large codebases

Running this in a Rust binary with ONNX inference -- sub-100ms queries, zero API calls, zero data leak -- is the technical differentiator that no cloud-first competitor can replicate.

---

## Research Papers Reference List

| Technique               | Paper                | Year | Where to Apply                     |
| ----------------------- | -------------------- | ---- | ---------------------------------- |
| RAPTOR hierarchical RAG | arXiv:2401.18059     | 2024 | Subsystem 1 (Chunking)             |
| Late Chunking           | arXiv:2409.04701     | 2024 | Subsystem 1 (Chunking)             |
| Contextual Retrieval    | Anthropic blog       | 2024 | Subsystem 1 (Chunking)             |
| HyDE                    | arXiv:2212.10496     | 2022 | Subsystem 6 (Query)                |
| SPLADE v3               | arXiv:2403.06789     | 2024 | Subsystem 4 (Search)               |
| BGE-M3                  | arXiv:2402.03216     | 2024 | Subsystem 2 + 4                    |
| ColBERT v2              | arXiv:2112.01488     | 2022 | Subsystem 4 (Search)               |
| SPLATE (ColBERT+SPLADE) | arXiv:2405.17609     | 2024 | Subsystem 4 (Search)               |
| DiskANN                 | NEURIPS 2019         | 2019 | Subsystem 3 (Vector Index, future) |
| HNSW                    | arXiv:1603.09320     | 2018 | Subsystem 3 (Vector Index)         |
| RRF                     | Cormack et al. SIGIR | 2009 | Subsystem 4 (in use)               |
| MS MARCO cross-encoder  | Microsoft            | 2021 | Subsystem 5 (Reranker)             |
