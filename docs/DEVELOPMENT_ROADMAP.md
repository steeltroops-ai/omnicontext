# OmniContext Development Roadmap

**Version**: 1.0
**Date**: 2026-02-28
**Status**: Active Development

---

## Architecture Principle: Decoupled Subsystems

Every subsystem is a Rust module with:

1. A **public trait or struct** defining its API
2. Its **own error types** (converted to `OmniError` at boundaries)
3. **Independent unit tests** (no cross-module test dependencies)
4. **Clear data flow** via shared types in `omni_core::types`

This means you can work on the parser without touching the embedder.
You can debug search without understanding the dependency graph.
You can replace the vector index without changing anything else.

```
  watcher --> parser --> chunker --> embedder --> index
                |                                  |
                v                                  v
              graph <----- search <------------- query
```

---

## Phase 0: Project Scaffold [COMPLETE]

| Task                                | Status | Notes                                                 |
| ----------------------------------- | ------ | ----------------------------------------------------- |
| Rust toolchain verification         | DONE   | 1.93.0 stable                                         |
| Cargo workspace + 3 crates          | DONE   | omni-core, omni-mcp, omni-cli                         |
| Module architecture (10 subsystems) | DONE   | All compiling with stubs                              |
| Domain types (types.rs)             | DONE   | Language, Chunk, Symbol, DependencyEdge, SearchResult |
| Error taxonomy (error.rs)           | DONE   | Recoverable/Degraded/Fatal hierarchy                  |
| Configuration system (config.rs)    | DONE   | 5-level precedence chain                              |
| SQLite schema (schema.sql)          | DONE   | FTS5, triggers, indexes                               |
| Language analyzer registry          | DONE   | 5 languages registered                                |
| 20 unit tests passing               | DONE   | All green                                             |
| .agents/ infrastructure             | DONE   | Persona, rules, 6 workflows                           |
| Documentation (8 docs)              | DONE   | ADRs, concurrency, errors, testing, security          |
| .gitignore, rustfmt.toml, CHANGELOG | DONE   |                                                       |

---

## Phase 1: Core Engine -- Parser & Chunker

**Goal**: Parse source files into structured, searchable chunks.
**Duration**: ~2 weeks
**Subsystems**: `parser`, `chunker`

### Phase 1a: Python Analyzer (3 days)

| Task | Subsystem                   | Description                                                | Acceptance                                          |
| ---- | --------------------------- | ---------------------------------------------------------- | --------------------------------------------------- |
| 1a.1 | `parser::languages::python` | Implement `extract_structure` for Python AST               | Extracts functions, classes, imports from fixture   |
| 1a.2 | `parser::languages::python` | Handle decorators (@property, @staticmethod, @classmethod) | Decorator kind is preserved in metadata             |
| 1a.3 | `parser::languages::python` | Extract docstrings (triple-quoted)                         | doc_comment populated for all docstring'd functions |
| 1a.4 | `parser::languages::python` | Detect visibility (\_private vs public convention)         | Visibility correctly set based on name prefix       |
| 1a.5 | `parser::languages::python` | Handle nested functions and classes                        | Nested elements have correct symbol_path hierarchy  |
| 1a.6 | tests                       | 15+ unit tests + 1 integration test with fixture repo      | All pass                                            |

### Phase 1b: TypeScript/JavaScript Analyzer (3 days)

| Task | Subsystem                       | Description                                    | Acceptance                           |
| ---- | ------------------------------- | ---------------------------------------------- | ------------------------------------ |
| 1b.1 | `parser::languages::typescript` | Functions, arrow functions, class declarations | All structural elements extracted    |
| 1b.2 | `parser::languages::typescript` | Interface + type alias declarations            | Kind = Trait/TypeDef                 |
| 1b.3 | `parser::languages::typescript` | Export analysis (named, default, re-exports)   | Visibility correctly determined      |
| 1b.4 | `parser::languages::typescript` | JSDoc extraction                               | doc_comment from `/** */` blocks     |
| 1b.5 | `parser::languages::javascript` | Same as TS minus type-specific nodes           | Shared base with TS-specific overlay |
| 1b.6 | tests                           | 15+ tests per language                         | All pass                             |

### Phase 1c: Rust Analyzer (2 days)

