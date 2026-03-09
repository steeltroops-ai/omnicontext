---
title: Introduction
description: OmniContext is a semantic code search engine that provides AI agents with structured codebase context through the Model Context Protocol (MCP).
category: Getting Started
order: 1
---

# Introduction

OmniContext is a natively-compiled semantic code search engine that provides AI agents with structured codebase context through the Model Context Protocol (MCP). All processing runs locally without external APIs.

## What is OmniContext?

OmniContext transforms how AI agents understand and navigate codebases. Instead of reading entire files or relying on simple keyword search, it provides intelligent, context-aware code retrieval that understands:

- **Semantic relationships** between code elements
- **Dependency graphs** and architectural patterns
- **Historical context** from git commits
- **Symbol definitions** and their usage across files

## Key Features

- **Hybrid Search Engine**: Combines AST-based keyword search with vector embeddings for semantic understanding
- **16 MCP Tools**: Comprehensive API for AI agents to query code, dependencies, symbols, and architecture
- **16+ Languages**: Full support for major programming languages with tree-sitter parsing
- **Local-First**: All processing runs on your machine with no external API calls
- **Fast Performance**: Sub-50ms P99 search latency on 100k chunk indexes
- **Zero Configuration**: Auto-downloads models, auto-detects languages, works out of the box

## How It Works

OmniContext uses a multi-stage pipeline to understand your codebase:

1. **Parse**: Extract AST structure from source files using tree-sitter
2. **Chunk**: Split code into semantic chunks with context preservation
3. **Embed**: Generate vector embeddings using local ONNX models
4. **Index**: Store in SQLite with HNSW vector index for fast retrieval
5. **Search**: Hybrid retrieval with graph-boosted reranking

## Use Cases

- **AI-Powered IDEs**: Provide context to Claude Desktop, Cursor, Windsurf, Cline, Kiro, Continue.dev
- **Code Navigation**: Quickly find relevant code across large codebases
- **Architecture Understanding**: Map dependencies and component relationships
- **Code Review**: Identify related changes and potential impacts
- **Documentation**: Generate context-aware documentation

## Getting Started

Ready to get started? Check out our [Quickstart](/docs/quickstart) guide to install OmniContext and index your first codebase.

For detailed installation instructions, see the [Installation](/docs/installation) guide.
