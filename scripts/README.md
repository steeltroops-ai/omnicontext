# OmniContext Programmatic Wrapper

This directory contains programmatic wrappers for MCP servers that give you full control over context injection and token usage.

## Why Use Programmatic Wrappers?

When you connect MCP servers normally, response bodies are automatically injected into context, often consuming tens of thousands of tokens per response. Programmatic wrappers solve this by:

1. **Controlled Context Injection** - You decide what goes into context
2. **Response Filtering** - Extract only relevant information
3. **Token Budget Control** - Set strict limits on output size
4. **Tool Chaining** - Combine multiple MCP calls in one script
5. **Better Performance** - Research shows agents perform better with programmatic tools

## OmniContext Wrapper

### Installation

```bash
# Make the script executable
chmod +x scripts/omnicontext_wrapper.py

# Install Python dependencies (if needed)
pip install -r requirements.txt  # Currently no external deps needed
```

### Usage

#### As a CLI Tool

```bash
# Get status summary (< 200 tokens)
python scripts/omnicontext_wrapper.py /path/to/repo status

# Search code with filtered results
python scripts/omnicontext_wrapper.py /path/to/repo search "error handling"

# Get symbol info (minimal output)
python scripts/omnicontext_wrapper.py /path/to/repo symbol "Engine"

# Get dependencies graph
python scripts/omnicontext_wrapper.py /path/to/repo deps "crates::omni-core::Engine"

# Full symbol analysis (chained calls)
python scripts/omnicontext_wrapper.py /path/to/repo analyze "Engine"

# Health check
python scripts/omnicontext_wrapper.py /path/to/repo health
```

#### As a Python Library

```python
from scripts.omnicontext_wrapper import OmniContextWrapper

# Initialize wrapper
wrapper = OmniContextWrapper("/path/to/repo")

# Get concise status (< 200 tokens vs 2000+ from raw MCP)
status = wrapper.get_status_summary()
print(status)

# Search with token control
results = wrapper.search_code_filtered(
    query="authentication",
    limit=5,
    max_tokens_per_result=200
)

# Get symbol info without full code dumps
info = wrapper.get_symbol_info("Engine")

# Get dependencies in structured format
deps = wrapper.get_dependencies_graph("Engine", direction="both")

# Context window with strict budget
context = wrapper.context_window_compact(
    query="error handling",
    token_budget=2000  # Hard limit
)

# Chain multiple operations
analysis = wrapper.analyze_symbol_full("Engine")
# Returns: symbol info + dependencies + context in one call

# Search and analyze top results
results = wrapper.search_and_analyze("authentication", top_n=3)
```

### Key Features

#### 1. Token Control

```python
# Raw MCP: 10,000+ tokens
raw_response = mcp_call("search_code", {"query": "auth"})

# Wrapper: < 500 tokens
filtered = wrapper.search_code_filtered("auth", limit=5)
```

#### 2. Response Filtering

The wrapper automatically:
- Removes verbose formatting
- Truncates code snippets
- Extracts only metadata (file paths, line numbers, symbols)
- Limits result counts

#### 3. Chained Operations

```python
# Single call that chains: search → get_symbol → get_dependencies → context_window
analysis = wrapper.analyze_symbol_full("MyClass")
```

#### 4. Structured Output

Instead of markdown text, get structured data:

```python
{
  "query": "Engine",
  "found": 3,
  "symbols": [
    "crates::omni-core::Engine",
    "crates::omni-core::Engine::new",
    "crates::omni-core::Engine::search"
  ]
}
```

## Token Savings Comparison

| Operation | Raw MCP | Wrapper | Savings |
|-----------|---------|---------|---------|
| Status | ~2,000 | ~150 | 92% |
| Search (5 results) | ~8,000 | ~600 | 92% |
| Symbol lookup | ~3,000 | ~200 | 93% |
| Dependencies | ~4,000 | ~400 | 90% |
| Context window | ~15,000 | ~2,000 | 87% |

## Integration with Kiro

### Option 1: Use as a Shell Tool

Add to your Kiro configuration to call the wrapper instead of the MCP directly:

```python
# In your Kiro workflow
result = execute_command(
    "python scripts/omnicontext_wrapper.py . search 'error handling'"
)
```

### Option 2: Create a Custom Kiro Tool

Wrap the Python script in a Kiro tool definition for seamless integration.

### Option 3: Replace MCP Server

Instead of connecting the MCP server, use the wrapper exclusively for all omnicontext operations.

## Advanced Usage

### Custom Filtering

Extend the wrapper with your own filters:

```python
class CustomWrapper(OmniContextWrapper):
    def search_only_rust_files(self, query: str):
        results = self.search_code_filtered(query, limit=20)
        return [r for r in results if r.get("file", "").endswith(".rs")]
```

### Batch Operations

```python
# Analyze multiple symbols efficiently
symbols = ["Engine", "SearchEngine", "Embedder"]
analyses = [wrapper.analyze_symbol_full(s) for s in symbols]
```

### Caching

Add caching to avoid repeated MCP calls:

```python
from functools import lru_cache

@lru_cache(maxsize=100)
def cached_search(query: str):
    return wrapper.search_code_filtered(query)
```

## Best Practices

1. **Always set token budgets** - Use `token_budget` parameters
2. **Filter early** - Extract only what you need from responses
3. **Chain operations** - Combine related calls to reduce overhead
4. **Use structured output** - Parse responses into dicts/lists
5. **Cache results** - Avoid redundant MCP calls

## Troubleshooting

### MCP Executable Not Found

```python
# Specify explicit path
wrapper = OmniContextWrapper(
    repo_path="/path/to/repo",
    mcp_exe_path="/custom/path/to/omnicontext-mcp"
)
```

### Timeout Issues

Increase timeout in `_call_tool`:

```python
result = subprocess.run(..., timeout=60)  # 60 seconds
```

### JSON Parse Errors

The wrapper handles malformed responses gracefully and returns error dicts.

## Future Enhancements

- [ ] Async support for parallel tool calls
- [ ] Response caching with TTL
- [ ] Streaming for large results
- [ ] Integration with other MCP servers
- [ ] Auto-retry on failures
- [ ] Metrics and logging

## Contributing

To add support for more MCP tools:

1. Add parameter class
2. Implement filtered method
3. Add CLI command
4. Update documentation

## License

Same as parent project.
