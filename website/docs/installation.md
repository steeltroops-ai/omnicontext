---
title: Installation
description: Install OmniContext on Windows, macOS, or Linux with zero configuration required.
category: Getting Started
order: 3
---

# Installation

OmniContext provides multiple installation methods for Windows, macOS, and Linux. Choose the approach that best suits your workflow.

## Prerequisites

- **Operating System**: Windows 10+, macOS 11+, or Linux (Ubuntu 20.04+, Fedora 35+)
- **Disk Space**: ~600 MB for binaries and embedding models
- **Memory**: 4 GB RAM minimum, 8 GB recommended for large codebases

---

## Quick Install (Recommended)

The install script covers all three platforms and places the binaries in your PATH automatically.

### macOS and Linux

```bash
curl -fsSL https://omnicontext.dev/install.sh | sh
```

The script installs `omnicontext` and `omnicontext-mcp` to `~/.local/bin/` and updates your shell profile.

### Windows (PowerShell)

```powershell
irm https://omnicontext.dev/install.ps1 | iex
```

Binaries are installed to `%USERPROFILE%\.omnicontext\bin\`, which the script adds to your `PATH`.

---

## Platform-Specific Methods

### Windows

**PowerShell installer** (recommended — covers both binaries):

```powershell
irm https://omnicontext.dev/install.ps1 | iex
```

**Manual binary download**:

1. Download the latest release archive from [GitHub Releases](https://github.com/steeltroops-ai/omnicontext/releases).
2. Extract `omnicontext.exe` and `omnicontext-mcp.exe`.
3. Move both files to `%USERPROFILE%\.omnicontext\bin\`.
4. Add `%USERPROFILE%\.omnicontext\bin\` to your `PATH` environment variable.

### macOS

**Install script**:

```bash
curl -fsSL https://omnicontext.dev/install.sh | sh
```

**Homebrew** (tap):

```bash
brew tap steeltroops-ai/omnicontext
brew install omnicontext
```

**Manual binary download**:

1. Download the latest `.tar.gz` for your architecture (`x86_64-apple-darwin` or `aarch64-apple-darwin`) from [GitHub Releases](https://github.com/steeltroops-ai/omnicontext/releases).
2. Extract and move `omnicontext` and `omnicontext-mcp` to `~/.local/bin/`.
3. Make them executable: `chmod +x ~/.local/bin/omnicontext ~/.local/bin/omnicontext-mcp`

### Linux

**Install script** (Ubuntu, Debian, Fedora, Arch, and others):

```bash
curl -fsSL https://omnicontext.dev/install.sh | sh
```

**Manual binary download**:

1. Download the latest `.tar.gz` for your architecture from [GitHub Releases](https://github.com/steeltroops-ai/omnicontext/releases).
2. Extract and move the binaries:

```bash
tar -xzf omnicontext-*.tar.gz
mv omnicontext omnicontext-mcp ~/.local/bin/
chmod +x ~/.local/bin/omnicontext ~/.local/bin/omnicontext-mcp
```

---

## Install via Cargo

If you have Rust and Cargo installed, you can build and install from source or from [crates.io](https://crates.io):

```bash
cargo install omnicontext
```

This builds and installs both `omnicontext` and `omnicontext-mcp` into `~/.cargo/bin/`.

To install from the repository directly:

```bash
git clone https://github.com/steeltroops-ai/omnicontext.git
cd omnicontext
cargo install --path crates/omni-cli
cargo install --path crates/omni-mcp
```

> **Note**: A Rust toolchain version ≥ 1.80 is required.

---

## VS Code Extension

The OmniContext VS Code extension provides automatic MCP server configuration — no manual JSON editing required.

1. Open VS Code and go to the Extensions view (`Ctrl+Shift+X` / `Cmd+Shift+X`).
2. Search for **OmniContext**.
3. Click **Install**.

The extension automatically detects your workspace and configures the `omnicontext-mcp` server in `mcp.json` on first activation.

Alternatively, install from the command line:

```bash
code --install-extension steeltroops-ai.omnicontext
```

---

## Install Locations

| Platform | Binaries | Data & Cache |
|----------|----------|--------------|
| Linux / macOS | `~/.local/bin/` | `~/.omnicontext/` |
| Windows | `%USERPROFILE%\.omnicontext\bin\` | `%USERPROFILE%\.omnicontext\` |

The data directory stores embedding models, vector indexes, and SQLite databases. It is shared across all projects.

---

## Verify Installation

After installation, confirm both binaries are on your PATH:

```bash
omnicontext --version
omnicontext-mcp --version
```

Expected output:

```
omnicontext 1.1.1
```

---

## First Index

Index your first codebase from the project root:

```bash
cd /path/to/your/project
omnicontext index .
```

OmniContext will:

1. Auto-detect all supported languages in your project.
2. Download the embedding model (~550 MB, one-time) to `~/.omnicontext/models/`.
3. Parse and index all source files using tree-sitter.
4. Build a vector index for semantic search.

Indexing speed: approximately 500 files per second on modern hardware.

---

## IDE and Agent Setup

After indexing, wire OmniContext into every detected AI IDE and agent automatically:

```bash
omnicontext setup --all
```

This single command injects a universal `omnicontext` MCP server entry (using `--repo .`) into all installed IDEs, including Claude Desktop, Claude Code, Cursor, Windsurf, VS Code, Cline, RooCode, Continue.dev, Zed, Kiro, PearAI, Trae, Gemini CLI, Amazon Q CLI, and Augment Code.

To preview changes without writing any files:

```bash
omnicontext setup --all --dry-run
```

To target a specific IDE only:

```bash
omnicontext autopilot --ide cursor
```

---

## Download the Embedding Model Separately

To pre-download the embedding model without running a full index (useful in CI or restricted environments):

```bash
omnicontext setup model-download
```

Check the current model status:

```bash
omnicontext setup model-status
```

The model is Jina embeddings v2 base code in ONNX format (~550 MB). It is stored in `~/.omnicontext/models/` and shared across all repositories.

If the automatic download fails, manually download the ONNX weights from:
- https://huggingface.co/jinaai/jina-embeddings-v2-base-code

Place the model files in: `~/.omnicontext/models/jina-embeddings-v2-base-code/`

---

## Configuration File

OmniContext works with zero configuration out of the box. To customize behavior, create `.omnicontext/config.toml` in your project root:

```bash
omnicontext config --init
```

This writes a commented default configuration you can edit. See the [Configuration](/docs/configuration) guide for all available options.

---

## Environment Variables

Override configuration at runtime:

```bash
export OMNI_MODEL_PATH=/custom/path/to/model
export OMNI_INDEX_PATH=/custom/index/location
export OMNI_LOG_LEVEL=debug
export OMNI_SKIP_MODEL_DOWNLOAD=1   # Start in keyword-only mode without downloading the model
```

---

## Troubleshooting

### `omnicontext: command not found`

Add the install directory to your `PATH`:

```bash
# Linux / macOS
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

