---
inclusion: always
---

# OmniContext Project Structure

## Workspace Layout

This is a Cargo workspace with 4 binary crates and 1 library crate:

```
omnicontext/
├── crates/
│   ├── omni-core/       # Library: Core engine (parser, embedder, index, search)
│   ├── omni-mcp/        # Binary: MCP server (stdio transport)
│   ├── omni-cli/        # Binary: CLI interface
│   └── omni-daemon/     # Binary: Background file watcher
├── docs/                # Architecture docs, ADRs, specs
├── distribution/        # Package manifests (Homebrew, Scoop)
├── editors/vscode/      # VS Code extension
├── tests/               # Integration test fixtures
└── benches/             # Criterion benchmarks
```

## Module Architecture (omni-core)

The core library follows a pipeline architecture with clear module boundaries:

```
omni-core/src/
├── lib.rs               # Public API - expose only what's needed
├── parser/              # AST parsing with tree-sitter
│   ├── registry.rs      # Language registry (add new languages here)
│   └── languages/       # Per-language extractors (python.rs, typescript.rs, etc.)
├── chunker/             # Semantic chunking with token limits
├── embedder/            # ONNX inference + model management
├── index/               # SQLite + FTS5 + vector index (schema.sql)
├── search/              # Hybrid search with RRF fusion
├── graph/               # Dependency graph (petgraph)
├── vector/              # usearch HNSW index wrapper
├── watcher/             # File system event handling
├── pipeline/            # Orchestrates: parse → chunk → embed → index
├── reranker/            # Search result reranking
├── workspace.rs         # Workspace management
├── config.rs            # Configuration handling
├── types.rs             # Shared types across modules
└── error.rs             # Aggregated error types
```

## File Placement Rules

When adding new code, follow these rules:

- New language support: Add to `parser/languages/<lang>.rs` and register in `parser/registry.rs`
- New MCP tools: Add to `omni-mcp/src/tools.rs`
- New CLI commands: Add to `omni-cli/src/main.rs`
- Shared types: Add to `types.rs` if used across modules, otherwise keep in module
- Error types: Define in module, re-export from `error.rs`
- Tests: Unit tests in `#[cfg(test)] mod tests` at bottom of file, integration tests in `tests/`
- Benchmarks: Add to `benches/` using criterion

## Naming Conventions (Rust Standard)

- Modules: `snake_case` (e.g., `model_manager.rs`)
- Types/Structs/Enums: `PascalCase` (e.g., `ChunkKind`, `SearchResult`)
- Functions/Methods: `snake_case` (e.g., `parse_source`, `embed_batch`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `MAX_CHUNK_SIZE`)
- Test functions: `test_<function>_<scenario>_<expected>` (e.g., `test_parse_python_function_returns_correct_chunk_kind`)

## Module Boundaries & Responsibilities

Respect these boundaries to maintain clean architecture:

- `parser`: AST parsing, symbol extraction, language-specific logic
- `chunker`: Semantic chunking, token counting, chunk size limits
- `embedder`: ONNX model loading, batch inference, model downloads
- `index`: SQLite persistence, FTS5 indexing, vector index management
- `search`: Query processing, hybrid search, RRF ranking
- `graph`: Dependency graph construction, traversal, cycle detection
- `watcher`: File system events, debouncing, incremental updates
- `pipeline`: Orchestrates the full indexing flow (parse → chunk → embed → index)

## Visibility Guidelines

- Public API: Only expose through `lib.rs` what external consumers need
- Internal APIs: Use `pub(crate)` for workspace-internal visibility
- Module-private: Default to private, expose only when necessary

## Test Organization

- Unit tests: `#[cfg(test)] mod tests` at bottom of each module file
- Integration tests: `tests/` directory with fixture repositories
- Test fixtures: Located in `tests/fixtures/` (python_project, typescript_project, rust_project, mixed_project, edge_cases)
- Benchmarks: `benches/` using criterion framework

## Documentation References

When working on specific areas, consult these docs:

- Architecture decisions: `docs/ADR.md`
- Testing strategy: `docs/TESTING_STRATEGY.md`
- Language support: `docs/SUPPORTED_LANGUAGES.md`
- Concurrency patterns: `docs/CONCURRENCY_ARCHITECTURE.md`
- Error handling: `docs/ERROR_RECOVERY.md`
- Security: `docs/SECURITY_THREAT_MODEL.md`

## Common Patterns

- Error handling: Use `thiserror` for error types, `anyhow` for application errors
- Async: Use `tokio` runtime, prefer async/await over callbacks
- Logging: Use `tracing` macros (`info!`, `debug!`, `error!`)
- Configuration: Use `serde` for serialization, store in `~/.omnicontext/config.toml`
- Concurrency: Use `DashMap` for concurrent data structures, avoid `Arc<Mutex<T>>` when possible
