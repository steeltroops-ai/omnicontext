# MCP Tools Documentation

OmniContext exposes 6 tools through the Model Context Protocol for AI agent integration.

## Tool Catalog

### 1. search_codebase
**Purpose**: Hybrid semantic + keyword search with graph boosting

**Parameters**:
- `query` (string, required): Search query
- `limit` (number, optional, default: 10): Max results

**Returns**: Array of search results with:
- `chunk`: Code content, symbol path, file path, line numbers
- `score`: Relevance score (0-1)
- `score_breakdown`: RRF, reranker, graph boost components

**Example**:
```json
{
  "query": "authentication middleware",
  "limit": 5
}
```

---

### 2. get_architectural_context
**Purpose**: N-hop dependency neighborhood for architectural understanding

**Parameters**:
- `file_path` (string, required): Target file path
- `max_hops` (number, optional, default: 2): Traversal depth

**Returns**: Architectural context with:
- `focal_file`: Target file
- `neighbors`: Array of related files with distance, edge types, importance
- `total_files`: Total files in neighborhood
- `max_hops`: Actual traversal depth

**Edge Types**: IMPORTS, INHERITS, CALLS, INSTANTIATES, HISTORICAL_CO_CHANGE

**Example**:
```json
{
  "file_path": "src/auth/middleware.rs",
  "max_hops": 2
}
```

---

### 3. get_dependencies
**Purpose**: Direct dependencies for a specific symbol

**Parameters**:
- `symbol_path` (string, required): Fully qualified symbol path
- `depth` (number, optional, default: 1): Traversal depth

**Returns**: Dependency information with:
- `upstream`: Symbols this depends on
- `downstream`: Symbols that depend on this
- `depth`: Actual traversal depth

**Example**:
```json
{
  "symbol_path": "omni_core::auth::validate_token",
  "depth": 1
}
```

---

### 4. get_commit_context
**Purpose**: Relevant commits for understanding code evolution

**Parameters**:
- `query` (string, optional): Search query for commit messages
- `file_paths` (array of strings, optional): Filter by files
- `limit` (number, optional, default: 20): Max commits

**Returns**: Array of commits with:
- `hash`: Git commit hash
- `message`: Commit message
- `author`: Author name
- `timestamp`: ISO 8601 timestamp
- `summary`: Generated summary (e.g., "feat affecting 3 files. +50 -10 lines")
- `files_changed`: Array of file paths

**Example**:
```json
{
  "query": "authentication",
  "limit": 10
}
```

---

### 5. get_workspace_stats
**Purpose**: Repository-level statistics and health metrics

**Parameters**: None

**Returns**: Workspace statistics with:
- `files_indexed`: Total files
- `chunks_indexed`: Total chunks
- `vectors_indexed`: Total embeddings
- `embedding_coverage_percent`: Coverage percentage
- `search_mode`: "hybrid", "keyword_only", or "vector_only"
- `graph_nodes`: Dependency graph nodes
- `graph_edges`: Dependency graph edges
- `last_indexed`: ISO 8601 timestamp

**Example**:
```json
{}
```

---

### 6. context_window
**Purpose**: Optimized context assembly for LLM consumption

**Parameters**:
- `query` (string, required): Context query
- `token_budget` (number, optional, default: 4000): Max tokens
- `priority_files` (array of strings, optional): High-priority files

**Returns**: Token-optimized context with:
- `chunks`: Array of prioritized chunks
- `total_tokens`: Actual token count
- `files_included`: Number of files
- `truncated`: Whether context was truncated

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
| search_codebase | <50ms (P99) | 100K+ chunks |
| get_architectural_context | <10ms (1-hop) | 10K+ files |
| get_dependencies | <5ms | 100K+ symbols |
| get_commit_context | <20ms | 1000+ commits |
| get_workspace_stats | <1ms | Cached |
| context_window | <100ms | 10K+ chunks |

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
- `-32000`: Internal error (index not initialized, file not found)
- `-32001`: Server overloaded (backpressure triggered)
- `-32602`: Invalid parameters
- `-32603`: Internal JSON-RPC error

---

## Best Practices

1. **Use context_window for LLM queries**: Automatically handles token budgets and prioritization
2. **Combine search + architectural context**: Get both semantic matches and structural relationships
3. **Filter commit context by files**: Reduce noise when investigating specific changes
4. **Set appropriate limits**: Start with 10 results, increase if needed
5. **Cache workspace stats**: Call once per session, not per query
6. **Use priority_files for focused context**: Ensures critical files are included

---

## See Also

- [Architecture Documentation](./architecture/intelligence/summary.md)
- [API Reference](./api/README.md)
- [Performance Benchmarks](../crates/omni-core/benches/README.md)
