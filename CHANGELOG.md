# Changelog

All notable changes to OmniContext are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- Correct file discovery to index files with uppercase or mixed-case extensions on
  case-insensitive filesystems (macOS, Windows); files such as `Main.RS`, `App.PY`, and
  `Index.TS` are now correctly detected and indexed instead of being silently skipped

## [1.2.1] - 2026-03-12

### Fixed
- Restore distribution manifest fields (`license`, `bottle :unneeded`, `post_install`,
  `caveats`/`notes`) stripped from Homebrew formula and Scoop manifest by release automation
- Fix unused-variable CI failures on Linux by referencing pre-declared `app_support` and
  `appdata` variables in `orchestrator.rs` and `main.rs` instead of inlining platform paths

## [1.2.0] - 2026-03-12

### Added
- **Antigravity IDE support**: OmniContext MCP server now auto-configures in Antigravity IDE
  (`~/.config/Antigravity/User/mcp.json` on Linux/macOS, `%APPDATA%\Antigravity\User\mcp.json`
  on Windows) using the `servers` key format compatible with Antigravity's MCP protocol
- **17 AI client auto-configuration**: Distribution scripts and `setup --all` now cover Claude
  Desktop, Claude Code, Cursor, Windsurf, VS Code, VS Code Insiders, Cline, RooCode,
  Continue.dev, Zed, Kiro, PearAI, Trae, Antigravity, Gemini CLI, Amazon Q CLI, Augment Code
- **Embedding model selection command**: New `OmniContext: Select Embedding Model` command in
  the VS Code Command Palette lets users choose between `jina-embeddings-v2-base-code` (550 MB),
  `jina-embeddings-v2-small-en` (130 MB), and `all-minilm-l6-v2` (22 MB); saved to
  `omnicontext.embeddingModel`; daemon restarts automatically on change
- **IPC startup resilience**: Added 2-second initial delay before first IPC connection attempt
  when the daemon starts cold to allow ONNX model initialization; status bar shows
  `Engine loading...` during startup rather than immediately showing disconnected
- **Cross-platform ONNX detection**: `resolveBinaries()` now checks for `libonnxruntime.so`,
  `libonnxruntime.dylib`, and versioned variants on Linux/macOS so ONNX is correctly detected
  and not unnecessarily re-downloaded
- **Improved indexed repository visibility**: Sidebar now shows a registry of indexed
  repositories with normalized paths, repository count at top level, and one-click re-index
  actions per repository
- **Distribution script enterprise flags**: Install, uninstall, and update scripts now accept
  `--help`, `--no-model`, `--no-mcp`, `--no-onnx`, `--dry-run`, `--dir`, and `--model` flags

### Fixed
- Mark ONNX-dependent FFI integration tests with `#[ignore]` so `cargo test --workspace`
  completes in reasonable time without requiring the 550 MB model download; run with
  `-- --ignored` for full integration coverage
- Fix 29 clippy warnings across `omni-core`, `omni-mcp`, `omni-daemon`, `omni-ffi`, and
  `omni-cli`: `doc_markdown`, `map_or_else`, `manual_clamp`, `similar_names`, `clone_on_copy`,
  `case_sensitive_file_extension_comparisons`, and `unnecessary_sort_by`
- Update `quinn-proto` to 0.11.14 to resolve CVE RUSTSEC-2026-0037 DoS vulnerability
- Auto-sync MCP configuration now runs after every binary update rather than only on first
  install, ensuring newly supported IDE clients are registered automatically

### Changed
- Sidebar repository list now shows normalized, consistently formatted paths for all indexed
  repositories

## [1.1.2] - 2026-03-11

### Fixed
- Harden indexing pipeline against partial failures during concurrent file processing to
  prevent the pipeline from stalling when individual file parse errors occur
- Stabilize VS Code extension daemon lifecycle: prevent premature daemon shutdown during active
  indexing operations and ensure clean process teardown on workspace close

## [1.1.1] - 2026-03-09

### Fixed
- Resolve 404 errors on documentation site pages caused by incorrect static asset path
  resolution in the Next.js routing configuration
- Fix Mermaid diagram rendering failures caused by missing initialization on certain page routes

## [1.1.0] - 2026-03-09

### Added
- **Documentation site**: Markdown-based documentation system with architecture, configuration,
  contributing, and enterprise reference pages served from the Next.js website
- **Interactive Mermaid diagrams**: Architecture diagrams with pan, mouse-wheel zoom, and
  focus controls integrated into the documentation site
- **Smooth scrolling**: Lenis smooth scrolling across documentation pages and the homepage;
  table-of-contents sidebar with scroll-spy for accurate active-section tracking

