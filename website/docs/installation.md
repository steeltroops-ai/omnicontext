---
title: Installation
description: Install OmniContext on Windows, macOS, and Linux
category: Getting Started
order: 3
---

# Installation

Get OmniContext running on your machine. Choose your platform below.

## Windows

### PowerShell Script (Recommended)

```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex
```

This script:
- Downloads latest binaries from GitHub releases
- Installs to `$env:LOCALAPPDATA\omnicontext`
- Adds to PATH automatically
- Configures MCP for Claude Desktop, Cursor, Windsurf

### Scoop

```powershell
scoop bucket add omnicontext https://github.com/steeltroops-ai/omnicontext
scoop install omnicontext
```

### WinGet

```powershell
winget install omnicontext
```

## macOS

### Bash Script (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
```

This script:
- Downloads latest binaries from GitHub releases
- Installs to `~/.local/bin`
- Adds to PATH in `.zshrc` or `.bashrc`
- Configures MCP for Claude Desktop, Cursor

### Homebrew

```bash
brew tap steeltroops-ai/omnicontext
brew install omnicontext
```

## Linux

### Bash Script (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
```

This script:
- Downloads latest binaries from GitHub releases
- Installs to `~/.local/bin`
- Adds to PATH in `.bashrc` or `.zshrc`
- Configures MCP for supported clients

### Package Managers

```bash
# Debian/Ubuntu (coming soon)
sudo apt install omnicontext

# Fedora/RHEL (coming soon)
sudo dnf install omnicontext

# Arch Linux (coming soon)
yay -S omnicontext
```

## Cross-Platform

### Cargo (From Source)

Requires Rust 1.80+ and a C compiler:

```bash
cargo binstall omni-cli
```

Or build from source:

```bash
git clone https://github.com/steeltroops-ai/omnicontext.git
cd omnicontext
cargo build --release --workspace
```

Binaries will be in `target/release/`:
- `omnicontext` (CLI)
- `omnicontext-mcp` (MCP server)
- `omnicontext-daemon` (background process)

### Docker

```bash
docker pull steeltroops/omnicontext:latest
docker run -v $(pwd):/workspace steeltroops/omnicontext index /workspace
```

## Verify Installation

Check that binaries are in your PATH:

```bash
omnicontext --version
omnicontext-mcp --version
omnicontext-daemon --version
```

Expected output:
```
omnicontext 0.14.0
omnicontext-mcp 0.14.0
omnicontext-daemon 0.14.0
```

## Post-Installation

### Download Embedding Model

The first time you run indexing, OmniContext will download the embedding model (~550MB):

```bash
omnicontext index .
```

The model is cached in:
- Windows: `%LOCALAPPDATA%\omnicontext\models`
- macOS/Linux: `~/.cache/omnicontext/models`

### Configure MCP Clients

The installer automatically configures supported MCP clients. To manually configure:

**Claude Desktop**:
```bash
# macOS
open ~/Library/Application\ Support/Claude/claude_desktop_config.json

# Windows
notepad %APPDATA%\Claude\claude_desktop_config.json
```

Add:
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

See [MCP Server Setup](/docs/mcp-server-setup) for other clients.

## Uninstallation

### Windows

```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/uninstall.ps1 | iex
```

Or with Scoop:
```powershell
scoop uninstall omnicontext
```

### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/uninstall.sh | bash
```

Or with Homebrew:
```bash
brew uninstall omnicontext
```

### Manual Cleanup

Remove binaries:
```bash
# Windows
rm -r $env:LOCALAPPDATA\omnicontext

# macOS/Linux
rm -rf ~/.local/bin/omnicontext*
rm -rf ~/.cache/omnicontext
```

Remove indexes (optional):
```bash
# In each indexed project
rm -rf .omnicontext
```

## Troubleshooting

### Command not found

Restart your terminal or source your shell RC:

```bash
# macOS/Linux
source ~/.zshrc  # or ~/.bashrc

# Windows PowerShell
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
```

### Permission denied

On macOS/Linux, make binaries executable:

```bash
chmod +x ~/.local/bin/omnicontext*
```

### Model download fails

Download manually:

```bash
omnicontext download-model
```

Or specify a custom model path:

```bash
omnicontext index . --model-path /path/to/model
```

### Antivirus blocking installation

Add exception for:
- Windows: `%LOCALAPPDATA%\omnicontext`
- macOS/Linux: `~/.local/bin` and `~/.cache/omnicontext`

## Next Steps

- [Quick Start](/docs/quick-start) - Index your first codebase
- [MCP Server Setup](/docs/mcp-server-setup) - Configure AI clients
- [Configuration](/docs/configuration) - Customize OmniContext settings
