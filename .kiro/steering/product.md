---
inclusion: always
---

# OmniContext Product Context

OmniContext is a high-performance, local-first code context engine that provides AI agents with deep semantic understanding of codebases. Built in Rust, exposed via Model Context Protocol (MCP).

## Product Principles

When working on OmniContext, prioritize:

1. Local-first: All processing on developer's machine. No cloud dependencies, no API keys required.
2. Performance: Target sub-50ms search latency, <200ms incremental re-indexing.
3. Zero-config: Auto-download models, auto-index on startup. Minimize user friction.
4. Offline-capable: Fully functional without internet after initial model download.
5. Universal compatibility: Work with any MCP-compatible AI agent.

## Component Architecture

- `omni-core`: Library crate containing core engine (parser, chunker, embedder, index, search, graph)
- `omni-mcp`: Binary crate for MCP server (stdio transport, auto-indexing)
- `omni-cli`: Binary crate for CLI interface (index, search, status, config commands)
- `omni-daemon`: Binary crate for background file watching and incremental updates

## MCP Tool Capabilities

When implementing or modifying MCP tools, understand their purpose:

- `search_code`: Hybrid search combining keyword (FTS5) + semantic (vector) with RRF ranking
- `get_symbol`: Lookup symbols by name or fully qualified name (e.g., `module.Class.method`)
- `get_file_summary`: Return structural overview (exports, classes, functions) without full content
- `get_dependencies`: Traverse dependency graph (upstream/downstream) for impact analysis
- `find_patterns`: Identify recurring code patterns across codebase
- `get_architecture`: Generate high-level architecture overview for onboarding
- `explain_codebase`: Comprehensive project explanation for new developers
- `get_status`: Report engine status, index statistics, and health metrics

## Language Support

Phase 1 languages (fully supported): Python, TypeScript, JavaScript, Rust, Go, Java

When adding new language support:
- Add parser in `parser/languages/<lang>.rs`
- Register in `parser/registry.rs`
- Follow existing language extractor patterns
- Update `docs/SUPPORTED_LANGUAGES.md`

## Performance Targets

Maintain these benchmarks when making changes:

- Initial index (10k files): <60 seconds
- Incremental re-index: <200ms
- Search latency (P99): <50ms
- Memory footprint (10k files): <100MB
- Binary size: <50MB

## User Experience Expectations

- First run: Model auto-downloads (~550MB), index builds automatically
- Subsequent runs: Index loads from disk, incremental updates only
- Error handling: Graceful degradation, clear error messages, no crashes
- Logging: Use `tracing` with appropriate levels (debug for verbose, info for user-facing)

## Licensing & Business Model

- Open source: Apache 2.0 license for core functionality
- Free tier: Unlimited single-repo usage via local MCP server
- Pro tier: Multi-repo workspaces, commit lineage, advanced pattern recognition
- Enterprise: Hosted API, team knowledge sharing, SSO/SAML, custom models

When implementing features, consider which tier they belong to and document accordingly.
