# OmniContext Tech Stack

## Build System

Cargo workspace (Rust 2021 edition, stable toolchain, minimum version 1.80)

## Core Technologies

- Language: Rust
- Database: SQLite with FTS5 (full-text search)
- Vector Index: usearch (HNSW algorithm, mmap-backed)
- ML Runtime: ONNX Runtime (ort crate)
- Embedding Model: jina-embeddings-v2-base-code (~550MB, auto-downloaded)
- AST Parsing: tree-sitter with language-specific grammars
- Async Runtime: tokio
- Dependency Graph: petgraph
- File Watching: notify + notify-debouncer-mini
- Git Integration: gix
- MCP Protocol: rmcp (stdio and SSE transports)

## Key Dependencies

- Serialization: serde, serde_json, bincode
- Error Handling: thiserror, anyhow
- Logging: tracing, tracing-subscriber
- CLI: clap
- HTTP: axum (enterprise API), reqwest (model downloads)
- Concurrent Data Structures: dashmap
- Configuration: toml, dirs
- Tokenization: tokenizers

## Common Commands

### Build
```bash
# Build all crates
cargo build --release

# Build specific crate
cargo build -p omni-core --release
cargo build -p omni-mcp --release
cargo build -p omni-cli --release
```

### Test
```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p omni-core

# Run integration tests
cargo test --test '*'

# Run with output
cargo test -- --nocapture
```

### Lint & Format
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy -- -D warnings

# Security audit
cargo audit
```

### Benchmarks
```bash
# Run all benchmarks
cargo bench --workspace

# Run specific benchmark
cargo bench --bench search_bench
```

### Run Binaries
```bash
# CLI
cargo run -p omni-cli -- index /path/to/repo
cargo run -p omni-cli -- search "query" --limit 10
cargo run -p omni-cli -- status

# MCP Server
cargo run -p omni-mcp -- --repo /path/to/repo

# Daemon
cargo run -p omni-daemon
```

### Release Build
```bash
# Optimized release build
cargo build --release

# Binaries output to: target/release/
# - omnicontext (CLI)
# - omnicontext-mcp (MCP server)
# - omnicontext-daemon (daemon)
```

## Development Environment

- Rust toolchain via rustup
- ONNX Runtime binaries auto-downloaded via ort crate
- No external services required for local development
- SQLite bundled via rusqlite

## Performance Targets

- Initial index (10k files): <60 seconds
- Incremental re-index: <200ms
- Search latency: <50ms (P99)
- Memory (10k files): <100MB
- Binary size: <50MB
