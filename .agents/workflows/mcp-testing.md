---
description: How to test MCP server tools end-to-end
---

# MCP Tool Testing Workflow

## Prerequisites

- OmniContext built in debug mode
- A reference test repository indexed
- MCP Inspector tool (optional but recommended)

## Steps

### 1. Start the MCP Server in Debug Mode

```bash
# stdio transport
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run -p omni-mcp -- --repo tests/fixtures/python_project

# SSE transport
cargo run -p omni-mcp -- --transport sse --port 3179 --repo tests/fixtures/python_project
```

### 2. Verify Tool Registration

Send `tools/list` request:

```json
{ "jsonrpc": "2.0", "method": "tools/list", "id": 1 }
```

Expected: All 8 tools returned with correct schemas.

### 3. Test Each Tool

#### search_code

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "search_code",
    "arguments": {
      "query": "error handling",
      "limit": 5
    }
  },
  "id": 2
}
```

Verify:

- Returns <= 5 results
- Results are relevant to "error handling"
- Each result has: content, file_path, symbol_path, score

#### get_symbol

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "get_symbol",
    "arguments": {
      "name": "main"
    }
  },
  "id": 3
}
```

Verify:

- Returns the `main` function definition
- Includes doc comment if present
- Includes file path and line numbers

#### get_dependencies

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "get_dependencies",
    "arguments": {
      "symbol": "main",
      "depth": 2,
      "direction": "downstream"
    }
  },
  "id": 4
}
```

#### get_file_summary / find_patterns / get_architecture / get_recent_changes / explain_codebase

Test each similarly with appropriate parameters.

### 4. Error Handling Tests

```json
// Non-existent symbol
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "get_symbol",
    "arguments": { "name": "does_not_exist_xyz" }
  },
  "id": 10
}
```

Verify: Returns graceful error or empty result, not crash.

### 5. Concurrent Request Test

Send multiple requests simultaneously and verify:

- No deadlocks
- All responses are correct
- Search during active indexing works

### 6. Using MCP Inspector

```bash
npx @modelcontextprotocol/inspector -- cargo run -p omni-mcp -- --repo tests/fixtures/python_project
```

This provides a web UI to interactively test all tools.

## Automated Test Suite

Run the full MCP integration test suite:

```bash
cargo test -p omni-mcp --test mcp_integration
```