### Fixed
- Resolve hydration errors caused by unstable generated ID values in server-rendered components
- Correct documentation sidebar path so all documentation pages appear in navigation
- Fix MCP tool count displayed on homepage (corrected from 8 to 16)
- Resolve rendering and layout issues in documentation sidebar organization

### Documentation
- Add architecture, configuration, contributing, and enterprise documentation pages
- Restructure documentation site with production-ready organization and naming conventions

## [1.0.1] - 2026-03-09

### Fixed
- Align changelog version headers with corresponding git tags to correct mismatched entries
  introduced by earlier automated changelog generation
- Fix VS Code extension `engines.vscode` compatibility field to correctly specify the minimum
  supported VS Code version

## [0.16.1] - 2026-03-09

### Fixed
- Resolve broken doc tests in `omni-core` caused by async runtime initialization conflicts in
  the test harness
- Fix compilation errors in module exports and type annotation mismatches introduced in 0.16.0

## [0.16.0] - 2026-03-09

### Added
- **SQLite connection pooling**: Concurrent database access via a connection pool, eliminating
  lock contention during parallel file indexing operations
- **Contextual chunking**: Chunk boundaries respect semantic context windows, preserving
  complete function signatures and improving retrieval quality for multi-construct queries
- **Query result caching**: Search results cached per-query with LRU eviction; cache entries
  invalidated on file reindex events to prevent stale results from surfacing
- **Embedding enhancements**: Contrastive learning fine-tuning support and optional
  quantization for reduced ONNX model memory footprint during inference

### Documentation
- Restructure documentation with comprehensive reference guides for configuration, API usage,
  and integration patterns

### Testing
- Add criterion benchmarks for indexing throughput, search latency, and embedding throughput
- Add golden query test suite for regression detection on search relevance across reference
  repositories

## [0.15.0] - 2026-03-09

### Added
- **Resilience monitoring**: Per-subsystem health monitor tracks failure rates and latencies;
  circuit breakers on the index and embedder paths prevent cascading failures during degraded
  conditions
- **File-level dependency graph**: `FileDependencyGraph` tracks file-to-file dependency
  relationships independently of symbol-level edges, enabling architectural context queries
- **Graph and performance monitoring UI**: VS Code sidebar panels expose dependency graph
  metrics, connectivity statistics, P50/P95/P99 search latency, embedding throughput, and
  index pool utilization in real time
- **Extended IPC handlers**: Daemon IPC now serves reranker metrics, resilience circuit-breaker
  status, embedder metrics, index pool statistics, and compression statistics

### Fixed
- Update embedder test fixtures to reference the correct `RERANKER_MODEL` constant name

## [0.14.0] - 2026-03-08

### Added
- **Branch-aware diff indexing**: Engine computes files changed relative to the default branch
  and applies a retrieval score boost to those files, surfacing branch-relevant results without
  requiring a full reindex on branch switch
- **SOTA performance optimizations**: Adaptive batch sizing for parallel chunk embedding;
  result deduplication across overlapping chunks from the same file; structural weight boost
  applies `kind` and `visibility` multipliers to RRF fusion scores for better ranking precision

## [0.13.1] - 2026-03-08

### Fixed
- Harden MCP path resolution to prevent silent wrong-directory indexing when the working
  directory is ambiguous; tries multiple path variants (relative, absolute, UNC-stripped,
  canonicalized) before reporting a file-not-found error
- Resolve ONNX Runtime version mismatch at runtime by dynamically selecting the compatible
  shared library version rather than failing on binary incompatibility

## [0.13.0] - 2026-03-08

### Added
- **Batch embedding with backpressure**: Embedding requests batched in configurable windows
  (default 80 chunks) before ONNX inference; a backpressure monitor tracks queue depth and
  applies flow control to prevent memory pressure during large repository indexing

## [0.12.0] - 2026-03-07

### Fixed
- Fix release workflow commit-parser regex to accept scopes containing commas (e.g.,
  `feat(core,mcp,vscode):`), which previously caused multi-scope commits to be silently
  omitted from generated changelogs

## [0.11.0] - 2026-03-07

### Added
- **Sidebar UI overhaul**: Repository selector, indexed file counts, language distribution
  chart, and one-click workspace switching added to the VS Code sidebar
- **Path normalization**: All file paths stored and displayed using consistent cross-platform
  normalization, eliminating duplicates caused by mixed Windows UNC and forward-slash paths
- **`set_workspace` MCP tool**: AI agents can explicitly switch the active repository context
  without restarting the daemon

### Documentation
- Add VS Code Marketplace badges and pre-rendered Mermaid architecture diagrams to README files

## [0.10.0] - 2026-03-07

### Added
- **Engine and MCP optimizations**: Reduce per-query latency by doubling the RRF candidate
  pre-fetch count before re-ranking; optimize daemon IPC message serialization; eliminate
  redundant index lookups in the search hot path

