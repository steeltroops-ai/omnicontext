---
title: Introduction
description: Universal semantic code context engine for AI agents
category: Getting Started
order: 1
---

# Introduction

OmniContext is a natively-compiled semantic code search engine that provides AI agents with structured codebase context through the Model Context Protocol (MCP). All processing runs locally without external APIs.

## What is OmniContext?

OmniContext indexes your codebase using AST parsing, semantic chunking, and vector embeddings to enable fast, accurate code search. It exposes 6 MCP tools that AI agents use to understand your code, navigate dependencies, and assemble relevant context.

## Key Features

- **Hybrid search**: Combines keyword matching with semantic vector search for accurate retrieval
- **AST-aware parsing**: Understands code structure across 16+ languages using tree-sitter
- **Graph reranking**: Boosts results based on dependency relationships and import patterns
- **Local execution**: All embeddings and indexing run on your machine (no cloud APIs)
- **MCP integration**: Native support for Claude Desktop, Cursor, Windsurf, Kiro, and other MCP clients
- **Real-time updates**: File watcher detects changes and incrementally updates the index

## How It Works

1. **Parse**: Tree-sitter extracts AST structure from source files
2. **Chunk**: Semantic chunking splits code into meaningful units
3. **Embed**: ONNX-based model generates vector embeddings locally
4. **Index**: SQLite + HNSW vector index stores chunks for fast retrieval
5. **Search**: Hybrid engine combines keyword and vector search with graph reranking
6. **Serve**: MCP server exposes tools to AI agents via stdio or SSE transport

## Performance

OmniContext is optimized for speed:

- **Indexing**: > 500 files/sec
- **Embedding**: > 800 chunks/sec on CPU
- **Search**: < 50ms P99 latency (100k chunk index)
- **Memory**: < 2KB per indexed chunk

## Supported Languages

Full AST parsing support for:

- JavaScript, TypeScript, JSX, TSX
- Python, Ruby, PHP
- Rust, Go, C, C++, C#
- Java, Kotlin, Swift
- CSS, HTML, Markdown

## Architecture

OmniContext is a Cargo workspace with four crates:

- `omni-core`: Core library (indexing, search, embeddings)
- `omni-cli`: Command-line interface
- `omni-daemon`: Background process with IPC
- `omni-mcp`: MCP server for AI agent integration

## Use Cases

### Individual Developers
- Semantic code search across large codebases
- Architectural understanding for new projects
- Historical context for debugging

### Teams
- Shared codebase knowledge
- Onboarding new developers
- Code review assistance

### Enterprise
- Multi-repository workspaces
- REST API for custom integrations
- RBAC and audit logs

## Next Steps

Ready to get started? Follow the [Quick Start](/docs/quick-start) guide to index your first codebase in under 5 minutes.
