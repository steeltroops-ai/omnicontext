# Hybrid Search Engine

**Location**: `crates/omni-core/src/search/mod.rs`

---

## Overview

The hybrid search engine combines three distinct retrieval signals — FTS5 keyword search, HNSW vector search, and exact symbol lookup — using Reciprocal Rank Fusion (RRF) with query-type adaptive weights. The result set is further enriched by structural boosting via the dependency graph.

---

## Retrieval Signals

### 1. FTS5 Keyword Search

SQLite FTS5 provides BM25-ranked full-text search over chunk content. The query is first expanded via `expand_query()` (identifier splitting + synonym expansion) before submission to FTS5.

### 2. HNSW Semantic Search

The query is embedded via the same jina-embeddings-v2-base-code model used for indexing (with the query-side asymmetric prefix). The resulting 768-dimensional vector is used for approximate nearest neighbor search in the HNSW index.

For natural language queries, HyDE (Hypothetical Document Embeddings) is applied before vector search: a template-based hypothetical code snippet is generated from the query, embedded, and used as the search vector. This reduces the semantic gap between natural language questions and code-style embeddings.

### 3. Symbol Lookup

When the query matches the symbol pattern (see [Query Engine](./query-engine.md)), an exact lookup against the symbol metadata table returns direct matches. Symbol results carry a boosted RRF weight reflecting the precision of exact name matching.

---

## RRF Fusion

Results from all three signals are merged using Reciprocal Rank Fusion:

```
score(r) = w_keyword * 1/(k + rank_keyword(r))
         + w_semantic * 1/(k + rank_semantic(r))
         + w_symbol   * 1/(k + rank_symbol(r))
```

Where `k = 60` (default, configurable via `config.search.rrf_k`).

### Query-Type Adaptive Weights

Signal weights are determined by the classified query type:

| Query Type | Keyword Weight | Semantic Weight | Symbol Weight |
|------------|---------------|-----------------|---------------|
| Symbol | 0.2 | 0.3 | 0.5 |
| Keyword | 0.4 | 0.4 | 0.2 |
| Natural Language | 0.1 | 0.7 | 0.2 |
| Mixed | 0.3 | 0.4 | 0.3 |

Adaptive weights are selected after `analyze_query()` classifies the query type.

---

## Structural Boosting Pipeline

After RRF fusion, the result set passes through a structural boosting pipeline that adjusts scores based on code structure signals:

| Signal | Mechanism | Weight |
|--------|-----------|--------|
| In-degree | Nodes with more dependents are more architecturally central | Additive |
| PageRank | Global importance score from the dependency graph | Multiplicative |
| Freshness | Recently modified files receive a small boost for debugging queries | Conditional |
| Branch-changed | Files modified on the current git branch receive a boost | Conditional |

Freshness and branch-changed boosts are only applied for query intents that benefit from recency (e.g., `DebugQuery`, `UsageSearch`).

---

## Graph-Augmented Retrieval (GAR)

For queries with intent types that require dependency context (`DependencyQuery`, `DataFlowQuery`), the search engine augments the initial result set by traversing the dependency graph from the top-ranked anchors:

1. Take the top-3 chunks from the initial RRF result as graph anchors
2. Execute N-hop traversal (depth determined by intent type, typically 1–2 hops)
3. Fetch chunks for neighboring nodes not already in the result set
4. Merge into the candidate pool with a graph-proximity RRF score

This ensures that when a user asks about a function, the functions it calls and the functions that call it are also surfaced.

---

## Deduplication

Overlapping line ranges within the same file are deduplicated before results are returned. When two chunks from the same file have overlapping line ranges (due to the 10–15% overlap strategy in the chunker), the higher-scored chunk is retained and the lower-scored duplicate is dropped.

---

## Query Caching

An LRU cache with 100 entries caches embedded query vectors. Repeated queries with identical text bypass the embedding step entirely, reducing per-query latency from ~5ms to < 1ms for cache hits.

---

## Search Flow

```
Query
  → analyze_query() → QueryType + QueryIntent
  → expand_query() → expanded tokens + synonym expansion
  → parallel retrieval:
      keyword_search(expanded_tokens)  [FTS5 BM25]
      semantic_search(embed(query))    [HNSW ANN, with HyDE for NL]
      symbol_search(raw_query)         [exact metadata lookup]
  → adaptive_rrf_fusion(weights[query_type])
  → structural_boost(in_degree, pagerank, freshness, branch_changed)
  → graph_augment(anchors, intent_depth)  [for graph-aware intents]
  → dedup_overlapping_ranges()
  → rerank_with_priority(top-20)       [cross-encoder]
  → context_assemble(token_budget)
  → [SearchResult]
```

---

## Performance

| Stage | Latency |
|-------|---------|
| Query classification + expansion | < 1ms |
| FTS5 keyword search | < 5ms |
| HNSW vector search (50K chunks) | < 3ms |
| Symbol lookup | < 1ms |
| RRF fusion + boosting | < 1ms |
| Cross-encoder rerank (top-20) | 5–15ms |
| **Total P99** | **< 50ms** |

---

## See Also

- [Query Engine](./query-engine.md) — query classification and expansion
- [Reranker](./reranker.md) — cross-encoder reranking
- [Vector Index](./vector-index.md) — HNSW semantic signal
- [Chunking](./chunking.md) — chunk metadata used in structural boosting