## [0.9.4] - 2026-03-07

### Fixed
- Generate separate scoped changelog entries for the VS Code extension and the Rust engine so
  extension releases only reflect extension-relevant changes
- Correct VS Code changelog structure to accurately group 0.9.x feature entries under the
  correct version headers

## [0.9.3] - 2026-03-07

### Fixed
- Resolve multiple release workflow failures: license allowlist evaluation, security gate
  condition logic, release archive packaging, and changelog output formatting
- Fix all CI workflow failures: cargo-deny configuration, CodeQL action version compatibility,
  and secrets context usage in conditional workflow job steps

## [0.9.2] - 2026-03-07

### Fixed
- Remove invalid `default` key from `deny.toml` licenses section that caused cargo-deny to
  fail during parsing with a configuration schema error
- Whitelist accepted RUSTSEC advisories for transitive dependencies that have no available
  upstream fix

## [0.9.1] - 2026-03-07

### Fixed
- Migrate `deny.toml` to cargo-deny v2 schema, removing all deprecated configuration keys
- Eliminate ORT mutex poisoning in the test suite caused by shared static ONNX session state
  across parallel test cases

### Documentation
- Restructure and normalize all documentation files to kebab-case naming conventions

## [0.9.0] - 2026-03-07

### Added
- **Zero-friction bootstrap**: Extension auto-downloads the OmniContext engine binary and ONNX
  Runtime on first install; VS Code notification shows real-time download progress during setup
- **ONNX auto-repair**: Platform-specific ONNX Runtime shared libraries are fetched
  automatically when missing from the installation directory
- **Sidebar circuit breaker**: All IPC calls are gated behind a circuit breaker that opens
  after repeated failures, preventing unbounded promise queuing when the daemon is offline

## [0.8.0] - 2026-03-07

### Added
- **Managed setup command**: `omnicontext setup` orchestrates binary download, ONNX Runtime
  installation, and MCP configuration in a single command with unified progress reporting
- Consistent premium UX across all distribution scripts with standardized error messages and
  progress indicators

## [0.7.1] - 2026-03-06

### Fixed
- Invert version resolution to query the GitHub Releases API directly rather than inferring
  the latest version from git tags, fixing incorrect version detection in install scripts
- Remove Unicode em-dash characters from distribution script output that caused rendering
  artifacts in certain terminal environments

## [0.7.0] - 2026-03-06

### Added
- **Zero-config MCP sync**: On daemon start, the extension automatically writes the MCP server
  entry into all detected AI client configurations (Claude Desktop, Cursor, Continue.dev, Kiro,
  Windsurf, Cline, RooCode, Trae, Antigravity, Claude Code CLI)
- **Kiro IDE support**: MCP server entry written using the `powers.mcpServers` namespace
  required by Kiro's MCP protocol implementation
- **`OmniContext: Sync MCP` command**: Manual re-synchronization of the MCP server entry
  across all detected AI clients without requiring a daemon restart

## [0.6.1] - 2026-03-06

### Fixed
- Add `write` permission to the CI build job to allow uploading compiled binaries as release
  assets to GitHub Releases; previously the job silently failed to attach any binaries

## [0.6.0] - 2026-03-06

### Added
- **Zero-Config MCP manifest**: MCP manifest auto-published to
  `~/.omnicontext/mcp-manifest.json` for AI client auto-discovery without manual configuration
- **PageRank-based symbol ranking**: Symbol importance derived from in-degree centrality in
  the dependency graph; wired into RRF search scoring as a percentile-normalized boost
- **Historical co-change edges**: Co-change relationship edges mined from git commit history
  stored in the dependency graph for relationship-aware context assembly
- **Temporal freshness scoring**: File freshness scores computed from indexed-at timestamps
  using exponential decay (7-day half-life); recently modified files surface higher in results

## [0.5.3] - 2026-03-02

### Fixed
- Update MCP install and test scripts to reflect current binary location and invocation pattern

## [0.5.2] - 2026-03-02

### Fixed
- Skip release build CI jobs when the version bump step determines no new release is required,
  preventing unnecessary binary compilation on documentation-only commits

## [0.5.1] - 2026-03-02

### Fixed
- Correct `deny.toml` configuration values after cargo-deny schema validation failures caused
  by stale configuration keys

## [0.5.0] - 2026-03-02

### Added
- **Dynamic version detection**: Version string read from workspace `Cargo.toml` at build time
  so all binaries and install scripts automatically reflect the current version without manual
  updates to embedded version strings

## [0.4.0] - 2026-03-02

