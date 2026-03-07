# OmniContext API Documentation

This directory contains the formal API contracts, command-line interface specifications, and Inter-Process Communication (IPC) interfaces for the OmniContext execution engine.

## Model Context Protocol (MCP) Interface

The primary integration layer for external AI agents operates through the Model Context Protocol (MCP). The OmniContext daemon exposes the following execution tools natively.

### MCP Tool Surface

| Tool Identifier    | Description                                                                                 | Parameters                    | Time Complexity     |
| :----------------- | :------------------------------------------------------------------------------------------ | :---------------------------- | :------------------ |
| `search_code`      | Executes hybrid search (dense embeddings + sparse keywords) on the active repository index. | `query` (str), `limit` (int)  | $O(\log N)$ (HNSW)  |
| `get_symbol`       | Point-lookups for absolute symbol definitions and their direct AST traits.                  | `name` (str)                  | $O(1)$ (SQLite FTS) |
| `get_file_summary` | Generates a high-level AST outline and metric summary of a specified document.              | `path` (str)                  | $O(1)$              |
| `get_dependencies` | Performs a directed graph traversal to resolve upstream and downstream dependency edges.    | `symbol` (str), `depth` (int) | $O(V + E)$          |
| `find_patterns`    | Executes a regex or abstract semantic filter against indexed chunks.                        | `regex` (str)                 | $O(N)$              |
| `get_architecture` | Synthesizes a macro-level overview of the target repository's directory boundaries.         | None                          | $O(1)$              |
| `explain_codebase` | Triggers a full-context heuristic extraction of the project's purpose and state.            | None                          | $O(\log N)$         |
| `get_status`       | Returns internal SQLite and usearch telemetry (index count, latency metrics).               | None                          | $O(1)$              |

> Implementation reference: `crates/omni-mcp/src/tools.rs`

## Command-Line Interface (CLI)

The CLI acts as the primary operational trigger for index instantiation, telemetry polling, and daemon lifecycle management.

### Execution Commands

| Command              | Arguments               | Operation Goal                                                                                                     |
| :------------------- | :---------------------- | :----------------------------------------------------------------------------------------------------------------- |
| `omnicontext index`  | `<path>`                | Forces an AST traversal and chunking cycle on the specified root. Triggers the ONNX embedding generation pipeline. |
| `omnicontext search` | `<query> [--limit <N>]` | Invokes the hybrid search ranker in the terminal for debugging chunk relevance.                                    |
| `omnicontext status` | None                    | Emits current vector count, un-indexed files, and database byte constraints.                                       |
| `omnicontext config` | `[set\|get] <key>`      | Modifies the global TOML overrides located at `~/.config/omnicontext/config.toml`.                                 |

> Implementation reference: `crates/omni-cli/src/main.rs`

## Extension Development

All extensions interacting with the core OmniContext Engine must:

1. Bind over standard IO using the MCP `jsonrpc` protocol.
2. Adhere unconditionally to the limits and boundaries established in the MCP payload schemas.
3. Fallback gracefully if the `index` operation has not yet synchronized local artifacts.
