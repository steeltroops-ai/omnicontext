# OmniContext

> Universal Code Context Engine for AI Coding Agents

OmniContext is a **high-performance, locally-runnable code context engine** that gives AI coding agents deep understanding of any codebase. Built in Rust, exposed via the Model Context Protocol (MCP).

## Quick Start

```bash
# Build from source
cargo build --release

# Index a repository
omnicontext index /path/to/your/project

# Search the codebase
omnicontext search "error handling patterns" --limit 5

# Start MCP server for AI agent integration
omnicontext mcp --repo /path/to/your/project
```

## Architecture

```
omnicontext/
  crates/
    omni-core/     # Core engine (parser, chunker, embedder, index, search, graph)
    omni-mcp/      # MCP server (stdio + SSE transports)
    omni-cli/      # CLI interface
  models/          # ONNX embedding models
  docs/            # Design docs, ADRs, specs
  tests/           # Fixture repos, integration tests
```

## Features

- **Local-first**: All processing happens on your machine. No API keys, no cloud.
- **Universal**: Works with any AI agent via MCP (Claude Code, Copilot, Cursor, Windsurf, Codex)
- **Fast**: Sub-50ms search, <200ms incremental re-indexing
- **Semantic**: Understands code structure, not just text patterns
- **Offline**: Fully functional without internet

## Supported Languages

- Python
- TypeScript / JavaScript
- Rust
- Go

## Documentation

- [Product Specification](docs/OMNICONTEXT_PRODUCT_SPEC.md)
- [Architecture Decisions](docs/ADR.md)
- [Concurrency Architecture](docs/CONCURRENCY_ARCHITECTURE.md)
- [Error Recovery](docs/ERROR_RECOVERY.md)
- [Testing Strategy](docs/TESTING_STRATEGY.md)
- [Security Model](docs/SECURITY_THREAT_MODEL.md)

## License

Apache 2.0
