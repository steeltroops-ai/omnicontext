# Subsystem 6: Query Understanding Engine

**Status**: Heuristic regex classification. Functional but brittle and missing semantic expansion.
**Priority**: Medium. Upgrade after embedding coverage (Subsystem 2) and reranker bug (Subsystem 5) are fixed.

---

## Current Implementation Audit

### What Exists

`crates/omni-core/src/search/mod.rs` -- `analyze_query()` + `expand_query()`  
`crates/omni-core/src/search/intent.rs` -- `QueryIntent` + `ContextStrategy` types

**Query Classification (analyze_query)**:

```rust
fn analyze_query(query: &str) -> QueryType {
    // Symbol-like: contains :: or . or is camelCase without spaces
    if !trimmed.contains(' ') {
        if trimmed.contains("::") || trimmed.contains('.') { return Symbol; }
    }
    // NL: starts with question words or ends with ?
    if lower.starts_with("how ") || lower.ends_with('?') { return NaturalLanguage; }
    // Short = Mixed, Long = NL
    if words.len() <= 3 { return Mixed; }
    NaturalLanguage
}
```

**Query Expansion (expand_query)**:

```rust
fn expand_query(query: &str) -> String {
    // 1. Split identifiers: "getUserById" -> "get user by id"
    // 2. Split snake_case: "get_user_by_id" -> "get user by id"
    // 3. Remove stop words
    // 4. Join remaining tokens
}
```

**Intent types** (in `intent.rs`):

- `QueryIntent` -- more nuanced 6-type classification used in the context assembler
- `ContextStrategy` -- defines how to assemble context for each intent type

### What Works

- Identifier splitting (`getUserById` → `get user by id`) is correct and non-trivial
- The intent-based context strategy selection in `intent.rs` is a good design pattern
- LRU cache on embeddings prevents repeated embedding of the same query

### Critical Flaws

**Flaw 1: Query classification has no semantic understanding**

The regex classifier can't handle:

- "where is the webhook handler" → classified as NL but is actually a Symbol query (should prioritize symbol search)
- "PaymentService webhook" → classified as Mixed but semantically is a class-member search
- "list all functions that touch the database" → NL but requires a structural query, not semantic similarity

**Flaw 2: Keyword expansion is purely syntactic**

No semantic expansion. "caching" doesn't expand to include "memoize", "lru", "redis", "persist". The only expansion is identifier splitting.

**Flaw 3: No query rewriting / normalization**

Queries like "whats the difference between authenticate and authorize" are long natural language questions. Passing them verbatim to the FTS5 engine returns noise. The optimal strategy is to extract the key entities ("authenticate", "authorize") and issue a more targeted query.

**Flaw 4: No HyDE (Hypothetical Document Embedding)**

For NL queries, the vector mismatch between "how does caching work?" (question) and "fn cache_get(key: &str) -> Option<Value>" (code) is large. The embedding spaces are trained on code-to-code and text-to-code pairs -- but question-to-code is the hardest case.

---

## State-of-the-Art Research

### Technique 1: HyDE -- Hypothetical Document Embeddings (Gao et al., 2022)

**Paper**: arXiv:2212.10496. Cited in: arXiv:2409.04701 (late chunking), arXiv:2410.05684 (knowledge-aware expansion)

**Core idea**: For NL queries, generate a hypothetical code snippet that would answer the query, then embed the hypothetical snippet (not the original query) for vector search.

```
User query: "how does the payment retry logic work?"

HyDE generates:
  "// Hypothetical answer:
   fn retry_payment(tx_id: u64, attempts: u32) -> Result<Payment, Error> {
       for i in 0..attempts {
           match process_payment(tx_id) {
               Ok(p) => return Ok(p),
               Err(e) if i < attempts - 1 => continue,
               Err(e) => return Err(e),
           }
       }
   }"

Embed the hypothetical → vector search → real code matches
```

**Why it works**: The hypothetical snippet lives in the same code embedding space as the indexed chunks. The semantic match between two code snippets is stronger than between a natural language question and code.

**Without an LLM (critical for local operation)**: Use a template-based approach for common query patterns:

```rust
fn generate_hypothetical(query: &str, intent: &QueryIntent) -> String {
    match intent {
        QueryIntent::HowDoesXWork { entity } =>
            format!("fn {}(...) -> ... {{\n    // Implementation\n}}", entity),
        QueryIntent::FindImplementation { feature } =>
            format!("// {} implementation\nfn handle_{}(...) -> ...", feature, feature),
        _ => query.to_string() // fallback to original
    }
}
```

This is a 5-10% quality improvement without any LLM dependency.

**With a small local LLM** (opt-in, advanced mode): Use a quantized `codellama-3b` or `phi-3.5-mini` (ONNX available) to generate genuine hypothetical snippets. Enable with `OMNI_HyDE=1`.

### Technique 2: Code-Aware Intent Classification

Replace the heuristic classifier with a small fine-tuned classification model. Alternatively, use rule-based patterns that understand code semantics:

```rust
pub enum QueryIntent {
    // Current types in intent.rs
    SymbolLookup,      // exact name lookup
    DefinitionSearch,  // "where is X defined"
    UsageSearch,       // "find all places that use X"
    ArchitectureQuery, // "how does X work"
    DebugQuery,        // "why is X failing"
    ComparisonQuery,   // "difference between X and Y"
    // New types:
    DataFlowQuery,     // "how does data flow from X to Y"
    DependencyQuery,   // "what does X depend on"
    TestCoverageQuery, // "which tests cover X"
}
```

For each intent, a different search strategy is optimal:

- `SymbolLookup` → 90% symbol search, 10% BM25
- `ArchitectureQuery` → 80% semantic, 10% BM25, 10% graph
- `DataFlowQuery` → 70% graph traversal, 20% semantic, 10% BM25

### Technique 3: Code Vocabulary Synonym Expansion

A hand-curated code vocabulary map. This requires no model but can lift BM25 precision significantly:

```rust
static CODE_SYNONYMS: LazyLock<HashMap<&str, Vec<&str>>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("cache", vec!["lru", "memoize", "store", "redis", "memcached", "persist", "evict"]);
    m.insert("auth", vec!["authentication", "authorize", "jwt", "token", "session", "oauth", "bearer"]);
    m.insert("error", vec!["exception", "err", "panic", "unwrap", "result", "failure", "fault"]);
    m.insert("database", vec!["db", "sql", "postgres", "mysql", "sqlite", "orm", "query", "transaction"]);
    m.insert("api", vec!["endpoint", "route", "handler", "controller", "rest", "http"]);
    m.insert("test", vec!["spec", "assert", "mock", "stub", "fixture", "unit", "integration"]);
    m.insert("config", vec!["configuration", "settings", "env", "environment", "dotenv"]);
    m.insert("log", vec!["logging", "debug", "trace", "info", "warn", "error", "span"]);
    m.insert("retry", vec!["backoff", "exponential", "attempt", "repeat", "idempotent"]);
    m.insert("async", vec!["await", "future", "promise", "concurrent", "thread", "tokio", "spawn"]);
    // ~100 more entries covering common code domains
    m
});

fn expand_with_synonyms(tokens: &[&str]) -> Vec<String> {
    let mut expanded = tokens.iter().map(|t| t.to_string()).collect::<Vec<_>>();
    for token in tokens {
        if let Some(synonyms) = CODE_SYNONYMS.get(token) {
            expanded.extend(synonyms.iter().take(3).map(|s| s.to_string()));
        }
    }
    expanded.dedup();
    expanded
}
```

**Implementation cost**: 2-3 days. **Expected quality gain**: 10-20% improvement in BM25 precision for cross-domain queries.

### Technique 4: Multi-Query Retrieval

For complex NL queries, decompose into multiple simpler sub-queries and retrieve independently:

```
Query: "how does the payment service handle failed webhook retries?"

Decomposed:
  sub-query 1: "webhook handling"
  sub-query 2: "payment retry logic"
  sub-query 3: "failed payment"

Retrieve top-10 for each sub-query, merge with RRF
```

Implementation: Simple sentence splitting + entity extraction using `regex` patterns for code entities.

---

## Implementation Plan

### Phase A (1 week): Zero-model improvements

1. Add code vocabulary synonym map (100 entries, hand-curated)
2. Expand `analyze_query()` with better heuristics for code-specific patterns
3. Add query-type-specific RRF weights (see Subsystem 4 Phase A)
4. Add multi-query decomposition for queries with "and", "or", "difference between"

### Phase B (2-3 weeks): Template-based HyDE

1. Classify NL queries by pattern (ArchitectureQuery, DataFlowQuery, etc.)
2. Generate template-based hypothetical code snippets for each NL query type
3. Embed the hypothetical snippet, use as additional vector signal
4. Merge with existing signals via RRF

### Phase C (6-8 weeks): Full intent pipeline

1. Add `QueryIntent` resolution with the full 9-type taxonomy
2. Per-intent retrieval strategy (signal weights differ per intent type)
3. Graph-aware query planning: for DependencyQuery, route to graph traversal

---

## Flows with Problems

```
Current Query Flow:
raw query
  → analyze_query() [8-line heuristic]
  → expand_query() [stop word removal + identifier split]
  → keyword_search(expanded) + semantic_search(raw) + symbol_search(raw)

Problems:
- NL questions embedded directly -- large semantic gap vs code
- No synonym expansion -- "caching" misses "lru", "memoize"
- Same query format for all signals -- suboptimal for each

Target Query Flow:
raw query
  → classify_intent() [9 types, regex + learned]
  → expand_tokens() [synonyms + identifier split]
  → route_by_intent():
       SymbolLookup    → symbol_search(raw) + BM25(raw)
       ArchitectureQuery → HyDE(raw) → semantic(hypothetical) + BM25(keywords)
       DependencyQuery → graph.traverse(entity) + semantic(raw)
       default         → BM25(expanded) + semantic(raw) + splade(raw)
  → merge_signals(adaptive_rrf_weights[intent])
  → cross_encoder_rerank(top-20)
  → [SearchResult]
```
