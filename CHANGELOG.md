# Changelog

## [0.16.0] - 2026-03-09

## What's Changed

### Features

- add connection pooling for concurrent database access (9a8ae9e)
- add contextual chunking and query result caching (d668215)
- add batching, contrastive learning, and quantization support (b18fce6)

### Maintenance

- restructure documentation with comprehensive guides (97d8db9)
- add benchmarks and golden query test suite (8a9dd68)

## [0.15.0] - 2026-03-09

## What's Changed

### Features

- add resilience monitoring and file dependency infrastructure (f8ea24a)
- add graph visualization and performance monitoring UI (55293d7)
- add IPC handlers for VS Code extension phases 4-6 (a25b9f8)
- add file-level dependency graph for architectural context (841d4b5)

### Bug Fixes

- update embedder tests to use RERANKER_MODEL (16c2bf6)

### Maintenance

- harden automation suite with extension vetting and performance monitoring (36b94e3)

### Other

- security(vscode): fix high-severity rce in serialize-javascript via dependency overrides (aa306f0)

## [0.14.0] - 2026-03-08

## What's Changed

### Features

- implement branch-aware diff indexing and sota performance optimizations (464ab1f)

## [0.13.1] - 2026-03-08

## What's Changed

### Bug Fixes

- harden path resolution to prevent silent wrong-dir indexing (38ad9a0)
- resolve onnx runtime version mismatch dynamically (8943d04)

## [0.13.0] - 2026-03-08

## What's Changed

### Features

- implement batch embedding and backpressure for indexing (ea880bf)

## [0.12.0] - 2026-03-07

## What's Changed

### Maintenance

- fix release workflow regex to allow commas in scopes (e086fef)

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

## [0.9.3] - 2026-03-07

## What's Changed

### Bug Fixes

- resolve release workflow failures and rewrite changelog generation (85161e3)
- resolve all workflow failures - license allowlist, security gate logic, release archive, changelog output (f38e62f)

## [0.9.2] - 2026-03-07

### Bug Fixes

- Resolve all CI workflow failures: license allowlist, security gate logic, release archive packaging
- Upgrade CodeQL action from v3 to v4

### Maintenance

- Whitelist accepted RUSTSEC advisories in deny.toml for transitive dependencies
- Restructure and normalize documentation to kebab-case naming conventions

## [0.9.1] - 2026-03-07

### Bug Fixes

- Migrate deny.toml to cargo-deny v2 schema, eliminate deprecated keys
- Fix ort mutex poisoning in parallel test execution via ONNX model skip

### Maintenance

- Harden CI pipelines and resolve supply chain audit failures

## [0.9.0] - 2026-03-07

### Bug Fixes

- Remove invalid `default` key from deny.toml licenses section

### Maintenance

- Remove `Phase N` terminology from codebase, replace with descriptive category names
- Update distribution scripts to use consistent staging terminology

All notable changes to OmniContext are documented here.  
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).  
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

- Bootstrap service: zero-friction binary auto-download when extension installs — no Rust required
- ONNX Runtime auto-download for Windows (`.dll`), Linux (`.so`), macOS (`.dylib`) — no manual setup
- Circuit breaker in sidebar: IPC calls are now skipped when daemon is offline, eliminating sidebar freeze
- IPC timeout reduced from 30s to 3s for status requests (15s for user-initiated preflight)
- Event tracker gated on daemon connection — no CPU wasted on keystroke events when engine is offline
- `sendBootstrapStatus()` on sidebar provider — shows download progress in sidebar during first install
- `onnxruntime_providers_shared.dll` co-location detection and install on Windows

### Fixed

- Sidebar freeze caused by stacked timed-out IPC promises when daemon is not running
- `getBinaryPath()` was using blocking `execSync` on activation — now uses `fs.existsSync`
- Extension ignored `~/.omnicontext/bin` (standalone installer path) — now checked as candidate
- Cache stats `capacity` was hardcoded to 100 — now reads `omnicontext.prefetch.cacheSize` from config
- `install.ps1` gave passive warning on missing ONNX DLL — now auto-downloads from Microsoft
- `update.ps1` terminated processes too late (binary was locked) — kill now runs before download

---

## [0.8.0] - 2026-03-07

### Added

- Zero-Config MCP sync: extension auto-writes to Claude Desktop, Cursor, Continue.dev, Kiro, Windsurf, Cline, RooCode, Trae, Antigravity, Claude Code CLI configs on daemon start
- `Repair Environment` command: one-click re-download of ONNX Runtime and re-index
- Distribution scripts (`install.ps1`, `install.sh`) now auto-detect and configure all installed AI clients
- `omnicontext setup model-download` CLI command to trigger model download without full indexing
- `omnicontext setup model-status --json` for machine-readable model readiness check

### Changed

- Distribution scripts now show `[v]` / `[!]` / `[x]` status indicators with color support
- Install step count increased from 6 to 7 to include dedicated MCP auto-configure step
- Model setup uses new `setup` subcommand when available; falls back to legacy `index .` trigger

---

## [0.7.1] - 2026-03-06

### Fixed

- Version resolution now queries GitHub Releases API first, falling back to `Cargo.toml` source parse
- Removed unicode em-dash characters that caused PowerShell rendering issues on older terminals

---

## [0.7.0] - 2026-03-06

### Added

