# OmniContext Feature Audit & Improvement Roadmap

**Purpose**: Comprehensive end-to-end audit of every feature from Rust backend → IPC/MCP → VS Code extension, identifying gaps, improvements, and state-of-the-art opportunities.

**Status**: Living document - updated as features are audited and improved

**Last Updated**: 2026-03-09

---

## Audit Methodology

Each feature is evaluated across:

1. **Backend Implementation** (Rust core)
2. **IPC/MCP Interface** (Protocol layer)
3. **Extension Integration** (VS Code frontend)
4. **End-to-End Flow** (Complete user journey)
5. **Industry Standards Compliance**
6. **State-of-the-Art Opportunities**

### Rating System

- ✅ **Production Ready**: Meets enterprise standards
- ⚠️ **Needs Improvement**: Functional but has gaps
- ❌ **Critical Gap**: Missing or broken
- 🔬 **Research Opportunity**: Could be state-of-the-art

---

## Core Features Audit

### 1. Code Indexing Pipeline

**Backend (omni-core/pipeline)**
- ✅ Tree-sitter AST parsing for 16 languages
- ✅ Semantic chunking with token counting
- ✅ ONNX embedding generation (jina-embeddings-v2-base-code)
- ✅ SQLite storage with FTS5 full-text search
- ✅ HNSW vector index
- ⚠️ Batch embedding with backpressure (needs tuning)
- ⚠️ Contextual chunking (recently added, needs validation)
- ❌ Incremental re-indexing (full rebuild only)
- 🔬 Adaptive chunking based on code complexity

**IPC/MCP Interface**
- ✅ `index` command via CLI
- ⚠️ No progress reporting during indexing
- ❌ No cancellation support
- ❌ No partial index recovery on failure

**Extension Integration**
- ✅ "Index Workspace" command
- ⚠️ No progress bar (just notification)
- ❌ No index status indicator in UI
- ❌ No automatic re-index on file changes (watcher exists but not connected)

**End-to-End Flow**
1. User clicks "Index Workspace"
2. Extension spawns CLI process
3. CLI indexes entire workspace
4. User gets completion notification
5. ❌ No feedback during long operations
6. ❌ No way to know if index is stale

**Improvement Opportunities**
- [ ] Add streaming progress updates via IPC
- [ ] Implement incremental indexing (only changed files)
- [ ] Add index health monitoring
- [ ] Surface index statistics in extension UI
- [ ] Auto-reindex on file save (debounced)
- [ ] 🔬 Research: Hierarchical indexing for large monorepos

---

### 2. Semantic Code Search

**Backend (omni-core/search)**
- ✅ Hybrid search (BM25 + vector + graph-boosted)
- ✅ RRF fusion scoring
- ✅ Query expansion with synonyms
- ✅ Intent classification
- ✅ HyDE (Hypothetical Document Embeddings)
- ✅ Dependency graph proximity reranking
- ⚠️ Query result caching (recently added, needs validation)
- ❌ No query history or learning
- 🔬 Personalized ranking based on user behavior

**IPC/MCP Interface**
- ✅ `search_code` MCP tool
- ✅ `get_context` MCP tool (search + assembly)
- ✅ Configurable result limits
- ⚠️ No search filters (language, file type, date)
- ❌ No search suggestions/autocomplete

**Extension Integration**
- ✅ "Search Code" command
- ✅ Chat participant for context injection
- ✅ Pre-fetch caching on cursor movement
- ⚠️ Search results shown in output panel (not interactive)
- ❌ No search history
- ❌ No search refinement UI

**End-to-End Flow**
1. User types query in search command OR uses chat
2. Extension calls MCP `search_code` or `get_context`
3. Backend performs hybrid search
4. Results returned as JSON
5. Extension displays in output panel or injects into chat
6. ⚠️ No way to refine search
7. ❌ No way to save searches

**Improvement Opportunities**
- [ ] Add interactive search results panel (tree view)
- [ ] Implement search filters (language, path, date range)
- [ ] Add search history with quick access
- [ ] Surface search quality metrics (relevance scores)
- [ ] Add "Find Similar" action on code selections
- [ ] 🔬 Research: Neural reranking with user feedback

