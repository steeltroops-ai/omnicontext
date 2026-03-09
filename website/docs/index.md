---
title: Documentation
description: OmniContext semantic code search engine for AI agents
category: Overview
order: 1
---

# OmniContext Documentation

Natively-compiled semantic code search engine that provides AI agents with structured codebase context through the Model Context Protocol (MCP).

## Quick Links

- [Installation](#installation)
- [MCP Setup](#mcp-setup)
- [API Reference](#api-reference)

## Installation

**Windows**:
```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex
```

**macOS / Linux**:
```bash
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
```

**Package Managers**:
```bash
brew install omnicontext  # macOS
scoop install omnicontext # Windows
cargo binstall omni-cli   # Cross-platform
```

## MCP Setup

Configure your AI client to use OmniContext MCP server.

**Claude Desktop** (`~/Library/Application Support/Claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "omnicontext-mcp",
      "args": []
    }
  }
}
```

**Cursor** (`.cursor/mcp/config.json`):
```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "omnicontext-mcp",
      "args": []
    }
  }
}
```

**Kiro** (`~/.kiro/settings/mcp.json`):
```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "omnicontext-mcp",
      "args": []
    }
  }
}
```

## API Reference

OmniContext exposes 6 MCP tools:

### search_codebase
Hybrid semantic + keyword search with graph boosting.

```json
{
  "query": "authentication middleware",
  "limit": 10
}
```

### get_architectural_context
N-hop dependency neighborhood for architectural understanding.

```json
{
  "file_path": "src/auth/middleware.rs",
  "max_hops": 2
}
```

### get_dependencies
Direct dependencies for a specific symbol.

```json
{
  "symbol_path": "omni_core::auth::validate_token",
  "depth": 1
}
```

### get_commit_context
Relevant commits for understanding code evolution.

```json
{
  "query": "authentication",
  "limit": 10
}
```

### get_workspace_stats
Repository-level statistics and health metrics.

```json
{}
```

### context_window
Token-optimized context assembly for LLM consumption.

```json
{
  "query": "how does authentication work",
  "token_budget": 8000
}
```

## Performance

- **Indexing**: > 500 files/sec
- **Embedding**: > 800 chunks/sec (CPU)
- **Search**: < 50ms P99 (100k chunks)
- **Memory**: < 2KB per chunk

## Supported Languages

JavaScript, TypeScript, Python, Rust, Go, Java, C++, C#, Ruby, PHP, Kotlin, Swift, CSS, HTML, Markdown

## Architecture

- **Parser**: Tree-sitter AST extraction
- **Chunker**: Semantic code chunking
- **Embedder**: ONNX local inference (jina-v2-base-code)
- **Index**: SQLite + HNSW vector search
- **Search**: Hybrid RRF + cross-encoder reranking + graph boost

## Support

- GitHub: [steeltroops-ai/omnicontext](https://github.com/steeltroops-ai/omnicontext)
- Issues: [Report a bug](https://github.com/steeltroops-ai/omnicontext/issues)
- Discord: [Join community](https://discord.gg/omnicontext)
