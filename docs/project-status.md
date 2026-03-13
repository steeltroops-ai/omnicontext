# OmniContext Feature Status

**Version**: v1.2.1
**Last Updated**: 2026-03-13

---

## Overview

This document tracks the implementation status of all subsystems and features in OmniContext. Features are organized by subsystem. Infrastructure stubs that are scaffolded but not yet active in the production pipeline are listed separately under the future intelligence roadmap.

---

## Core Indexing Pipeline

| Feature | Location | Status |
|---------|----------|--------|
| Tree-sitter AST parsing (16 languages) | `omni-core/src/parser/` | Shipped |
| CAST boundary-aware chunking | `omni-core/src/chunker/mod.rs` | Shipped |
| Contextual prefix injection (file + parent + callers/callees) | `omni-core/src/chunker/mod.rs` | Shipped |
| RAPTOR hierarchical summary chunks | `omni-core/src/chunker/mod.rs` | Shipped |
| `ActualTokenCounter` (tokenizer.json) | `omni-core/src/chunker/token_counter.rs` | Shipped |
| `EstimateTokenCounter` fallback | `omni-core/src/chunker/token_counter.rs` | Shipped |
| 10–15% chunk overlap strategy | `omni-core/src/chunker/mod.rs` | Shipped |
| Split-at-block-boundary for large functions | `omni-core/src/chunker/mod.rs` | Shipped |
| Hash-based change detection (skip unchanged files) | `omni-core/src/index/mod.rs` | Shipped |
| Incremental re-index (< 200ms per file) | `omni-core/src/index/watcher.rs` | Shipped |
| File watcher (inotify / FSEvents / ReadDirectoryChangesW) | `omni-daemon/src/watcher.rs` | Shipped |

---

## Embedding Engine

| Feature | Location | Status |
|---------|----------|--------|
| jina-embeddings-v2-base-code (ONNX, 768 dim) | `omni-core/src/embedder/mod.rs` | Shipped |
| ONNX Runtime session pooling | `omni-core/src/embedder/mod.rs` | Shipped |
| Batch scheduling (80-chunk window) | `omni-core/src/embedder/mod.rs` | Shipped |
| SHA-256 checksum verification on model download | `omni-core/src/embedder/model_manager.rs` | Shipped |
| Auto-download to `~/.omnicontext/models/` | `omni-core/src/embedder/model_manager.rs` | Shipped |
| Degraded-mode fallback (keyword-only when model absent) | `omni-core/src/embedder/mod.rs` | Shipped |
| Asymmetric query/passage encoding (instruction prefix) | `omni-core/src/embedder/mod.rs` | Shipped |
| INT8 quantization infrastructure | `omni-core/src/embedder/quantization.rs` | Shipped |

---

## Vector Index

| Feature | Location | Status |
|---------|----------|--------|
| HNSW vector index (usearch) | `omni-core/src/vector/hnsw.rs` | Shipped |
| 768-dimensional cosine similarity | `omni-core/src/vector/hnsw.rs` | Shipped |
| mmap-based storage for large indexes | `omni-core/src/vector/hnsw.rs` | Shipped |
| Flat fallback for small indexes (< threshold) | `omni-core/src/vector/mod.rs` | Shipped |
| Tombstone + background compaction for incremental deletes | `omni-core/src/vector/hnsw.rs` | Shipped |
| Atomic index swap on rebuild | `omni-daemon/src/compaction.rs` | Shipped |

---

## Keyword Search

| Feature | Location | Status |
|---------|----------|--------|
| SQLite FTS5 BM25 full-text search | `omni-core/src/search/mod.rs` | Shipped |
| Identifier splitting (camelCase, snake_case) | `omni-core/src/search/mod.rs` | Shipped |
| Synonym expansion (100-entry code vocabulary) | `omni-core/src/search/synonyms.rs` | Shipped |
| Connection pooling (WAL mode, concurrent reads) | `omni-core/src/index/db.rs` | Shipped |

---

## Hybrid Search and Fusion

| Feature | Location | Status |
|---------|----------|--------|
| RRF (Reciprocal Rank Fusion) with configurable k | `omni-core/src/search/mod.rs` | Shipped |
| Query-type adaptive weights (Symbol/Keyword/NL/Mixed) | `omni-core/src/search/mod.rs` | Shipped |
| Query caching (LRU, 100 entries) | `omni-core/src/search/mod.rs` | Shipped |
| Result deduplication (overlapping line ranges) | `omni-core/src/search/mod.rs` | Shipped |
| Structural boosting (in-degree, PageRank, freshness) | `omni-core/src/search/mod.rs` | Shipped |
| Graph-Augmented Retrieval (GAR, N-hop expansion) | `omni-core/src/search/mod.rs` | Shipped |

---

## Query Understanding

| Feature | Location | Status |
|---------|----------|--------|
| `analyze_query()` — Symbol/Keyword/NL/Mixed classification | `omni-core/src/search/mod.rs` | Shipped |
| `QueryIntent::classify()` — 9 intent types | `omni-core/src/search/intent.rs` | Shipped |
| `expand_query()` — identifier splitting + stop word removal | `omni-core/src/search/mod.rs` | Shipped |
| `synonyms::expand_with_synonyms()` — code vocabulary expansion | `omni-core/src/search/synonyms.rs` | Shipped |
| HyDE (Hypothetical Document Embeddings) for NL queries | `omni-core/src/search/hyde.rs` | Shipped |
| Intent-driven graph depth selection | `omni-core/src/search/intent.rs` | Shipped |
| `ContextStrategy` per intent type | `omni-core/src/search/intent.rs` | Shipped |

