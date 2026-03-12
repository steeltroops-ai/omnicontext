---
title: MCP Tools
description: Complete API reference for OmniContext's 19 MCP tools for AI agent integration
category: API Reference
order: 10
---

# MCP Tools

OmniContext exposes **19 tools** through the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) that give AI agents structured, high-fidelity access to your codebase. All tools communicate over stdio and work locally without any external API calls.

---

## Starting the MCP Server

### From the CLI (recommended)

```bash
omnicontext mcp --repo /path/to/your/project
# or, from within the project directory:
omnicontext mcp --repo .
```

### Direct binary invocation

```bash
omnicontext-mcp --repo /path/to/your/project
# Skip auto-index on startup (use existing index only):
omnicontext-mcp --repo . --no-auto-index
```

The MCP server automatically indexes the repository on first startup if no existing index is found.

---

## Tool Catalog

### 1. `search_code`

**Purpose**: Hybrid semantic + keyword search over the indexed codebase. Returns ranked code chunks with file paths, scores, and source content.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `query` | string | ✓ | — | Natural language or keyword query (e.g., `"authentication middleware"`, `"validate_token"`) |
| `limit` | integer | — | 10 | Maximum number of results to return (max 200) |
| `min_rerank_score` | number | — | 0.0 | Minimum reranker score threshold (0.0–1.0) |

**Returns**: Ranked code chunks with file path, symbol path, line numbers, optional doc comment, and source code.

**Example**:
```json
{ "query": "JWT authentication middleware", "limit": 5 }
```

---

### 2. `context_window`

**Purpose**: Assembles a token-budget-aware context window for a query. Groups results by file, pulls in graph-neighbor definitions, and fits optimally within the specified token budget. Use this for maximum relevant context when understanding or modifying code.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `query` | string | ✓ | — | The topic or task to gather context for |
| `limit` | integer | — | 20 | Maximum number of chunks to retrieve |
| `token_budget` | integer | — | 8192 | Maximum tokens to include in the assembled context |
| `min_rerank_score` | number | — | 0.0 | Minimum reranker score threshold |
| `shadow_headers` | boolean | — | false | Include shadow header definitions from graph neighbors |

**Returns**: A formatted context window with token counts and file groupings.

**Example**:
```json
{ "query": "how does the authentication flow work", "token_budget": 8000 }
```

---

### 3. `get_symbol`

**Purpose**: Look up a specific code symbol by its fully qualified name or search by name prefix. Returns the complete definition with documentation.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `name` | string | ✓ | — | Fully qualified symbol name (e.g., `"auth::validate_token"`) or name prefix (e.g., `"UserService"`) |
| `limit` | integer | — | 5 | Maximum prefix-match results if the exact name is not found |

**Returns**: Symbol kind, file, line number, doc comment, and full source code.

**Example**:
```json
{ "name": "auth::validate_token" }
```

---

### 4. `get_file_summary`

**Purpose**: Returns a structural summary of a file — its exported symbols, classes, functions, and chunks — without reading the raw file content.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `path` | string | ✓ | — | File path relative to the repository root (e.g., `"src/auth/middleware.rs"`) |

**Returns**: File language, size, and a list of all indexed chunks with their kinds, symbol paths, and line ranges.

**Example**:
```json
{ "path": "src/auth/middleware.rs" }
```

---

### 5. `get_status`

**Purpose**: Returns the current state of the OmniContext engine: indexed file counts, chunks, symbols, vectors, embedding coverage, search mode, and dependency graph metrics.

**Parameters**: None.

**Returns**: Engine statistics, language distribution, and diagnostic hints (e.g., warnings when embedding coverage is low).

**Example**:
```json
{}
```

---

### 6. `get_dependencies`

**Purpose**: Returns the dependency relationships for a symbol: what it depends on (upstream) and what depends on it (downstream), using the dependency graph built during indexing.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `symbol` | string | ✓ | — | Fully qualified symbol name or prefix |
| `direction` | string | — | `"both"` | `"upstream"`, `"downstream"`, or `"both"` |

