# Subsystem 1: Chunking Engine

**Status**: Functional but naive. Missing critical context propagation techniques.
**Priority**: High -- chunk quality is the ceiling of retrieval quality.

---

## Current Implementation Audit

### What Exists

`crates/omni-core/src/chunker/mod.rs` (880 lines)

- Tree-sitter AST-level boundary detection -- correct, non-trivial
- Backward context: grabs N lines before each element -- good
- Module declarations injected into chunk header -- good
- Split strategy per kind: Class splits at method boundaries, Function at statement boundaries
- 10-15% overlap fraction on splits
- Token budget: character-count estimate (`len / 4`) -- **inaccurate**

### Critical Flaws

**Flaw 1: Token estimation is wrong**

```rust
pub fn estimate_tokens(content: &str) -> u32 {
    let estimate = (content.len() / 4) as u32;  // <-- character count / 4
    estimate.max(1)
}
```

The `jina-embeddings-v2-base-code` tokenizer (WordPiece) produces approximately 1 token per 3.5 chars for English but 1 token per 1.5-2 chars for code identifiers with underscores and camelCase. The current estimate **undershoots by 40-60% on dense code**. This causes chunks to exceed the model's 8192 token limit silently, and the model truncates them -- you lose the tail of every large chunk. This directly contributes to degraded embedding quality.

**Flaw 2: No hierarchical chunking**

Every chunk is a leaf. There is no summary node. When a user asks "explain the overall architecture of the payment module", no single chunk answers this -- it requires aggregating across 40 function-level chunks. Without a summarized parent chunk, the retrieval misses the question entirely.

**Flaw 3: Module declarations are naive line-prefix matching**

```rust
let is_declaration = trimmed.starts_with("import ")
    || trimmed.starts_with("use ")
    || ...
```

This breaks on multi-line imports, conditional imports, re-exports, and any import with a comment before it. A tree-sitter query for import nodes would be exact where this is approximate.

**Flaw 4: No cross-file context injection**

When function `foo()` calls `bar()` from another module, the chunk for `foo` has no knowledge of what `bar()` does. The dependency graph exists (`graph/`) but is not used to enrich chunks at index time.

**Flaw 5: Overlap is line-count based, not semantic**

The overlap grabs the N preceding lines. These lines may be a closing brace, a comment, or whitespace -- zero semantic value for the next chunk.

---

## State-of-the-Art Research

### Technique 1: RAPTOR -- Recursive Abstractive Processing for Tree-Organized Retrieval

**Paper**: Sarthi et al., 2024 (arXiv:2401.18059)

**Core idea**: Build a tree of chunks. Leaf nodes = raw code chunks. Parent nodes = LLM-generated summaries of their children. At query time, retrieve from any level of the tree depending on query scope.

```
File: payment_service.py
├── [Summary] "PaymentService handles Stripe webhooks, validates HMAC,
│             routes to charge/refund handlers, logs all events"
├── class PaymentService
│   ├── [Summary] "PaymentService class with 4 methods: charge, refund,
│   │             validate_webhook, _log_event"
│   ├── def charge(...)
│   ├── def refund(...)
│   ├── def validate_webhook(...)
│   └── def _log_event(...)
```

**For OmniContext**: Generate two summary levels:

1. Method-group summaries (per class or per file section)
2. File summaries (one per file, generated from method summaries)

No LLM needed -- use the embedding model itself with a summarization prompt, or use a small local summarizer like `facebook/bart-base` (ONNX available, 140MB).

**Implementation in Rust**:

- After chunking, group leaf chunks by file
- Call a summarizer ONNX model on batched leaf content
- Store summary chunks with `kind = Summary`, higher weight
- At query time, retrieve from both leaf and summary levels

**Expected gain**: Architectural queries ("how does X work?") go from 0% recall to 60-80% because the summary chunk is now retrievable.

---

### Technique 2: Late Chunking (Jina AI, 2024)

**Paper**: arXiv:2409.04701