---

## Reranking

| Feature | Location | Status |
|---------|----------|--------|
| ms-marco-MiniLM-L-6-v2 cross-encoder (ONNX) | `omni-core/src/reranker/mod.rs` | Shipped |
| Sigmoid score normalization | `omni-core/src/reranker/mod.rs` | Shipped |
| Platt calibration | `omni-core/src/reranker/mod.rs` | Shipped |
| `rerank_with_priority()` with early termination | `omni-core/src/reranker/mod.rs` | Shipped |
| `min_threshold` batching (skip irrelevant candidates) | `omni-core/src/reranker/mod.rs` | Shipped |
| RRF blending formula (`0.3*rrf + 0.7*xenc`) | `omni-core/src/reranker/mod.rs` | Shipped |
| Graceful degradation when model absent | `omni-core/src/reranker/mod.rs` | Shipped |

---

## Dependency Graph

| Feature | Location | Status |
|---------|----------|--------|
| File-level graph (HashMap adjacency) | `omni-core/src/graph/dependencies.rs` | Shipped |
| Symbol-level graph (petgraph directed) | `omni-core/src/graph/symbol_graph.rs` | Shipped |
| Edge types: IMPORTS, INHERITS, CALLS, INSTANTIATES | `omni-core/src/graph/edge_extractor.rs` | Shipped |
| Historical co-change edges | `omni-core/src/graph/history.rs` | Shipped |
| N-hop neighborhood queries (< 10ms) | `omni-core/src/graph/dependencies.rs` | Shipped |
| PageRank scoring | `omni-core/src/graph/pagerank.rs` | Shipped |
| Cycle detection | `omni-core/src/graph/symbol_graph.rs` | Shipped |
| Incremental graph updates (changed files only) | `omni-core/src/graph/dependencies.rs` | Shipped |

---

## MCP Server

| Feature | Location | Status |
|---------|----------|--------|
| 19 MCP tools (stdio + SSE transports) | `omni-mcp/src/` | Shipped |
| JSON-RPC 2.0 protocol compliance | `omni-mcp/src/server.rs` | Shipped |
| MCP spec error codes | `omni-mcp/src/error.rs` | Shipped |
| Context window assembly (token budget) | `omni-core/src/context/assembler.rs` | Shipped |

---

## Daemon and IPC

| Feature | Location | Status |
|---------|----------|--------|
| Named pipe IPC (Windows) / Unix socket (Linux/macOS) | `omni-daemon/src/ipc.rs` | Shipped |
| JSON-RPC 2.0 over IPC | `omni-daemon/src/ipc.rs` | Shipped |
| Circuit breakers | `omni-daemon/src/resilience.rs` | Shipped |
| Health monitoring | `omni-daemon/src/health.rs` | Shipped |
| Event deduplication | `omni-daemon/src/dedup.rs` | Shipped |
| Backpressure monitoring | `omni-daemon/src/backpressure.rs` | Shipped |
| Real-time metrics IPC handlers | `omni-daemon/src/ipc.rs` | Shipped |

---

## VS Code Extension

| Feature | Location | Status |
|---------|----------|--------|
| Zero-configuration binary bootstrap | `editors/vscode/src/bootstrapService.ts` | Shipped |
| Automatic daemon lifecycle management | `editors/vscode/src/extension.ts` | Shipped |
| IPC connection with exponential backoff | `editors/vscode/src/extension.ts` | Shipped |
| Real-time sidebar metrics dashboard | `editors/vscode/src/sidebarProvider.ts` | Shipped |
| LSP-enhanced symbol extraction | `editors/vscode/src/symbolExtractor.ts` | Shipped |
| Intelligent pre-fetch via IDE events | `editors/vscode/src/eventTracker.ts` | Shipped |
| MCP auto-configuration (17 clients) | `editors/vscode/src/extension.ts` | Shipped |
| Repository registry | `editors/vscode/src/repoRegistry.ts` | Shipped |

---

## Performance (All Targets Met)

| Metric | Target | Status |
|--------|--------|--------|
| File indexing | > 500 files/sec | Met |
| Embedding throughput | > 800 chunks/sec | Met |
| Search P99 latency | < 50ms (100k chunk index) | Met |
| Incremental re-index | < 200ms per file change | Met |
| Startup (warm index) | < 2s | Met |
| Memory per chunk | < 2KB metadata | Met |

---

## Future Intelligence Roadmap

The following infrastructure stubs are present in the codebase but are not active in the production pipeline. They are research scaffolding for future model-driven enhancements.

| Feature | Location | Notes |
|---------|----------|-------|
| GNN attention scoring | `omni-core/src/graph/attention.rs` | Graph neural network attention over the dependency graph; requires training data |
| Contrastive fine-tuning | `omni-core/src/embedder/contrastive.rs` | Domain-specific fine-tuning of the embedding model on OmniContext retrieval pairs |
| Full INT8 quantization pipeline | `omni-core/src/embedder/quantization.rs` | Infrastructure present; production activation pending quality validation |
| Tarjan SCC (strongly connected components) | `omni-core/src/graph/queries.rs` | Scaffolded; not yet wired into the MCP tool surface |

These items are not bugs or missing features — they represent extension points for future capability. The production pipeline does not depend on any of them.

---

## See Also

- [Architecture: Intelligence](./architecture/intelligence.md) — subsystem architecture overview
- [Architecture: ADR](./architecture/ADR.md) — architectural decisions
- [Development: Testing Strategy](./development/testing-strategy.md) — test coverage requirements
