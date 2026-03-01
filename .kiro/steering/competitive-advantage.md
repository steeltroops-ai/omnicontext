---
inclusion: always
---

# OmniContext Competitive Advantage Strategy

This document outlines how to make OmniContext the most advanced code context engine, surpassing Augment Code, Cursor AI, Sourcegraph Cody, and other competitors.

## Critical Gaps to Address (Priority Order)

### 1. Two-Stage Retrieval with Cross-Encoder Reranking (CRITICAL)

Current: Single-stage hybrid search (BM25 + vector)
Target: Bi-encoder recall → Cross-encoder precision

Implementation:
- Create `omni-reranker` crate with ONNX cross-encoder model (ms-marco-MiniLM-L-6-v2)
- Stage 1: Fast recall via HNSW + BM25 (top-100 candidates)
- Stage 2: Cross-encoder scores query-chunk pairs jointly
- Expected impact: 40-60% MRR improvement, NDCG@10 from ~0.10 to 0.70+

This is the highest ROI upgrade. Every competitor uses cross-encoder reranking. We don't.

### 2. Full Embedding Coverage (CRITICAL)

Current: Only 13.5% of chunks get embeddings
Target: 100% coverage with graceful degradation

Implementation:
- Fix chunk validation to accept all valid code
- Implement batch embedding with retry logic
- Add fallback: TF-IDF vectors when embedding fails
- Track coverage metric in status output

### 3. Populated Dependency Graph (CRITICAL)

Current: petgraph exists but has 0 edges
Target: Dense semantic graph with 5000+ edges

Implementation:
- Fix import resolution to populate edges
- Extract call sites from tree-sitter AST
- Add type hierarchy (implements, extends)
- Add temporal edges (co-change analysis from git)
- Implement graph-based relevance propagation

### 4. AST Micro-Chunking with Overlap (HIGH)

Current: Strict isolated AST blocks, zero overlap
Target: Overlapping context windows (100-200 tokens)

Implementation:
- Rewrite `omni-core::chunker` with CAST algorithm
- Add configurable token overlap margin
- Ensure functions include surrounding module context
- Prevent orphaned chunks that lack declarative context

### 5. Quantized Vector Search (HIGH)

Current: Full f32 vectors (1.5KB per chunk)
Target: uint8 quantized (384 bytes per chunk, 4x reduction)

Implementation:
- Implement scalar quantization in `omni-core::vector`
- Convert f32 → uint8 with min/max normalization
- Hybrid: quantized for recall, full precision for final scoring
- Target: 100k chunks @ 40MB vs current 150MB

### 6. Intent-Aware Context Delivery (HIGH)

Current: Pull model via MCP tools
Target: Push model with intent classification

Implementation:
- Add intent classifier (Edit/Explain/Debug/Refactor)
- Speculative pre-fetch based on IDE cursor position
- Pre-flight context injection via daemon
- Context assembly must be < 100ms (hidden from user)

### 7. Context Lineage & Temporal Intelligence (MEDIUM)

Current: Unused git integration
Target: Temporal edges in semantic graph

Implementation:
- Extract co-change patterns from git log
- Weight search by recency (recent = more relevant)
- Track "last modified by" and "co-changed files"
- Expose via MCP: get_code_history(symbol)

## Advanced Techniques for Superiority

### Contextual Chunk Enrichment

Every chunk should carry neighborhood context:
- Core code content
- File-level imports
- Parent scope (enclosing function/class signature)
- Sibling signatures (other methods in same class)
- AI-generated summary of chunk purpose

### Query Intent Classification

Different intents need different search strategies:

| Intent    | Search Strategy                   | Context Assembly                    |
|-----------|-----------------------------------|-------------------------------------|
| Edit      | Implementation details, patterns  | Surrounding code, imports, tests    |
| Explain   | Architectural context, docs       | Module map, call graph              |
| Debug     | Error paths, recent changes       | Error types, commits, stack traces  |
| Refactor  | All usages, dependents, hierarchy | Callers, implementors, tests        |

### Graph-Based Relevance Propagation

Use semantic graph to boost related code:
1. Execute search → get initial results R
2. For each result r: find graph neighbors N(r)
3. Propagate relevance: score(n) += alpha × score(r) × edge_weight(r,n)
4. Re-rank combined set R + N(R)
5. Apply token budget

