# OmniContext

> Universal Code Context Engine for AI Coding Agents

OmniContext is a **high-performance, locally-runnable code context engine** that gives AI coding agents deep understanding of any codebase. Built in Rust, exposed via the Model Context Protocol (MCP).

[![Status](https://img.shields.io/badge/Status-Alpha-orange)](https://github.com/steeltroops-ai/omnicontext)
[![Version](https://img.shields.io/badge/Version-v0.1.0-blue)](https://github.com/steeltroops-ai/omnicontext/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/steeltroops-ai/omnicontext/ci.yml?branch=main&label=Build)](https://github.com/steeltroops-ai/omnicontext/actions)
[![Tests](https://img.shields.io/badge/Tests-149%20passing-brightgreen)](https://github.com/steeltroops-ai/omnicontext)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](https://github.com/steeltroops-ai/omnicontext)
[![License](<https://img.shields.io/badge/License-Open%20Core%20(Apache%202.0%20%2F%20Commercial)-blue>)](./LICENSE)


## Tech Stack

[![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![SQLite](https://img.shields.io/badge/SQLite-07405E?logo=sqlite&logoColor=white)](https://www.sqlite.org/)
[![ONNX](https://img.shields.io/badge/ONNX-005CED?logo=onnx&logoColor=white)](https://onnx.ai/)
[![MCP](https://img.shields.io/badge/MCP-Protocol-purple)](https://modelcontextprotocol.io/)

## Installation

See [INSTALL.md](INSTALL.md) for detailed installation instructions.

### Quick Install

**Windows:**
```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install/install.ps1 | iex
```

**macOS/Linux:**
```bash
curl -sSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install/install.sh | bash
```

### Package Managers

```bash
# Homebrew (macOS/Linux)
brew tap steeltroops-ai/omnicontext
brew install omnicontext

# Scoop (Windows)
scoop bucket add omnicontext https://github.com/steeltroops-ai/omnicontext
scoop install omnicontext
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

```text
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

```text
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
- **Pre-Fetch Caching**: VS Code extension intelligently caches context based on your IDE activity (⚡ cache hits, 🔍 fresh searches)

## Supported Languages

- Python
- TypeScript / JavaScript
- Rust
- Go

## VS Code Extension

The OmniContext VS Code extension provides intelligent pre-fetch caching and automatic context injection for AI coding assistants.

> **📸 Screenshot Coming Soon**: VS Code sidebar showing OmniContext cache statistics and controls

### Key Features

- **🚀 Pre-Fetch Caching**: Tracks IDE events (file opens, cursor movements, edits) and pre-fetches relevant context before you ask
- **⚡ Cache Indicators**: Visual indicators show when context is served from cache (instant) vs. fresh search
- **📊 Real-Time Statistics**: Monitor cache hit rate, hits, misses, and size directly in the sidebar
- **🎯 Automatic Injection**: Seamlessly injects cached context into AI chat requests (GitHub Copilot, etc.)
- **🔧 Configurable**: Fine-tune cache size (10-1000 entries), TTL (60-3600s), and debounce timing (50-1000ms)

### Installation

```bash
# Install from VS Code Marketplace (coming soon)
# Or build from source
cd editors/vscode
npm install
npm run compile
```

### Configuration

Configure pre-fetch behavior in VS Code Settings (`Ctrl+,` or `Cmd+,`) → Search "OmniContext":

- `omnicontext.prefetch.enabled` - Enable/disable pre-fetch (default: true)
- `omnicontext.prefetch.cacheSize` - Max cache entries (default: 100, range: 10-1000)
- `omnicontext.prefetch.cacheTtlSeconds` - Cache TTL in seconds (default: 300, range: 60-3600)
- `omnicontext.prefetch.debounceMs` - Event debounce delay (default: 200ms, range: 50-1000)

### Cache Hit Rate Expectations

- **Good (>60%)**: Focused work in specific codebase area
- **Fair (30-60%)**: Exploring different parts of codebase
- **Poor (<30%)**: Rapid context switching - consider increasing cache size or TTL

For detailed documentation, troubleshooting, and configuration guide, see [VS Code Extension README](editors/vscode/README.md).

## Documentation

- [Installation Guide](INSTALL.md) - Complete installation instructions
- [Architecture Decisions](docs/ADR.md) - Design decisions and rationale
- [Concurrency Architecture](docs/CONCURRENCY_ARCHITECTURE.md) - Thread safety and performance
- [Error Recovery](docs/ERROR_RECOVERY.md) - Error handling patterns
- [Testing Strategy](docs/TESTING_STRATEGY.md) - Test coverage and approach
- [Security Model](docs/SECURITY_THREAT_MODEL.md) - Security considerations
- [Supported Languages](docs/SUPPORTED_LANGUAGES.md) - Language support matrix

## License

OmniContext uses an **Open-Core** licensing model:

- **Base Engine** (`omni-core`, `omni-mcp`, `omni-cli`): Licensed under [Apache 2.0](./LICENSE).
- **Pro / Enterprise Features**: Licensed under a [Custom Commercial License](docs/COMMERCIAL_LICENSE.md).