---

### 3. Context Assembly & Injection

**Backend (omni-core/search/context_assembler)**
- ✅ Token budget management
- ✅ Dependency-aware context expansion
- ✅ Deduplication
- ⚠️ Fixed assembly strategy (no customization)
- ❌ No context quality metrics
- 🔬 LLM-guided context selection

**IPC/MCP Interface**
- ✅ `get_context` MCP tool
- ✅ Token budget parameter
- ⚠️ No context assembly strategy selection
- ❌ No context provenance tracking

**Extension Integration**
- ✅ Chat participant auto-injects context
- ✅ Pre-flight context command
- ✅ Cache hit indicator
- ⚠️ No visibility into what context was selected
- ❌ No way to manually adjust context
- ❌ No context quality feedback loop

**End-to-End Flow**
1. User asks question in AI chat
2. Extension intercepts via chat participant
3. Calls `get_context` with query + token budget
4. Backend assembles relevant context
5. Extension injects into chat prompt
6. ⚠️ User has no visibility into context selection
7. ❌ No way to provide feedback on context quality

**Improvement Opportunities**
- [ ] Add context preview panel (show what will be injected)
- [ ] Allow manual context adjustment (add/remove files)
- [ ] Track context usage metrics (which files helped)
- [ ] Add context quality scoring
- [ ] Implement context templates for common tasks
- [ ] 🔬 Research: Reinforcement learning from user feedback

---

### 4. Dependency Graph Analysis

**Backend (omni-core/graph)**
- ✅ Import extraction for TS/JS/Go/Python/Rust
- ✅ Dependency edge storage
- ✅ Graph traversal (dependencies/dependents)
- ✅ Community detection
- ✅ Attention scoring
- ⚠️ File-level only (no symbol-level)
- ❌ No circular dependency detection
- ❌ No dependency health metrics
- 🔬 Call graph analysis

**IPC/MCP Interface**
- ✅ `get_dependencies` MCP tool
- ✅ `get_dependents` MCP tool
- ✅ `get_module_map` MCP tool
- ⚠️ No graph visualization data format
- ❌ No graph query language

**Extension Integration**
- ✅ "Show Module Map" command
- ⚠️ Module map shown in output panel (text only)
- ❌ No interactive graph visualization
- ❌ No dependency explorer panel
- ❌ No "Go to Definition" across dependencies

**End-to-End Flow**
1. User runs "Show Module Map"
2. Extension calls `get_module_map`
3. Backend returns text representation
4. Extension shows in output panel
5. ❌ No interactivity
6. ❌ No visual graph

**Improvement Opportunities**
- [ ] Add interactive graph visualization (D3.js/Cytoscape)
- [ ] Implement dependency explorer tree view
- [ ] Add "Find All References" across files
- [ ] Surface circular dependency warnings
- [ ] Add dependency health metrics (coupling, cohesion)
- [ ] 🔬 Research: Architectural smell detection

---

### 5. File Watching & Incremental Updates

**Backend (omni-core/watcher)**
- ✅ File system watcher with debouncing
- ✅ Hash-based change detection
- ⚠️ Watcher exists but not fully integrated
- ❌ No incremental index updates (full rebuild)
- ❌ No conflict resolution on concurrent changes

**IPC/MCP Interface**
- ❌ No IPC events for file changes
- ❌ No incremental update API

**Extension Integration**
- ❌ No file watcher integration
- ❌ No automatic re-index on save
- ❌ No stale index warnings

**End-to-End Flow**
1. User edits file and saves
2. ❌ Nothing happens
3. User must manually re-index
4. ❌ Index becomes stale silently

**Improvement Opportunities**
- [ ] Connect watcher to incremental indexing
- [ ] Add IPC events for file changes
- [ ] Implement debounced auto-reindex on save
- [ ] Add stale index indicator in UI
- [ ] Surface indexing queue status
- [ ] 🔬 Research: Predictive pre-indexing

