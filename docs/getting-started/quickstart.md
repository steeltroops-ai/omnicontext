---
title: Quickstart
description: Get started with OmniContext in 5 minutes
category: Getting Started
order: 1
---

# Quickstart

Index your first codebase and start serving context to AI agents in under 5 minutes.

## Prerequisites

- Rust 1.80+ installed
- A codebase to index (any supported language)
- An MCP-compatible AI client (Claude Desktop, Cursor, etc.)

## Index your codebase

Navigate to your project directory and run the indexer:

```bash
cd /path/to/your/project
omnicontext index .
```

The indexer will:
- Detect all supported languages automatically
- Parse AST structures using tree-sitter
- Generate embeddings with jina-embeddings-v2-base-code
- Build HNSW vector index for fast retrieval
- Store everything in `.omnicontext/` directory

## Start the MCP server

Launch the MCP server to expose tools to AI agents:

```bash
omnicontext-mcp
```

The server listens on stdio by default. For SSE transport:

```bash
omnicontext-mcp --transport sse --port 3000
```

## Configure your AI client

Add OmniContext to your MCP client configuration. For Claude Desktop, edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

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

Restart Claude Desktop to load the MCP server.

## Test the integration

Ask Claude to search your codebase:

> "Search for authentication logic in my codebase"

Claude will use the `search_code` tool to query your index and return relevant code snippets with context.

## Available MCP tools

OmniContext exposes 16 MCP tools for semantic code search:

- `search_code` - Hybrid search across codebase
- `context_window` - Assemble context for specific files
- `get_symbol` - Resolve symbol definitions
- `get_dependencies` - Analyze dependency graph
- `get_architecture` - Generate architecture maps
- `get_recent_changes` - Track git history

See [MCP Tools](/docs/api-reference/mcp-tools) for complete reference.

## Next steps

- [Configuration](/docs/configuration) - Customize indexing behavior
- [Hybrid Search](/docs/search) - Understand the search engine
- [Dependency Graph](/docs/dependency-graph) - Explore graph analysis