Similar to PageRank but seeded by search relevance.

### Adaptive Chunking

Current: Fixed AST-level chunks
Target: Adaptive based on semantic density

- Dense code (complex algorithms) → smaller chunks
- Boilerplate (config, imports) → larger chunks
- Critical paths (auth, payment) → overlapping chunks with extra context

## Key Differentiators vs Competitors

### vs Augment Code

1. Local-first: All processing on-device, no cloud dependency
2. Open source: Apache 2.0 core, community-driven
3. Agent-agnostic: Works with ANY MCP-compatible agent
4. Transparent context: Show exactly which chunks selected and why
5. Pluggable models: Users can bring their own fine-tuned models

### vs Cursor AI

1. Privacy-first: Code never leaves machine
2. Offline-capable: Fully functional without internet
3. Multi-agent support: Multiple concurrent sessions
4. Open architecture: Extensible via plugins
5. No vendor lock-in: Not tied to specific LLM provider

### vs Sourcegraph Cody

1. Zero-config: Auto-downloads models, auto-indexes
2. Lightweight: <50MB binary, <100MB memory for 10k files
3. Fast: Sub-50ms search, <200ms incremental updates
4. Local deployment: No enterprise server required
5. Community patterns: Shareable code intelligence rules

## Performance Targets (v3)

| Metric                    | Current (v2) | Target (v3) |
|---------------------------|--------------|-------------|
| MRR@5                     | ~0.15        | 0.75        |
| Recall@10                 | ~0.20        | 0.85        |
| NDCG@10                   | ~0.10        | 0.70        |
| Embedding Coverage        | 13.5%        | 100%        |
| Graph Edges               | 0            | 5000+       |
| Indexing (10k files)      | <60s         | <30s        |
| Search Latency (p95)      | <500ms       | <200ms      |
| Memory (100k chunks)      | ~150MB       | ~40MB       |
| Pre-flight Latency        | N/A          | <100ms      |

## Implementation Roadmap

### Phase A: Reranking Pipeline (2-3 weeks)
- Add omni-reranker crate with ONNX cross-encoder
- Integrate as post-processing in SearchEngine
- Benchmark MRR/NDCG improvements
- Expected: 40-60% MRR gain

### Phase B: Graph Population (3-4 weeks)
- Rewrite chunker for overlapping context
- Fix import resolution
- Extract call sites from AST
- Implement type hierarchy extraction
- Expected: Dense graph + holistic context

### Phase C: Quantization & Scale (2-3 weeks)
- Implement uint8 scalar quantization
- Add incremental re-embedding
- Benchmark at 100k+ files
- Expected: 4x memory reduction

### Phase D: Speculative Pre-Fetch (2-3 weeks)
- Monitor editor state via VS Code extension
- Implement pre-fetch cache with TTL
- Add cursor-aware context biasing
- Expected: Near-zero latency

### Phase E: Multi-Session (3-4 weeks)
- Add SessionManager for per-agent tracking
- Implement cross-repo symbol resolution
- Add workspace-level configuration
- Expected: Enterprise-ready

## Success Metrics

Track these KPIs to measure progress:

- Search relevance: MRR@5, Recall@10, NDCG@10
- Coverage: % of chunks with embeddings
- Graph density: Number of populated edges
- Performance: Indexing speed, search latency, memory usage
- User satisfaction: NPS, weekly active tool invocations

## Key Research Papers (2024-2025)

Must-read for implementation:

1. DeepCodeSeek (2025): Cross-encoder reranking for code
2. Microsoft GraphRAG (2024): Graph-augmented retrieval
3. Codegrag (2025): AST-to-Graph conversion
4. IBM Granite R2 (2025): Bi-encoder/cross-encoder optimization
5. CODEXGRAPH (2025): Hypergraph structures for source code

## Immediate Next Steps

1. Run benchmark suite to establish baseline metrics
2. Fix embedding coverage (lowest-hanging fruit, highest impact)
3. Prototype cross-encoder reranker with ms-marco model
4. Populate dependency graph by fixing import resolution
5. Measure pre-flight latency end-to-end

The single most impactful change: Cross-encoder reranking. This alone can double search relevance.
