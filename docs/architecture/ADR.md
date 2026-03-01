# OmniContext Architecture Decision Records

## ADR-001: Rust as Primary Language

**Status**: Accepted
**Date**: 2026-02-28

### Context

OmniContext runs as a local daemon on developer machines. It must be fast, memory-efficient, and produce a single binary with no runtime dependencies.

### Decision

Use Rust (2021 edition, stable toolchain) for all backend components.

### Consequences

- (+) Zero-cost abstractions, predictable performance
- (+) Single static binary, no JVM/Python/Node runtime needed
- (+) Memory safety without GC (critical for long-running processes)
- (+) Excellent async ecosystem (tokio)
- (+) tree-sitter FFI is natural (C bindings via Rust)
- (-) Slower iteration speed compared to Python/Go
- (-) Steeper learning curve for contributors
- (-) Build times can be long for full workspace

---

## ADR-002: SQLite over PostgreSQL for Metadata

**Status**: Accepted
**Date**: 2026-02-28

### Context

We need a relational store for file metadata, chunks, symbols, and full-text search (FTS5).

### Decision

Use embedded SQLite via `rusqlite` with `bundled` feature.

### Consequences

- (+) Zero configuration, single file
- (+) FTS5 provides excellent full-text search with BM25
- (+) WAL mode enables concurrent read during write
- (+) No port conflicts with other tools
- (+) Portable with the index directory
- (-) Write concurrency limited (single writer)
- (-) No built-in replication

### Mitigations

- Use WAL mode for non-blocking reads during indexing
- Batch writes in transactions to minimize lock contention
- For enterprise tier, consider PostgreSQL behind REST API

---

## ADR-003: usearch over Qdrant/Milvus for Vector Index

**Status**: Accepted
**Date**: 2026-02-28

### Context

We need approximate nearest neighbor (ANN) search for semantic code retrieval. Must be embeddable (no external process).

### Decision

Use usearch (HNSW algorithm) with mmap-backed persistence.

### Consequences

- (+) Embedded, no separate process
- (+) mmap-backed: vectors don't consume RSS until accessed
- (+) Sub-millisecond query latency for < 1M vectors
- (+) Small binary footprint
- (-) No built-in filtering (must post-filter)
- (-) No in-place vector updates (must delete + re-insert)
- (-) Less mature than Qdrant/Milvus ecosystem

### Mitigations

- Post-filtering after ANN retrieval (over-fetch by 3x, then filter)
- rebuild strategy for vector updates: mark-as-deleted + periodic compaction

---

## ADR-004: Code-Specific Embedding Model

**Status**: Proposed
**Date**: 2026-02-28

### Context

The original spec proposed `all-MiniLM-L6-v2` which is trained on natural language, not code. Code has fundamentally different semantic structure.

### Decision

Default to a code-specific embedding model. Candidate: `jinaai/jina-embeddings-v2-base-code` or `Salesforce/codet5p-110m-embedding`.

Fallback to `all-MiniLM-L6-v2` for low-resource environments.

### Evaluation Criteria

- ONNX export availability
- Inference speed on CPU (must be > 500 embeddings/sec)
- Model size (must be < 500MB)
- Code retrieval quality (measure on CodeSearchNet benchmark)

### Status

Requires benchmark evaluation during Phase 1.

---

## ADR-005: MCP as Primary Interface

**Status**: Accepted
**Date**: 2026-02-28

### Context

We need a standard protocol that works across all AI coding agents (Claude Code, Copilot, Cursor, Windsurf, Codex, etc.)

### Decision

Implement MCP (Model Context Protocol) as the primary interface. Support both `stdio` and `SSE` transports.

### Consequences

- (+) Universal agent compatibility
- (+) Standard protocol, reducing integration burden
- (+) Local-first by design
- (-) Newer protocol, ecosystem still maturing
- (-) Some agents have limited MCP support
- (-) Streaming support varies by transport

---

## ADR-006: Monorepo Workspace Structure

**Status**: Accepted
**Date**: 2026-02-28

### Context

OmniContext has multiple components: core engine, MCP server, CLI, API server, VS Code extension.

### Decision

Use a Cargo workspace monorepo with separate crates per component.

### Crates

| Crate       | Type | Purpose                             |
| ----------- | ---- | ----------------------------------- |
| `omni-core` | lib  | Core indexing, search, graph engine |
| `omni-mcp`  | bin  | MCP server binary                   |
| `omni-cli`  | bin  | CLI interface                       |
| `omni-api`  | bin  | Enterprise REST API                 |

### Consequences

- (+) Shared dependencies, single `cargo build`
- (+) Cross-crate integration tests
- (+) Single CI pipeline
- (-) Longer build times
- (-) Tighter coupling between release cycles
