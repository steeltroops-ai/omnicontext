# OmniContext Installation Guide

Universal code context engine for AI coding agents. Works on Windows, macOS, and Linux.

## Quick Install

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install/install.ps1 | iex
```

### macOS/Linux (Bash)

```bash
curl -sSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install/install.sh | bash
```

## Package Managers

### Homebrew (macOS/Linux)

```bash
brew tap steeltroops-ai/omnicontext
brew install omnicontext
```

### Scoop (Windows)

```powershell
scoop bucket add omnicontext https://github.com/steeltroops-ai/omnicontext
scoop install omnicontext
```

## What Gets Installed

1. **Binaries** (added to PATH automatically):
   - `omnicontext` - CLI for indexing and searching
   - `omnicontext-mcp` - MCP server for AI agents

2. **AI Model** (~550MB, auto-downloaded):
   - Jina AI code embedding model
   - Enables semantic code search
   - Stored in `~/.omnicontext/models/`

3. **Installation Locations**:
   - Windows: `%USERPROFILE%\.omnicontext\bin\`
   - macOS/Linux: `~/.local/bin/`

## Getting Started

### 1. Index Your Repository

```bash
cd /path/to/your/repo
omnicontext index .
```

This creates:
- SQLite database with code metadata
- Vector index for semantic search
- Dependency graph

### 2. Search Your Code

```bash
# Keyword search
omnicontext search "authentication"

# Semantic search
omnicontext search "how to validate user input"

# Symbol lookup
omnicontext search "UserService.authenticate"
```

### 3. Connect to AI Agents

Add to your MCP configuration (e.g., `~/.kiro/settings/mcp.json`):

```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "omnicontext-mcp",
      "args": ["--repo", "/path/to/your/repo"],
      "disabled": false
    }
  }
}
```

Restart your IDE/editor to load the configuration.

## MCP Tools Available

Once connected, AI agents can use:

- `search_code` - Hybrid keyword + semantic search
- `get_symbol` - Lookup symbols by name
- `get_file_summary` - Get file structure overview
- `get_dependencies` - Traverse dependency graph
- `find_patterns` - Identify code patterns
- `get_architecture` - Generate architecture overview
- `explain_codebase` - Comprehensive project explanation
- `get_status` - Engine status and metrics

## Updating

### Re-run Installation Script

```powershell
# Windows
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install/install.ps1 | iex
```

```bash
# macOS/Linux
curl -sSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install/install.sh | bash
```

### Package Managers

```bash
# Homebrew
brew upgrade omnicontext

# Scoop
scoop update omnicontext
```

## Uninstalling

### Windows

```powershell
# Remove binaries
Remove-Item -Recurse -Force "$env:USERPROFILE\.omnicontext"

# Remove from PATH (manually edit environment variables)
```

### macOS/Linux

```bash
# Remove binaries
rm -rf ~/.local/bin/omnicontext*
rm -rf ~/.omnicontext

# If installed via Homebrew
brew uninstall omnicontext
```

## Troubleshooting

### Model Download Fails

If the AI model download is interrupted:

```bash
# Manually trigger download
omnicontext index .
```

The model will auto-download with progress bar.

### Binary Not Found

Ensure installation directory is in PATH:

```powershell
# Windows - Check PATH
$env:PATH -split ';' | Select-String "omnicontext"

# Add to PATH if missing
[Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$env:USERPROFILE\.omnicontext\bin", "User")
```

```bash
# macOS/Linux - Check PATH
echo $PATH | grep -o "[^:]*omnicontext[^:]*"

# Add to PATH if missing (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.local/bin:$PATH"
```

### MCP Server Not Connecting

1. Verify binary executes: `omnicontext-mcp --help`
2. Check MCP configuration path
3. Restart IDE/editor
4. Check IDE logs for MCP errors

### Indexing Fails

```bash
# Check status
omnicontext status

# Re-index with verbose logging
RUST_LOG=debug omnicontext index .
```

## System Requirements

- **OS**: Windows 10+, macOS 10.15+, Linux (glibc 2.31+)
- **Architecture**: x86_64 (Intel/AMD) or ARM64 (Apple Silicon)
- **Disk Space**: ~600MB (binaries + model)
- **Memory**: ~100MB per 10k files indexed
- **Internet**: Required for initial model download only

## Offline Usage

After initial installation and model download, OmniContext works completely offline:
- No API keys required
- No cloud dependencies
- All processing on your machine
- Privacy-first design

## Developer Installation

If you're developing OmniContext, see:
- `scripts/README.md` - Developer scripts
- `CONTRIBUTING.md` - Development guide

Build from source:

```bash
# Clone repository
git clone https://github.com/steeltroops-ai/omnicontext.git
cd omnicontext

# Build all binaries
cargo build --release

# Binaries in: target/release/
```

## Support

- **Issues**: https://github.com/steeltroops-ai/omnicontext/issues
- **Discussions**: https://github.com/steeltroops-ai/omnicontext/discussions
- **Documentation**: https://github.com/steeltroops-ai/omnicontext/tree/main/docs

## License

Apache 2.0 - See LICENSE file for details.
