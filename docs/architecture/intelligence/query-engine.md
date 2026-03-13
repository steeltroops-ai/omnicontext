# Query Understanding Engine

**Location**: `crates/omni-core/src/search/mod.rs` (`analyze_query`, `expand_query`), `crates/omni-core/src/search/intent.rs` (`QueryIntent`, `ContextStrategy`)

---

## Overview

The query understanding engine classifies incoming queries, expands tokens for better recall, and selects retrieval strategies appropriate for each intent type. It operates entirely without external models — all classification and expansion logic is local, deterministic, and sub-millisecond.

---

## Query Type Classification

`analyze_query()` returns one of four coarse query types used to select adaptive RRF weights:

| Type | Classification Criteria |
|------|------------------------|
| `Symbol` | No spaces, contains `::` or `.`, or is a camelCase/snake_case identifier without sentence structure |
| `Keyword` | 1–3 words, no question structure, all code-like tokens |
| `NaturalLanguage` | Starts with a question word, ends with `?`, or exceeds 4 words with sentence structure |
| `Mixed` | Short multi-word query that does not clearly fit the above |

---

## Intent Classification

`QueryIntent::classify()` maps queries to one of 9 fine-grained intent types. Intent determines the retrieval strategy (signal weights, graph depth, HyDE activation):

| Intent | Description | Primary Signal |
|--------|-------------|----------------|
| `SymbolLookup` | Exact name lookup: "validate_token" | Symbol (90%), BM25 (10%) |
| `DefinitionSearch` | "where is X defined" | Symbol + BM25 |
| `UsageSearch` | "find all places that use X" | BM25 + Graph |
| `ArchitectureQuery` | "how does X work" | Semantic + Graph |
| `DebugQuery` | "why is X failing" | Semantic + freshness boost |
| `ComparisonQuery` | "difference between X and Y" | Semantic + BM25 |
| `DataFlowQuery` | "how does data flow from X to Y" | Graph traversal + Semantic |
| `DependencyQuery` | "what does X depend on" | Graph traversal |
| `TestCoverageQuery` | "which tests cover X" | BM25 + Symbol |

Each intent also carries a `ContextStrategy` that controls how the context assembler builds the final LLM-ready output (e.g., how many hops of the dependency graph to include, whether to include test files, etc.).

---

## Query Expansion

`expand_query()` applies two transformation passes before the expanded tokens are submitted to keyword search:

### Pass 1: Identifier Splitting

Splits camelCase and snake_case identifiers into component tokens:

```
"getUserById"  → ["get", "user", "by", "id"]
"get_user_by_id" → ["get", "user", "by", "id"]
"PaymentService" → ["payment", "service"]
```

### Pass 2: Synonym Expansion

`synonyms::expand_with_synonyms()` consults a hand-curated code vocabulary map and injects up to 3 synonyms per recognized token:

```
"cache"    → ["cache", "lru", "memoize", "store"]
"auth"     → ["auth", "authentication", "authorize", "jwt"]
"error"    → ["error", "exception", "err", "panic"]
"database" → ["database", "db", "sql", "orm"]
"async"    → ["async", "await", "future", "tokio"]
```

The synonym map covers approximately 100 high-frequency code domain terms. Expanded tokens are passed to FTS5 as an OR query with the original tokens.

---

## HyDE — Hypothetical Document Embeddings

For `NaturalLanguage` and `ArchitectureQuery` intent types, the query engine generates a hypothetical code snippet before embedding the query for vector search.

The generation is template-based — no LLM is required:

```rust
fn generate_hypothetical(query: &str, intent: &QueryIntent) -> String {
    match intent {
        QueryIntent::ArchitectureQuery { entity } =>
            format!("fn {}(...) -> ... {{\n    // Implementation\n}}", entity),
        QueryIntent::DefinitionSearch { entity } =>
            format!("// {} definition\npub fn {}(...) -> ...", entity, entity),
        _ => query.to_string()
    }
}
```

The hypothetical snippet is embedded and used as the query vector for HNSW search. Because the snippet is in the same code embedding space as indexed chunks, the semantic match is significantly stronger than embedding a natural language question directly.

---

## Intent-Driven Graph Depth

The dependency graph traversal depth for GAR (Graph-Augmented Retrieval) is determined by the classified intent:

| Intent | Graph Depth |
|--------|-------------|
| `SymbolLookup` | 0 (no traversal) |
| `UsageSearch` | 1 hop downstream |
| `DependencyQuery` | 2 hops upstream |
| `DataFlowQuery` | 2 hops in both directions |
| `ArchitectureQuery` | 1 hop, all directions |
| Default | 1 hop |

---

## Query Cache

Embedded query vectors are cached in an LRU with 100 entries. Cache key is the normalized query string. Cache hits bypass the embedding call entirely.

---

## Performance

| Operation | Latency |
|-----------|---------|
| `analyze_query()` | < 0.1ms |
| `QueryIntent::classify()` | < 0.1ms |
| `expand_query()` + synonym lookup | < 0.5ms |
| HyDE template generation | < 0.1ms |
| Query embedding (cache miss) | ~1.5ms |
| Query embedding (cache hit) | < 0.01ms |

---

## See Also

- [Hybrid Search](./hybrid-search.md) — downstream signal fusion
- [Reranker](./reranker.md) — final result ordering
- [Chunking](./chunking.md) — how chunk metadata supports intent routing