**Returns**: Upstream and downstream symbol lists with file locations.

**Example**:
```json
{ "symbol": "omni_core::auth::validate_token", "direction": "both" }
```

---

### 7. `find_patterns`

**Purpose**: Finds code patterns by combining keyword and semantic search. Useful for locating similar implementations, idioms, or constructs across the codebase.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `pattern` | string | ✓ | — | Pattern description (e.g., `"error handling"`, `"API endpoint handlers"`) |
| `limit` | integer | — | 5 | Maximum number of examples to return |

**Returns**: Multiple example matches with file path, symbol, line range, and source code.

**Example**:
```json
{ "pattern": "retry with exponential backoff", "limit": 5 }
```

---

### 8. `get_architecture`

**Purpose**: Provides a high-level overview of the codebase architecture — file structure, language distribution, module relationships, and dependency graph statistics. No parameters needed.

**Parameters**: None.

**Returns**: Index statistics, language breakdown, dependency graph summary, and top-level file tree.

**Example**:
```json
{}
```

---

### 9. `get_module_map`

**Purpose**: Returns the module/crate/package hierarchy as a tree structure. Files are grouped by directory with their exported symbols. Useful for navigating large codebases.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `max_depth` | integer | — | unlimited | Maximum directory depth to include in the map |

**Returns**: A tree-structured module map showing directories, files, languages, and top-level symbols.

**Example**:
```json
{ "max_depth": 3 }
```

---

### 10. `search_by_intent`

**Purpose**: Natural language search with automatic query expansion. Classifies the intent of the query (edit, explain, debug, refactor) and expands it with synonyms and related terms for better recall. Returns a pre-assembled context window.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `query` | string | ✓ | — | Natural language task or question |
| `limit` | integer | — | 10 | Maximum number of chunks to retrieve |
| `token_budget` | integer | — | 8192 | Token budget for the assembled context |
| `shadow_headers` | boolean | — | false | Include shadow header definitions |

**Returns**: Intent classification, expanded query, and a token-optimized context window.

**Example**:
```json
{ "query": "refactor the database connection pool", "token_budget": 6000 }
```

---

### 11. `get_blast_radius`

**Purpose**: Analyzes the impact of changing a symbol. Traverses the dependency graph to find all code that would be transitively affected — answers the question: *"What breaks if I change this?"*

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `symbol` | string | ✓ | — | Fully qualified symbol name or prefix |
| `max_depth` | integer | — | 5 | Maximum traversal depth |

**Returns**: Symbols affected at each depth level, sorted by proximity.

**Example**:
```json
{ "symbol": "UserService::authenticate", "max_depth": 3 }
```

---

### 12. `get_recent_changes`

**Purpose**: Returns recently modified files from git history, showing which files were changed, added, or deleted in recent commits. Useful for understanding what is actively being developed.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `commit_count` | integer | — | 10 | Number of recent commits to inspect (max 100) |
| `include_diff` | boolean | — | false | Include diff hunks in the output |

**Returns**: Commit list with hashes, authors, timestamps, messages, AI-generated summaries, and files changed. Also shows current uncommitted changes.

**Example**:
```json
{ "commit_count": 20, "include_diff": false }
```

---

### 13. `get_call_graph`

**Purpose**: Returns the call graph for a symbol — what it calls (upstream) and what calls it (downstream) — with typed edges (Calls, Imports, Extends, Implements). Can render as a Mermaid diagram.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `symbol` | string | ✓ | — | Fully qualified symbol name or prefix |
| `depth` | integer | — | 2 | Traversal depth in both directions |
| `mermaid` | boolean | — | false | Render output as a Mermaid diagram |

**Returns**: Direct edges, upstream dependencies, and downstream dependents. Optionally a Mermaid `graph TD` diagram.

**Example**:
```json
{ "symbol": "api::handlers::login", "depth": 2, "mermaid": true }
```

---

