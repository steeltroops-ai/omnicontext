# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial project scaffold with Cargo workspace
- omni-core: module architecture (parser, chunker, embedder, index, vector, graph, search, watcher, pipeline)
- omni-mcp: MCP server binary with stdio/SSE transport args
- omni-cli: CLI with index, search, status, mcp, config subcommands
- Tree-sitter grammar registrations for Python, TypeScript, JavaScript, Rust, Go
- SQLite schema with FTS5 full-text search and sync triggers
- Configuration system with precedence chain (CLI > env > project > user > defaults)
- Core domain types: Language, Chunk, Symbol, DependencyEdge, SearchResult
- Hierarchical error taxonomy (OmniError) with recoverable/degraded/fatal classification
- Dependency graph with RwLock-protected petgraph DiGraph
- RRF fusion scoring math for hybrid search
