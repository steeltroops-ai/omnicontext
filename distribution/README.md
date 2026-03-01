# Distribution Scripts

This folder contains installation scripts and package manager manifests for END USERS.

## Structure

```
distribution/
├── homebrew/           # Homebrew formula for macOS/Linux
│   └── omnicontext.rb
├── scoop/              # Scoop manifest for Windows
│   └── omnicontext.json
└── install/            # Direct installation scripts
    ├── install.ps1     # Windows installer (downloads from GitHub releases)
    └── install.sh      # macOS/Linux installer (downloads from GitHub releases)
```

## Usage

### Windows (PowerShell)

```powershell
# One-line install from GitHub
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install/install.ps1 | iex

# Or with Scoop
scoop bucket add omnicontext https://github.com/steeltroops-ai/omnicontext
scoop install omnicontext
```

### macOS/Linux (Bash)

```bash
# One-line install from GitHub
curl -sSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install/install.sh | bash

# Or with Homebrew (macOS/Linux)
brew tap steeltroops-ai/omnicontext
brew install omnicontext
```

## For Developers

If you're developing OmniContext, use the scripts in `../scripts/` instead:
- `scripts/install-mcp.ps1` - Build and install MCP server from source
- `scripts/test-mcp.ps1` - Test MCP server functionality

## Package Manager Updates

When releasing a new version:

1. Update version in `homebrew/omnicontext.rb`
2. Update version in `scoop/omnicontext.json`
3. Update SHA256 hashes after building release binaries
4. Test installation on all platforms

## Notes

- These scripts download pre-built binaries from GitHub releases
- They automatically download the Jina AI embedding model (~550MB)
- They add binaries to PATH automatically
- They work offline after initial installation
