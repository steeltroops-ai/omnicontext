# Subsystem 4: Hybrid Search Engine

**Status**: Architecturally sound but suboptimal fusion. Missing learned sparse signals.
**Priority**: Medium-High. High impact after embedding coverage is fixed.

---

## Current Implementation Audit

### What Exists

`crates/omni-core/src/search/mod.rs` (1106 lines)

**Three retrieval signals:**

1. BM25 full-text search via SQLite FTS5
2. Dense vector search via flat VectorIndex
3. Exact symbol name lookup from metadata index

**Fusion:** Reciprocal Rank Fusion (RRF, Cormack et al. 2009)

```rust
let rank_score = 1.0 / (f64::from(self.rrf_k) + (rank as f64) + 1.0);
```

**Post-fusion boosting:**

- Structural weight (class > function > test > snippet)
- Graph proximity boost (chunks close to anchor in dep graph)
- Visibility weight (public > protected > private)

**Query analysis:** Heuristic regex-based classification (Symbol / Keyword / NL / Mixed)

### What Works Well

- RRF as a fusion strategy is principled and has good theoretical backing
- Graph proximity boost is novel and adds real signal for related code discovery
- Deduplication of overlapping line ranges prevents redundant results
- LRU cache on query embeddings (100 entries) prevents redundant embedding calls

### Critical Flaws

**Flaw 1: Symbol signal weight is arbitrary**

```rust
// Symbol matches get a higher weight than positional RRF
let rank_score = 1.5 / (self.rrf_k + rank + 1);  // 1.5x multiplier
```

The 1.5x symbol boost has no empirical basis. It was likely chosen intuitively. On code queries where the exact function name is present (e.g., "authenticate_user"), this boost causes it to dominate over semantically richer BM25 results for related code. A learned weight would be better.

**Flaw 2: Query expansion is stop-word removal only**

```rust
fn expand_query(query: &str) -> String {
    // strips stop words, splits identifiers by case/underscore
}
```

This is a 1990s-era technique. It has zero semantic expansion. A query for "how does caching work" produces keywords `["caching", "work"]` -- missing synonyms like `cache`, `memoize`, `store`, `persist`, `TTL`.

**Flaw 3: BM25 via SQLite FTS5 has a fundamental limitation**

FTS5 uses BM25 but does not support IDF (Inverse Document Frequency) correction across the full corpus in the same way Elasticsearch does. For very common code tokens like `let`, `fn`, `return`, FTS5 cannot correctly downweight them. This produces false positives.

**Flaw 4: No learned sparse retrieval**

The current system has dense vectors (jina) and lexical BM25. There is a third class: **learned sparse vectors** (SPLADE) that combine the best of both -- they produce sparse term vectors that are semantically expanded but lexically interpretable. This is the missing signal.

**Flaw 5: RRF k=60 is not tuned**

The RRF constant k=60 is the default from the original paper (tuned on TREC document retrieval). For code search, the optimal k may be different. A lower k gives more weight to top-ranked results, higher k flattens the distribution. No tuning has been done.

---

## State-of-the-Art Research

### Technique 1: SPLADE -- Learned Sparse Retrieval (2021, ongoing)

**Paper**: Formal et al., "SPLADE: Sparse Lexical and Expansion Model" (SIGIR 2021), v3 (2024)

**Core idea**: Train a BERT-based model to produce a sparse term weight vector over the full vocabulary. Unlike BM25 which uses exact term matching, SPLADE learns which vocabulary terms each document "expands" to:

```
Code: "def validate_token(jwt_string):..."
BM25 terms:   {validate, token, jwt, string}
SPLADE terms: {validate, token, jwt, authentication, authorization,
               bearer, secret, signature, verify, expire, ...}
              (all with learned weights, most are 0)
```

SPLADE bridges the vocabulary gap between a user's natural language query and the exact code terms.

**Why it matters for code search**: Developers write code in technical vocabulary. Users query in natural language. SPLADE learns the mapping between them.

**ONNX availability**: `naver/splade-v3` has an official ONNX export on HuggingFace. File size: ~160MB. Integrates with the existing `ort` pipeline.

```
OMNI_SPLADE_MODEL_URL =
  "https://huggingface.co/naver/splade-v3-onnx/resolve/main/model.onnx"
```

**Storage**: SPLADE vectors are sparse -- typically 200-500 non-zero terms out of 30,000+ vocabulary. Store as `HashMap<u32, f32>` (term_id -> weight). Inverted index lookup is O(1) per term.

