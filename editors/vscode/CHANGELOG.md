# Changelog

All notable changes to the OmniContext VS Code extension are documented here.

## [1.1.0] - 2026-03-09

_No direct changes in this release._


## [1.0.1] - 2026-03-09


- align changelog versions with git tags and fix vscode engines compatibility ([f22c2e0](https://github.com/steeltroops-ai/omnicontext/commit/f22c2e0))



## [0.16.1] - 2026-03-09

_No direct changes in this release._

## [0.16.0] - 2026-03-09

_No direct changes in this release._

## [0.15.0] - 2026-03-09

_No direct changes in this release._

## [0.14.0] - 2026-03-08

### Added
- Branch-aware diff indexing and SOTA performance optimizations ([464ab1f](https://github.com/steeltroops-ai/omnicontext/commit/464ab1f))

## [0.13.1] - 2026-03-08

### Fixed
- Hardened path resolution to prevent silent wrong-directory indexing ([38ad9a0](https://github.com/steeltroops-ai/omnicontext/commit/38ad9a0))

## [0.13.0] - 2026-03-08

### Added
- Batch embedding and backpressure for indexing ([ea880bf](https://github.com/steeltroops-ai/omnicontext/commit/ea880bf))

## [0.12.0] - 2026-03-07

_No direct changes in this release._

## [0.11.0] - 2026-03-07

### Added
- Overhauled sidebar UI with path normalization and set_workspace tool ([4a8cf35](https://github.com/steeltroops-ai/omnicontext/commit/4a8cf35))

### Changed
- Added marketplace badges and pre-rendered mermaid diagrams to READMEs ([1b38992](https://github.com/steeltroops-ai/omnicontext/commit/1b38992))

## [0.10.0] - 2026-03-07

### Added
- Core MCP and daemon optimizations ([f4f4450](https://github.com/steeltroops-ai/omnicontext/commit/f4f4450))

## [0.9.4] - 2026-03-07

### Fixed
- Generated separate scoped changelog for VS Code extension ([1f29da2](https://github.com/steeltroops-ai/omnicontext/commit/1f29da2))

### Changed
- Cleaned up unused markdown link references in VS Code changelog ([f608990](https://github.com/steeltroops-ai/omnicontext/commit/f608990))
- Restructured VS Code changelog to correctly group 0.9.x features under 0.9.2 ([ea23b10](https://github.com/steeltroops-ai/omnicontext/commit/ea23b10))

## [0.9.2] - 2026-03-07

### Added
- Zero-friction install: extension auto-downloads OmniContext engine and ONNX Runtime on first install
- Auto ONNX repair: platform-specific ONNX Runtime libraries fetched automatically if missing
- Bootstrap progress bar: VS Code notification shows real-time download progress during first-time setup
- Sidebar offline state: clean offline state instead of freezing when engine is not running

### Fixed
- Sidebar freeze: fixed critical bug where sidebar became permanently unresponsive when daemon was not running
- IPC timeouts: reduced from 30 seconds to 3 seconds for status/metrics requests
- Event tracker CPU waste: keystrokes and cursor moves no longer enqueue IPC events when daemon is offline
- Cache capacity: now reads `omnicontext.prefetch.cacheSize` setting instead of hardcoded 100
- Resolved all CI workflow failures: license allowlist, security gate logic, release archive packaging
- Migrated deny.toml to cargo-deny v2 schema, eliminated deprecated keys

### Changed
- Whitelisted accepted RUSTSEC advisories in deny.toml for transitive dependencies
- Restructured and normalized documentation to kebab-case naming conventions
- Hardened CI pipelines and resolved supply chain audit failures
- Removed `Phase N` terminology from codebase, replaced with descriptive category names
- Updated distribution scripts to use consistent staging terminology

## [0.7.2] - 2026-03-07

### Added
- Extension now checks `~/.omnicontext/bin` (standalone installer path) as a binary candidate
- Sidebar `Repair Environment` command triggers ONNX Runtime re-download and re-index in one click

### Fixed
- Binary lookup no longer blocks the VS Code UI thread via `execSync`

## [0.7.1] - 2026-03-06

### Fixed
- Version resolution now queries GitHub Releases API correctly during auto-sync
- Removed unicode characters that caused rendering issues in some terminal environments

## [0.7.0] - 2026-03-06

### Added
- Zero-config MCP sync: on daemon start, extension automatically writes MCP server entry into all detected AI client configs (Claude Desktop, Cursor, Continue.dev, Kiro, Windsurf, Cline, RooCode, Trae, Antigravity, Claude Code CLI)
- Kiro IDE MCP support via the `powers.mcpServers` namespace
- `OmniContext: Sync MCP` command for manual re-sync to all AI clients

## [0.6.0] - 2026-03-06

### Added
- MCP manifest auto-published to `~/.omnicontext/mcp-manifest.json` for client auto-discovery
- Engine status exposes language distribution (per-language file counts) in the sidebar

## [0.4.0] - 2026-03-02

### Added
- Enhanced sidebar UI: real-time performance metrics (P50/P95/P99 search latency), activity log, repository info panel, and language distribution chart
- Professional VS Code codicons throughout the sidebar
- One-click cache clear button with immediate UI feedback

## [0.3.0] - 2026-03-01

### Added
- Pre-fetch caching: extension monitors cursor position, file opens, and text edits to pre-fetch relevant code context
- Cache statistics panel: hit rate, cache size, hits/misses visible in the sidebar
- Context injection speed: cache hits provide context in `<10ms` vs. 50–200ms for fresh searches
- Configuration: `omnicontext.prefetch.enabled`, `cacheSize`, `cacheTtlSeconds`, `debounceMs`
- Context injection now uses IPC to the daemon first, with CLI fallback

### Changed
- Extension auto-starts daemon on workspace open (`omnicontext.autoStartDaemon: true` by default)

## [0.2.0] - 2026-03-01

### Added
- VS Code sidebar with system status, cache metrics, and one-click controls
- `PrefetchCache` module in the engine daemon with LRU eviction and TTL expiration
- IPC interface: `prefetch_stats`, `clear_cache`, `update_config`, `shutdown`

## [0.1.0] - 2026-02-28

### Added
- Initial VS Code extension release
- Commands: `Index`, `Search`, `Status`, `Start Daemon`, `Stop Daemon`, and `Preflight`
- Chat participant registration for VS Code Copilot Chat integration (context injection)
- IPC client with named-pipe/Unix-socket transport and exponential backoff reconnection
- Status bar item showing daemon state
- Basic sidebar with connection status and cache hit counter

[0.16.1]: https://github.com/steeltroops-ai/omnicontext/compare/v0.16.0...v0.16.1
[0.16.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.15.0...v0.16.0
[0.15.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.14.0...v0.15.0
[0.14.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.13.1...v0.14.0
[0.13.1]: https://github.com/steeltroops-ai/omnicontext/compare/v0.13.0...v0.13.1
[0.13.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.9.4...v0.10.0
[0.9.4]: https://github.com/steeltroops-ai/omnicontext/compare/v0.9.2...v0.9.4
[0.9.2]: https://github.com/steeltroops-ai/omnicontext/compare/v0.7.2...v0.9.2
[0.7.2]: https://github.com/steeltroops-ai/omnicontext/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/steeltroops-ai/omnicontext/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.4.0...v0.6.0
[0.4.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/steeltroops-ai/omnicontext/releases/tag/v0.1.0