---

### 6. MCP Server Integration

**Backend (omni-mcp)**
- ✅ 8 MCP tools implemented
- ✅ JSON-RPC 2.0 protocol
- ✅ stdio transport
- ✅ Tool descriptions for AI agents
- ⚠️ No tool usage analytics
- ❌ No rate limiting
- ❌ No authentication/authorization

**IPC/MCP Interface**
- ✅ Standard MCP protocol
- ✅ Error handling with codes
- ⚠️ No streaming responses
- ❌ No batch operations

**Extension Integration**
- ✅ Auto-sync MCP config to AI clients
- ✅ "Start MCP Server" command
- ✅ "Sync MCP" command
- ⚠️ No MCP server status indicator
- ❌ No MCP tool usage visibility
- ❌ No MCP server logs in extension

**End-to-End Flow**
1. Extension auto-configures MCP on daemon start
2. AI client (Claude/Cursor) connects to MCP server
3. AI agent calls MCP tools
4. ❌ User has no visibility into MCP usage
5. ❌ No way to debug MCP issues from extension

**Improvement Opportunities**
- [ ] Add MCP server status indicator
- [ ] Surface MCP tool usage metrics in UI
- [ ] Add MCP request/response logging panel
- [ ] Implement MCP tool testing UI
- [ ] Add MCP server health monitoring
- [ ] 🔬 Research: Adaptive tool selection based on usage

---

### 7. Daemon Process Management

**Backend (omni-daemon)**
- ✅ Background process with IPC
- ✅ Named pipe (Windows) / Unix socket transport
- ✅ JSON-RPC 2.0 protocol
- ✅ Pre-fetch cache with LRU eviction
- ✅ Connection pooling for SQLite
- ⚠️ No graceful shutdown on system sleep
- ❌ No daemon health monitoring
- ❌ No automatic restart on crash

**IPC/MCP Interface**
- ✅ `prefetch_stats` method
- ✅ `clear_cache` method
- ✅ `update_config` method
- ✅ `shutdown` method
- ⚠️ No heartbeat/ping method
- ❌ No daemon status method

**Extension Integration**
- ✅ Auto-start daemon on workspace open
- ✅ "Start Daemon" / "Stop Daemon" commands
- ✅ Daemon status in sidebar
- ⚠️ No automatic restart on crash
- ❌ No daemon logs in extension
- ❌ No daemon performance metrics

**End-to-End Flow**
1. Extension starts daemon on activation
2. Daemon runs in background
3. Extension communicates via IPC
4. ❌ If daemon crashes, extension doesn't recover
5. ❌ No visibility into daemon health

**Improvement Opportunities**
- [ ] Add daemon health monitoring with heartbeat
- [ ] Implement automatic restart on crash
- [ ] Surface daemon logs in extension output panel
- [ ] Add daemon performance metrics (CPU, memory)
- [ ] Implement graceful shutdown on system events
- [ ] 🔬 Research: Distributed daemon for large teams

---

### 8. Configuration Management

**Backend (omni-core/config)**
- ✅ 5-level precedence (CLI > env > project > user > defaults)
- ✅ TOML configuration files
- ✅ Environment variable overrides
- ⚠️ No configuration validation
- ❌ No configuration migration on version updates
- ❌ No configuration UI

**IPC/MCP Interface**
- ✅ `update_config` IPC method
- ⚠️ No `get_config` method
- ❌ No configuration schema exposure

**Extension Integration**
- ✅ VS Code settings integration
- ✅ Settings for token budget, cache size, etc.
- ⚠️ Settings not synced to daemon config
- ❌ No configuration validation in UI
- ❌ No configuration import/export

**End-to-End Flow**
1. User changes VS Code setting
2. Extension reads setting
3. ⚠️ Setting may not apply to daemon immediately
4. ❌ No feedback on invalid settings

**Improvement Opportunities**
- [ ] Add configuration validation with error messages
- [ ] Sync VS Code settings to daemon config automatically
- [ ] Add configuration UI panel
- [ ] Implement configuration profiles (dev/prod)
- [ ] Add configuration import/export
- [ ] 🔬 Research: ML-based configuration tuning

