---
title: Installation
description: Install OmniContext v1.2.1 on your machine
category: Getting Started
order: 2
---

# Installation

Get OmniContext **v1.2.1** running on your machine in under two minutes. Index your codebase and start serving context to your AI agents across 17 supported AI clients.

## Quick install

The fastest way to get started on any platform:

```bash
cargo install omnicontext
```

## Package managers

### Windows — WinGet

```powershell
winget install steeltroops.omnicontext
```

### Windows — Scoop

```powershell
scoop bucket add steeltroops https://github.com/steeltroops-ai/scoop-bucket
scoop install omnicontext
```

### macOS — Homebrew

```bash
brew install steeltroops-ai/tap/omnicontext
```

### Cross-platform — Cargo

```bash
cargo install omnicontext
```

## Install from source

OmniContext requires Rust 1.80+ and a C compiler for Tree-sitter grammar compilation. Clone the repository and build the workspace:

```bash
git clone https://github.com/steeltroops-ai/omnicontext.git
cd omnicontext
cargo build --release
```

The release build produces three binaries in `target/release/`: `omnicontext` (CLI), `omnicontext-mcp` (MCP server), and the core library.

## Install options

The installer supports several flags to control what gets set up:

| Flag | Description |
|------|-------------|
| `--no-model` | Skip downloading the embedding model (useful in air-gapped environments or CI) |
| `--no-mcp` | Skip MCP server configuration for AI clients |
| `--no-onnx` | Skip ONNX runtime installation (disables local embedding inference) |
| `--dry-run` | Preview all actions without writing any files or making system changes |

**Example — preview a full install without executing:**

```bash
omnicontext install --dry-run
```

**Example — install CLI only, without model or MCP setup:**

```bash
omnicontext install --no-model --no-mcp
```

## Verify installation

Check that the binaries are in your PATH:

```bash
omnicontext --version
omnicontext-mcp --version
```

Expected output:

```
omnicontext 1.2.1
omnicontext-mcp 1.2.1
```

## Supported AI clients

OmniContext v1.2.1 supports **17 AI clients** out of the box:

| Client | Type |
|--------|------|
| Claude Desktop | Desktop app |
| Claude Code | CLI agent |
| Cursor | IDE |
| Windsurf | IDE |
| VS Code | Editor |
| VS Code Insiders | Editor |
| Cline | VS Code extension |
| RooCode | VS Code extension |
| Continue.dev | VS Code / JetBrains extension |
| Zed | Editor |
| Kiro | IDE |
| PearAI | IDE |
| Trae | IDE |
| Antigravity | IDE |
| Gemini CLI | CLI agent |
| Amazon Q CLI | CLI agent |
| Augment Code | IDE extension |

## Next steps

- [Quickstart Guide](/docs/quickstart) — Index your first codebase
- [Configuration](/docs/configuration) — Customize OmniContext settings
- [MCP Server](/docs/mcp-server) — Connect to AI agents
