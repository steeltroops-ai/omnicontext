# MCP Tools Reference

OmniContext v1.2.0 exposes **19 tools** through the Model Context Protocol for AI agent integration.

## Tool Catalog

---

### 1. index_repository
**Purpose**: Trigger or re-trigger full indexing of the current repository.

**Parameters**:
- `path` (string, optional): Repository root path. Defaults to the active workspace.
- `force` (boolean, optional, default: `false`): Force a full re-index even if an index already exists.

**Returns**:
- `status`: `"started"` or `"already_running"`
- `message`: Human-readable status description

**Example**:
```json
{
  "path": "/home/user/myproject",
  "force": true
}
```

---

### 2. search_code
**Purpose**: Hybrid semantic + keyword search with graph boosting across the indexed codebase.

**Parameters**:
- `query` (string, required): Natural language or keyword search query
- `limit` (number, optional, default: `10`): Maximum number of results to return

**Returns**: Array of search results, each containing:
- `chunk`: Code content, symbol path, file path, line numbers
- `score`: Relevance score (0–1)
- `score_breakdown`: RRF, reranker, and graph-boost components

**Example**:
```json
{
  "query": "authentication middleware",
  "limit": 5
}
```

---

### 3. get_file_context
**Purpose**: Retrieve semantic context for a specific file, including its symbols, imports, and neighbours in the dependency graph.

**Parameters**:
- `file_path` (string, required): Path to the file, relative to the workspace root
- `max_hops` (number, optional, default: `1`): Dependency graph traversal depth

**Returns**:
- `file`: File metadata and content
- `symbols`: Array of top-level symbols defined in the file
- `imports`: Array of direct imports
- `neighbours`: Files within `max_hops` in the dependency graph

**Example**:
```json
{
  "file_path": "src/auth/middleware.rs",
  "max_hops": 2
}
```

---

### 4. get_symbol_context
**Purpose**: Resolve a fully-qualified symbol and expand its definition, usages, and local dependency neighbourhood.

**Parameters**:
- `symbol` (string, required): Fully-qualified symbol name (e.g. `omni_core::auth::validate_token`)
- `include_usages` (boolean, optional, default: `true`): Include call-sites and references

**Returns**:
- `definition`: Source location and code snippet
- `usages`: Array of call-sites across the codebase
- `upstream`: Symbols this depends on
- `downstream`: Symbols that depend on this

**Example**:
```json
{
  "symbol": "omni_core::auth::validate_token",
  "include_usages": true
}
```

---

### 5. list_files
**Purpose**: List all files tracked in the active index, optionally filtered by glob pattern.

**Parameters**:
- `pattern` (string, optional): Glob filter (e.g. `src/**/*.rs`)
- `limit` (number, optional, default: `100`): Maximum number of file entries to return

**Returns**: Array of file entries with:
- `path`: Relative file path
- `language`: Detected language
- `indexed_at`: ISO 8601 timestamp of last indexing

**Example**:
```json
{
  "pattern": "src/**/*.rs",
  "limit": 50
}
```

---

### 6. get_file_contents
**Purpose**: Return the raw contents of a file from the workspace.

**Parameters**:
- `file_path` (string, required): Path to the file, relative to the workspace root
- `start_line` (number, optional): First line to return (1-indexed)
- `end_line` (number, optional): Last line to return (inclusive)

**Returns**:
- `content`: File content (or the requested slice)
- `total_lines`: Total line count of the file
- `language`: Detected language

**Example**:
```json
{
  "file_path": "src/main.rs",
  "start_line": 1,
  "end_line": 80
}
```

---

### 7. set_workspace
**Purpose**: Switch the active workspace to a different indexed repository.

**Parameters**:
- `path` (string, required): Absolute path to the workspace root

**Returns**:
- `previous`: Previously active workspace path
- `current`: Newly active workspace path

**Example**:
```json
{
  "path": "/home/user/another-project"
}
```

---

### 8. get_workspace
**Purpose**: Return the currently active workspace path and high-level index statistics.

**Parameters**: None

