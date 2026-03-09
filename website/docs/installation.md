---
title: Installation
description: Install OmniContext on Windows, macOS, or Linux with zero configuration required.
category: Getting Started
order: 3
---

# Installation

OmniContext provides native installers for Windows, macOS, and Linux. Choose your platform below for installation instructions.

## Prerequisites

- **Operating System**: Windows 10+, macOS 11+, or Linux (Ubuntu 20.04+, Fedora 35+)
- **Disk Space**: ~600MB for binaries and embedding models
- **Memory**: 4GB RAM minimum, 8GB recommended for large codebases

## Quick Install

### Windows

Download and run the installer:

```powershell
# Using PowerShell
irm https://omnicontext.dev/install.ps1 | iex
```

Or install via package managers:

```powershell
# Scoop
scoop bucket add omnicontext https://github.com/omnicontext/scoop-bucket
scoop install omnicontext

# WinGet
winget install OmniContext.OmniContext
```

### macOS

Install via Homebrew:

```bash
brew tap omnicontext/tap
brew install omnicontext
```

Or download the installer:

```bash
curl -fsSL https://omnicontext.dev/install.sh | bash
```

### Linux

Install via package manager or script:

```bash
# Ubuntu/Debian
curl -fsSL https://omnicontext.dev/install.sh | bash

# Arch Linux (AUR)
yay -S omnicontext

# Fedora/RHEL
dnf install omnicontext
```

## Verify Installation

After installation, verify OmniContext is working:

```bash
omni --version
```

You should see output like:

```
omnicontext 0.1.0
```

## First Index

Index your first codebase:

```bash
cd /path/to/your/project
omni index
```

OmniContext will:
1. Auto-detect supported languages in your project
2. Download embedding models (~550MB, one-time)
3. Parse and index all source files
4. Build vector index for semantic search

Indexing speed: ~500 files/second on modern hardware.

## Configuration

OmniContext works with zero configuration, but you can customize behavior:

### Configuration File

Create `.omnicontext/config.toml` in your project root:

```toml
[index]
# Exclude patterns (glob syntax)
exclude = ["node_modules/**", "dist/**", "*.test.ts"]

# Maximum file size to index (bytes)
max_file_size = 1048576  # 1MB

[embedder]
# Model path (auto-downloads if not present)
model_path = "~/.omnicontext/models/jina-embeddings-v2-base-code"

# Batch size for embedding generation
batch_size = 32

[search]
# Number of results to return
limit = 20

# Minimum similarity score (0.0 - 1.0)
min_score = 0.3
```

### Environment Variables

Override configuration with environment variables:

```bash
export OMNI_MODEL_PATH=/custom/path/to/model
export OMNI_INDEX_PATH=/custom/index/location
export OMNI_LOG_LEVEL=debug
```

## MCP Integration

To use OmniContext with AI agents via MCP:

1. Start the MCP server:

```bash
omni-mcp
```

2. Configure your AI client (e.g., Claude Desktop) to connect to the MCP server.

See [MCP Tools](/docs/mcp-tools) for detailed API documentation.

## Troubleshooting

### Model Download Fails

If model download fails, manually download from:
- https://huggingface.co/jinaai/jina-embeddings-v2-base-code

Extract to: `~/.omnicontext/models/jina-embeddings-v2-base-code/`

### Indexing Hangs

Check for large binary files being indexed. Add exclusions to config:

```toml
[index]
exclude = ["*.bin", "*.so", "*.dll", "*.dylib"]
```

### Permission Errors

Ensure write access to:
- Project directory (for `.omnicontext/` folder)
- Home directory (for `~/.omnicontext/` cache)

## Next Steps

- [Quickstart](/docs/quickstart): Index your first project and run searches
- [MCP Tools](/docs/mcp-tools): Integrate with AI agents
- [Supported Languages](/docs/supported-languages): See which languages are supported
