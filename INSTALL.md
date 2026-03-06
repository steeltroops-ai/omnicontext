# Installation Guide

OmniContext provides a streamlined, zero-config installation experience across Windows, macOS, and Linux. Automated tooling resolves binary releases directly from CI/CD artifacts to your standard system routes.

## Recommended One-Liners (Zero-Config)

These scripts download the latest optimized binary for your architecture, update your `PATH`, pre-cache the Jina AI embedding model (~550MB), and **auto-configure MCP** for Claude Desktop, Cursor, Windsurf, Kiro, Cline, RooCode, Trae, Antigravity, and Claude Code.

**Windows (PowerShell)**:

```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex
```

**macOS / Linux (Bash)**:

```bash
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
```

---

## Package Managers

For developers who prefer managed lifecycle and version tracking.

### Windows

**Scoop (Recommended)**:

```powershell
scoop bucket add omnicontext https://github.com/steeltroops-ai/omnicontext
scoop install omnicontext
```

**WinGet**:

```powershell
winget install omnicontext
```

### macOS & Linux

**Homebrew**:

```bash
brew tap steeltroops-ai/omnicontext
brew install omnicontext
```

### Modern Cross-Platform

**Pkgx**:

```bash
pkgx install omnicontext
```

**Cargo Binstall (Pre-compiled Rust standard)**:

```bash
cargo binstall omni-cli
```

---

## Developer Lifecycle

OmniContext includes dedicated scripts for seamless updates and clean removals.

### Updating

To sync with the latest stable release while preserving your indexed data and MCP configurations:

**Windows**:

```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/update.ps1 | iex
```

**macOS / Linux**:

```bash
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/update.sh | bash
```

### Uninstalling

Routinely wipe binaries and cache. Use `--keep-data` or `-KeepData` if you wish to preserve your vector indices.

**Windows**:

```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/uninstall.ps1 | iex
```

**macOS / Linux**:

```bash
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/uninstall.sh | bash
```

---

## Verification & Usage

After installation, restart your terminal or source your shell RC (`~/.zshrc` or `~/.bashrc`).

```bash
# Verify version
omnicontext --version

# Index a repository (one-time setup for the repo)
omnicontext index .

# Perform a semantic search
omnicontext search "how do we handle database migrations?"
```

## IDE & MCP Integration Targets

OmniContext auto-detects and injects its MCP server into the following environments:

| Client              | Configuration Path                 | Support |
| :------------------ | :--------------------------------- | :------ |
| **Claude Desktop**  | `claude_desktop_config.json`       | native  |
| **Claude Code**     | `~/.claude.json`                   | native  |
| **Cursor**          | `cursor.mcp/config.json`           | native  |
| **Trae IDE**        | `.trae/mcp.json` / `globalStorage` | native  |
| **Antigravity**     | `mcp_config.json`                  | native  |
| **Windsurf**        | `mcp_config.json`                  | native  |
| **Cline / RooCode** | `mcp_settings.json`                | native  |
| **Continue.dev**    | `config.json`                      | native  |
| **Kiro**            | `mcp.json`                         | native  |

---

Documentation: [GitHub Repository](https://github.com/steeltroops-ai/omnicontext)
Issues: [Report a Bug](https://github.com/steeltroops-ai/omnicontext/issues)