| Task | Subsystem                 | Description                                       | Acceptance                     |
| ---- | ------------------------- | ------------------------------------------------- | ------------------------------ |
| 1c.1 | `parser::languages::rust` | fn, struct, enum, trait, impl, const, type, mod   | All node types extracted       |
| 1c.2 | `parser::languages::rust` | Visibility (pub, pub(crate), pub(super), private) | Correct Visibility variant     |
| 1c.3 | `parser::languages::rust` | `#[test]` attribute detection                     | Kind = Test for test functions |
| 1c.4 | `parser::languages::rust` | Doc comments (///, //!, /\*\* \*/)                | doc_comment extracted          |
| 1c.5 | tests                     | 15+ tests                                         | All pass                       |

### Phase 1d: Go Analyzer (1 day)

| Task | Subsystem               | Description                         | Acceptance                        |
| ---- | ----------------------- | ----------------------------------- | --------------------------------- |
| 1d.1 | `parser::languages::go` | Function, method, struct, interface | All extracted                     |
| 1d.2 | `parser::languages::go` | Capitalization-based visibility     | Public/Private correctly detected |
| 1d.3 | tests                   | 10+ tests                           | All pass                          |

### Phase 1e: Semantic Chunker (3 days)

| Task | Subsystem        | Description                                          | Acceptance                           |
| ---- | ---------------- | ---------------------------------------------------- | ------------------------------------ |
| 1e.1 | `chunker`        | AST-boundary splitting (never mid-expression)        | No chunk starts/ends mid-statement   |
| 1e.2 | `chunker`        | Per-language token estimation                        | Within 10% of actual tokenizer count |
| 1e.3 | `chunker`        | Large function/class splitting at block boundaries   | Classes split at method boundaries   |
| 1e.4 | `chunker`        | Boundary overlap (10-15% of prev chunk appended)     | Context continuity at boundaries     |
| 1e.5 | `chunker`        | Weight computation (kind \* visibility)              | Weights match spec table             |
| 1e.6 | tests + proptest | Property: chunks cover all content, never exceed max | proptest with 100+ iterations        |

---

## Phase 2: Core Engine -- Index & Search

**Goal**: Store chunks and enable hybrid retrieval.
**Duration**: ~2 weeks
**Subsystems**: `embedder`, `index`, `vector`, `search`

### Phase 2a: Embedding Engine (4 days)

| Task | Subsystem  | Description                                     | Acceptance                           |
| ---- | ---------- | ----------------------------------------------- | ------------------------------------ |
| 2a.1 | `embedder` | ONNX model loading and session initialization   | Model loads, session created         |
| 2a.2 | `embedder` | Tokenizer loading (from model or separate file) | Tokenizes correctly                  |
| 2a.3 | `embedder` | Batch inference with padding/truncation         | 32-chunk batches, correct dimensions |
| 2a.4 | `embedder` | L2 normalization of output vectors              | All vectors unit length              |
| 2a.5 | `embedder` | Graceful fallback when model missing            | keyword-only mode, no crash          |
| 2a.6 | `embedder` | Benchmark: >= 500 embeddings/sec on CPU         | Criterion bench passes               |

### Phase 2b: SQLite Index Operations (3 days)

| Task | Subsystem | Description                                  | Acceptance                           |
| ---- | --------- | -------------------------------------------- | ------------------------------------ |
| 2b.1 | `index`   | Insert/update/delete files                   | CRUD operations work                 |
| 2b.2 | `index`   | Insert/update/delete chunks (with FTS5 sync) | FTS5 triggers fire correctly         |
| 2b.3 | `index`   | Insert/update symbols and dependencies       | Symbol table populated               |
| 2b.4 | `index`   | BM25 keyword search via FTS5                 | Relevant results for keyword queries |
| 2b.5 | `index`   | Integrity check on startup                   | Corruption detected and reported     |
| 2b.6 | tests     | 15+ tests including concurrent read/write    | No deadlocks                         |

### Phase 2c: Vector Index (3 days)

| Task | Subsystem | Description                              | Acceptance                           |
| ---- | --------- | ---------------------------------------- | ------------------------------------ |
| 2c.1 | `vector`  | Evaluate usearch vs other options        | Decision documented in ADR           |
| 2c.2 | `vector`  | Create/open/persist HNSW index           | Index survives process restart       |
| 2c.3 | `vector`  | Add/remove/search operations             | KNN search returns correct neighbors |
| 2c.4 | `vector`  | mmap-backed persistence                  | < 2KB resident per vector            |
| 2c.5 | `vector`  | Benchmark: search < 5ms for 100k vectors | Criterion bench passes               |

### Phase 2d: Hybrid Search Engine (4 days)

| Task | Subsystem         | Description                                       | Acceptance                                      |
| ---- | ----------------- | ------------------------------------------------- | ----------------------------------------------- |
| 2d.1 | `search`          | Query analyzer (semantic vs keyword vs symbol)    | Correct strategy selection                      |
| 2d.2 | `search`          | Semantic retrieval via vector index               | Returns relevant chunks by embedding similarity |
| 2d.3 | `search`          | Keyword retrieval via FTS5                        | Returns relevant chunks by BM25                 |
| 2d.4 | `search`          | RRF fusion (already has math)                     | Fused ranking improves over either signal alone |
| 2d.5 | `search`          | Structural weight boosting                        | Public APIs ranked above private                |
| 2d.6 | `search`          | Context builder (parent, sibling, import context) | Rich context per result                         |
| 2d.7 | `search`          | Token budget management                           | Response fits within configured budget          |
| 2d.8 | tests + NDCG eval | NDCG@10 > 0.60 on fixture repos                   | Automated evaluation                            |

---

## Phase 3: Pipeline & CLI

**Goal**: Wire everything together into a working end-to-end system.
**Duration**: ~1 week
**Subsystems**: `watcher`, `pipeline`, `omni-cli`

### Phase 3a: File Watcher (2 days)

| Task | Subsystem | Description                                  | Acceptance                               |
| ---- | --------- | -------------------------------------------- | ---------------------------------------- |
| 3a.1 | `watcher` | Platform-native file watching (notify crate) | Events received for create/modify/delete |
| 3a.2 | `watcher` | Debouncing (100ms, configurable)             | Rapid edits batched into single event    |
| 3a.3 | `watcher` | Exclude pattern filtering                    | .git, node_modules, etc. ignored         |
| 3a.4 | `watcher` | Full directory scan on startup               | All source files discovered              |
| 3a.5 | `watcher` | Periodic full scan (catch missed events)     | Stale files detected and re-indexed      |

### Phase 3b: Pipeline Orchestrator (3 days)

| Task | Subsystem  | Description                                  | Acceptance                                                    |
| ---- | ---------- | -------------------------------------------- | ------------------------------------------------------------- |
| 3b.1 | `pipeline` | Channel-based pipeline wiring                | Events flow watcher -> parser -> chunker -> embedder -> store |
| 3b.2 | `pipeline` | Bounded channels with backpressure           | No OOM under heavy load                                       |
| 3b.3 | `pipeline` | spawn_blocking for CPU-bound work            | Async runtime not blocked                                     |
| 3b.4 | `pipeline` | Incremental re-indexing (only changed files) | Unchanged files skipped                                       |
| 3b.5 | `pipeline` | Graceful shutdown (drain channels, flush)    | No data loss on shutdown                                      |
| 3b.6 | `pipeline` | Progress reporting (tracing spans)           | Indexing progress visible in logs                             |

### Phase 3c: CLI (2 days)

| Task | Subsystem  | Description                                     | Acceptance                              |
| ---- | ---------- | ----------------------------------------------- | --------------------------------------- |
| 3c.1 | `omni-cli` | `omnicontext index .` works end-to-end          | Files indexed, chunks stored            |
| 3c.2 | `omni-cli` | `omnicontext search "query"` returns results    | Results displayed with file path, score |
| 3c.3 | `omni-cli` | `omnicontext status` shows index statistics     | File/chunk/symbol counts displayed      |
| 3c.4 | `omni-cli` | `omnicontext config --init` creates config file | .omnicontext/config.toml created        |

---

## Phase 4: MCP Server

**Goal**: Expose the engine to AI agents via MCP.
**Duration**: ~2 weeks
**Subsystems**: `omni-mcp`

### Phase 4a: MCP Protocol Core (3 days)

| Task | Description                                    | Acceptance                     |
| ---- | ---------------------------------------------- | ------------------------------ |
| 4a.1 | Evaluate `rmcp` crate vs custom implementation | Decision in ADR                |
| 4a.2 | stdio transport (JSON-RPC over stdin/stdout)   | Claude Code can connect        |
| 4a.3 | SSE transport (HTTP Server-Sent Events)        | Cursor can connect             |
| 4a.4 | Tool registration and dispatch                 | tools/list returns all 8 tools |

### Phase 4b: MCP Tools (8 days, 1 per tool)

| Tool                 | Description                   | Acceptance                             |
| -------------------- | ----------------------------- | -------------------------------------- |
| `search_code`        | Hybrid search with filters    | Returns ranked chunks with context     |
| `get_symbol`         | Symbol lookup by name/FQN     | Returns full definition + docs         |
| `get_dependencies`   | Dependency graph traversal    | Returns upstream/downstream symbols    |
| `get_file_summary`   | File structural summary       | Returns exports, classes, functions    |
| `find_patterns`      | Pattern detection in codebase | Returns code examples matching pattern |
| `get_architecture`   | Module relationship overview  | Returns module graph description       |
| `get_recent_changes` | Git change analysis           | Returns relevant commits               |
| `explain_codebase`   | High-level codebase overview  | Returns tech stack, structure, purpose |

### Phase 4c: Testing & Validation (3 days)

| Task | Description                               | Acceptance                                |
| ---- | ----------------------------------------- | ----------------------------------------- |
| 4c.1 | MCP Inspector integration testing         | All tools tested via Inspector UI         |
| 4c.2 | Claude Code end-to-end test               | Agent uses OmniContext tools successfully |
| 4c.3 | VS Code / Antigravity end-to-end test     | Agent connects and queries work           |
| 4c.4 | Error handling (invalid inputs, timeouts) | Graceful errors, no crashes               |
| 4c.5 | Concurrent request handling               | Multiple simultaneous queries work        |

---

## Phase 5: Dependency Graph

**Goal**: Cross-file dependency analysis for smarter search.
**Duration**: ~2 weeks
**Subsystems**: `graph`, `parser` (import resolvers)

### Phase 5a: Import Resolvers (6 days)

| Language      | Description                                      | Acceptance                      |
| ------------- | ------------------------------------------------ | ------------------------------- |
| Python        | `import foo`, `from foo.bar import baz`          | Resolved to file paths          |
| TypeScript/JS | `import {} from`, `require()`, barrel re-exports | Resolved including node_modules |
| Rust          | `use crate::`, `mod`, `pub use`                  | Resolved within workspace       |
| Go            | `import "pkg/path"`                              | Resolved to package directory   |

### Phase 5b: Graph Construction & Queries (4 days)

| Task | Description                                  | Acceptance                     |
| ---- | -------------------------------------------- | ------------------------------ |
| 5b.1 | Build dependency graph from resolved imports | Graph has correct edges        |
| 5b.2 | Dependency proximity boost in search         | Related symbols ranked higher  |
| 5b.3 | `get_dependencies` tool uses real graph      | Returns actual dependencies    |
| 5b.4 | Circular dependency detection                | Cycles identified and reported |

---

## Phase 6: Distribution & Polish

**Goal**: Ship a usable product.
**Duration**: ~2 weeks

| Task | Description                                  | Acceptance                         |
| ---- | -------------------------------------------- | ---------------------------------- |
| 6.1  | Cross-compilation CI (Linux, macOS, Windows) | Binaries built for all 3           |
| 6.2  | GitHub Actions release pipeline              | Tag triggers binary upload         |
| 6.3  | Homebrew formula                             | `brew install omnicontext` works   |
| 6.4  | Scoop manifest                               | `scoop install omnicontext` works  |
| 6.5  | Cargo publish                                | `cargo install omnicontext` works  |
| 6.6  | VS Code extension scaffold                   | Status bar, config UI              |
| 6.7  | Documentation site                           | README, quick start, API reference |
| 6.8  | Benchmark suite in CI                        | Performance regressions blocked    |
| 6.9  | NDCG evaluation in CI                        | Search quality regressions blocked |
| 6.10 | Public launch                                | GitHub release, announcement       |

---

## Phase 7: Pro Features (Post-Launch)

| Feature               | Description                                        | Duration |
| --------------------- | -------------------------------------------------- | -------- |
| Multi-repo workspace  | Link multiple repos, cross-repo search             | 7 days   |
| Commit lineage engine | Index git history, LLM-summarized commits          | 7 days   |
| Pattern recognition   | Detect conventions (error handling, logging, auth) | 5 days   |
| License server        | Pro feature gating                                 | 4 days   |

## Phase 8: Enterprise (Post-Launch)

| Feature                  | Description                  | Duration |
| ------------------------ | ---------------------------- | -------- |
| REST API server (axum)   | Hosted OmniContext for teams | 7 days   |
| Auth (API key + JWT)     | Secure access                | 4 days   |
| Usage metering + billing | Pay-per-query model          | 5 days   |
| Team knowledge sharing   | Shared indexes across org    | 7 days   |
| Docker + Kubernetes      | Container deployment         | 5 days   |

---

## Development Order Summary

```
Phase 0  [DONE]     Scaffold, CI, docs
Phase 1  [NEXT]     Parser + Chunker (Python, TS/JS, Rust, Go)
Phase 2             Embedder + Index + Vector + Search
Phase 3             Pipeline + CLI (end-to-end demo)
Phase 4             MCP Server (AI agent integration)
Phase 5             Dependency Graph (smarter search)
Phase 6             Distribution (package managers, VS Code)
Phase 7             Pro Features
Phase 8             Enterprise
```

Each phase produces a **working, testable increment**.
After Phase 3, you have a usable CLI tool.
After Phase 4, you have a usable MCP server.
Everything after that is enhancement and distribution.
