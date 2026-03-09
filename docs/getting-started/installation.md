---
title: Installation
description: Install OmniContext on your machine
category: Getting Started
order: 2
---

# Installation

Get OmniContext running on your machine in under two minutes. Index your codebase and start serving context to your AI agents.

## Install from source

OmniContext requires Rust 1.80+ and a C compiler for Tree-sitter grammar compilation. Clone the repository and build the workspace:

```bash
git clone https://github.com/steeltroops-ai/omnicontext.git
cd omnicontext
cargo build --release
```

The release build produces three binaries in `target/release/`: `omnicontext` (CLI), `omnicontext-mcp` (MCP server), and the core library.

## Package managers

Alternative lifecycle management via standard package managers:

### Windows (Scoop)

```powershell
scoop bucket add omnicontext https://github.com/steeltroops-ai/omnicontext
scoop install omnicontext
```

### Windows (WinGet)

```powershell
winget install omnicontext
```

### macOS (Homebrew)

```bash
brew tap steeltroops-ai/omnicontext
brew install omnicontext
```

### Cross-platform (Cargo)

```bash
cargo binstall omni-cli
```

## Verify installation

Check that the binaries are in your PATH:

```bash
omnicontext --version
omnicontext-mcp --version
```

## Next steps

- [Quickstart Guide](/docs/quickstart) - Index your first codebase
- [Configuration](/docs/configuration) - Customize OmniContext settings
- [MCP Server](/docs/mcp-server) - Connect to AI agents