---

### 9. Performance Monitoring & Telemetry

**Backend (omni-core/resilience)**
- ✅ Health monitor with circuit breaker
- ✅ Performance metrics collection
- ⚠️ Metrics stored in memory only
- ❌ No metrics export
- ❌ No alerting on performance degradation

**IPC/MCP Interface**
- ⚠️ `get_stats` MCP tool (basic stats only)
- ❌ No detailed performance metrics API
- ❌ No real-time metrics streaming

**Extension Integration**
- ✅ Sidebar shows basic metrics (P50/P95/P99)
- ✅ Activity log
- ⚠️ Metrics refresh manually only
- ❌ No performance trend visualization
- ❌ No alerting in UI

**End-to-End Flow**
1. Backend collects metrics
2. Extension polls for metrics
3. Sidebar displays current values
4. ❌ No historical data
5. ❌ No anomaly detection

**Improvement Opportunities**
- [ ] Add metrics persistence (time-series DB)
- [ ] Implement real-time metrics streaming
- [ ] Add performance trend charts
- [ ] Implement anomaly detection with alerts
- [ ] Add metrics export (Prometheus format)
- [ ] 🔬 Research: Predictive performance modeling

---

### 10. Error Handling & Recovery

**Backend (omni-core/error)**
- ✅ Hierarchical error taxonomy (Recoverable/Degraded/Fatal)
- ✅ Structured error types
- ✅ Error context propagation
- ⚠️ No error aggregation
- ❌ No error reporting/telemetry
- ❌ No automatic recovery strategies

**IPC/MCP Interface**
- ✅ Standard error codes
- ✅ Error messages with context
- ⚠️ No error categorization in responses
- ❌ No retry hints

**Extension Integration**
- ⚠️ Errors shown as notifications (not actionable)
- ❌ No error log panel
- ❌ No error recovery suggestions
- ❌ No error reporting to maintainers

**End-to-End Flow**
1. Error occurs in backend
2. Error returned via IPC/MCP
3. Extension shows notification
4. ❌ User has no context or recovery options
5. ❌ Error not logged for debugging

**Improvement Opportunities**
- [ ] Add error log panel with filtering
- [ ] Implement actionable error messages (with fix suggestions)
- [ ] Add automatic retry with exponential backoff
- [ ] Implement error reporting (opt-in telemetry)
- [ ] Add error recovery wizard
- [ ] 🔬 Research: ML-based error diagnosis

---

### 11. Language Support

**Backend (omni-core/parser/languages)**
- ✅ 16 languages supported
- ✅ Tree-sitter grammars
- ✅ Symbol extraction
- ✅ Import detection (subset of languages)
- ⚠️ Language-specific features inconsistent
- ❌ No language plugin system
- 🔬 Language-agnostic semantic understanding

**IPC/MCP Interface**
- ✅ `search_by_kind` MCP tool (symbol type filtering)
- ⚠️ No language-specific query syntax
- ❌ No language capability discovery

**Extension Integration**
- ✅ Language distribution shown in sidebar
- ❌ No per-language configuration
- ❌ No language-specific features in UI

**End-to-End Flow**
1. Backend detects file language
2. Applies language-specific parser
3. Extracts symbols and dependencies
4. ❌ User has no visibility into language support
5. ❌ No way to configure language-specific behavior

**Improvement Opportunities**
- [ ] Add language capability matrix documentation
- [ ] Implement language plugin system
- [ ] Add per-language configuration
- [ ] Surface language support status in UI
- [ ] Add language-specific search filters
- [ ] 🔬 Research: Cross-language semantic search

---

### 12. Git Integration

**Backend (omni-core/commits, branch_diff)**
- ✅ Commit history analysis
- ✅ Branch-aware diff indexing
- ⚠️ Git integration not fully utilized
- ❌ No blame information
- ❌ No commit-based search

**IPC/MCP Interface**
- ❌ No git-related MCP tools
- ❌ No commit history API