### Added
- **Enhanced VS Code sidebar**: Real-time performance metrics panel (P50/P95/P99 search
  latency), activity log with per-operation details, repository info panel, and language
  distribution chart; professional VS Code codicons used throughout
- **Language distribution in engine status**: `omnicontext status` and the IPC `system_status`
  response include per-language file counts for all indexed languages

### Fixed
- Prevent SQLite lock conflicts in concurrent test execution by serializing test database
  access through a per-test mutex

### Documentation
- Add comprehensive install, update, and uninstall commands for Linux, macOS, and Windows

## [0.3.0] - 2026-03-01

### Added
- **Per-language file distribution**: Index engine tracks and exposes a breakdown of indexed
  files by language (Rust, TypeScript, Python, Go, Java, and more) in the status response

### Documentation
- Reorganize to enterprise-grade structure with comprehensive Mermaid architecture diagrams,
  consolidated reference guides, and professional naming conventions throughout

## [0.2.0] - 2026-03-01

### Added
- **CAST micro-chunking with configurable overlap**: Adjacent chunks share configurable overlap
  (default 12%) to preserve cross-boundary context for functions spanning chunk boundaries
- **Cross-encoder reranking pipeline**: Optional cross-encoder model re-scores RRF top-k
  candidates for higher ranking precision; falls back to RRF-only when model is unavailable
- **Symbol-level dependency graph**: Knowledge graph built from import resolution and reference
  extraction across all 16 supported languages; supports proximity-boosted search
- **Graph-boosted search ranking**: Dependency distance between query anchors and candidate
  symbols contributes a proximity boost to final RRF scores
- **Query expansion**: NL queries undergo stop-word stripping and OR-join expansion; code
  token queries split on `snake_case` and `CamelCase` boundaries for improved BM25 recall
- **Contextual Enricher**: Chunk content enriched with parent scope signature and import
  header before embedding to improve semantic retrieval quality
- **Module-qualified FQNs**: Fully-qualified symbol names include the root-relative module
  path across all 16 language analyzers (e.g., `auth::user::MyStruct` vs `user::MyStruct`)
- **Enterprise-grade CI**: Stricter cargo-deny license and advisory checks; CodeQL security
  scanning; SBOM generation; release provenance attestation via GitHub Actions
- **One-click install scripts**: Shell and PowerShell scripts for automatic engine binary and
  ONNX model download on Linux, macOS, and Windows

### Changed
- Reorganize installation scripts into a unified `distribution/` directory with consistent
  staging, platform detection, and error handling

### Fixed
- Fix MCP server test reliability by eliminating shared state across concurrent test cases
- Resolve `get_file_summary` path normalization failures on Windows UNC paths by trying
  multiple path forms before reporting file-not-found
- Guarantee 100% chunk embedding coverage on ONNX partial batch failures: individual chunks
  are embedded when a batch fails rather than dropping the entire file's embeddings
- Resolve all clippy warnings across `omni-core`, `omni-mcp`, and `omni-cli`

## [0.1.0] - 2026-02-28

### Added
- Initial release of OmniContext — a locally-runnable code context engine for AI coding agents
- **Rust workspace** with three crates: `omni-core` (retrieval engine), `omni-mcp` (MCP
  protocol server), `omni-cli` (terminal interface)
- **16 language analyzers** via tree-sitter: Python, Rust, TypeScript, JavaScript, Go, Java,
  C, C++, C#, CSS, Ruby, PHP, Swift, Kotlin; plus document formats (Markdown, TOML, YAML,
  JSON, HTML, Shell)
- **Hybrid search engine**: RRF fusion of BM25 keyword search (SQLite FTS5) and semantic
  vector search (HNSW via usearch) with configurable `k` parameter
- **ONNX Runtime embedding**: Code-optimized embedding model (`jina-embeddings-v2-base-code`)
  served locally via the `ort` crate; no external API calls required
- **SQLite metadata index**: FTS5 full-text search, symbol table, dependency edge table, and
  file registry in a single embedded database
- **File watcher**: `notify`-based incremental watcher with configurable debounce; SHA-256
  hash cache skips unchanged files on re-scans
- **MCP server**: 16 AI-facing tools including `search_code`, `get_file_summary`,
  `get_symbol_definition`, `get_related_symbols`, `get_code_context`, and `set_workspace`
- **VS Code extension**: Sidebar with system status display, IPC client using named-pipe and
  Unix-socket transport with exponential backoff reconnection, status bar indicator, and
  Copilot Chat participant for context injection
- **Configuration system**: Five-level precedence chain (CLI flags → env vars → project config
  → user config → compiled defaults) with full TOML serialization
- **Cross-platform support**: Linux (primary), macOS, and Windows; platform-appropriate data
  directories via the `dirs` crate

<!-- generated by git-cliff -->