**Integration with existing RRF**: Add SPLADE as a 4th signal alongside BM25, dense vector, and symbol:

```
Final score = RRF(keyword_rank, semantic_rank, splade_rank, symbol_rank)
```

### Technique 2: BGE-M3 Triple-Representation

**Model**: BAAI/bge-m3

BGE-M3 produces three representations in a single forward pass:

1. Dense vector (768 dim) for ANN search
2. Sparse lexical vector (SPLADE-style) for inverted index
3. ColBERT multi-vector for late interaction

Using BGE-M3 ONNX would replace both the current embedding model AND add SPLADE, ColBERT in one. But it would require replacing the current index architecture significantly.

**Recommendation**: Not for immediate implementation. Consider as the target architecture for v2.0 of the search engine after SPLADE is proven out separately.

### Technique 3: Learned Fusion (Replace RRF with a Learned Model)

RRF is a heuristic. A small MLP trained on click data or relevance feedback can learn optimal signal weights:

```
Input:  [keyword_score, semantic_score, symbol_score, splade_score,
         chunk_kind, visibility, dep_graph_degree, recency]
Output: [final_relevance_score]
```

**Challenge**: Requires labeled data (user feedback on which results were useful). OmniContext currently has no feedback loop.

**Practical path**: Log which results users click/use in the VS Code sidebar and MCP tool calls. Use this implicit feedback to train the fusion model every N weeks.

**Timeline**: This is Phase 3+ work. Requires a feedback collection infrastructure first.

### Technique 4: Query-Adaptive RRF Weights

Instead of uniform RRF, adapt the signal weights based on query type:

| Query Type                                     | Keyword Weight | Semantic Weight | Symbol Weight |
| ---------------------------------------------- | -------------- | --------------- | ------------- |
| Symbol lookup (e.g., "authenticate_user")      | 0.2            | 0.3             | 0.5           |
| Keyword (e.g., "jwt validation")               | 0.4            | 0.4             | 0.2           |
| Natural language (e.g., "how does auth work?") | 0.1            | 0.7             | 0.2           |

The query type is already classified in `analyze_query()`. Applying different weights per type requires only a config table change, no new model.

---

## Implementation Plan

### Phase A (1-2 weeks): Tune what exists

1. Replace uniform RRF weights with query-type-adaptive weights
2. Add query synonym expansion using a small hand-coded code vocabulary:
   ```
   "cache" → ["cache", "memoize", "store", "lru", "redis", "persist"]
   "error" → ["error", "exception", "err", "panic", "unwrap", "result"]
   "auth"  → ["auth", "authentication", "authorize", "jwt", "token", "session"]
   ```
3. Tune `rrf_k` via offline evaluation on 50 test queries

### Phase B (3-4 weeks): Add SPLADE as 4th signal

1. Add `splade-v3` ONNX model to `model_manager.rs` as optional secondary model
2. Implement sparse vector type: `Vec<(u32, f32)>` stored in SQLite `chunk_sparse_vectors` table
3. Implement sparse vector search: inverted index lookup with TF-IDF-style scoring
4. Add SPLADE rank to RRF fusion

### Phase C (6-8 weeks): Feedback loop for learned fusion

1. Add implicit feedback logging to VS Code sidebar (which result did user click?)
2. Add feedback logging to MCP tool responses (which chunks did the LLM reference?)
3. Build nightly training job that fine-tunes fusion weights
4. Ship as A/B test

---

## Flows with Problems

```
Current Search Flow:
query → analyze_query() [heuristic regex]
     → keyword_search(BM25) → [(chunk_id, score)]
     → embed_single(query) → semantic_search() → [(chunk_id, score)]
     → symbol_search(query) → [chunk_ids]
     → fuse_results(RRF, k=60, uniform weights)
     → structural_boost()
     → graph_proximity_boost()
     → dedup_overlap()
     → [SearchResult]

Flaws:
- No SPLADE signal (missing vocabulary bridging)
- Uniform RRF weights regardless of query type
- Keyword expansion is stop-word removal only

Target Flow:
query → query_classifier() [fine-grained, 6 types]
     → query_expander() [synonym + SPLADE expansion]
     → parallel:
         [BM25] keyword_search(expanded)
         [Dense] semantic_search(embedded)
         [Sparse] splade_search(splade_vec)
         [Symbol] symbol_search(exact)
     → adaptive_rrf_fusion(weights[query_type])
     → cross_encoder_rerank(top-20)
     → context_assemble()
     → [SearchResult]
```
