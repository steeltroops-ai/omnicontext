---
title: Quickstart
description: Get started with OmniContext v1.2.1 in 5 minutes
category: Getting Started
order: 1
---

# Quickstart

Index your first codebase and start serving context to AI agents in under 5 minutes using OmniContext **v1.2.1**.

## Prerequisites

- OmniContext v1.2.1 installed — see [Installation](/docs/getting-started/installation)
- A codebase to index (any supported language)
- Any of the **17 supported AI clients**: Claude Desktop, Claude Code, Cursor, Windsurf, VS Code, VS Code Insiders, Cline, RooCode, Continue.dev, Zed, Kiro, PearAI, Trae, Antigravity, Gemini CLI, Amazon Q CLI, or Augment Code

## Step 1 — Choose and download an embedding model

OmniContext uses a local embedding model for semantic search. Two options are available:

**Default model** — higher accuracy, larger download (~550 MB):

```bash
omnicontext setup model-download
```

**Smaller model** — faster startup, lower memory footprint (~30 MB):

```bash
omnicontext setup model-download --model jina-embeddings-v2-small-en
```

Use the smaller model if you are on a memory-constrained machine or want faster cold-start times. Both models produce high-quality results for code search.

## Step 2 — Index your codebase

Navigate to your project directory and run the indexer:

```bash
cd /path/to/your/project
omnicontext index .
```

The indexer will:
- Detect all supported languages automatically
- Parse AST structures using Tree-sitter
- Generate embeddings with the selected model
- Build an HNSW vector index for fast retrieval
- Store everything in a local `.omnicontext/` directory

For a preview of what will be indexed without writing any data:

```bash
omnicontext index . --dry-run
```

## Step 3 — Start the MCP server

Launch the MCP server to expose tools to AI agents:

```bash
omnicontext-mcp
```

The server listens on stdio by default. For SSE transport:

```bash
omnicontext-mcp --transport sse --port 3000
```

For a full list of available flags:

```bash
omnicontext --help
omnicontext-mcp --help
```

## Step 4 — Configure your AI client

Add OmniContext to your MCP client configuration. Configuration paths by client:

**Claude Desktop** — `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

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

**Cursor / Windsurf / Kiro / PearAI / Trae / Antigravity** — add the same `mcpServers` block to the editor's MCP settings file. Refer to each editor's documentation for the exact config path.

**Gemini CLI / Amazon Q CLI** — pass `--mcp omnicontext-mcp` when launching the agent, or add the server block to the CLI's config file.

Restart your AI client after editing its configuration to load the MCP server.

## Step 5 — Test the integration

Ask your AI agent to search the codebase:

> "Search for authentication logic in my codebase"

The agent will call the `search_codebase` tool, query the local index, and return relevant code snippets with file paths and line numbers.

## Available MCP tools

OmniContext v1.2.1 exposes **19 MCP tools** for semantic code search and analysis:

| Tool | Description |
|------|-------------|
| `index_repository` | Trigger or re-trigger codebase indexing |
| `search_codebase` | Hybrid semantic + keyword search |
| `get_file_context` | Retrieve context for a specific file |
| `get_symbol_context` | Resolve and expand a symbol definition |
| `list_files` | List files in the indexed repository |
| `get_file_contents` | Return raw file contents |
| `set_workspace` | Switch the active workspace |
| `get_workspace` | Return the current workspace path |
| `list_indexed_repos` | List all indexed repositories |
| `check_index_status` | Report index health and coverage |
| `search_by_symbol` | Search by symbol name across the index |
| `get_module_map` | Generate a module-level map of the codebase |
| `get_dependency_graph` | Visualize file and symbol dependencies |
| `search_by_pattern` | Regex / glob pattern search |
| `get_code_context` | Assemble optimized context for LLM consumption |
| `preflight_check` | Validate runtime and configuration |
| `clear_cache` | Invalidate the local cache |
| `get_cache_stats` | Report cache hit rate and memory usage |
| `shutdown` | Gracefully stop the MCP server |

See [MCP Tools Reference](/docs/api-reference/mcp-tools) for full parameter documentation.

## Next steps

- [Configuration](/docs/configuration) — Customize indexing behavior
- [Hybrid Search](/docs/search) — Understand the search engine
- [Dependency Graph](/docs/dependency-graph) — Explore graph analysis
- [MCP Tools Reference](/docs/api-reference/mcp-tools) — Complete tool catalog
