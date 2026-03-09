---
title: Quick Start
description: Get started with OmniContext in 5 minutes
category: Getting Started
order: 2
---

# Quick Start

Index your first codebase and start serving context to AI agents in under 5 minutes.

## Prerequisites

- A codebase to index (any supported language)
- An MCP-compatible AI client (Claude Desktop, Cursor, etc.)

## Step 1: Install OmniContext

Choose your platform:

**Windows (PowerShell)**:
```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex
```

**macOS / Linux (Bash)**:
```bash
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
```

**Package Managers**:
```bash
# macOS (Homebrew)
brew tap steeltroops-ai/omnicontext
brew install omnicontext

# Windows (Scoop)
scoop bucket add omnicontext https://github.com/steeltroops-ai/omnicontext
scoop install omnicontext

# Cross-platform (Cargo)
cargo binstall omni-cli
```

## Step 2: Verify Installation

```bash
omnicontext --version
omnicontext-mcp --version
```

You should see version numbers for both binaries.

## Step 3: Index Your Codebase

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

**Expected output**:
```
[info] Scanning workspace...
[info] Parsing AST via Tree-sitter (Rust, TypeScript, Python)
[info] Generating embeddings (ONNX local, jina-v2-base-code)
[info] Building dependency graph
Done. Indexed 42,104 symbols in 2.1s
```

## Step 4: Test Search

Try a semantic search:

```bash
omnicontext search "authentication logic" --limit 5
```

You should see relevant code snippets with scores and file paths.

## Step 5: Start MCP Server

Launch the MCP server to expose tools to AI agents:

```bash
omnicontext-mcp
```

The server listens on stdio by default. For SSE transport:

```bash
omnicontext-mcp --transport sse --port 3000
```

## Step 6: Configure Your AI Client

Add OmniContext to your MCP client configuration.

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
      "args": [],
      "env": {}
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

Restart your AI client to load the MCP server.

## Step 7: Test Integration

Ask your AI agent to search your codebase:

> "Search for authentication logic in my codebase"

The agent will use the `search_codebase` tool to query your index and return relevant code snippets with context.

## What's Next?

- [Installation Guide](/docs/installation) - Detailed installation instructions for all platforms
- [MCP Server Setup](/docs/mcp-server-setup) - Configure MCP for your AI client
- [Available Tools](/docs/available-tools) - Learn about all 6 MCP tools
- [Integration Guides](/docs/integration-guides) - Client-specific setup instructions

## Troubleshooting

**Index not found**:
```bash
# Re-run indexing
omnicontext index .
```

**MCP server not connecting**:
```bash
# Check server is running
ps aux | grep omnicontext-mcp

# Check logs
tail -f ~/.omnicontext/logs/mcp.log
```

**Slow indexing**:
```bash
# Use more threads
omnicontext index . --threads 8
```

For more help, see [Troubleshooting](/docs/troubleshooting) or join our [Discord](https://discord.gg/omnicontext).