**Returns**:
- `path`: Active workspace root
- `files_indexed`: Number of indexed files
- `last_indexed`: ISO 8601 timestamp of the most recent index run

**Example**:
```json
{}
```

---

### 9. list_indexed_repos
**Purpose**: List all repositories that have been indexed by the running OmniContext instance.

**Parameters**: None

**Returns**: Array of repository entries with:
- `path`: Repository root
- `files_indexed`: File count
- `last_indexed`: ISO 8601 timestamp

**Example**:
```json
{}
```

---

### 10. check_index_status
**Purpose**: Report the health, coverage, and staleness of the current index.

**Parameters**:
- `path` (string, optional): Repository path to check. Defaults to the active workspace.

**Returns**:
- `status`: `"ready"`, `"indexing"`, `"stale"`, or `"empty"`
- `files_indexed`: Total files in the index
- `files_total`: Total files detected in the workspace
- `coverage_percent`: Percentage of files covered
- `last_indexed`: ISO 8601 timestamp

**Example**:
```json
{}
```

---

### 11. search_by_symbol
**Purpose**: Search the index by symbol name across all indexed files.

**Parameters**:
- `name` (string, required): Symbol name or partial name (case-insensitive prefix match)
- `kind` (string, optional): Filter by symbol kind: `"function"`, `"struct"`, `"enum"`, `"trait"`, `"class"`, `"method"`, `"variable"`
- `limit` (number, optional, default: `20`): Maximum results

**Returns**: Array of matching symbols with:
- `symbol`: Fully-qualified symbol path
- `kind`: Symbol kind
- `file_path`: Defining file
- `line`: Definition line number

**Example**:
```json
{
  "name": "validate_token",
  "kind": "function",
  "limit": 10
}
```

---

### 12. get_module_map
**Purpose**: Generate a high-level module-level map of the codebase showing module boundaries and inter-module relationships.

**Parameters**:
- `depth` (number, optional, default: `2`): Module hierarchy depth to expand

**Returns**:
- `modules`: Tree of module nodes with file counts and public API surfaces
- `edges`: Inter-module dependency edges

**Example**:
```json
{
  "depth": 3
}
```

---

### 13. get_dependency_graph
**Purpose**: Return the full or filtered dependency graph for files and symbols.

**Parameters**:
- `file_path` (string, optional): Scope the graph to a specific file and its neighbourhood
- `max_hops` (number, optional, default: `2`): Traversal depth from the focal node
- `edge_types` (array of strings, optional): Filter by edge type: `"IMPORTS"`, `"INHERITS"`, `"CALLS"`, `"INSTANTIATES"`, `"HISTORICAL_CO_CHANGE"`

**Returns**:
- `nodes`: Array of graph nodes (files or symbols)
- `edges`: Array of typed edges
- `focal_node`: The root node if `file_path` was provided

**Example**:
```json
{
  "file_path": "src/auth/middleware.rs",
  "max_hops": 2,
  "edge_types": ["IMPORTS", "CALLS"]
}
```

---

### 14. search_by_pattern
**Purpose**: Search across the indexed codebase using a regex or glob pattern.

**Parameters**:
- `pattern` (string, required): Regular expression or glob pattern
- `file_glob` (string, optional): Limit search to files matching this glob (e.g. `**/*.ts`)
- `limit` (number, optional, default: `20`): Maximum number of matches

**Returns**: Array of matches with:
- `file_path`: Relative file path
- `line`: Line number
- `column`: Column offset
- `snippet`: Surrounding code snippet

**Example**:
```json
{
  "pattern": "TODO|FIXME|HACK",
  "file_glob": "src/**/*.rs",
  "limit": 50
}
```

---

### 15. get_code_context
**Purpose**: Assemble token-optimised context from the index for direct LLM consumption.

**Parameters**:
- `query` (string, required): Context query
- `token_budget` (number, optional, default: `4000`): Maximum tokens to include
- `priority_files` (array of strings, optional): Files to prioritise in context assembly

**Returns**:
- `chunks`: Array of prioritised code chunks
- `total_tokens`: Actual token count
- `files_included`: Number of unique files included
- `truncated`: Whether the budget was reached and content was trimmed

