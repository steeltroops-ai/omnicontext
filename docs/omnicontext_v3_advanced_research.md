# OmniContext v3: Advanced Research & Competitive Analysis

> Strategic technical document for achieving parity and superiority over Augment Code, Cursor, and Sourcegraph Cody context engines.

---

## 1. Competitive Landscape Analysis

### 1.1 Augment Code

| Capability              | Augment Code                                                            | OmniContext v2 (Current)                      | Gap          |
| ----------------------- | ----------------------------------------------------------------------- | --------------------------------------------- | ------------ |
| **Chunking**            | AST micro-chunking with overlapping contextual windows                  | Tree-sitter chunking, no overlap              | Medium       |
| **Embedding Coverage**  | 100% of codebase embedded, real-time sync                               | 13.5% fixed coverage                          | **Critical** |
| **Knowledge Graph**     | Full dependency graph with type hierarchy, call graph, cross-repo edges | Shallow import-based graph, 0 edges populated | **Critical** |
| **Search Fusion**       | RRF + cross-encoder reranker + graph boost                              | RRF only, uniform scores                      | High         |
| **Context Delivery**    | Push model via "Auggie CLI" local daemon, silent pre-flight injection   | Pull model via MCP tools                      | High         |
| **Query Understanding** | Intent classification (edit/explain/debug/refactor), query expansion    | Basic literal matching                        | High         |
| **Multi-Repo**          | Cross-repo symbol resolution, workspace-level context                   | Single repo only                              | Medium       |
| **Scalability**         | 400k+ files, sub-200ms latency                                          | Untested at scale                             | Medium       |
| **Context Lineage**     | Commit history aware, understands code evolution                        | Basic commit module exists                    | Medium       |

### 1.2 Cursor AI

| Capability                  | Cursor                                                          | OmniContext v2           | Gap          |
| --------------------------- | --------------------------------------------------------------- | ------------------------ | ------------ |
| **Custom Embedding Model**  | Proprietary trained on code, privacy-preserving                 | Generic ONNX model       | High         |
| **Multi-Stage Pipeline**    | Bi-encoder recall -> Cross-encoder rerank                       | Single-stage BM25+vector | **Critical** |
| **IDE Integration**         | Deep editor integration, real-time context refresh              | Basic VS Code extension  | High         |
| **Agent Orchestration**     | 8 parallel agents, plan mode, apply model                       | Single-threaded engine   | High         |
| **User Context Signals**    | Active file, recently viewed files, edit history, linter errors | Active file only         | Medium       |
| **Infinite Context Vision** | Model ensembles forming "infinite context engine"               | Fixed token budget       | High         |

### 1.3 Sourcegraph Cody

| Capability               | Cody                                                  | OmniContext v2         | Gap          |
| ------------------------ | ----------------------------------------------------- | ---------------------- | ------------ |
| **Code Graph**           | Repo-level Semantic Graph (RSG) with link prediction  | Empty petgraph         | **Critical** |
| **Context Ranking**      | BM25 + embeddings + graph expansion + refine          | BM25 + embeddings only | High         |
| **MCP Deep Search**      | Cross-repo, historical, architectural queries via MCP | Basic MCP tools        | High         |
| **Quantized Search**     | 8x memory reduction via quantized vectors             | Full-precision vectors | Medium       |
| **Context Transparency** | "No invisible magic" -- user sees injected context    | Opaque context flow    | Medium       |
| **Enterprise Scale**     | 400k files, sub-200ms                                 | Untested               | Medium       |

---

## 2. Critical Capability Gaps (Ranked by Strategic Impact)

### 2.1 Code Representation: AST Micro-Chunking & Overlap (PRIORITY: CRITICAL)

**Current State**: Tree-sitter parser strictly extracts isolated AST nodes with zero overlap.
**Target State**: AST-aware micro-chunking with configurable token overlap margins.

1. **Semantic Completeness**: Ensures functions mapped within traits aren't severed from their trait declarations.
2. **Context Windowing**: Uses a sliding window across the AST nodes to stitch overlapping fragments securely.

**Implementation**:

