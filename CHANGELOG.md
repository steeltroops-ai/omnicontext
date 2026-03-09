# Changelog

All notable changes to OmniContext are documented here.  
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).  
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.16.1] - 2026-03-09

### Fixed
- Broken doctests and async runtime issues ([c5380e8](https://github.com/steeltroops-ai/omnicontext/commit/c5380e8))
- Compilation errors in module exports and type annotations ([5ddfc63](https://github.com/steeltroops-ai/omnicontext/commit/5ddfc63))

### Changed
- Resolved workspace clippy warnings and test failures ([7a51cfb](https://github.com/steeltroops-ai/omnicontext/commit/7a51cfb))

## [0.16.0] - 2026-03-09

### Added
- Connection pooling for concurrent database access ([9a8ae9e](https://github.com/steeltroops-ai/omnicontext/commit/9a8ae9e))
- Contextual chunking and query result caching ([d668215](https://github.com/steeltroops-ai/omnicontext/commit/d668215))
- Batching, contrastive learning, and quantization support ([b18fce6](https://github.com/steeltroops-ai/omnicontext/commit/b18fce6))

### Documentation
- Restructured documentation with comprehensive guides ([97d8db9](https://github.com/steeltroops-ai/omnicontext/commit/97d8db9))

### Testing
- Added benchmarks and golden query test suite ([8a9dd68](https://github.com/steeltroops-ai/omnicontext/commit/8a9dd68))

## [0.15.0] - 2026-03-09

### Added
- Resilience monitoring and file dependency infrastructure ([f8ea24a](https://github.com/steeltroops-ai/omnicontext/commit/f8ea24a))
- Graph visualization and performance monitoring UI ([55293d7](https://github.com/steeltroops-ai/omnicontext/commit/55293d7))
- IPC handlers for VS Code extension integration ([a25b9f8](https://github.com/steeltroops-ai/omnicontext/commit/a25b9f8))
- File-level dependency graph for architectural context ([841d4b5](https://github.com/steeltroops-ai/omnicontext/commit/841d4b5))

### Fixed
- Updated embedder tests to use RERANKER_MODEL ([16c2bf6](https://github.com/steeltroops-ai/omnicontext/commit/16c2bf6))
- High-severity RCE in serialize-javascript via dependency overrides ([aa306f0](https://github.com/steeltroops-ai/omnicontext/commit/aa306f0))

### Changed
- Hardened automation suite with extension vetting and performance monitoring ([36b94e3](https://github.com/steeltroops-ai/omnicontext/commit/36b94e3))

## [0.14.0] - 2026-03-08

### Added
- Branch-aware diff indexing and SOTA performance optimizations ([464ab1f](https://github.com/steeltroops-ai/omnicontext/commit/464ab1f))

## [0.13.1] - 2026-03-08

### Fixed
- Hardened path resolution to prevent silent wrong-directory indexing ([38ad9a0](https://github.com/steeltroops-ai/omnicontext/commit/38ad9a0))
- Resolved ONNX Runtime version mismatch dynamically ([8943d04](https://github.com/steeltroops-ai/omnicontext/commit/8943d04))

## [0.13.0] - 2026-03-08

### Added
- Batch embedding and backpressure for indexing ([ea880bf](https://github.com/steeltroops-ai/omnicontext/commit/ea880bf))

## [0.12.0] - 2026-03-07

### Fixed
- Release workflow regex to allow commas in scopes ([e086fef](https://github.com/steeltroops-ai/omnicontext/commit/e086fef))

## [0.11.0] - 2026-03-07

### Added
- Overhauled sidebar UI with path normalization and set_workspace tool ([4a8cf35](https://github.com/steeltroops-ai/omnicontext/commit/4a8cf35))
- Marketplace badges and pre-rendered mermaid diagrams to READMEs ([1b38992](https://github.com/steeltroops-ai/omnicontext/commit/1b38992))

## [0.10.0] - 2026-03-07

### Added
- Core MCP and daemon optimizations ([f4f4450](https://github.com/steeltroops-ai/omnicontext/commit/f4f4450))

## [0.9.4] - 2026-03-07

### Fixed
- Generated separate scoped changelog for VS Code extension ([1f29da2](https://github.com/steeltroops-ai/omnicontext/commit/1f29da2))

### Changed
- Cleaned up unused markdown link references in VS Code changelog ([f608990](https://github.com/steeltroops-ai/omnicontext/commit/f608990))
- Restructured VS Code changelog to correctly group 0.9.x features under 0.9.2 ([ea23b10](https://github.com/steeltroops-ai/omnicontext/commit/ea23b10))

## [0.9.3] - 2026-03-07

### Fixed
- Resolved release workflow failures and rewrote changelog generation ([85161e3](https://github.com/steeltroops-ai/omnicontext/commit/85161e3))
- Resolved all workflow failures: license allowlist, security gate logic, release archive, changelog output ([f38e62f](https://github.com/steeltroops-ai/omnicontext/commit/f38e62f))

## [0.9.2] - 2026-03-07

### Fixed
- Resolved all CI workflow failures: license allowlist, security gate logic, release archive packaging
- Upgraded CodeQL action from v3 to v4

### Changed
- Whitelisted accepted RUSTSEC advisories in deny.toml for transitive dependencies
- Restructured and normalized documentation to kebab-case naming conventions

## [0.9.1] - 2026-03-07

### Fixed
- Migrated deny.toml to cargo-deny v2 schema, eliminated deprecated keys
- Fixed ort mutex poisoning in parallel test execution via ONNX model skip

### Changed
- Hardened CI pipelines and resolved supply chain audit failures

## [0.9.0] - 2026-03-07

### Fixed
- Removed invalid `default` key from deny.toml licenses section

### Changed
- Removed `Phase N` terminology from codebase, replaced with descriptive category names
- Updated distribution scripts to use consistent staging terminology

## [0.8.0] - 2026-03-07

### Added
- Zero-config MCP sync: extension auto-writes to Claude Desktop, Cursor, Continue.dev, Kiro, Windsurf, Cline, RooCode, Trae, Antigravity, Claude Code CLI configs on daemon start
- `Repair Environment` command: one-click re-download of ONNX Runtime and re-index
- Distribution scripts now auto-detect and configure all installed AI clients
- `omnicontext setup model-download` CLI command to trigger model download without full indexing
- `omnicontext setup model-status --json` for machine-readable model readiness check

### Changed
- Distribution scripts now show `[v]` / `[!]` / `[x]` status indicators with color support
- Install step count increased from 6 to 7 to include dedicated MCP auto-configure step
- Model setup uses new `setup` subcommand when available; falls back to legacy `index .` trigger

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
- Zero-config MCP sync: on daemon start, extension automatically writes MCP server entry into all detected AI client configs
- Kiro IDE MCP support via the `powers.mcpServers` namespace
- `OmniContext: Sync MCP` command for manual re-sync to all AI clients

## [0.6.1] - 2026-03-06

### Fixed
- GitHub Actions release job missing `write` permission for release asset upload
- Release workflow skips redundant build jobs when no release tag is detected

## [0.6.0] - 2026-03-06

### Added
- MCP manifest auto-published to `~/.omnicontext/mcp-manifest.json` for client auto-discovery
- Engine status exposes language distribution (per-language file counts) in the sidebar

## [0.5.3] - 2026-03-02

### Fixed
- MCP install/test scripts updated for new binary layout
- Daemon database lock conflicts in concurrent test runs resolved

## [0.5.0] - 2026-03-02

### Added
- Graph-boosted hybrid search: dependency proximity used to re-rank results
- In-degree and graph distance integrated into RRF scoring
- Dynamic version detection in distribution scripts from `Cargo.toml` and GitHub API

### Fixed
- ONNX partial batch failures no longer drop entire file
- `get_file_summary` MCP tool path normalization for Windows UNC paths

## [0.4.0] - 2026-03-02

### Added
- Enhanced VS Code sidebar with professional codicons, real-time performance metrics, activity log, and cache statistics panel
- Language distribution visualization in sidebar
- One-click environment repair from sidebar

## [0.3.0] - 2026-03-01

### Added
- Context pre-fetching system: VS Code extension pre-fetch context caching
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

## [0.2.0] - 2026-03-01

### Added
- CI/CD release pipeline, MCP graph statistics, and automated benchmarks
- Distribution architecture: installation system, VSIX packaging, and cross-platform setup scripts
- Java, C, C++, C#, CSS language analyzers
- Markdown, TOML, YAML, JSON, HTML, Shell document indexing
- 8 MCP AI-facing tools: `search_code`, `get_context`, `get_file_summary`, `get_dependencies`, `get_dependents`, `get_stats`, `get_module_map`, `search_by_kind`
- Dependency graph with import extraction for TS, JS, Go
- Query expansion for natural-language queries
- Module-qualified FQNs across all 16 supported languages
- One-click install scripts for binary + model auto-download

## [0.1.0-alpha] - 2026-02-28

### Added
- Core framework: Cargo workspace with 10 decoupled subsystem modules
- Semantic parsers: language analyzers for Python, Rust, TypeScript, JavaScript, Go
- Hybrid retrieval: BM25 + semantic vector search with RRF fusion scoring
- Graph topology: dependency graph with petgraph, RwLock, and cycle detection
- SQLite schema with FTS5 full-text search, sync triggers, performance indexes
- Configuration system with 5-level precedence chain
- Hierarchical error taxonomy: Recoverable / Degraded / Fatal
- Core domain types: `Language`, `Chunk`, `Symbol`, `DependencyEdge`, `SearchResult`
- Tree-sitter grammar registrations for all supported languages
- IPC daemon with named-pipe/Unix-socket transport and JSON-RPC 2.0 protocol

[Unreleased]: https://github.com/steeltroops-ai/omnicontext/compare/v0.16.1...HEAD
[0.16.1]: https://github.com/steeltroops-ai/omnicontext/compare/v0.16.0...v0.16.1
[0.16.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.15.0...v0.16.0
[0.15.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.14.0...v0.15.0
[0.14.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.13.1...v0.14.0
[0.13.1]: https://github.com/steeltroops-ai/omnicontext/compare/v0.13.0...v0.13.1
[0.13.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.9.4...v0.10.0
[0.9.4]: https://github.com/steeltroops-ai/omnicontext/compare/v0.9.3...v0.9.4
[0.9.3]: https://github.com/steeltroops-ai/omnicontext/compare/v0.9.2...v0.9.3
[0.9.2]: https://github.com/steeltroops-ai/omnicontext/compare/v0.9.1...v0.9.2
[0.9.1]: https://github.com/steeltroops-ai/omnicontext/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/steeltroops-ai/omnicontext/compare/v0.7.2...v0.8.0
[0.7.2]: https://github.com/steeltroops-ai/omnicontext/compare/v0.7.1...v0.7.2
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

<!-- generated by git-cliff -->