### 14. `get_branch_context`

**Purpose**: Returns the current git branch status — uncommitted and unpushed changes, diff hunks, and all files modified on this branch relative to the base branch. Helps agents give branch-aware suggestions.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `include_diffs` | boolean | — | false | Include full diff hunk content |

**Returns**: Branch name, base branch, lists of uncommitted and unpushed files, total lines changed, and optionally full diff hunks.

**Example**:
```json
{ "include_diffs": true }
```

---

### 15. `get_co_changes`

**Purpose**: Finds files that frequently change together with a given file, based on git commit history. Discovers hidden coupling between files that are not connected through import or call dependencies.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `file_path` | string | ✓ | — | File path relative to the repository root |
| `min_frequency` | integer | — | 2 | Minimum number of shared commits to include a partner |
| `limit` | integer | — | 10 | Maximum number of co-change partners to return |

**Returns**: Files that co-change with the target, sorted by co-change frequency.

**Example**:
```json
{ "file_path": "src/auth/middleware.rs", "min_frequency": 3, "limit": 10 }
```

---

### 16. `audit_plan`

**Purpose**: Analyzes a task plan for architectural risks, blast radius, co-change warnings, and breaking changes. Send a plan description and receive a structural critique with risk levels and recommendations.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `plan` | string | ✓ | — | Task plan text (markdown or plain text, max 500 000 characters) |
| `max_depth` | integer | — | 3 | Graph traversal depth for blast-radius analysis |

**Returns**: A Markdown critique with identified symbols, risk levels, blast radius estimates, and co-change warnings.

**Example**:
```json
{
  "plan": "Refactor UserService.authenticate to use the new JWT library.\nUpdate all callers to pass the new token format.",
  "max_depth": 3
}
```

---

### 17. `explain_codebase`

**Purpose**: Provides a comprehensive, human-readable explanation of the codebase — tech stack, languages, entry points, and top-level directory structure. Ideal for onboarding to an unfamiliar project.

**Parameters**: None.

**Returns**: A Markdown overview with statistics, language distribution, top-level directory structure, and entry point hints.

**Example**:
```json
{}
```

---

### 18. `set_workspace`

**Purpose**: Switches the MCP server to a different repository or workspace path at runtime. Use this when the server was started with the wrong directory, or when you need to query multiple projects in a single session.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `path` | string | ✓ | — | Absolute path to the new repository root |
| `auto_index` | boolean | — | true | Whether to automatically index the new workspace if it has no existing index |

**Returns**: Confirmation of the new workspace path and indexing status.

**Example**:
```json
{ "path": "/home/user/projects/my-api", "auto_index": true }
```

---

### 19. `generate_manifest`

**Purpose**: Auto-generates a `CLAUDE.md` project guide or `.context_map.json` from live index data. Use `"claude"` format for a human/AI-readable project overview, `"json"` for structured data, or `"both"`.

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `format` | string | ✓ | — | `"claude"`, `"json"`, or `"both"` |

**Returns**: Generated manifest content as a string.

**Example**:
```json
{ "format": "both" }
```

---

## IDE / Agent Integration Examples

### Claude Desktop

