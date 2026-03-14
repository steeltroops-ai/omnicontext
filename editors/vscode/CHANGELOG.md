# Changelog

All notable changes to the OmniContext VS Code extension are documented here.

## [1.3.0] - 2026-03-14


- migrate model surface from jina to CodeRankEmbed ([b7a1162](https://github.com/steeltroops-ai/omnicontext/commit/b7a1162))



## [1.2.3] - 2026-03-13

_No direct changes in this release._


## [1.2.2] - 2026-03-13


- implement SCC cycle detection, fix extension context bug, remove phase labels ([fdc0033](https://github.com/steeltroops-ai/omnicontext/commit/fdc0033))
- normalize file extensions to lowercase before language detection ([f0f8459](https://github.com/steeltroops-ai/omnicontext/commit/f0f8459))

- fill extension changelog gaps for versions 0.16.0 through 1.2.1 ([d592c4e](https://github.com/steeltroops-ai/omnicontext/commit/d592c4e))
- rewrite changelogs to enterprise standards and fix cliff.toml ([629e07a](https://github.com/steeltroops-ai/omnicontext/commit/629e07a))

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- Core engine now correctly indexes files with uppercase or mixed-case extensions on
  case-insensitive filesystems (macOS, Windows); `Main.RS`, `App.PY`, `Index.TS` are no
  longer silently skipped during directory scans

## [1.2.1] - 2026-03-12

### Fixed
- Restore distribution manifest fields stripped by release automation from Homebrew formula
  and Scoop manifest
- Fix unused-variable CI failures on Linux in the core engine build
- File extension normalization in daemon now case-insensitive; improves indexed file coverage displayed in sidebar

## [1.2.0] - 2026-03-12

### Added
- **Antigravity IDE support**: OmniContext MCP server now auto-configures in Antigravity IDE
  (`~/.config/Antigravity/User/mcp.json` on Linux/macOS, `%APPDATA%\Antigravity\User\mcp.json`
  on Windows)
- **17 AI client auto-configuration**: `setup --all` now covers Claude Desktop, Claude Code,
  Cursor, Windsurf, VS Code, VS Code Insiders, Cline, RooCode, Continue.dev, Zed, Kiro,
  PearAI, Trae, Antigravity, Gemini CLI, Amazon Q CLI, and Augment Code
- **Embedding model selection**: New `OmniContext: Select Embedding Model` command in the
  Command Palette; choose between `jina-embeddings-v2-base-code` (550 MB),
  `jina-embeddings-v2-small-en` (130 MB), or `all-minilm-l6-v2` (22 MB); saved to
  `omnicontext.embeddingModel`; daemon restarts automatically on change
- **IPC startup resilience**: 2-second initial delay before first IPC connection attempt when
  daemon starts cold; status bar shows `$(sync~spin) OmniContext: Engine loading...` during
  initialization rather than immediately showing disconnected
- **Cross-platform ONNX detection**: `resolveBinaries()` now checks for `libonnxruntime.so`,
  `libonnxruntime.dylib`, and versioned variants so ONNX is correctly detected on Linux/macOS
  without unnecessary re-downloads
- **Improved repository visibility**: Sidebar registry shows all indexed repositories with
  normalized paths, repository count at top level, and one-click re-index actions per entry
- **Distribution script enterprise flags**: Install, uninstall, and update scripts now accept
  `--help`, `--no-model`, `--no-mcp`, `--no-onnx`, `--dry-run`, `--dir`, and `--model`

### Fixed
- Auto-sync MCP configuration now runs after every binary update, ensuring newly supported
  IDE clients are registered without manual re-sync
- Addressed `quinn-proto` CVE RUSTSEC-2026-0037 by updating to 0.11.14; all Node.js
  dependencies pass `bun audit` with no vulnerabilities

### Changed
- Sidebar repository list now shows normalized, consistently formatted paths

## [1.1.2] - 2026-03-11

### Fixed
- Harden indexing pipeline against partial file-parse failures to prevent the pipeline from
  stalling mid-scan
- Stabilize daemon lifecycle management: prevent premature daemon shutdown during active
  indexing and ensure clean process teardown on workspace close
- Daemon lifecycle stability improvements reflected in connection state UI and status bar

## [1.1.1] - 2026-03-09

_No extension-specific changes. See root changelog for website fixes._

## [1.1.0] - 2026-03-09

_No extension-specific changes. See root changelog for documentation site additions._

## [1.0.1] - 2026-03-09

### Fixed
- Align changelog version headers with git tags to correct mismatched entries
- `engines.vscode` minimum version specifier corrected to `^1.109.0`

## [0.16.1] - 2026-03-09

_No extension-specific changes. See root changelog for core engine compilation fixes._

## [0.16.0] - 2026-03-09

### Changed
- Search result cache metrics now visible in performance panel

## [0.15.0] - 2026-03-09

### Added
- **Graph visualization panel**: Sidebar panel showing dependency graph metrics and symbol
  connectivity statistics powered by the new `FileDependencyGraph` in the core engine
- **Performance monitoring panel**: Real-time P50/P95/P99 search latency, embedding
  throughput, and index pool utilization exposed through new IPC handlers in the daemon

## [0.14.0] - 2026-03-08

### Added
- **Branch-aware context**: Extension benefits from branch-aware diff indexing in the core
  engine; search results automatically surface files changed on the current branch

## [0.13.1] - 2026-03-08

### Fixed
- Hardened path resolution in MCP tools prevents silent wrong-directory indexing when the
  workspace path is ambiguous

## [0.13.0] - 2026-03-08

### Added
- **Batch embedding with backpressure**: Core engine now batches embedding requests before ONNX
  inference with queue-depth flow control, reducing memory pressure during large repo indexing

## [0.12.0] - 2026-03-07

_No extension-specific changes. See root changelog for release workflow fix._

## [0.11.0] - 2026-03-07

### Added
- **Sidebar UI overhaul**: Repository selector, indexed file counts, language distribution
  chart, and one-click workspace switching
- **Path normalization**: All file paths stored and displayed with consistent cross-platform
  normalization, eliminating duplicates from mixed Windows UNC and forward-slash paths
- **`set_workspace` MCP tool**: AI agents can explicitly switch the active repository context
  without restarting the daemon

### Changed
- Add VS Code Marketplace badges and pre-rendered Mermaid architecture diagrams to README

## [0.10.0] - 2026-03-07

### Added
- **Engine and daemon optimizations**: Reduced per-query IPC latency via optimized message
  serialization; eliminated redundant index lookups in the search hot path

## [0.9.4] - 2026-03-07

### Fixed
- Generate separate scoped changelog entries for the VS Code extension and the Rust engine
- Correct changelog structure to accurately group 0.9.x feature entries under correct version
  headers and restore the `[Unreleased]` header

## [0.9.2] - 2026-03-07

### Added
- **Zero-friction install**: Extension auto-downloads the OmniContext engine binary and ONNX
  Runtime on first install with a VS Code progress notification showing download status
- **ONNX auto-repair**: Platform-specific ONNX Runtime shared libraries are fetched
  automatically when missing from the installation directory
- **Sidebar offline state**: Clean offline indicator replaces the frozen sidebar state that
  occurred when the engine was not running

### Fixed
- **Sidebar freeze**: Fixed critical bug where the sidebar became permanently unresponsive
  when the daemon was not running due to unbounded promise accumulation
- **IPC timeouts**: Reduced from 30 seconds to 3 seconds for status and metrics requests,
  preventing long UI hangs when the daemon is unreachable
- **Event tracker CPU waste**: Keystrokes and cursor moves no longer enqueue IPC events when
  the daemon is offline
- **Cache capacity**: Now reads the `omnicontext.prefetch.cacheSize` setting instead of a
  hardcoded value of 100
- Resolve all CI workflow failures: license allowlist, security gate logic, release archive
  packaging
- Migrate `deny.toml` to cargo-deny v2 schema

### Changed
- Whitelist accepted RUSTSEC advisories for transitive dependencies with no upstream fix
- Restructure and normalize documentation to kebab-case naming conventions

## [0.7.2] - 2026-03-07

### Added
- Extension now checks `~/.omnicontext/bin` as a binary candidate, supporting the standalone
  installer path without requiring VS Code settings configuration
- `Repair Environment` sidebar command triggers ONNX Runtime re-download and re-index in one
  action

### Fixed
- Binary lookup no longer blocks the VS Code UI thread via synchronous `execSync` calls

## [0.7.1] - 2026-03-06

### Fixed
- Version resolution now queries the GitHub Releases API correctly during auto-update checks
- Remove Unicode characters that caused rendering issues in certain terminal environments

## [0.7.0] - 2026-03-06

### Added
- **Zero-config MCP sync**: On daemon start, extension automatically writes the MCP server
  entry into all detected AI client configurations (Claude Desktop, Cursor, Continue.dev, Kiro,
  Windsurf, Cline, RooCode, Trae, Antigravity, Claude Code CLI)
- **Kiro IDE support**: MCP server entry written using the `powers.mcpServers` namespace
- **`OmniContext: Sync MCP` command**: Manual re-sync of MCP server entry across all detected
  AI clients without requiring a daemon restart

## [0.6.0] - 2026-03-06

### Added
- MCP manifest auto-published to `~/.omnicontext/mcp-manifest.json` for AI client
  auto-discovery
- Engine status exposes per-language file distribution (counts by language) in the sidebar

## [0.4.0] - 2026-03-02

### Added
- **Enhanced sidebar**: Real-time performance metrics panel (P50/P95/P99 search latency),
  activity log with per-operation detail, repository info panel, and language distribution
  chart; professional VS Code codicons used throughout
- One-click cache clear button with immediate UI feedback

## [0.3.0] - 2026-03-01

### Added
- **Pre-fetch caching**: Extension monitors cursor position, file opens, and text edits to
  pre-fetch relevant code context in the background
- **Cache statistics panel**: Hit rate, cache size, hits and misses visible in the sidebar
- Context injection via cache hits delivers results in `<10ms` versus 50-200ms for fresh
  queries
- Configuration: `omnicontext.prefetch.enabled`, `cacheSize`, `cacheTtlSeconds`, `debounceMs`
- Context injection uses daemon IPC first with CLI fallback

### Changed
- Extension auto-starts daemon on workspace open (`omnicontext.autoStartDaemon: true` by
  default)

## [0.2.0] - 2026-03-01

### Added
- **VS Code sidebar**: System status display, cache metrics panel, and one-click controls
- `PrefetchCache` module in the engine daemon with LRU eviction and TTL expiration
- IPC interface: `prefetch_stats`, `clear_cache`, `update_config`, `shutdown`

## [0.1.0] - 2026-02-28

### Added
- Initial VS Code extension release
- Commands: `Index`, `Search`, `Status`, `Start Daemon`, `Stop Daemon`, `Preflight`
- Chat participant registration for VS Code Copilot Chat integration (context injection)
- IPC client with named-pipe and Unix-socket transport and exponential backoff reconnection
- Status bar item showing daemon connection state
- Basic sidebar with connection status and cache hit counter

[1.2.1]: https://github.com/steeltroops-ai/omnicontext/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/steeltroops-ai/omnicontext/compare/v1.1.2...v1.2.0
[1.1.2]: https://github.com/steeltroops-ai/omnicontext/compare/v1.1.1...v1.1.2
[1.1.1]: https://github.com/steeltroops-ai/omnicontext/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/steeltroops-ai/omnicontext/compare/v1.0.1...v1.1.0
[1.0.1]: https://github.com/steeltroops-ai/omnicontext/compare/v0.16.1...v1.0.1
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