**Extension Integration**
- ❌ No git integration in UI
- ❌ No "Search in Commit" feature
- ❌ No blame-aware context

**End-to-End Flow**
1. Backend can read git history
2. ❌ Not exposed to users
3. ❌ No git-aware features

**Improvement Opportunities**
- [ ] Add "Search in Commit" feature
- [ ] Implement blame-aware context assembly
- [ ] Add commit history search
- [ ] Surface code evolution in UI
- [ ] Add git-aware dependency tracking
- [ ] 🔬 Research: Temporal code understanding

---

## Cross-Cutting Concerns

### Security & Privacy

**Current State**
- ✅ Local-only processing (no cloud)
- ✅ No telemetry by default
- ⚠️ No encryption at rest
- ❌ No access control
- ❌ No audit logging

**Improvement Opportunities**
- [ ] Add index encryption at rest
- [ ] Implement access control for MCP tools
- [ ] Add audit logging for sensitive operations
- [ ] Add privacy-preserving telemetry (opt-in)
- [ ] 🔬 Research: Differential privacy for usage analytics

### Testing & Quality

**Current State**
- ✅ Unit tests (364 passing)
- ✅ Integration tests
- ✅ Benchmarks
- ⚠️ No end-to-end tests
- ❌ No UI tests for extension
- ❌ No performance regression tests in CI

**Improvement Opportunities**
- [ ] Add end-to-end test suite
- [ ] Implement VS Code extension UI tests
- [ ] Add performance regression detection in CI
- [ ] Implement property-based testing for parsers
- [ ] Add chaos engineering tests
- [ ] 🔬 Research: Automated test generation

### Documentation

**Current State**
- ✅ Architecture documentation
- ✅ API documentation
- ✅ Development guides
- ⚠️ User documentation incomplete
- ❌ No video tutorials
- ❌ No interactive examples

**Improvement Opportunities**
- [ ] Complete user documentation
- [ ] Add video tutorials
- [ ] Create interactive playground
- [ ] Add troubleshooting guide
- [ ] Implement in-app help system
- [ ] 🔬 Research: AI-powered documentation assistant

---

## Priority Matrix

### P0 - Critical (Blocks Production Use)
1. Incremental indexing (Feature 1)
2. Automatic daemon restart (Feature 7)
3. Error recovery UI (Feature 10)
4. Index staleness detection (Feature 5)

### P1 - High (Major UX Improvements)
1. Interactive search results (Feature 2)
2. Context preview panel (Feature 3)
3. Progress reporting for indexing (Feature 1)
4. Dependency graph visualization (Feature 4)
5. MCP server monitoring (Feature 6)

### P2 - Medium (Quality of Life)
1. Search history (Feature 2)
2. Configuration UI (Feature 8)
3. Performance trend charts (Feature 9)
4. Error log panel (Feature 10)
5. Language-specific configuration (Feature 11)

### P3 - Low (Nice to Have)
1. Git integration features (Feature 12)
2. Configuration profiles (Feature 8)
3. Metrics export (Feature 9)
4. Language plugin system (Feature 11)

### Research - State-of-the-Art Opportunities
1. Neural reranking with user feedback (Feature 2)
2. LLM-guided context selection (Feature 3)
3. Call graph analysis (Feature 4)
4. Predictive pre-indexing (Feature 5)
5. Cross-language semantic search (Feature 11)
6. Temporal code understanding (Feature 12)

---

## Next Steps

1. **Prioritize**: Review priority matrix with team
2. **Spec**: Create detailed specs for P0 items
3. **Implement**: Execute P0 → P1 → P2 → P3
4. **Research**: Prototype state-of-the-art features
5. **Iterate**: Update this document as features evolve

---

## Audit Schedule

- **Weekly**: Review P0 progress
- **Bi-weekly**: Audit one feature end-to-end
- **Monthly**: Update priority matrix
- **Quarterly**: Research review and roadmap planning

---

**Document Owner**: Engineering Team  
**Last Audit**: 2026-03-09  
**Next Audit**: 2026-03-16
