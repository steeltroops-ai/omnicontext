# OmniContext

> A universal, natively-compiled semantic code context engine. Exposes structured codebase abstraction to AI agents through the Model Context Protocol (MCP).

OmniContext is engineered to perform high-speed code parsing, relationship extraction, and semantic embeddings locally, bridging repository structures with large language models seamlessly.

[![Status](https://img.shields.io/badge/Status-Beta-orange)](https://github.com/steeltroops-ai/omnicontext)
[![Version](https://img.shields.io/badge/Version-v0.6.1-blue)](https://github.com/steeltroops-ai/omnicontext/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/steeltroops-ai/omnicontext/ci.yml?branch=main&label=Build)](https://github.com/steeltroops-ai/omnicontext/actions)
[![Tests](https://img.shields.io/badge/Tests-149%20passing-brightgreen)](https://github.com/steeltroops-ai/omnicontext)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](https://github.com/steeltroops-ai/omnicontext)
[![License](<https://img.shields.io/badge/License-Open%20Core%20(Apache%202.0%20%2F%20Commercial)-blue>)](./LICENSE)

## Tech Stack

[![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![SQLite](https://img.shields.io/badge/SQLite-07405E?logo=sqlite&logoColor=white)](https://www.sqlite.org/)
[![ONNX](https://img.shields.io/badge/ONNX-005CED?logo=onnx&logoColor=white)](https://onnx.ai/)
[![MCP](https://img.shields.io/badge/MCP-Protocol-purple)](https://modelcontextprotocol.io/)

## Architecture

```mermaid
graph LR
    watcher[Watcher] --> parser[Parser]
    parser --> chunker[Chunker]
    chunker --> embedder[Embedder]
    embedder --> index[(Index)]

    parser --> dep_graph(Graph)

    query(Query) --> search[Search]
    index --> search
    search --> dep_graph
```

1. **Locality**: Embeddings and full indexing run locally (`jina-embeddings-v2-base-code`). No external APIs.
2. **Speed**: Sub-millisecond keyword retrieval combined with HNSW-optimized vector search.
3. **Integration**: Full MCP compliance (`omnicontext-mcp`) allows automatic connections to Claude Code, Cursor, Windsurf, or VS Code extensions dynamically.

## Quick Start

For platform-specific deployment, package manager support, and deep-dive integrations, consult the full [**Installation Guide**](INSTALL.md).

```bash
# 1. Directory indexing
omnicontext index /path/to/project

# 2. Semantic query
omnicontext search "user authentication flow" --limit 5

# 3. Model Context Protocol Server (Zero-Config)
omnicontext-mcp --repo /path/to/project
```

## Contributing & Structure

Comprehensive workflow policies, architecture discussions, and codebase conventions are detailed in [**CONTRIBUTING.md**](CONTRIBUTING.md).

Source Code:

- `crates/omni-core`: Internal compilation and search execution.
- `crates/omni-mcp`: Transport layer mapping logic.
- `crates/omni-cli` / `omni-daemon`: Executables for direct user or IDE interaction.

## License

OmniContext relies on an Open-Core model:

- The base engine tools are licensed under [**Apache 2.0**](LICENSE).
- Proprietary scaling functionality operates under a Custom Commercial License.