Config file location:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **Linux**: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "/path/to/omnicontext-mcp",
      "args": ["--repo", "."],
      "env": {}
    }
  }
}
```

### Claude Code

Config file: `~/.claude.json` (user-scoped MCP servers).

> **Important**: `~/.claude/settings.json` is for permissions and environment settings, **not** MCP server registration. Use `~/.claude.json` for MCP servers.

```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "/path/to/omnicontext-mcp",
      "args": ["--repo", "."]
    }
  }
}
```

### Cursor

Config file: `~/.cursor/mcp.json` (Linux/macOS) or `%APPDATA%\Cursor\User\mcp.json` (Windows).

```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "/path/to/omnicontext-mcp",
      "args": ["--repo", "."],
      "env": {}
    }
  }
}
```

### Windsurf

Config file: `~/.codeium/windsurf/mcp_config.json`

```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "/path/to/omnicontext-mcp",
      "args": ["--repo", "."]
    }
  }
}
```

### VS Code

Config file: `%APPDATA%\Code\User\mcp.json` (Windows) or `~/.config/Code/User/mcp.json` (Linux).

> **Important**: VS Code uses the key `"servers"` (not `"mcpServers"`) in its `mcp.json`.

```json
{
  "servers": {
    "omnicontext": {
      "command": "/path/to/omnicontext-mcp",
      "args": ["--repo", "."]
    }
  }
}
```

### Zed

Config file: `~/.config/zed/settings.json` (uses `"context_servers"` key).

```json
{
  "context_servers": {
    "omnicontext": {
      "command": "/path/to/omnicontext-mcp",
      "args": ["--repo", "."]
    }
  }
}
```

### Automatic Setup (All IDEs)

Instead of manually editing each config file, run the setup command to configure all installed IDEs at once:

```bash
omnicontext setup --all
```

---

## `OMNICONTEXT_REPO` Environment Variable

When an IDE launches the MCP server from a different working directory than your project, use the `OMNICONTEXT_REPO` environment variable to specify the correct path:

```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "/path/to/omnicontext-mcp",
      "args": ["--repo", "."],
      "env": {
        "OMNICONTEXT_REPO": "/absolute/path/to/your/project"
      }
    }
  }
}
```

---

## Performance Characteristics

| Tool | Typical Latency | Notes |
|------|----------------|-------|
| `search_code` | < 50 ms (P99) | 100 K+ chunks |
| `context_window` | < 100 ms | Includes graph neighbor enrichment |
| `get_symbol` | < 5 ms | Direct index lookup |
| `get_file_summary` | < 5 ms | Metadata-only, no file I/O |
| `get_status` | < 1 ms | Cached statistics |
| `get_dependencies` | < 5 ms | Graph traversal |
| `find_patterns` | < 50 ms | Reuses search pipeline |
| `get_architecture` | < 20 ms | Aggregated metadata |
| `get_module_map` | < 20 ms | Metadata traversal |
| `search_by_intent` | < 100 ms | Query expansion + context window |
| `get_blast_radius` | < 10 ms (1-hop) | Graph BFS |
| `get_recent_changes` | < 20 ms | Git log via indexed commits |
| `get_call_graph` | < 10 ms | Graph traversal |
| `get_branch_context` | < 50 ms | Git diff + graph |
| `get_co_changes` | < 20 ms | Commit history analysis |
| `audit_plan` | < 200 ms | Multi-symbol blast radius |
| `generate_manifest` | < 500 ms | Full index scan |

---

## Error Handling

All tools return standard MCP error responses:

```json
{
  "error": {
    "code": -32000,
    "message": "Index not initialized. Run 'omnicontext index .' first."
  }
}
```

**Common error codes**:
| Code | Meaning |
|------|---------|
| `-32000` | Internal error (index not initialized, file not found) |
| `-32001` | Server overloaded (backpressure triggered) |
| `-32602` | Invalid parameters |
| `-32603` | Internal JSON-RPC error |

---

## Best Practices

1. **Use `context_window` for LLM queries**: It automatically handles token budgets, prioritizes high-relevance chunks, and pulls in graph-neighbor definitions.
2. **Use `search_by_intent` for task-driven queries**: Intent classification and query expansion improve recall for ambiguous or high-level questions.
3. **Combine `search_code` with `get_blast_radius`**: Get relevant results then understand impact before making changes.
4. **Use `audit_plan` before large refactors**: Identify architectural risks before writing a single line.
5. **Check `get_status` on first connection**: Verify embedding coverage and index completeness before issuing search queries.
6. **Use `get_co_changes` alongside `get_dependencies`**: Import graphs show structural coupling; co-change analysis reveals behavioral coupling not captured by static analysis.
7. **Use `get_branch_context` for PR reviews**: Quickly understand what a developer has been working on and which files are in flight.
