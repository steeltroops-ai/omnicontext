# OmniContext Website Documentation Plan

## Documentation Structure for Production Website

This document outlines the enterprise-grade documentation needed for the OmniContext website deployment on Vercel.

---

## Getting Started (Priority: CRITICAL)

### 1. Introduction
- What is OmniContext
- Key features and benefits
- Use cases (individual developers, teams, enterprise)
- How it works (high-level architecture)

### 2. Quick Start
- Installation (all platforms)
- First index
- First search
- MCP client setup

### 3. Installation
- Windows (PowerShell, Scoop, WinGet)
- macOS (Homebrew, Bash)
- Linux (Bash, package managers)
- From source (Cargo)
- Verification steps
- Troubleshooting

---

## Core Concepts (Priority: HIGH)

### 4. Indexing Pipeline
- AST parsing with tree-sitter
- Semantic chunking
- Embedding generation (ONNX local)
- Vector index (HNSW)
- Incremental updates

### 5. Hybrid Search
- Keyword search (FTS5)
- Vector search (HNSW)
- Reciprocal Rank Fusion (RRF)
- Cross-encoder reranking
- Graph boosting

### 6. Dependency Graph
- Edge types (IMPORTS, INHERITS, CALLS, INSTANTIATES)
- N-hop traversal
- Architectural context
- Historical co-change patterns

### 7. Context Assembly
- Token budget management
- Priority-based ranking
- Compression strategies
- LLM-optimized output

---

## MCP Integration (Priority: CRITICAL)

### 8. MCP Server Setup
- What is MCP
- Supported clients (Claude, Cursor, Windsurf, Kiro, etc.)
- Configuration files
- Transport protocols (stdio, SSE)

### 9. Available Tools
- search_codebase
- get_architectural_context
- get_dependencies
- get_commit_context
- get_workspace_stats
- context_window

### 10. Integration Guides
- Claude Desktop
- Cursor
- Windsurf
- Cline / RooCode
- Kiro
- Continue.dev
- Custom MCP clients

---

## Architecture (Priority: MEDIUM)

### 11. System Overview
- Component architecture
- Data flow
- Performance characteristics
- Scalability limits

### 12. Supported Languages
- Full list (16 languages)
- Parser capabilities per language
- Adding new languages

### 13. Configuration
- Config file format (.omnicontext/config.toml)
- Environment variables
- CLI flags
- Priority hierarchy

### 14. Performance
- Benchmarks
- Optimization tips
- Resource requirements
- Scaling strategies

---

## Advanced Features (Priority: MEDIUM)

### 15. Multi-Repository Workspaces
- Configuration
- Cross-repo search
- Priority weighting
- Use cases

### 16. Commit History Context
- Indexing commits
- Diff summarization
- Historical queries
- Evolution tracking

### 17. Health Monitoring
- Circuit breakers
- Health states
- Automatic recovery
- Telemetry

### 18. Daemon Mode
- IPC protocol
- Event deduplication
- Backpressure handling
- Message compression

---

## Enterprise (Priority: HIGH)

### 19. Pricing Tiers
- Free (local MCP)
- Pro ($20/mo per developer)
- Enterprise (custom)
- Feature comparison

### 20. REST API
- Authentication
- Endpoints
- Rate limits
- Examples

### 21. Security & Auth
- RBAC (Role-Based Access Control)
- Document-Level Security (DLS)
- Audit logs
- Compliance (SOC 2, GDPR)

### 22. Deployment
- Docker
- Kubernetes (Helm charts)
- Cloud providers (AWS, GCP, Azure)
- On-premise

---

## Developer Resources (Priority: LOW)

### 23. API Reference
- Rust API docs
- MCP protocol spec
- Error codes
- Type definitions

### 24. Contributing
- Development setup
- Code standards
- Commit conventions
- PR process

### 25. Troubleshooting
- Common issues
- Debug mode
- Log analysis
- Support channels

### 26. FAQ
- General questions
- Technical questions
- Licensing questions
- Enterprise questions

---

## Implementation Priority

### Phase 1: MVP (Week 1)
1. Introduction
2. Quick Start
3. Installation
8. MCP Server Setup
9. Available Tools
19. Pricing Tiers

### Phase 2: Core (Week 2)
4. Indexing Pipeline
5. Hybrid Search
6. Dependency Graph
10. Integration Guides
11. System Overview

### Phase 3: Advanced (Week 3)
7. Context Assembly
12. Supported Languages
13. Configuration
14. Performance
15. Multi-Repository Workspaces

### Phase 4: Enterprise (Week 4)
20. REST API
21. Security & Auth
22. Deployment
23. API Reference

### Phase 5: Polish (Week 5)
16. Commit History Context
17. Health Monitoring
18. Daemon Mode
24. Contributing
25. Troubleshooting
26. FAQ

---

## Content Guidelines

### Writing Style
- Concise and actionable
- Code examples for every feature
- Real-world use cases
- Performance metrics where relevant
- No marketing fluff

### Structure
- Start with "what" and "why"
- Follow with "how"
- Include examples
- End with troubleshooting/tips

### Code Examples
- Complete and runnable
- Multiple languages where applicable
- Commented for clarity
- Include expected output

### Diagrams
- Architecture diagrams (Mermaid)
- Flow charts for processes
- Sequence diagrams for interactions
- Keep simple and focused

---

## Success Metrics

### Documentation Quality
- Time to first successful index: <5 minutes
- MCP setup success rate: >95%
- Support ticket reduction: >50%
- User satisfaction: >4.5/5

### SEO & Discovery
- Organic search traffic
- Documentation page views
- Time on page
- Bounce rate

### Conversion
- Free → Pro conversion rate
- Enterprise inquiry rate
- GitHub stars growth
- VS Code extension installs

