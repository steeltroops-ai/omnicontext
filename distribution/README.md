# OmniContext Distribution

Quick installation commands for end users on all platforms.

> **Note**: Pre-built releases are not yet available. You'll need to build from source.
> See [CONTRIBUTING.md](../CONTRIBUTING.md) for build instructions.

## Build from Source (Required for Now)

Until pre-built releases are available, follow these steps:

### Prerequisites
- Rust toolchain (install from https://rustup.rs/)
- Git

### Build Steps

```bash
# Clone the repository
git clone https://github.com/steeltroops-ai/omnicontext.git
cd omnicontext

# Build release binaries
cargo build --release

# Binaries will be in target/release/
# - omnicontext (CLI)
# - omnicontext-mcp (MCP server)
# - omnicontext-daemon (file watcher)
```

### Manual Installation

**Windows:**
```powershell
# Copy binaries to a directory in your PATH
$binDir = "$env:LOCALAPPDATA\omnicontext\bin"
New-Item -ItemType Directory -Force -Path $binDir
Copy-Item target\release\omnicontext.exe $binDir\
Copy-Item target\release\omnicontext-mcp.exe $binDir\
Copy-Item target\release\omnicontext-daemon.exe $binDir\

# Add to PATH
$path = [Environment]::GetEnvironmentVariable("Path", "User")
[Environment]::SetEnvironmentVariable("Path", "$path;$binDir", "User")
```

**macOS / Linux:**
```bash
# Copy binaries to /usr/local/bin
sudo cp target/release/omnicontext /usr/local/bin/
sudo cp target/release/omnicontext-mcp /usr/local/bin/
sudo cp target/release/omnicontext-daemon /usr/local/bin/
sudo chmod +x /usr/local/bin/omnicontext*
```

---

## Quick Install (When Releases Are Available)

### Windows

```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex
```

### macOS / Linux

```bash
curl -sSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
```

## Update

### Windows

```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/update.ps1 | iex
```

### macOS / Linux

```bash
# Re-run the install script (it updates existing installation)
curl -sSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
```

## Uninstall

### Windows

```powershell
# Remove binaries
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\omnicontext"

# Remove from PATH (manual step - edit System Environment Variables)
# Or use PowerShell to remove from User PATH:
$path = [Environment]::GetEnvironmentVariable("Path", "User")
$newPath = ($path.Split(';') | Where-Object { $_ -notlike "*omnicontext*" }) -join ';'
[Environment]::SetEnvironmentVariable("Path", $newPath, "User")

# Remove config and cache
Remove-Item -Recurse -Force "$env:USERPROFILE\.omnicontext"
```

### macOS

```bash
# Remove binaries
sudo rm -rf /usr/local/bin/omnicontext*

# Remove config and cache
rm -rf ~/.omnicontext
```

### Linux

```bash
# Remove binaries
sudo rm -rf /usr/local/bin/omnicontext*

# Remove config and cache
rm -rf ~/.omnicontext
```

## Package Managers

### Windows (Scoop)

```powershell
# Install
scoop bucket add omnicontext https://github.com/steeltroops-ai/omnicontext
scoop install omnicontext

# Update
scoop update omnicontext

# Uninstall
scoop uninstall omnicontext
```

### macOS / Linux (Homebrew)

```bash
# Install
brew tap steeltroops-ai/omnicontext
brew install omnicontext

# Update
brew upgrade omnicontext

# Uninstall
brew uninstall omnicontext
```

## What Gets Installed

- `omnicontext` - CLI tool for indexing and searching
- `omnicontext-mcp` - MCP server for AI agent integration
- `omnicontext-daemon` - Background daemon for file watching
- Jina AI embedding model (~550MB, auto-downloaded on first run)

## Installation Locations

### Windows
- Binaries: `%LOCALAPPDATA%\omnicontext\bin\`
- Config: `%USERPROFILE%\.omnicontext\`
- Models: `%USERPROFILE%\.omnicontext\models\`

### macOS / Linux
- Binaries: `/usr/local/bin/`
- Config: `~/.omnicontext/`
- Models: `~/.omnicontext/models/`

## Verify Installation

```bash
# Check version
omnicontext --version

# Check MCP server
omnicontext-mcp --version

# Check daemon
omnicontext-daemon --version
```

## Troubleshooting

### Command not found after install

**Windows**: Restart PowerShell or run:
```powershell
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User")
```

**macOS/Linux**: Restart terminal or run:
```bash
source ~/.bashrc  # or ~/.zshrc for zsh
```

### Permission denied (macOS/Linux)

Run with sudo:
```bash
curl -sSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | sudo bash
```

### Model download fails

The embedding model (~550MB) downloads on first use. If it fails:
```bash
# Retry indexing - it will resume download
omnicontext index /path/to/repo
```

## For Developers

If you're developing OmniContext, use the scripts in `../scripts/` instead:
- `scripts/install-mcp.ps1` - Build and install MCP server from source
- `scripts/test-mcp.ps1` - Test MCP server functionality
- `scripts/setup-dev.ps1` - Set up development environment

## Directory Structure

```
distribution/
├── README.md           # This file
├── install.ps1         # Windows installer
├── install.sh          # macOS/Linux installer
├── update.ps1          # Windows updater
├── uninstall.ps1       # Windows uninstaller
├── homebrew/           # Homebrew formula
│   └── omnicontext.rb
└── scoop/              # Scoop manifest
    └── omnicontext.json
```

## Release Process

When releasing a new version:

1. Update version in `homebrew/omnicontext.rb`
2. Update version in `scoop/omnicontext.json`
3. Build release binaries for all platforms
4. Update SHA256 hashes in package manifests
5. Test installation on Windows, macOS, and Linux
6. Create GitHub release with binaries attached
7. Update installation scripts if needed

## Support

- Issues: https://github.com/steeltroops-ai/omnicontext/issues
- Documentation: https://github.com/steeltroops-ai/omnicontext
- Installation Guide: https://github.com/steeltroops-ai/omnicontext/blob/main/INSTALL.md