```
Enhance: omni-core::chunker
- Implement CAST (Chunking via Abstract Syntax Trees) algorithm.
- Add an overlapping window tokenizer checking backward context (e.g., 10-line or 100-token semantic overlap).
- Expected impact: Vastly reduces structural hallucinations from LLM generation.
```

> [!IMPORTANT]
> A chunk without its surrounding module/class context creates high latency when the LLM asks secondary clarifying questions.

### 2.2 Two-Stage Retrieval Pipeline (PRIORITY: CRITICAL)

**Current State**: Single-pass hybrid search (BM25 + vector similarity + symbol match).
**Target State**: Two-stage pipeline:

1. **Recall Stage**: k-NN approximate nearest neighbors (HNSW) + BM25 FTS5 for broad candidate retrieval (top-100)
2. **Precision Stage**: Cross-encoder reranker scores query-document pairs jointly for precise relevance

**Implementation**:

```
New Crate: omni-reranker
- Download cross-encoder model (e.g., ms-marco-MiniLM-L-6-v2 ONNX)
- Implement batch scoring: (query, chunk_content) -> relevance_score
- Integrate as optional pipeline stage after SearchEngine::fuse_results()
- Expected impact: 40-60% MRR improvement based on literature
```

> [!IMPORTANT]
> This is the single highest-ROI upgrade. Cross-encoder reranking typically doubles search relevance with minimal architectural change.

### 2.3 Repo-Level Semantic Graph (PRIORITY: CRITICAL)

**Current State**: `DependencyGraph` uses `petgraph` but populates 0 edges.
**Target State**: Dense semantic graph with:

- **Import edges**: File A imports symbol from File B
- **Call edges**: Function A calls Function B (from AST analysis)
- **Type hierarchy**: Struct implements Trait, Class extends Class
- **Module containment**: File belongs to Module belongs to Crate
- **Temporal edges**: Symbol X changed in commit Y alongside Symbol Z

**Implementation**:

```
Enhance: omni-core::graph
- Phase 1: Fix import resolution (match import paths to indexed symbols)
- Phase 2: Extract call sites from AST (function_call nodes in tree-sitter)
- Phase 3: Type hierarchy from impl blocks, extends, implements
- Phase 4: Link prediction for inferred edges (co-change analysis)
```

### 2.4 Full Embedding Coverage (PRIORITY: CRITICAL)

**Current State**: Only 13.5% of chunks get embeddings.
**Root Cause**: Embedding pipeline skips chunks that fail validation.
**Target State**: 100% coverage with graceful degradation.

**Implementation**:

- Fix chunk content validation to accept all valid code
- Implement batch embedding with retry logic
- Add fallback: if embedding fails, use TF-IDF vector as proxy
- Track coverage metric in [status](file:///c:/Omniverse/Projects/omnicontext/crates/omni-core/src/pipeline/mod.rs#503-525) output

### 2.5 Context Lineage (PRIORITY: HIGH)

**Current State**: Basic `commits` module exists but is disconnected from search.
**Target State**: Code evolution awareness:

- "This function was last modified 3 days ago by author X"
- "This API endpoint has had 5 breaking changes in the last month"
- "These 4 files always change together (co-change coupling)"

**Implementation**:

```
Enhance: omni-core::commits + omni-core::graph
- Extract co-change patterns from git log (files changed together)
- Weight search results by recency (recently modified = more relevant)
- Temporal decay: older code gets lower graph centrality
- Expose via MCP: get_code_history(symbol) -> change timeline
```

### 2.6 Quantized Vector Search (PRIORITY: HIGH)

**Current State**: Full f32 vectors (384 dimensions \* 4 bytes = 1.5KB per chunk).
**Target State**: uint8 quantized vectors (384 bytes per chunk, 4x reduction).

**Implementation**:

```
Enhance: omni-core::vector
- Implement scalar quantization: f32 -> uint8 with min/max normalization
- Modify HNSW distance function to use quantized dot product
- Hybrid approach: quantized for recall, full precision for final scoring
- Expected memory: 100k chunks @ 384B = 37MB vs current 150MB
```

### 2.7 Multi-Agent Context Distribution (PRIORITY: HIGH)

**Current State**: Single engine instance behind Mutex.
**Target State**: Context distribution to multiple concurrent agents:

- Agent A is debugging auth -> gets auth context
- Agent B is refactoring DB layer -> gets DB context
- Both context windows are independent, optimized per-intent

**Implementation**:

```
New Module: omni-core::session
- SessionManager tracks active agent sessions
- Each session has its own context budget, active files, intent
- Daemon multiplexes sessions over single engine instance
- Session-aware search: boost results relevant to session's scope
```

---

## 3. Advanced Techniques for v3

### 3.1 Contextual Chunk Enrichment

Every chunk should carry its "neighborhood" context:

```
struct EnrichedChunk {
    core: Chunk,                     // The actual code
    imports: Vec<String>,             // File-level imports
    parent_scope: Option<String>,     // Enclosing function/class signature
    sibling_signatures: Vec<String>,  // Other methods in same class
    doc_summary: Option<String>,      // AI-generated summary of chunk purpose
}
```

### 3.2 Query Intent Classification

Instead of treating all queries the same:

| Intent       | Search Strategy                                     | Context Assembly                                  |
| ------------ | --------------------------------------------------- | ------------------------------------------------- |
| **Edit**     | Focus on implementation details, similar patterns   | Include surrounding code, imports, tests          |
| **Explain**  | Broad architectural context, doc comments           | Include module map, call graph excerpts           |
| **Debug**    | Error handling paths, recent changes, test failures | Include error types, recent commits, stack traces |
| **Refactor** | All usages, downstream dependents, type hierarchy   | Include all callers, implementors, tests          |

Classification via keyword heuristics + embedding similarity to intent exemplars.

### 3.3 Speculative Pre-Fetch

Monitor IDE state and pre-compute likely context before the user asks:

- User opens file X -> pre-fetch context for "explain X"
- User navigates to function F -> pre-fetch callers, callees, tests
- User starts typing comment -> pre-fetch related documentation

Cache pre-fetched contexts with TTL, invalidate on file changes.

### 3.4 Graph-Based Relevance Propagation

Use the semantic graph to boost related code:

```
1. Execute search -> get initial results R
2. For each result r in R:
   a. Find graph neighbors N(r) with edge weight
   b. Propagate relevance: score(n) += alpha * score(r) * edge_weight(r,n)
3. Re-rank combined set R + N(R)
4. Apply token budget
```

This is similar to PageRank but seeded by search relevance instead of link count.

### 3.5 Adaptive Chunking

Current: Fixed AST-level chunks (one per function/class).
Advanced: Adaptive chunks based on semantic density:

- Dense code (complex algorithms) -> smaller chunks
- Boilerplate (config, imports) -> larger chunks
- Critical path (auth, payment) -> overlapping chunks with extra context

---

## 4. Architecture Comparison: Where We Stand

```
                 Augment Code          OmniContext v2           Target v3
                 ------------          ----------------         -----------
Indexing:        Micro-chunker         AST Chunker              Adaptive Chunker
                 Batch Embedder        ONNX Embedder            Batch + Quantized
                 Real-time Sync        File Watcher             Incremental Delta

Storage:         Vector DB (HNSW)      usearch (HNSW)           Quantized HNSW
                 Graph Store           petgraph (empty)         Enriched Hypergrph
                 Metadata Store        SQLite + FTS5            SQLite + FTS5

Retrieval:       Query Analyzer        Literal Matching         Intent Classifier
                 Query Expander        (none)                   Synonym + Graph Exp
                 Cross-Encoder         (none)                   ONNX Cross-Encoder
                 Graph Boost           (none)                   Relevance Propagation

Delivery:        Push (Auggie CLI)     Pull (MCP tools)         Push (Daemon + Ext)
                 Pre-Flight Inject     (none)                   Pre-Flight Inject
                 Multi-Session         Single Thread            Session Manager

Evaluation:      Internal Suite        (none)                   Benchmark Suite
```

---

## 5. Implementation Roadmap: v3 Phases

### Phase A: Reranking Pipeline (2-3 weeks)

1. Add `omni-reranker` crate with ONNX cross-encoder model
2. Integrate as optional post-processing step in [SearchEngine](file:///c:/Omniverse/Projects/omnicontext/crates/omni-core/src/search/mod.rs#21-32)
3. Benchmark: measure MRR/NDCG before vs after
4. Expected outcome: 40-60% MRR improvement

### Phase B: Graph Population & Micro-Chunks (3-4 weeks)

1. Rewrite parser chunker for CAST overlapping context
2. Fix import resolution to actually populate edges
3. Extract call sites from tree-sitter AST
4. Implement type hierarchy extraction
5. Expected outcome: Dense graph + holistic context preservation

### Phase C: Quantized Vectors + Scale (2-3 weeks)

1. Implement uint8 scalar quantization for vectors
2. Add incremental re-embedding (hash-based dirty tracking)
3. Benchmark at 100k+ file scale
4. Expected outcome: 4x memory reduction, linear indexing speed

### Phase D: Speculative Pre-Fetch (2-3 weeks)

1. Monitor editor state changes via VS Code extension
2. Implement pre-fetch cache with TTL
3. Add cursor-position-aware context biasing
4. Expected outcome: Near-zero latency for common queries

### Phase E: Multi-Session + Cross-Repo (3-4 weeks)

1. Add SessionManager with per-agent context tracking
2. Implement cross-repo symbol resolution
3. Add workspace-level configuration for multi-repo setups
4. Expected outcome: Enterprise-ready multi-agent support

---

## 6. Key Metrics for Success

| Metric                         | Current     | v2 Target | v3 Target |
| ------------------------------ | ----------- | --------- | --------- |
| **MRR@5**                      | ~0.15 (est) | 0.45      | 0.75      |
| **Recall@10**                  | ~0.20 (est) | 0.55      | 0.85      |
| **NDCG@10**                    | ~0.10 (est) | 0.40      | 0.70      |
| **Embedding Coverage**         | 13.5%       | 95%       | 100%      |
| **Graph Edges**                | 0           | 500+      | 5000+     |
| **Indexing Speed (10k files)** | N/A         | <60s      | <30s      |
| **Search Latency (p95)**       | N/A         | <500ms    | <200ms    |
| **Memory (100k chunks)**       | ~150MB      | ~150MB    | ~40MB     |
| **Pre-flight Latency**         | N/A         | <1s       | <100ms    |

---

## 7. What Would Make OmniContext Superior to Augment Code

The key differentiator would be **transparency + local-first + open-source**:

1. **Open Architecture**: Unlike Augment's proprietary cloud engine, OmniContext runs entirely local. Code never leaves the machine. This is a non-negotiable requirement for security-conscious enterprises.

2. **Pluggable Reranker**: Let users bring their own cross-encoder model -- fine-tuned on their codebase conventions. Augment uses a one-size-fits-all model.

3. **Transparent Context**: Expose the full context assembly trace to the user. Show exactly which chunks were selected, why, and with what scores. Cody does this ("no invisible magic"), but Augment and Cursor do not.

4. **Agent-Agnostic Delivery**: OmniContext delivers context to ANY AI agent via MCP, daemon IPC, or REST API. Not locked to a single LLM provider.

5. **Community-Driven Patterns**: Open-source pattern library where teams can contribute and share code intelligence rules (e.g., "in this framework, controller files always relate to service files").

---

## 8. Immediate Next Steps

1. **Run the benchmark suite** against the current OmniContext to establish baseline metrics
2. **Fix embedding coverage** -- this is the lowest-hanging fruit with highest impact
3. **Prototype cross-encoder reranker** -- add ms-marco-MiniLM-L-6-v2 ONNX model
4. **Populate the dependency graph** -- fix import resolution in `omni-core::graph`
5. **Measure pre-flight latency** -- test the daemon IPC path end-to-end

> [!TIP]
> The single most impactful change is the cross-encoder reranker. Every competitor uses one. We don't. This alone can double search relevance.