**Priority Levels**: Critical (4), High (3), Medium (2), Low (1)

**Example**:
```json
{
  "query": "how does authentication work",
  "token_budget": 8000,
  "priority_files": ["src/auth/middleware.rs"]
}
```

---

### 16. preflight_check
**Purpose**: Validate the runtime environment and configuration before indexing or serving.

**Parameters**: None

**Returns**:
- `ok`: `true` if all checks passed
- `checks`: Array of individual check results (name, status, message)

Checks performed: binary availability, model file presence, ONNX runtime, IPC socket, workspace writability.

**Example**:
```json
{}
```

---

### 17. clear_cache
**Purpose**: Invalidate the local semantic-search cache, forcing subsequent queries to re-compute results from the index.

**Parameters**:
- `scope` (string, optional, default: `"all"`): `"all"` to clear everything, or `"embeddings"` / `"search"` for targeted eviction

**Returns**:
- `cleared`: Number of cache entries evicted
- `scope`: Scope that was cleared

**Example**:
```json
{
  "scope": "search"
}
```

---

### 18. get_cache_stats
**Purpose**: Report cache hit rate, memory usage, and entry counts.

**Parameters**: None

**Returns**:
- `hit_rate_percent`: Cache hit rate since last reset
- `entries`: Total entries currently held
- `memory_mb`: Estimated memory footprint in MB
- `oldest_entry_age_seconds`: Age of the oldest cached entry

**Example**:
```json
{}
```

---

### 19. shutdown
**Purpose**: Gracefully stop the OmniContext MCP server, flushing in-flight writes and releasing file locks.

**Parameters**:
- `timeout_seconds` (number, optional, default: `5`): Maximum time to wait for in-flight operations to complete before forcing shutdown

**Returns**:
- `status`: `"stopped"` or `"timeout"`

**Example**:
```json
{
  "timeout_seconds": 10
}
```

---

## Integration Examples

### Claude Desktop
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

### Cursor
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

### Kiro
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

---

## Performance Characteristics

| Tool | Typical Latency | Scalability |
|------|----------------|-------------|
| `index_repository` | Background | Any size |
| `search_code` | <50ms (P99) | 100K+ chunks |
| `get_file_context` | <10ms | 10K+ files |
| `get_symbol_context` | <5ms | 100K+ symbols |
| `list_files` | <5ms | Cached |
| `get_file_contents` | <2ms | File-size bound |
| `set_workspace` / `get_workspace` | <1ms | Cached |
| `list_indexed_repos` | <1ms | Cached |
| `check_index_status` | <1ms | Cached |
| `search_by_symbol` | <10ms | 100K+ symbols |
| `get_module_map` | <20ms | 10K+ modules |
| `get_dependency_graph` | <10ms (1-hop) | 10K+ files |
| `search_by_pattern` | <30ms | 100K+ files |
| `get_code_context` | <100ms | 10K+ chunks |
| `preflight_check` | <50ms | — |
| `clear_cache` | <5ms | — |
| `get_cache_stats` | <1ms | Cached |
| `shutdown` | <5s | — |

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

**Common Error Codes**:
- `-32000`: Internal error (index not initialised, file not found)
- `-32001`: Server overloaded (backpressure triggered)
- `-32602`: Invalid parameters
- `-32603`: Internal JSON-RPC error

---

## Best Practices

1. **Use `get_code_context` for LLM queries** — automatically handles token budgets and prioritisation
2. **Combine `search_code` + `get_dependency_graph`** — get both semantic matches and structural relationships
3. **Use `search_by_symbol` for precise lookups** — faster than full-text search when you know the symbol name
4. **Run `preflight_check` on startup** — surfaces configuration problems before the first query
5. **Filter `search_by_pattern` by `file_glob`** — dramatically reduces scan scope on large codebases
6. **Call `get_cache_stats` to tune `clear_cache` frequency** — evict only when hit rate drops below acceptable threshold

---

## See Also

- [Architecture Documentation](./architecture/intelligence/summary.md)
- [API Reference](./api/README.md)
- [Performance Benchmarks](../crates/omni-core/benches/README.md)