- Managed `setup` subcommand: `model-download`, `model-status --json` for headless automation
- Distribution scripts overhauled with cross-platform IDE auto-configuration
- Kiro IDE support via `powers.mcpServers` namespace (non-standard MCP location)

### Changed

- All distribution scripts share unified color helpers and status output format

---

## [0.6.1] - 2026-03-06

### Fixed

- GitHub Actions release job missing `write` permission for release asset upload
- Release workflow skips redundant build jobs when no release tag is detected

---

## [0.6.0] - 2026-03-06

### Added

- Zero-Config MCP architecture: manifest published to `~/.omnicontext/mcp-manifest.json`
- MCP manifest auto-discovery — clients can read location without hardcoded paths
- `syncMcp` VS Code command writes manifest path into all supported AI client configs

### Changed

- Engine status exposes `language_distribution` field (per-language file counts)

---

## [0.5.3] - 2026-03-02

### Fixed

- MCP install/test scripts updated for new binary layout
- Daemon database lock conflicts in concurrent test runs resolved

---

## [0.5.0] - 2026-03-02

### Added

- Graph-boosted hybrid search: dependency proximity used to re-rank results
- In-degree and graph distance integrated into RRF scoring
- Dynamic version detection in distribution scripts from `Cargo.toml` and GitHub API

### Fixed

- ONNX partial batch failures no longer drop entire file — unembedded chunks fall back to FTS-only
- `get_file_summary` MCP tool path normalization for Windows UNC paths

---

## [0.4.0] - 2026-03-02

### Added

- **Sidebar Control Center**: Enhanced VS Code sidebar with professional codicons, real-time performance metrics, activity log, and cache statistics panel
- Language distribution visualization in sidebar
- One-click environment repair from sidebar

---

## [0.3.0] - 2026-03-01

### Added

- **Context Pre-fetching System**: VS Code extension pre-fetch context caching system
- `EventTracker` module: debounced file-open, cursor-move, and text-edit tracking
- `SymbolExtractor` module: cursor-position symbol detection using VS Code language providers
- `CacheStatsManager`: hit rate, size, and TTL statistics surfaced in sidebar
- Pre-fetch daemon cache with LRU eviction and configurable TTL
- IPC methods: `clear_cache`, `prefetch_stats`, `update_config`
- Configuration: `omnicontext.prefetch.enabled`, `cacheSize`, `cacheTtlSeconds`, `debounceMs`
- Cache hit indicator (instant, `<10ms`) and fresh search indicator in context injection output

### Changed

- Extension auto-starts daemon and begins event tracking on workspace open
- Preflight handler checks cache first; stores result on miss
- IPC timeout and reconnection improved with exponential backoff

---

## [0.2.0] - 2026-03-01

### Added

- **Automation & Telemetry**: CI/CD release pipeline, MCP graph statistics, and automated benchmarks
- **Distribution Architecture**: Installation system, VSIX packaging, and cross-platform setup scripts
- Java, C, C++, C#, CSS language analyzers
- Markdown, TOML, YAML, JSON, HTML, Shell document indexing (`DocumentAnalyzer`)
- 8 MCP AI-facing tools: `search_code`, `get_context`, `get_file_summary`, `get_dependencies`, `get_dependents`, `get_stats`, `get_module_map`, `search_by_kind`
- Dependency graph with import extraction for TS, JS, Go (was empty before)
- Query expansion for natural-language queries (stop-word stripping, OR-join)
- Module-qualified FQNs across all 16 supported languages
- One-click install scripts for binary + model auto-download

---

## [0.1.0-alpha] - 2026-02-28

### Added

- **Core Framework**: Cargo workspace (`omni-core`, `omni-mcp`, `omni-cli`) with 10 decoupled subsystem modules
- **Semantic Parsers**: Language analyzers for Python, Rust, TypeScript, JavaScript, Go (81 tests, 0 failures)
- **Hybrid Retrieval**: Retrieval engine with BM25 + semantic vector search and RRF fusion scoring
- **Graph Topology**: Dependency graph with petgraph, RwLock, and cycle detection
- SQLite schema with FTS5 full-text search, sync triggers, performance indexes
- Configuration system with 5-level precedence chain (CLI > env > project > user > defaults)
- Hierarchical error taxonomy (`OmniError`): Recoverable / Degraded / Fatal
- Core domain types: `Language`, `Chunk`, `Symbol`, `DependencyEdge`, `SearchResult`
- Tree-sitter grammar registrations for all supported languages
- IPC daemon with named-pipe/Unix-socket transport and JSON-RPC 2.0 protocol

[Unreleased]: https://github.com/steeltroops-ai/omnicontext/compare/v0.9.2...HEAD
[0.9.2]: https://github.com/steeltroops-ai/omnicontext/compare/v0.9.1...v0.9.2
[0.9.1]: https://github.com/steeltroops-ai/omnicontext/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.7.1...v0.8.0
[0.7.1]: https://github.com/steeltroops-ai/omnicontext/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/steeltroops-ai/omnicontext/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.5.3...v0.6.0
[0.5.3]: https://github.com/steeltroops-ai/omnicontext/compare/v0.5.0...v0.5.3
[0.5.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.1.0-alpha...v0.2.0
[0.1.0-alpha]: https://github.com/steeltroops-ai/omnicontext/releases/tag/v0.1.0-alpha
