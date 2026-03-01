# OmniContext Installation Guide

## For End Users (Recommended Methods)

Choose ONE method based on your platform:

### Windows

**Option 1: Direct Install (Recommended)**
```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex
```

**Option 2: Scoop Package Manager**
```powershell
scoop bucket add omnicontext https://github.com/steeltroops-ai/omnicontext
scoop install omnicontext
```

### macOS / Linux

**Option 1: Direct Install (Recommended)**
```bash
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
```

**Option 2: Homebrew (macOS/Linux)**
```bash
brew tap steeltroops-ai/omnicontext
brew install omnicontext
```

## What Gets Installed

All installation methods install to the same locations:

### Binaries
- **Windows**: `%USERPROFILE%\.omnicontext\bin\`
  - `omnicontext.exe` - CLI tool
  - `omnicontext-mcp.exe` - MCP server
  - `omnicontext-daemon.exe` - Background indexer (optional)

- **Unix/Linux/macOS**: `~/.local/bin/`
  - `omnicontext` - CLI tool
  - `omnicontext-mcp` - MCP server
  - `omnicontext-daemon` - Background indexer (optional)

### Data & Models
- **All platforms**: `~/.omnicontext/`
  - `models/jina-embeddings-v2-base-code.onnx` (~550MB) - AI embedding model
  - `repos/{hash}/` - Indexed repository data per project
    - `index.db` - SQLite database (metadata, FTS5)
    - `vectors.usearch` - Vector index (HNSW)
    - `graph.bin` - Dependency graph

### Configuration
- **MCP Config**: `~/.kiro/settings/mcp.json` (if using with Kiro/Claude)

## Installation Process

All methods perform these steps:

1. **Download** - Fetch latest release binary for your platform
2. **Stop Processes** - Gracefully stop any running instances
3. **Install Binaries** - Place executables in PATH
4. **Download Model** - Fetch Jina AI embedding model (~550MB, one-time)
5. **Verify** - Test binary execution and model presence
6. **Configure PATH** - Ensure binaries are accessible

## Post-Installation

### Verify Installation
```bash
omnicontext --version
```

### Index Your First Repository
```bash
cd /path/to/your/code
omnicontext index .
```

### Search Your Code
```bash
omnicontext search "authentication"
```

### Configure MCP (for AI Agents)

Add to `~/.kiro/settings/mcp.json`:
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

## Updating

### All Platforms
```bash
# Windows
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/update.ps1 | iex

# Unix/Linux/macOS
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
```

### Package Managers
```bash
# Scoop (Windows)
scoop update omnicontext

# Homebrew (macOS/Linux)
brew upgrade omnicontext
```

**Note**: Updates preserve all indexed data and configuration.

## Uninstalling

### Windows
```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/uninstall.ps1 | iex
```

### Package Managers
```bash
# Scoop
scoop uninstall omnicontext

# Homebrew
brew uninstall omnicontext
```

**Options**:
- Keep indexed data: Add `-KeepData` flag
- Keep MCP config: Add `-KeepConfig` flag

## For Developers

If you're contributing to OmniContext or need to build from source:

### Build from Source
```bash
# Clone repository
git clone https://github.com/steeltroops-ai/omnicontext.git
cd omnicontext

# Build release binaries
cargo build --release

# Binaries in: target/release/
```

### Development Scripts

Located in `scripts/` (for contributors only):

- `install-mcp.ps1` - Build and configure MCP server from source
- `install-mcp-quick.ps1` - Quick config update (no build)
- `test-mcp.ps1` - Run MCP integration tests
- `test-mcp-protocol.py` - Protocol compliance tests

### Run Tests
```bash
cargo test --workspace
```

## Troubleshooting

### Model Download Fails
The installer automatically downloads the embedding model. If it fails:
```bash
# Manually trigger download
cd /tmp
omnicontext index .
```

### Binary Not in PATH

**Windows**: Add to PowerShell profile (`$PROFILE`):
```powershell
$env:PATH += ";$env:USERPROFILE\.omnicontext\bin"
```

**Unix/Linux/macOS**: Add to `~/.bashrc` or `~/.zshrc`:
```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Permission Denied (Unix/Linux/macOS)
```bash
chmod +x ~/.local/bin/omnicontext*
```

### Scoop/Homebrew Not Working
Fall back to direct install method - it's the most reliable.

## Edge Cases Handled

All installation methods handle:

- ✅ Existing installations (seamless updates)
- ✅ Running processes (graceful shutdown)
- ✅ Nested/flat archive structures
- ✅ Missing directories (auto-created)
- ✅ PATH not configured (instructions provided)
- ✅ Model already downloaded (skip re-download)
- ✅ Network failures (clear error messages)
- ✅ Architecture detection (x86_64, aarch64)
- ✅ OS detection (Windows, macOS, Linux)

## FAQ

**Q: Which installation method should I use?**
A: Direct install (`install.ps1` or `install.sh`) is recommended. Package managers are convenient if you already use them.

**Q: Does Scoop really work on Windows?**
A: Yes! Scoop is a popular Windows package manager. It works like Homebrew for macOS.

**Q: Can I use multiple installation methods?**
A: No, choose ONE method. All install to the same location, so mixing methods can cause conflicts.

**Q: Do I need to re-index after updating?**
A: No, indexed data is preserved. Re-index only if you want to pick up new features.

**Q: How much disk space is needed?**
A: ~600MB minimum (550MB model + 50MB binary). Indexed repos add ~1-5MB per 1000 files.

**Q: Can I install without internet?**
A: No, the embedding model must be downloaded. After first install, OmniContext works offline.

## Support

- Documentation: https://github.com/steeltroops-ai/omnicontext
- Issues: https://github.com/steeltroops-ai/omnicontext/issues
- Discussions: https://github.com/steeltroops-ai/omnicontext/discussions
