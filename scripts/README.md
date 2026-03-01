# Developer Scripts

This folder contains scripts for DEVELOPERS working on OmniContext.

## Structure

```
scripts/
├── install-mcp.ps1         # Full MCP server installation (build + test + configure)
├── install-mcp-quick.ps1   # Quick MCP config update (no build)
└── test-mcp.ps1            # Comprehensive MCP server test suite
```

## Usage

### Install MCP Server (Full)

Builds from source, runs tests, configures MCP, and indexes repository:

```powershell
.\scripts\install-mcp.ps1
```

Options:
- `-Repo "C:\Path\To\Repo"` - Specify repository to index
- `-SkipBuild` - Skip building (use existing binary)
- `-SkipTests` - Skip running tests
- `-ConfigPath` - Custom MCP config path

### Quick Install (Config Only)

Updates MCP configuration without rebuilding:

```powershell
.\scripts\install-mcp-quick.ps1
```

Use this when:
- Binary already built
- Just need to update config
- Switching repositories

### Test MCP Server

Runs comprehensive test suite:

```powershell
.\scripts\test-mcp.ps1
```

Tests include:
1. Binary existence and size
2. Help command execution
3. Version output
4. Repository indexing
5. Search functionality
6. Symbol lookup
7. File summary
8. Status reporting

## For End Users

If you're installing OmniContext as an end user, use the scripts in `../distribution/` instead:
- `distribution/install/install.ps1` - Windows installer (downloads from releases)
- `distribution/install/install.sh` - macOS/Linux installer (downloads from releases)

Or use package managers:
- Homebrew: `brew install steeltroops-ai/omnicontext/omnicontext`
- Scoop: `scoop install omnicontext`

## Development Workflow

1. Make code changes
2. Run tests: `cargo test --workspace`
3. Build MCP: `cargo build -p omni-mcp --release`
4. Install: `.\scripts\install-mcp.ps1 -SkipTests`
5. Test: `.\scripts\test-mcp.ps1`
6. Iterate

## Notes

- These scripts build from source (require Rust toolchain)
- They configure MCP for local development
- They work with the current repository
- They're designed for rapid iteration
