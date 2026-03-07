# Changelog

## [0.11.0] - 2026-03-07

## What's Changed

### Features

- overhaul sidebar UI, fix path normalization, and add set_workspace tool (4a8cf35)

### Maintenance

- add marketplace badges and pre-rendered mermaid diagrams to readmes (1b38992)

## [0.10.0] - 2026-03-07

## What's Changed

### Features

- core mcp and daemon optimizations (f4f4450)

## [0.9.4] - 2026-03-07

## What's Changed

### Bug Fixes

- generate separate scoped changelog for vscode extension (1f29da2)

### Maintenance

- clean up unused markdown link references in vscode changelog (f608990)
- restructure vscode changelog to correctly group 0.9.x features under 0.9.2 and restore Unreleased header (ea23b10)

## [0.9.2] - 2026-03-07

### Added

- **Zero-Friction Install**: Extension auto-downloads the OmniContext engine and ONNX Runtime on first install — no Rust, no `cargo install`, no terminal commands required
- **Auto ONNX Repair**: Windows `onnxruntime.dll`, Linux `libonnxruntime.so`, macOS `libonnxruntime.dylib` are fetched automatically if missing
- **Bootstrap Progress Bar**: A VS Code notification shows real-time download progress during first-time setup
- **Sidebar Offline State**: When the engine is not running, the sidebar shows a clean offline state instead of freezing

### Bug Fixes

- **Sidebar Freeze**: Fixed critical bug where the sidebar became permanently unresponsive when the engine daemon was not running. All IPC calls are now gated behind a circuit breaker
- **IPC Timeouts**: Reduced from 30 seconds to 3 seconds for status/metrics requests, eliminating cascading timeout stacks
- **Event Tracker CPU waste**: Keystrokes and cursor moves no longer enqueue IPC events when the daemon is offline
- **Cache capacity**: Was hardcoded to 100 — now reads your `omnicontext.prefetch.cacheSize` setting
- Resolve all CI workflow failures: license allowlist, security gate logic, release archive packaging
- Migrate deny.toml to cargo-deny v2 schema, eliminate deprecated keys
- Remove invalid `default` key from deny.toml licenses section

### Maintenance

- Whitelist accepted RUSTSEC advisories in deny.toml for transitive dependencies
- Restructure and normalize documentation to kebab-case naming conventions
- Harden CI pipelines and resolve supply chain audit failures
- Remove `Phase N` terminology from codebase, replace with descriptive category names
- Update distribution scripts to use consistent staging terminology

---

## [0.7.2] - 2026-03-07

### Added

- Extension now checks `~/.omnicontext/bin` (standalone installer path) as a binary candidate — bridge between the CLI installer and the VS Code extension
- Sidebar `Repair Environment` command triggers ONNX Runtime re-download and re-index in one click

### Fixed

- Binary lookup no longer blocks the VS Code UI thread via `execSync` — replaced with non-blocking `fs.existsSync` checks

---

## [0.7.1] - 2026-03-06

### Fixed

- Version resolution now queries GitHub Releases API correctly during auto-sync
- Removed unicode characters that caused rendering issues in some terminal environments

---

## [0.7.0] - 2026-03-06

### Added

- **Zero-Config MCP Sync**: On daemon start, the extension automatically writes your MCP server entry into all detected AI client configs (Claude Desktop, Cursor, Continue.dev, Kiro, Windsurf, Cline, RooCode, Trae, Antigravity, Claude Code CLI)
- Kiro IDE MCP support via the `powers.mcpServers` namespace
- `OmniContext: Sync MCP` command for manual re-sync to all AI clients

---

## [0.6.0] - 2026-03-06

### Added

- MCP manifest auto-published to `~/.omnicontext/mcp-manifest.json` for client auto-discovery
- Engine status exposes language distribution (per-language file counts) in the sidebar

---

## [0.4.0] - 2026-03-02

### Added

- **Enhanced Sidebar UI**: Real-time performance metrics (P50/P95/P99 search latency), activity log, repository info panel, and language distribution chart
- Professional VS Code codicons throughout the sidebar
- One-click cache clear button with immediate UI feedback

---

## [0.3.0] - 2026-03-01

### Added

- **Pre-Fetch Caching**: The extension now monitors your cursor position, file opens, and text edits to pre-fetch relevant code context before your AI agent requests it
- **Cache Statistics Panel**: Hit rate, cache size, hits/misses visible in the sidebar
- **Context Injection Speed**: Cache hits provide context in `<10ms` vs. 50–200ms for fresh searches. A `[Cached]` or `[Fresh]` indicator appears in injected context
- **Configuration**:
  - `omnicontext.prefetch.enabled` — toggle on/off
  - `omnicontext.prefetch.cacheSize` — max cache entries (10–1000, default 100)
  - `omnicontext.prefetch.cacheTtlSeconds` — cache lifetime (60–3600s, default 300s)
  - `omnicontext.prefetch.debounceMs` — event debounce delay (50–1000ms, default 200ms)
- Context injection now uses IPC to the daemon first, with CLI fallback

### Changed

- Extension auto-starts daemon on workspace open (`omnicontext.autoStartDaemon: true` by default)

---

## [0.2.0] - 2026-03-01

### Added

- VS Code sidebar with system status, cache metrics, and one-click controls
- `PrefetchCache` module in the engine daemon with LRU eviction and TTL expiration
- IPC interface: `prefetch_stats`, `clear_cache`, `update_config`, `shutdown`

---

## [0.1.0] - 2026-02-28

### Added

- Initial VS Code extension release
- `OmniContext: Index`, `Search`, `Status`, `Start Daemon`, `Stop Daemon`, and `Preflight` commands
- Chat participant registration for VS Code Copilot Chat integration (context injection)
- IPC client with named-pipe/Unix-socket transport and exponential backoff reconnection
- Status bar item showing daemon state
- Basic sidebar with connection status and cache hit counter

[0.9.2]: https://github.com/steeltroops-ai/omnicontext/compare/v0.7.2...v0.9.2
[0.7.2]: https://github.com/steeltroops-ai/omnicontext/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/steeltroops-ai/omnicontext/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.4.0...v0.6.0
[0.4.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/steeltroops-ai/omnicontext/releases/tag/v0.1.0
