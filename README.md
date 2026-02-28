# OmniContext

> Universal Code Context Engine for AI Coding Agents

OmniContext is a **high-performance, locally-runnable code context engine** that gives AI coding agents deep understanding of any codebase. Built in Rust, exposed via the Model Context Protocol (MCP).

[![Status](https://img.shields.io/badge/Status-Alpha-orange)](https://github.com/steeltroops-ai/omnicontext)
[![Version](https://img.shields.io/badge/Version-v0.1.0-blue)](https://github.com/steeltroops-ai/omnicontext/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/steeltroops-ai/omnicontext/deploy.yml?branch=main&label=Build)](https://github.com/steeltroops-ai/omnicontext/actions)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](https://github.com/steeltroops-ai/omnicontext)
[![License](<https://img.shields.io/badge/License-Open%20Core%20(Apache%202.0%20%2F%20Commercial)-blue>)](./LICENSE)

## Tech Stack

[![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![SQLite](https://img.shields.io/badge/SQLite-07405E?logo=sqlite&logoColor=white)](https://www.sqlite.org/)
[![ONNX](https://img.shields.io/badge/ONNX-005CED?logo=onnx&logoColor=white)](https://onnx.ai/)

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

OmniContext uses an **Open-Core** licensing model:

- **Base Engine** (`omni-core`, `omni-mcp`, `omni-cli`): Licensed under [Apache 2.0](./LICENSE).
- **Pro / Enterprise Features**: Licensed under a Custom Commercial License.
