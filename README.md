# OmniContext

> Universal Code Context Engine for AI Coding Agents

OmniContext is a **high-performance, locally-runnable code context engine** that gives AI coding agents deep understanding of any codebase. Built in Rust, exposed via the Model Context Protocol (MCP).

[![Status](https://img.shields.io/badge/Status-Alpha-orange)](https://github.com/steeltroops-ai/omnicontext)
[![Version](https://img.shields.io/badge/Version-v0.1.0-blue)](https://github.com/steeltroops-ai/omnicontext/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/steeltroops-ai/omnicontext/ci.yml?branch=main&label=Build)](https://github.com/steeltroops-ai/omnicontext/actions)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](https://github.com/steeltroops-ai/omnicontext)
[![License](<https://img.shields.io/badge/License-Open%20Core%20(Apache%202.0%20%2F%20Commercial)-blue>)](./LICENSE)
[![Tests](https://img.shields.io/badge/Tests-149%20passing-brightgreen)](https://github.com/steeltroops-ai/omnicontext)

## Tech Stack

[![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![SQLite](https://img.shields.io/badge/SQLite-07405E?logo=sqlite&logoColor=white)](https://www.sqlite.org/)
[![ONNX](https://img.shields.io/badge/ONNX-005CED?logo=onnx&logoColor=white)](https://onnx.ai/)
[![MCP](https://img.shields.io/badge/MCP-Protocol-purple)](https://modelcontextprotocol.io/)

## Installation

### One-Line Install (Recommended)

The easiest way to install OmniContext and auto-download the embedding AI model.

**macOS / Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install/install.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install/install.ps1 | iex
```

### From Source

```bash
# Clone and build
git clone https://github.com/steeltroops-ai/omnicontext.git
cd omnicontext
cargo build --release

# Binaries are in target/release/
# - omnicontext       (CLI)
# - omnicontext-mcp   (MCP server)
```

### Package Managers

```bash
# macOS (Homebrew)
brew tap steeltroops-ai/omnicontext
brew install omnicontext

# Windows (Scoop)
scoop bucket add omnicontext https://github.com/steeltroops-ai/scoop-omnicontext
scoop install omnicontext

# Rust (Cargo)
cargo install --path crates/omni-cli
cargo install --path crates/omni-mcp
```

### Docker

```bash
docker run -v /path/to/repo:/repo steeltroops/omnicontext:latest
```

## Quick Start

```bash
# Index a repository
omnicontext index /path/to/your/project

# Search the codebase
omnicontext search "error handling patterns" --limit 5

# Show index status
omnicontext status

# Start MCP server for AI agent integration
omnicontext-mcp --repo /path/to/your/project
```

## Embedding Model (Auto-Managed)

OmniContext uses **jina-embeddings-v2-base-code** for semantic search -- a model specifically trained on code retrieval tasks.

**First-run behavior**: The engine automatically downloads the model (~550MB) on first use and caches it permanently in `~/.omnicontext/models/`. No manual setup needed.

```bash
# Use a lighter model (~130MB, general-purpose)
OMNI_EMBEDDING_MODEL=small omnicontext index .

# Skip model download entirely (keyword-only search)
OMNI_SKIP_MODEL_DOWNLOAD=1 omnicontext index .

# Point to a custom ONNX model
OMNI_MODEL_PATH=/path/to/custom/model.onnx omnicontext index .
```

## MCP Integration

The MCP server **auto-indexes** the repository on first connection. No manual `index` step needed.

### Claude Code

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "omnicontext-mcp",
      "args": ["--repo", "/path/to/your/project"]
    }
  }
}
```

### VS Code / Antigravity / Cursor

Add to your MCP settings:

```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "omnicontext-mcp",
      "args": ["--repo", "."]
    }
  }
}
```

### Available MCP Tools

| Tool               | Description                                                |
| ------------------ | ---------------------------------------------------------- |
| `search_code`      | Hybrid search (keyword + semantic) with ranked results     |
| `get_symbol`       | Symbol lookup by name or fully qualified name              |
| `get_file_summary` | Structural summary of a file (exports, classes, functions) |
| `get_dependencies` | Upstream/downstream dependency graph traversal             |
| `find_patterns`    | Find recurring code patterns across the codebase           |
| `get_architecture` | High-level codebase architecture overview                  |
| `explain_codebase` | Comprehensive project explanation for onboarding           |
| `get_status`       | Engine status and index statistics                         |

## Architecture

```
omnicontext/
  crates/
    omni-core/     # Core engine (parser, chunker, embedder, index, search, graph)
    omni-mcp/      # MCP server (stdio transport, auto-index)
    omni-cli/      # CLI interface (index, search, status, config)
  distribution/    # Package manifests (Homebrew, Scoop)
  editors/         # Editor extensions (VS Code)
  docs/            # Design docs, ADRs, specs
```

### Engine Pipeline

```
watcher --> parser --> chunker --> embedder --> index
               |                                 |
               v                                 v
             graph <----- search <------------ query
```

## Features

- **Local-first**: All processing happens on your machine. No API keys, no cloud.
- **Universal**: Works with any AI agent via MCP (Claude Code, Copilot, Cursor, Windsurf, Antigravity)
- **Fast**: Sub-50ms search, <200ms incremental re-indexing
- **Semantic**: Code-trained embeddings understand structure, not just text patterns
- **Zero-hassle**: Auto-downloads model, auto-indexes on MCP startup
- **Offline**: Fully functional without internet (after initial model download)

## Supported Languages

- Python
- TypeScript / JavaScript
- Rust
- Go

## Documentation

- [Product Specification](docs/local/OMNICONTEXT_PRODUCT_SPEC.md)
- [Development Roadmap](docs/local/DEVELOPMENT_ROADMAP.md)
- [Architecture Decisions](docs/ADR.md)
- [Concurrency Architecture](docs/CONCURRENCY_ARCHITECTURE.md)
- [Error Recovery](docs/ERROR_RECOVERY.md)
- [Testing Strategy](docs/TESTING_STRATEGY.md)
- [Security Model](docs/SECURITY_THREAT_MODEL.md)
- [Embedding Model Evaluation](docs/local/EMBEDDING_MODEL_EVALUATION.md)

## License

OmniContext uses an **Open-Core** licensing model:

- **Base Engine** (`omni-core`, `omni-mcp`, `omni-cli`): Licensed under [Apache 2.0](./LICENSE).
- **Pro / Enterprise Features**: Licensed under a [Custom Commercial License](./COMMERCIAL_LICENSE.md).