On Windows, add `%USERPROFILE%\.omnicontext\bin\` to your system `PATH` via **System Properties → Environment Variables**.

### Model Download Fails

Run the download with verbose logging to diagnose network issues:

```bash
OMNI_LOG_LEVEL=debug omnicontext setup model-download
```

As a fallback, download the model manually from https://huggingface.co/jinaai/jina-embeddings-v2-base-code and extract it to `~/.omnicontext/models/jina-embeddings-v2-base-code/`.

To start the MCP server without waiting for a model download (keyword-only mode):

```bash
OMNI_SKIP_MODEL_DOWNLOAD=1 omnicontext mcp --repo .
```

### Indexing Hangs or Is Slow

Check for large binary files being processed. Add exclusions to `.omnicontext/config.toml`:

```toml
[indexing]
exclude_patterns = ["*.bin", "*.so", "*.dll", "*.dylib", "node_modules", "target"]
```

### Permission Errors

Ensure write access to:

- The project directory (for the `.omnicontext/` index folder)
- The home directory (for `~/.omnicontext/` model cache)

---

## Next Steps

- [Configuration](/docs/configuration): Customize indexing, search, and embedding behavior
- [MCP Tools](/docs/mcp-tools): Integrate with AI agents via the Model Context Protocol
- [Supported Languages](/docs/supported-languages): See which languages are supported
- [Architecture](/docs/architecture): Understand how OmniContext works internally