**Core idea**: Instead of chunking BEFORE embedding, embed the entire document at the token level first (using the model's full context window), then split the resulting contextually-rich token embeddings into chunk-level vectors.

```
Traditional:                    Late Chunking:
[chunk1] → embed → vec1         [full doc] → embed → [tok1, tok2, ..., tokN]
[chunk2] → embed → vec2                              ↓ pool by chunk boundary
[chunk3] → embed → vec3         [chunk1_tokens] → mean pool → vec1 (context-aware!)
                                [chunk2_tokens] → mean pool → vec2 (context-aware!)
```

**Benefit**: Each chunk vector encodes the full document context, not just its local window. A function that does `return self._cache.get(key)` has a vector that encodes what `_cache` is because the full class was seen.

**Limitation**: Requires a model that produces token-level outputs (not just a pooled sentence vector). `jina-embeddings-v2-base-code` outputs token-level embeddings in its ONNX form -- this is implementable today.

**Cost**: Must process the entire file per update (not per chunk). For incremental indexing this means re-embedding the whole file on any change -- acceptable for files under 8192 tokens, not for megafiles.

**Recommendation**: Use late chunking for files < 2000 tokens, traditional chunking with overlap for larger files.

---

### Technique 3: Contextual Retrieval (Anthropic, 2024)

**Core idea**: Before embedding each chunk, prepend a context sentence that describes where this chunk fits in the document:

```
Original chunk: "def validate_hmac(payload, signature): ..."

Contextual chunk: "This function is part of the PaymentService webhook
                   validation pipeline. It is called by process_webhook()
                   after the request arrives.
                   def validate_hmac(payload, signature): ..."
```

The context sentence is generated by a small LLM call (or structured by code metadata -- no LLM needed for code). For OmniContext, this is derivable from the dependency graph: "This function is called by X, Y, Z and calls A, B, C."

**Implementation**: Modify `build_context_header()` to include:

- Callers from the dependency graph (upstream dependencies)
- Callees (what this function calls)
- Class hierarchy if applicable

This is a **zero-cost** improvement that uses data already present in the graph layer.

---

### Technique 4: Accurate Tokenization for Budget Management

Replace character-count estimation with actual tokenizer calls. The HuggingFace `tokenizers` crate is already a dependency:

```rust
// Current (wrong):
let estimate = (content.len() / 4) as u32;

// Correct:
let encoding = tokenizer.encode(content, false)?;
let token_count = encoding.get_ids().len() as u32;
```

Cost: ~0.5ms per chunk during indexing (tokenization is CPU-fast). This ensures chunks never silently exceed model limits.

---

## Implementation Plan

### Phase A (1-2 weeks): Fix critical bugs, zero new techniques

1. Replace `estimate_tokens()` with actual tokenizer
2. Fix module declaration extraction to use tree-sitter import queries
3. Add caller/callee context to chunk headers from dep graph
4. Test: embedding coverage should rise from 15% to 85%+

### Phase B (3-4 weeks): Contextual Retrieval

1. Add dependency graph enrichment to `build_context_header()`
2. For each chunk: inject top-3 callers and top-3 callees as text
3. Add file-level context sentence to every chunk header

### Phase C (6-8 weeks): RAPTOR Hierarchical Indexing

1. Add `ChunkKind::Summary` variant
2. After indexing, generate method-group summaries by batching leaf chunks through the embedding model with a summary prompt
3. Generate file-level summaries from group summaries
4. Store in the same chunk table with `is_summary = true` flag
5. Query time: retrieve from both leaf and summary layers, merge with weighted RRF

---

## Flows with Problems

```
Current Flow:
Parser → StructuralElement[] → chunk_elements() → Chunk[]
                                      ↑
                               BROKEN: wrong token count → chunks exceed model limit silently

Correct Flow:
Parser → StructuralElement[] → chunk_elements(tokenizer) → Chunk[]
                                      ↓ Phase B
                              + dep_graph.callers/callees → enriched header
                                      ↓ Phase C
                              + summarizer → Summary chunks
                              merged into same index
```

---

## Who Can Build This

- **Phase A**: Any Rust developer familiar with the codebase (3-5 days)
- **Phase B**: Requires understanding of the graph module (1-2 weeks)
- **Phase C**: Requires understanding of the ONNX inference pipeline and a summarizer model integration (3-4 weeks, senior level)

Total time to maximum chunk quality: **6-8 development weeks**
