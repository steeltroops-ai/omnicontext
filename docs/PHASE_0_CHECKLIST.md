# Phase 0: Pre-Implementation Checklist

**Status**: COMPLETE
**Completed**: 2026-02-28

---

## 0.1 Development Environment [DONE]

- [x] Rust 1.93.0 stable installed
- [x] Cargo 1.93.0 installed
- [x] Git 2.49.0 installed
- [x] IDE configured (VS Code / Antigravity)

## 0.2 Repository Structure [DONE]

- [x] `Cargo.toml` workspace with 3 crates
- [x] `omni-core` library crate (10 subsystem modules)
- [x] `omni-mcp` binary crate (MCP server)
- [x] `omni-cli` binary crate (CLI tool)
- [x] `.gitignore` configured
- [x] `rustfmt.toml` configured
- [x] `CHANGELOG.md` initialized
- [x] `README.md` created

## 0.3 Module Architecture [DONE]

All subsystems scaffolded with trait interfaces and stub implementations:

- [x] `config` -- Configuration loading with precedence chain
- [x] `error` -- Hierarchical error taxonomy (Recoverable/Degraded/Fatal)
- [x] `types` -- Core domain types (Language, Chunk, Symbol, etc.)
- [x] `parser` -- Tree-sitter parsing with LanguageAnalyzer trait
- [x] `chunker` -- AST-aware semantic chunking
- [x] `embedder` -- ONNX inference with graceful degradation
- [x] `index` -- SQLite + FTS5 with schema and triggers
- [x] `vector` -- HNSW vector index (API defined, implementation deferred)
- [x] `graph` -- Dependency graph with petgraph + RwLock
- [x] `search` -- Hybrid search with RRF fusion scoring
- [x] `watcher` -- File system watching with debouncing
- [x] `pipeline` -- Engine orchestrator

## 0.4 Language Support [DONE]

Tree-sitter grammars registered for all Phase 1 languages:

- [x] Python (tree-sitter-python 0.23)
- [x] TypeScript (tree-sitter-typescript 0.23)
- [x] JavaScript (tree-sitter-javascript 0.23)
- [x] Rust (tree-sitter-rust 0.23)
- [x] Go (tree-sitter-go 0.23)

## 0.5 Testing Foundation [DONE]

- [x] 20 unit tests passing across all subsystems
- [x] Test infrastructure configured (tempfile for integration tests)
- [x] Test command: `cargo test --workspace`

## 0.6 Build Verification [DONE]

- [x] `cargo check --workspace` -- passes clean
- [x] `cargo test --workspace` -- 20 tests, 0 failures
- [x] Only warning: 1 dead_code warning in watcher stub (expected)

## 0.7 Documentation [DONE]

- [x] Product Specification
- [x] Architecture Decision Records (6 ADRs)
- [x] Concurrency Architecture
- [x] Error Recovery Strategy (FMEA)
- [x] Testing Strategy
- [x] Embedding Model Evaluation
- [x] Security Threat Model
- [x] Supported Languages
- [x] Development Roadmap (8 phases, task-level)

## 0.8 Agent Infrastructure [DONE]

- [x] `.agents/persona.md` -- Engineering agent identity
- [x] `.agents/rules.md` -- 13 mandatory rules (updated post-Phase 0)
- [x] `.agents/workflows/` -- 6 development workflows

---

## Lessons Learned During Phase 0

1. **Dependency version verification is mandatory** -- `ort` is at RC stage, not stable 2.x. `usearch` needs version evaluation. Always `cargo check` before committing dep changes.

2. **Borrow checker patterns for shared mutable state** -- `HashMap::entry().or_insert_with(|| ...)` doesn't work when the closure needs to borrow the parent struct mutably. Split into `contains_key()` + `insert()`.

3. **Workspace dependency management** -- All versions in root `Cargo.toml` prevents version skew between crates. RC versions need explicit specification.

4. **Module stubs need clean API surfaces** -- Even stub modules should have correct type signatures and documented error semantics. This caught 5 import issues during the first build.

These lessons have been captured as rules R11, R12, and R13.
