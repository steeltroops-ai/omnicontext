---
title: Introduction
description: OmniContext is a semantic code search engine that provides AI agents with structured codebase context through the Model Context Protocol (MCP).
category: Getting Started
order: 1
---

# Introduction

OmniContext is a natively compiled semantic code search engine that provides AI agents with structured, high-fidelity codebase context through the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/). All processing runs locally — no external APIs, no data leaving your machine.

---

## What is OmniContext?

OmniContext transforms how AI agents understand and navigate codebases. Instead of reading entire files or relying on simple keyword search, it provides intelligent, context-aware code retrieval that understands:

- **Semantic relationships** between code elements
- **Dependency graphs** and architectural patterns
- **Historical context** from git commits and co-change analysis
- **Symbol definitions** and their usage across files
- **Blast radius** — which code is affected when something changes

---

## Key Features

- **Hybrid Search Engine**: Combines AST-based keyword search (BM25) with vector embeddings for semantic understanding, fused via Reciprocal Rank Fusion
- **19 MCP Tools**: Comprehensive API for AI agents to query code, dependencies, symbols, architecture, call graphs, branch context, and more
- **13+ Languages**: Full AST parsing support for all major programming languages using tree-sitter grammars
- **Local-First**: All processing runs on your machine — no network calls during indexing or search
- **Fast Performance**: Sub-50 ms P99 search latency on 100 K+ chunk indexes
- **Zero Configuration**: Auto-downloads models, auto-detects languages, works immediately after install
- **Universal IDE Setup**: One command (`omnicontext setup --all`) auto-configures all installed AI IDEs and agents

---

## How It Works

OmniContext uses a multi-stage pipeline to understand your codebase:

1. **Parse**: Extract AST structure from source files using tree-sitter grammars
2. **Chunk**: Split code into semantic chunks with context preservation and doc-comment extraction
3. **Embed**: Generate 768-dimensional vector embeddings using the local Jina v2 base code ONNX model
4. **Index**: Store chunks in SQLite (FTS5) with an HNSW vector index for fast retrieval
5. **Graph**: Build a dependency graph from import/call relationships and git co-change history
6. **Search**: Hybrid retrieval (BM25 + vector + symbol) with graph-boosted cross-encoder reranking

---

## Supported AI IDEs and Agents

OmniContext integrates with every major AI coding assistant through MCP:

| Category | Tools |
|----------|-------|
| Desktop AI | Claude Desktop, Claude Code |
| AI IDEs | Cursor, Windsurf, VS Code (Copilot), Zed, Kiro, PearAI, Trae |
| VS Code Extensions | Cline, RooCode, Continue.dev, Augment Code |
| CLI Agents | Gemini CLI, Amazon Q CLI |

Run `omnicontext setup --all` to configure all installed tools automatically.

---

## Use Cases

- **AI-Powered Code Assistance**: Give AI agents precise, relevant context instead of entire files
- **Code Navigation**: Find relevant code across large or unfamiliar codebases
- **Architecture Understanding**: Map dependencies, module boundaries, and component relationships
- **Impact Analysis**: Understand what breaks before making a change (`get_blast_radius`)
- **Code Review**: Identify related changes and potential impacts using co-change analysis
- **Refactoring Planning**: Use `audit_plan` to get structural risk assessments before refactoring
- **Documentation Generation**: Generate `CLAUDE.md` project guides from live index data

---

## Getting Started

Install OmniContext in one command:

```bash
# macOS / Linux
curl -fsSL https://omnicontext.dev/install.sh | sh

# Windows (PowerShell)
irm https://omnicontext.dev/install.ps1 | iex
```

Then index your project and configure your IDE:

```bash
cd /path/to/your/project
omnicontext index .
omnicontext setup --all
```

For full installation instructions (including Cargo install, manual binary install, and VS Code extension), see the [Installation](/docs/installation) guide.

---

## Project Links

- **GitHub**: [github.com/steeltroops-ai/omnicontext](https://github.com/steeltroops-ai/omnicontext)
- **Website**: [omnicontext.dev](https://omnicontext.dev)
- **License**: Apache 2.0
