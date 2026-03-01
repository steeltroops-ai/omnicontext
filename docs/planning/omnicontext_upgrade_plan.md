# OmniContext Upgrade Plan: Path to Market Leadership

> Comprehensive upgrade strategy incorporating 2025 state-of-the-art research
> Date: 2026-03-01 | Author: Antigravity Research Team

---

## Executive Summary

OmniContext currently has a 13% pass rate on basic context queries. This document provides a complete upgrade path to achieve market-leading code intelligence through:
- **Two-stage retrieval** with ColBERT late-interaction reranking (40-60% MRR improvement)
- **GraphRAG architecture** with community detection for architectural understanding
- **100% embedding coverage** (currently 13.5%)
- **Quantized vectors** for 4x memory reduction
- **AST-aware chunking** with contextual enrichment
- **Parallel multi-tool calling** for 3-5x faster agent context gathering
- **Invisible context injection** (Augment Code model) for zero-tool-call agent responses

---

## Table of Contents

1. [Current State Analysis](#1-current-state-analysis)
2. [Competitive Landscape](#2-competitive-landscape)
3. [Critical Architecture Gaps](#3-critical-architecture-gaps)
4. [2025 State-of-the-Art Techniques](#4-2025-state-of-the-art-techniques)
5. [Parallel Multi-Tool Calling Architecture](#5-parallel-multi-tool-calling-architecture)
6. [Invisible Context Injection: The Augment Code Model](#6-invisible-context-injection-the-augment-code-model)
7. [Implementation Roadmap](#7-implementation-roadmap)
8. [Target Architecture](#8-target-architecture)
9. [Success Metrics](#9-success-metrics)

---

## 1. Current State Analysis

### 1.1 MCP Stress Test Results

Tested with 15+ queries across natural language, symbol lookups, and architectural questions.

**Summary Metrics:**
- **Pass Rate:** 2/15 (13.3%)
- **Partial:** 2/15 (13.3%)
- **Fail:** 11/15 (73.3%)
- **`get_file_summary` failure rate:** 6/6 (100%)
- **Dependency graph edges:** 0

### 1.2 Root Cause Analysis

| Issue | Root Cause | Impact |
|-------|------------|--------|
| **Uniform search scores** | All results score ~0.016 regardless of relevance | Cannot discriminate relevant from irrelevant |
| **13.5% embedding coverage** | Pipeline skips chunks that fail validation | 86.5% of code has no semantic search |
| **Empty dependency graph** | Import extraction returns empty Vec by default | No graph-based features work |
| **No contextual enrichment** | Chunks contain raw code only | Missing imports, parent context, relationships |
| **Broken `get_file_summary`** | Path normalization issues (UNC vs relative) | Tool completely non-functional |

> [!CAUTION]
> The engine cannot answer basic questions about its own codebase. This is not a functional context engine.

---

## 2. Competitive Landscape

### 2.1 Feature Comparison Matrix

| Capability | Augment Code | Cursor AI | Sourcegraph Cody | OmniContext (Current) | Gap Severity |
|------------|--------------|-----------|------------------|----------------------|--------------|
| **Chunking** | AST micro-chunks + overlap | Proprietary | AST-aware | Tree-sitter, no overlap | Medium |
| **Embedding Coverage** | 100% | 100% | 100% | 13.5% | **CRITICAL** |
| **Knowledge Graph** | Full graph + type hierarchy | Proprietary | RSG with link prediction | 0 edges | **CRITICAL** |
| **Reranking** | Cross-encoder | Multi-stage | Cross-encoder + graph | None | **CRITICAL** |
| **Context Delivery** | Push (pre-flight) | Push (deep IDE) | Transparent | Pull (MCP tools) | High |
| **Query Understanding** | Intent classification | 8 parallel agents | BM25+embeddings+graph | Literal matching | High |
| **Quantization** | Yes | Yes | 8x reduction | None | Medium |
| **Multi-Repo** | Yes | Yes | Yes | No | Medium |

### 2.2 Key Insight: Push vs Pull Paradigm

**Current (Pull Model - Slow)**:
```
User → LLM → search_code() → LLM → get_symbol() → LLM → Answer
(3 round trips, visible tool calls, token waste)
```

**Target (Push Model - Fast)**:
```
User → IDE → silent_query() → enriched_context → LLM → Answer
(0 tool calls, 1 round trip, pre-assembled context)
```

---

## 3. Critical Architecture Gaps

### Priority 1: Two-Stage Retrieval with Late-Interaction Reranking

**Current**: Single-pass hybrid search (BM25 + vector)
**Target**: Two-stage pipeline with ColBERT-style reranking

**2025 State-of-the-Art: ColBERT Late-Interaction**
- Token-level contextualized embeddings with MaxSim scoring
- Independent encoding (queries and documents separate) enables offline indexing
- 100x faster than cross-encoders with BERT-level effectiveness
- Centroid-residual quantization: 10x storage reduction

**Implementation Options**:

| Option | Model | Pros | Cons | Recommendation |
|--------|-------|------|------|----------------|
| A: Cross-Encoder | ms-marco-MiniLM-L-6-v2 | Easy integration, proven | Slower, joint encoding | Phase 1 |
| B: ColBERT | GTE-ModernColBERT | 100x faster, quantization | Complex architecture | Phase 2 |
| C: Hybrid | Both | Best quality | Most complex | Phase 3 |

**Implementation**:
```
New Crate: omni-reranker
- Stage 1: HNSW + BM25 recall (top-100 candidates)
- Stage 2: ColBERT/cross-encoder rerank (top-20)
- Stage 3: Final scoring with full precision (top-10)
- Integrate into search/mod.rs as post-processing
```

**Expected Impact**: 40-60% MRR improvement (cross-encoder), 50-70% (ColBERT)

> [!IMPORTANT]
> Every major retrieval system (Qdrant, Weaviate, Azure AI Search) now supports ColBERT. This is the 2025 standard.

### Priority 2: GraphRAG Architecture with Community Detection

**Current**: petgraph exists but has 0 edges
**Target**: Dense semantic graph with community detection

**Key 2025 Insight**: GraphRAG changes retrieval from "top-K chunks" to "connected communities". The model sees structure, not fragments.

**Architecture Components**:

1. **Entity-Relationship Graph**:
   - Nodes: Symbols (functions, classes, modules, files)
   - Edges: Relationships (imports, calls, implements, contains, co-changes)
   - Properties: Metadata (visibility, language, file path, commit history)

2. **Community Detection**:
   - Louvain/Leiden algorithm to detect architectural clusters
   - Each community = cohesive subsystem (e.g., "auth module")
   - Generate community summaries (LLM or extractive)

3. **Graph-Enhanced Retrieval**:
   - Vector search finds initial nodes
   - Graph traversal expands to connected entities (1-2 hops)
   - Community summaries provide architectural context
   - Relationship types guide relevance propagation

**Implementation**:
```
Enhance: omni-core::graph
- Phase 1: Dense edge population
  * Import edges: resolve import paths to symbols
  * Call edges: extract from AST function_call nodes
  * Type hierarchy: impl Trait, extends, inherits
  * Module containment: File → Module → Crate
  * Temporal edges: co-change coupling from git log

- Phase 2: Community detection
  * Implement Louvain algorithm
  * Generate community summaries
  * Store in SQLite with metadata

- Phase 3: Graph-augmented search
  * Initial retrieval: vector + BM25 (top-50)
  * Graph expansion: 1-hop neighbors (weighted)
  * Community context: include summaries
  * Relevance propagation: boost connected nodes
```

**Expected Impact**:
- Corpus-level questions: 60-80% improvement
- Architectural queries: 70-90% improvement
- Cross-file reasoning: 50-70% improvement

### Priority 3: 100% Embedding Coverage

**Current**: 13.5% coverage (122/906 chunks)
**Target**: 100% with graceful degradation

**Implementation**:
```
Fix: omni-core::embedder + omni-core::chunker
- Fix chunk validation to accept all valid code
- Batch embedding with retry logic
- TF-IDF fallback when ONNX fails
- Track coverage in get_status output
```

**Expected Impact**: Immediate 6x improvement in semantic search recall

### Priority 4: AST-Aware Chunking with Contextual Enrichment

**Current**: Fixed AST chunks, no overlap, no context
**Target**: cAST algorithm with enrichment

**2025 Research: cAST (Chunking via Abstract Syntax Trees)**
- Recursive AST chunking respecting semantic boundaries
- 100-200 token overlap between adjacent chunks
- Contextual enrichment: imports, parent scope, sibling signatures

**Enriched Chunk Structure**:
```rust
struct EnrichedChunk {
    core: Chunk,                      // Actual code
    imports: Vec<String>,              // File-level imports
    parent_scope: Option<String>,      // Enclosing function/class
    sibling_signatures: Vec<String>,   // Other methods in class
    doc_summary: Option<String>,       // Purpose summary
}
```

**Implementation**:
```
Enhance: omni-core::chunker
- Implement cAST recursive chunking
- Add 100-200 token overlap
- Enrich with context before embedding
- Modify ParsedChunk type in types.rs
```

**Expected Impact**: 49% reduction in retrieval failures (Anthropic benchmark)

### Priority 5: Quantized Vector Search

**Current**: Full f32 vectors (1.5KB per chunk)
**Target**: uint8 quantized (384 bytes, 4x reduction)

**2025 Quantization Techniques**:

| Technique | Compression | Quality Loss | Use Case |
|-----------|-------------|--------------|----------|
| Scalar (f32→uint8) | 4x | <2% | Production-ready |
| Binary (1-bit) | 32x | 5-10% | Initial filtering |
| Hybrid | 4x + precision | <1% | Best of both |

**Implementation**:
```
Enhance: omni-core::vector
- Phase 1: Scalar quantization (f32 → uint8)
- Phase 2: Hybrid precision (quantized HNSW, full precision scoring)
- Phase 3: On-disk HNSW with mmap (scale to 1M+ chunks)
```

**Expected Impact**:
- Memory: 100k chunks @ 37MB (vs 150MB)
- Scale: Support 1M+ chunks
- Latency: <5% increase
- Quality: <2% recall degradation

**Industry Adoption**: Azure AI Search, Qdrant, Milvus, Infinity v0.6.0 (500% faster, 90% memory savings)

---

## 4. 2025 State-of-the-Art Techniques

### 4.1 Reciprocal Rank Fusion (RRF) - Best Practices

**Current Issue**: k=60 standard but all signals return similar rankings

**2025 Insight**: RRF is robust, doesn't require score normalization, works well for heterogeneous signals

**Weighted RRF**:
```
RRF_score(d) = w_semantic * 1/(k + rank_semantic)
             + w_keyword  * 1/(k + rank_keyword)  
             + w_symbol   * 1/(k + rank_symbol)
             + w_graph    * 1/(k + dist_graph)
```

### 4.2 Query Expansion with Semantic Similarity

**Technique**: LeSeR (Lexical Reranking of Semantic Retrieval)
- Stage 1: Semantic retrieval with expanded query (broad recall)
- Stage 2: Lexical reranking with BM25 on original query (precision)

**Example**:
```
Original:  "how does authentication work"
Expanded:  ["authentication", "auth", "login", "authenticate", 
            "verify", "credential", "token", "session"]
```

### 4.3 Intent-Based Search Strategy

| Intent | Search Strategy | Context Assembly | Implementation |
|--------|----------------|------------------|----------------|
| Edit | Implementation details, patterns | Surrounding code, imports, tests | Boost ChunkKind::Function |
| Explain | Architectural context, docs | Module map, call graph | Include graph neighbors |
| Debug | Error paths, recent changes | Error types, commits, traces | Weight by recency |
| Refactor | All usages, dependents | Callers, implementors, tests | Graph downstream traversal |

### 4.4 Graph-Based Relevance Propagation

**Algorithm**:
```
1. Execute search → get results R with scores
2. For each result r: find graph neighbors N(r)
3. Propagate: score(n) += alpha × score(r) × edge_weight(r,n)
4. Re-rank combined set R ∪ N(R)
5. Apply token budget, return top-k
```

**Parameters**: alpha = 0.3, max_depth = 2 hops

### 4.5 Adaptive Chunking Strategy

- **Dense code** (complex algorithms) → smaller chunks (200-400 tokens)
- **Boilerplate** (config, imports) → larger chunks (600-800 tokens)
- **Critical paths** (auth, payment) → overlapping chunks with extra context

**Heuristic**: AST depth + cyclomatic complexity

---

## 5. Parallel Multi-Tool Calling Architecture

### 5.1 Current Limitation: Sequential Tool Calls

**Problem**: Agents currently make sequential MCP tool calls, causing latency multiplication:

```
Sequential (Current):
User query → search_code (200ms) → wait → get_symbol (50ms) → wait → get_dependencies (100ms)
Total: 350ms + agent processing time

Parallel (Target):
User query → [search_code, get_symbol, get_dependencies] in parallel
Total: max(200ms, 50ms, 100ms) = 200ms + agent processing time
Speedup: 1.75x
```

### 5.2 Parallel Execution Architecture

**Design Principle**: MCP tools must be stateless and thread-safe to enable concurrent execution.

**Implementation Requirements**:

1. **Thread-Safe Engine Access**:
```rust
// Current: Single Mutex lock
Arc<Mutex<Engine>>

// Target: Read-optimized concurrent access
Arc<RwLock<Engine>>  // or DashMap for hot paths
```

2. **Async Tool Handlers**:
```rust
// All MCP tool handlers must be async
async fn search_code(params: SearchParams) -> Result<SearchResults>
async fn get_symbol(params: SymbolParams) -> Result<SymbolInfo>
async fn get_dependencies(params: DepsParams) -> Result<DependencyGraph>

// Enable tokio::join! for parallel execution
let (search_results, symbol_info, deps) = tokio::join!(
    search_code(search_params),
    get_symbol(symbol_params),
    get_dependencies(deps_params)
);
```

3. **Batch-Optimized Operations**:
```rust
// New batch tools for common parallel patterns
async fn batch_get_symbols(symbols: Vec<String>) -> Vec<SymbolInfo>
async fn batch_get_files(paths: Vec<String>) -> Vec<FileContent>
async fn multi_search(queries: Vec<String>) -> Vec<SearchResults>
```

### 5.3 Concurrency Patterns

**Pattern 1: Independent Queries**
```
Agent needs: "auth implementation" + "test coverage" + "recent changes"
Parallel calls:
  - search_code("auth implementation")
  - find_patterns("test coverage")
  - get_code_history("auth module")
```

**Pattern 2: Dependency Fan-Out**
```
Agent needs: Symbol X + all its dependencies + all its dependents
Parallel calls:
  - get_symbol("X")
  - get_dependencies("X", direction="upstream")
  - get_dependencies("X", direction="downstream")
```

**Pattern 3: Multi-File Context**
```
Agent needs: Context for files [A, B, C, D]
Parallel calls:
  - get_file_summary("A")
  - get_file_summary("B")
  - get_file_summary("C")
  - get_file_summary("D")
```

### 5.4 Implementation Phases

**Phase 1: Make Engine Thread-Safe (Week 1-2)**
```
Changes:
- Replace Arc<Mutex<Engine>> with Arc<RwLock<Engine>>
- Audit all Engine methods for read vs write operations
- Use read locks for queries, write locks only for indexing
- Add concurrent access tests
```

**Phase 2: Async Tool Handlers (Week 3-4)**
```
Changes:
- Convert all MCP tool handlers to async fn
- Use tokio::spawn for CPU-intensive operations
- Add timeout handling per tool (prevent one slow tool blocking others)
- Implement graceful degradation (partial results if some tools fail)
```

**Phase 3: Batch Operations (Week 5-6)**
```
New MCP Tools:
- batch_get_symbols(symbols: Vec<String>)
- batch_get_files(paths: Vec<String>)
- batch_search(queries: Vec<String>)
- parallel_context_window(requests: Vec<ContextRequest>)
```

**Phase 4: Agent Optimization Hints (Week 7)**
```
MCP Tool Metadata:
- Add "parallelizable": true to tool definitions
- Add "estimated_latency_ms": 200 for agent planning
- Add "dependencies": [] to indicate which tools must run sequentially
- Document common parallel patterns in tool descriptions
```

### 5.5 Performance Targets

| Scenario | Sequential | Parallel | Speedup |
|----------|-----------|----------|---------|
| 3 independent searches | 600ms | 200ms | 3x |
| Symbol + dependencies (up/down) | 300ms | 100ms | 3x |
| 5 file summaries | 250ms | 50ms | 5x |
| Complex query (8 tools) | 1200ms | 300ms | 4x |

### 5.6 Agent Integration Examples

**Example 1: Claude with MCP Parallel Calls**
```json
{
  "tool_calls": [
    {"name": "search_code", "params": {"query": "authentication"}},
    {"name": "get_symbol", "params": {"name": "AuthService"}},
    {"name": "find_patterns", "params": {"pattern": "error handling"}}
  ]
}
```

**Example 2: Batch Context Assembly**
```json
{
  "tool": "parallel_context_window",
  "params": {
    "requests": [
      {"type": "search", "query": "auth implementation"},
      {"type": "symbol", "name": "AuthService"},
      {"type": "dependencies", "symbol": "AuthService", "direction": "both"},
      {"type": "file_summary", "path": "src/auth/service.rs"}
    ]
  }
}
```

### 5.7 Concurrency Safety Guarantees

**Read Operations (Parallelizable)**:
- `search_code` - Read-only index access
- `get_symbol` - Read-only symbol table
- `get_file_summary` - Read-only file metadata
- `get_dependencies` - Read-only graph traversal
- `find_patterns` - Read-only pattern matching
- `get_architecture` - Read-only module map
- `get_status` - Read-only statistics

**Write Operations (Require Exclusive Lock)**:
- Index updates (file watcher triggers)
- Embedding generation (batch operations)
- Graph edge additions (import resolution)

**Isolation Strategy**:
- Separate read and write paths
- Use MVCC (Multi-Version Concurrency Control) for index snapshots
- Write operations create new versions, reads use stable snapshots
- Periodic compaction to merge versions

### 5.8 Error Handling for Parallel Execution

**Partial Success Strategy**:
```rust
// Don't fail entire request if one tool fails
let results = tokio::join!(
    search_code(params1),
    get_symbol(params2),
    get_dependencies(params3)
);

// Return partial results with error annotations
{
  "search_results": results.0.ok(),
  "symbol_info": results.1.ok(),
  "dependencies": results.2.err().map(|e| format!("Failed: {}", e))
}
```

**Timeout Handling**:
```rust
// Per-tool timeouts prevent cascading failures
let search_future = timeout(Duration::from_millis(500), search_code(params));
let symbol_future = timeout(Duration::from_millis(200), get_symbol(params));

// Fast tools don't wait for slow tools
```

### 5.9 Benchmarking Parallel Performance

**New Benchmark Suite**:
```bash
# Parallel tool execution benchmarks
cargo bench --bench parallel_tools

# Measure:
- Throughput: queries per second with N concurrent agents
- Latency: p50, p95, p99 for parallel vs sequential
- Contention: lock wait times under load
- Scalability: performance with 1, 2, 4, 8, 16 concurrent agents
```

**Target Metrics**:
- 4x throughput improvement with 4 concurrent agents
- <10ms lock contention overhead
- Linear scalability up to 8 agents
- Graceful degradation beyond 8 agents

---

## 6. Invisible Context Injection: The Augment Code Model

### 6.1 How Augment Code's Context Engine Works

**Key Insight**: Augment Code doesn't make agents call MCP tools. Instead, it **automatically enriches the user's prompt with relevant context before it reaches the LLM**, making the agent instantly aware of the codebase without any tool calls.

**Architecture Overview**:
```
User types prompt in IDE: "Fix the authentication bug"
         ↓
IDE Extension intercepts message
         ↓
Extension sends prompt + cursor position + open files to Context Engine
         ↓
Context Engine performs automatic retrieval:
  - Semantic search on prompt
  - Graph expansion from active file
  - Recent commits analysis
  - Architectural pattern detection
         ↓
Context Engine assembles enriched context (within token budget)
         ↓
Extension MODIFIES the user's prompt by injecting context
         ↓
Modified prompt sent to LLM:
  "Fix the authentication bug
  
  <codebase_context>
  [Relevant code chunks automatically inserted here]
  [Dependency graph automatically inserted here]
  [Recent changes automatically inserted here]
  </codebase_context>"
         ↓
LLM responds with full codebase awareness (zero tool calls)
```

**Two Injection Strategies**:

1. **System Prompt Injection** (Invisible to user):
   - Context added to system prompt
   - User never sees the injected context
   - Cleaner chat history

2. **User Prompt Enrichment** (Augment's approach):
   - Context appended to user's message
   - User can see enriched prompt in debug mode
   - More explicit about what context was used

**Augment's Key Features** (from research):
- **200k+ token context window**: Maintains architectural awareness across 400k+ files
- **Real-time indexing**: Extension streams file changes to backend in near real-time
- **Persistent understanding**: Analyzes once, improves over time (not per-prompt)
- **Automatic prompt enhancement**: Auggie CLI adds relevant context, structure, conventions
- **70% performance improvement**: Benchmarked across Claude Code, Cursor, Codex

### 6.2 Why This Matters: Pull vs Push Comparison

**Current OmniContext (Pull Model)**:
```
User: "Fix the authentication bug"
Agent: Let me search... [calls search_code("authentication bug")]
Engine: [returns results after 200ms]
Agent: Let me get that symbol... [calls get_symbol("AuthService")]
Engine: [returns symbol after 50ms]
Agent: Let me check dependencies... [calls get_dependencies("AuthService")]
Engine: [returns deps after 100ms]
Agent: Here's the fix [after 350ms + 3 round trips]
```

**Target OmniContext (Push Model - Augment Style)**:
```
User: "Fix the authentication bug"
Extension: [silently queries context engine with prompt + active file]
Engine: [returns pre-assembled context in 100ms]
Extension: [injects context into system prompt]
Agent: Here's the fix [immediately, 0 tool calls, 1 round trip]
```

**Benefits**:
- 3-5x faster response (no tool call latency)
- Better context quality (engine knows what's relevant)
- Lower token costs (no tool call overhead)
- Seamless UX (user never sees context assembly)

### 6.3 Implementation Architecture

**Component 1: VS Code Extension (Context Interceptor & Prompt Enricher)**

Location: `editors/vscode/src/context-injector.ts`

```typescript
// Intercept chat messages before they reach the LLM
vscode.chat.onWillSendMessage(async (event) => {
  const originalPrompt = event.message;
  const activeFile = vscode.window.activeTextEditor?.document.uri.fsPath;
  const cursorPosition = vscode.window.activeTextEditor?.selection.active;
  const openFiles = vscode.workspace.textDocuments.map(d => d.uri.fsPath);
  
  // Query OmniContext daemon for relevant context
  const context = await omniContextClient.getContextWindow({
    prompt: originalPrompt,
    activeFile,
    cursorPosition,
    openFiles,
    tokenBudget: 50000  // Reserve 50k tokens for context
  });
  
  // STRATEGY 1: System Prompt Injection (Invisible)
  // Context added to system prompt, user never sees it
  event.systemPrompt = `
<codebase_context>
You have access to the following codebase context, automatically retrieved based on the user's query:

## Architecture Overview
${context.architectureOverview}

## Relevant Code (ranked by relevance)
${context.enrichedChunks.map(chunk => `
### ${chunk.file}:${chunk.line} (relevance: ${chunk.score})
\`\`\`${chunk.language}
${chunk.content}
\`\`\`
`).join('\n')}

## Dependency Graph
${context.graphNeighbors}

## Recent Changes
${context.recentCommits}
</codebase_context>

${event.systemPrompt}
  `;
  
  // STRATEGY 2: User Prompt Enrichment (Augment's approach)
  // Modify the user's message to include context
  event.message = `${originalPrompt}

<automatically_retrieved_context>
The following context was automatically retrieved from your codebase:

**Active File**: ${activeFile}
**Relevant Files** (${context.enrichedChunks.length} chunks):
${context.enrichedChunks.map(chunk => `- ${chunk.file} (${chunk.kind})`).join('\n')}

**Code Context**:
${context.enrichedChunks.map(chunk => `
\`\`\`${chunk.language}
// File: ${chunk.file}:${chunk.line}
// Relevance: ${chunk.score.toFixed(2)}
${chunk.content}
\`\`\`
`).join('\n')}

**Dependencies**:
${context.graphNeighbors}

**Recent Changes**:
${context.recentCommits}
</automatically_retrieved_context>

Please use this context to answer my question.`;

  // Optional: Show badge in UI
  vscode.window.showInformationMessage(
    `Context: ${context.tokenCount} tokens from ${context.fileCount} files`
  );
});
```

**Example: Before and After Prompt Enrichment**

**User's Original Prompt**:
```
Fix the authentication bug in the login flow
```

**Enriched Prompt (sent to LLM)**:
```
Fix the authentication bug in the login flow

<automatically_retrieved_context>
The following context was automatically retrieved from your codebase:

**Active File**: src/auth/login.ts
**Relevant Files** (8 chunks):
- src/auth/login.ts (Function)
- src/auth/service.ts (Class)
- src/auth/middleware.ts (Function)
- src/types/user.ts (Interface)
- tests/auth/login.test.ts (Function)

**Code Context**:
```typescript
// File: src/auth/login.ts:45
// Relevance: 0.92
export async function handleLogin(credentials: LoginCredentials): Promise<AuthResult> {
  const user = await validateCredentials(credentials);
  if (!user) {
    throw new AuthenticationError('Invalid credentials');
  }
  const token = await generateToken(user);
  return { user, token };
}
```

```typescript
// File: src/auth/service.ts:12
// Relevance: 0.87
export class AuthService {
  async validateCredentials(creds: LoginCredentials): Promise<User | null> {
    const user = await this.userRepo.findByEmail(creds.email);
    if (!user) return null;
    
    const isValid = await bcrypt.compare(creds.password, user.passwordHash);
    return isValid ? user : null;
  }
}
```

```typescript
// File: src/auth/middleware.ts:8
// Relevance: 0.81
export function requireAuth(req: Request, res: Response, next: NextFunction) {
  const token = req.headers.authorization?.split(' ')[1];
  if (!token) {
    return res.status(401).json({ error: 'No token provided' });
  }
  // Bug: Token validation is missing here!
  next();
}
```

**Dependencies**:
- handleLogin → validateCredentials (calls)
- handleLogin → generateToken (calls)
- AuthService → UserRepository (uses)
- requireAuth → verifyToken (should call, but doesn't)

**Recent Changes**:
- commit abc123 (2 days ago): "Add token generation" by @john
- commit def456 (1 day ago): "Update auth middleware" by @jane
  Modified: src/auth/middleware.ts (removed token validation - BUG!)

</automatically_retrieved_context>

Please use this context to answer my question.
```

**Result**: The LLM now sees:
1. The original user question
2. All relevant code automatically retrieved
3. Dependency relationships
4. Recent changes that might have introduced the bug
5. Zero tool calls needed - everything is in the prompt!

**Component 2: OmniContext Daemon (Persistent Process)**

Location: `crates/omni-daemon/src/context_server.rs`

```rust
// Long-running process that maintains index and serves context
pub struct ContextServer {
    engine: Arc<RwLock<Engine>>,
    session_manager: SessionManager,
    cache: ContextCache,
}

impl ContextServer {
    pub async fn get_context_window(&self, request: ContextRequest) -> ContextWindow {
        // Phase 1: Extract intent and entities
        let intent = self.classify_intent(&request.prompt);
        let entities = self.extract_entities(&request.prompt);
        
        // Phase 2: Multi-signal retrieval (parallel)
        let (semantic_hits, keyword_hits, symbol_hits, graph_context) = tokio::join!(
            self.semantic_search(&request.prompt, 50),
            self.keyword_search(&entities, 50),
            self.symbol_lookup(&entities, 20),
            self.graph_expand(&request.active_file, 2)  // 2-hop neighbors
        );
        
        // Phase 3: Fusion + Reranking
        let mut candidates = self.rrf_fuse(semantic_hits, keyword_hits, symbol_hits);
        candidates = self.graph_boost(candidates, &graph_context);
        candidates = self.rerank(request.prompt, candidates, 20);
        
        // Phase 4: Context packing (token-budget-aware)
        let context = self.assemble_context(candidates, request.token_budget, intent);
        
        // Phase 5: Cache for subsequent queries
        self.cache.insert(request.prompt.clone(), context.clone());
        
        context
    }
}
```

**Component 3: Context Assembly Engine**

Location: `crates/omni-core/src/context_assembly/mod.rs`

```rust
pub struct ContextAssembler {
    token_budget: usize,
    intent: QueryIntent,
}

impl ContextAssembler {
    pub fn assemble(&self, chunks: Vec<RankedChunk>) -> AssembledContext {
        let mut context = AssembledContext::new();
        let mut tokens_used = 0;
        
        // Priority 1: Active file context (always include)
        if let Some(active_chunk) = chunks.iter().find(|c| c.is_active_file) {
            context.add_chunk(active_chunk, ChunkPriority::Critical);
            tokens_used += active_chunk.token_count;
        }
        
        // Priority 2: Direct dependencies (high relevance)
        for chunk in chunks.iter().filter(|c| c.is_direct_dependency) {
            if tokens_used + chunk.token_count <= self.token_budget {
                context.add_chunk(chunk, ChunkPriority::High);
                tokens_used += chunk.token_count;
            }
        }
        
        // Priority 3: Semantic matches (fill remaining budget)
        for chunk in chunks.iter().filter(|c| !c.is_active_file && !c.is_direct_dependency) {
            if tokens_used + chunk.token_count <= self.token_budget {
                context.add_chunk(chunk, ChunkPriority::Medium);
                tokens_used += chunk.token_count;
            } else {
                // Compress low-priority chunks to fit more
                let compressed = self.compress_chunk(chunk);
                if tokens_used + compressed.token_count <= self.token_budget {
                    context.add_chunk(&compressed, ChunkPriority::Low);
                    tokens_used += compressed.token_count;
                }
            }
        }
        
        // Add architectural summary
        context.architecture_overview = self.generate_architecture_summary(&chunks);
        
        context
    }
}
```

### 6.4 Speculative Pre-Fetch (Augment's Secret Sauce)

**Concept**: Predict what context the user will need BEFORE they ask, based on IDE state.

**Triggers**:
```rust
pub enum PreFetchTrigger {
    FileOpened(PathBuf),           // User opens file → pre-fetch related files
    CursorMoved(Position),         // User navigates to function → pre-fetch callers/callees
    FileEdited(PathBuf),           // User edits file → pre-fetch tests, dependencies
    CommentStarted,                // User types "// " → pre-fetch documentation
    ErrorDetected(Diagnostic),     // Linter error → pre-fetch error handling patterns
    TestFailed(TestResult),        // Test fails → pre-fetch implementation + related tests
}
```

**Implementation**:
```rust
pub struct SpeculativePrefetcher {
    cache: Arc<DashMap<String, CachedContext>>,
    ttl: Duration,
}

impl SpeculativePrefetcher {
    pub async fn on_file_opened(&self, path: PathBuf) {
        // Pre-fetch likely queries
        let queries = vec![
            format!("explain {}", path.display()),
            format!("what does {} do", path.file_name().unwrap().to_str().unwrap()),
            format!("tests for {}", path.display()),
        ];
        
        for query in queries {
            let context = self.engine.get_context_window(ContextRequest {
                prompt: query.clone(),
                active_file: Some(path.clone()),
                token_budget: 30000,
                ..Default::default()
            }).await;
            
            self.cache.insert(query, CachedContext {
                context,
                expires_at: Instant::now() + self.ttl,
            });
        }
    }
    
    pub async fn on_cursor_moved(&self, position: Position, file: PathBuf) {
        // Identify symbol at cursor
        let symbol = self.engine.get_symbol_at_position(file.clone(), position).await;
        
        if let Some(sym) = symbol {
            // Pre-fetch callers, callees, tests
            tokio::spawn(async move {
                let _ = self.engine.get_dependencies(sym.name, DependencyDirection::Both).await;
            });
        }
    }
}
```

### 6.5 Intent-Based Context Selection

**Different intents need different context**:

| Intent | Context Strategy | Example |
|--------|------------------|---------|
| **Explain** | Architectural overview + module map + high-level flow | "How does authentication work?" → Include auth module structure, not implementation details |
| **Edit** | Implementation details + surrounding code + tests | "Fix the login bug" → Include AuthService implementation, related tests, error types |
| **Debug** | Error paths + recent changes + stack traces | "Why is this failing?" → Include error handling, recent commits, test failures |
| **Refactor** | All usages + downstream dependents + type hierarchy | "Rename this function" → Include all call sites, implementors, tests |
| **Generate** | Similar patterns + architectural conventions | "Add a new endpoint" → Include existing endpoint patterns, routing config |

**Implementation**:
```rust
pub enum QueryIntent {
    Explain,
    Edit,
    Debug,
    Refactor,
    Generate,
    Unknown,
}

impl QueryIntent {
    pub fn classify(prompt: &str) -> Self {
        // Keyword-based classification
        if prompt.contains("how") || prompt.contains("what") || prompt.contains("explain") {
            return QueryIntent::Explain;
        }
        if prompt.contains("fix") || prompt.contains("bug") || prompt.contains("error") {
            return QueryIntent::Debug;
        }
        if prompt.contains("rename") || prompt.contains("refactor") || prompt.contains("move") {
            return QueryIntent::Refactor;
        }
        if prompt.contains("add") || prompt.contains("create") || prompt.contains("implement") {
            return QueryIntent::Generate;
        }
        
        // Default to edit for ambiguous prompts
        QueryIntent::Edit
    }
    
    pub fn context_strategy(&self) -> ContextStrategy {
        match self {
            QueryIntent::Explain => ContextStrategy {
                include_architecture: true,
                include_implementation: false,
                include_tests: false,
                include_docs: true,
                graph_depth: 2,
            },
            QueryIntent::Edit => ContextStrategy {
                include_architecture: false,
                include_implementation: true,
                include_tests: true,
                include_docs: false,
                graph_depth: 1,
            },
            // ... other intents
        }
    }
}
```

### 6.6 Real-Time Index Streaming (Augment's Approach)

**Problem**: Index becomes stale as user edits code.

**Solution**: Stream file changes to daemon in real-time, update index incrementally.

**Implementation**:
```typescript
// VS Code Extension: Watch for file changes
vscode.workspace.onDidChangeTextDocument(async (event) => {
  const file = event.document.uri.fsPath;
  const content = event.document.getText();
  
  // Debounce rapid changes (200ms)
  clearTimeout(updateTimers.get(file));
  updateTimers.set(file, setTimeout(async () => {
    // Stream change to daemon
    await omniContextClient.updateFile({
      path: file,
      content,
      changeType: 'modified'
    });
  }, 200));
});
```

```rust
// Daemon: Incremental index update
pub async fn update_file(&self, update: FileUpdate) {
    // Parse changed file
    let chunks = self.parser.parse_file(&update.path, &update.content).await?;
    
    // Compute content hash
    let new_hash = hash_content(&update.content);
    let old_hash = self.index.get_file_hash(&update.path).await?;
    
    if new_hash != old_hash {
        // Delete old chunks
        self.index.delete_chunks_for_file(&update.path).await?;
        
        // Re-embed only changed chunks
        let embeddings = self.embedder.embed_batch(&chunks).await?;
        
        // Update index
        self.index.upsert_chunks(chunks, embeddings).await?;
        
        // Update graph edges
        self.graph.update_edges_for_file(&update.path, &chunks).await?;
        
        // Invalidate cache
        self.cache.invalidate_for_file(&update.path);
    }
}
```

### 6.7 Implementation Phases

**Phase 1: Daemon + IPC (Week 1-2)**
- Create persistent `omnicontext-daemon` process
- Implement Unix socket / named pipe IPC
- Add `get_context_window` RPC endpoint
- Test daemon startup, shutdown, reconnection

**Phase 2: VS Code Extension (Week 3-4)**
- Implement chat message interceptor
- Add context injection before LLM call
- Handle token budget management
- Add user settings (enable/disable, token budget)

**Phase 3: Context Assembly (Week 5-6)**
- Implement intent classification
- Build context assembler with priority system
- Add token-budget-aware packing
- Implement context compression for low-priority chunks

**Phase 4: Speculative Pre-Fetch (Week 7-8)**
- Monitor IDE state changes
- Implement pre-fetch triggers
- Add TTL-based cache
- Measure cache hit rate

**Phase 5: Real-Time Streaming (Week 9-10)**
- Watch file changes in extension
- Stream updates to daemon
- Implement incremental re-indexing
- Add debouncing and batching

### 6.8 Performance Targets

| Metric | Target | Validation |
|--------|--------|------------|
| Context assembly latency | <100ms | 95th percentile |
| Pre-fetch cache hit rate | >60% | Common queries |
| Incremental update latency | <200ms | File save to index update |
| Token budget utilization | >90% | Fill context window efficiently |
| Zero-tool-call rate | >80% | Queries answered without MCP tools |

### 6.9 Prompt Enrichment Strategies Comparison

| Strategy | Pros | Cons | Best For |
|----------|------|------|----------|
| **System Prompt Injection** | Clean chat history, invisible to user, doesn't consume user message tokens | Less transparent, harder to debug | Production use, clean UX |
| **User Prompt Enrichment** | Transparent, user can see context, easier to debug | Clutters chat history, consumes user message tokens | Development, debugging, transparency |
| **Hybrid** | Best of both - system prompt for structure, user prompt for key context | More complex implementation | Advanced users who want control |

**Augment Code uses User Prompt Enrichment** because:
1. Transparency: Users can see what context was used
2. Debugging: Easy to understand why agent gave certain answers
3. Trust: Users trust the agent more when they see the context
4. Feedback: Users can report if wrong context was retrieved

**OmniContext should support both** with user preference:
```json
{
  "omnicontext.contextInjection.strategy": "system" | "user" | "hybrid",
  "omnicontext.contextInjection.showInChat": false,  // Show enriched prompt in chat
  "omnicontext.contextInjection.showBadge": true     // Show "Context: 45k tokens" badge
}
```

### 6.10 User Experience

**Invisible Mode (System Prompt Injection)**:
```
User: "Fix the authentication bug"
[Extension silently queries context engine]
[Context injected into system prompt]
Agent: "I found the bug in src/auth/middleware.ts line 12..."
[User sees clean response, no context visible]
```

**Transparent Mode (User Prompt Enrichment)**:
```
User: "Fix the authentication bug"
[Extension queries context engine]
[Context appended to user's message]
Chat shows:
  User: "Fix the authentication bug
  
  <automatically_retrieved_context>
  [8 code chunks shown]
  [Dependencies shown]
  [Recent changes shown]
  </automatically_retrieved_context>"
  
Agent: "Based on the context, the bug is in src/auth/middleware.ts..."
[User can see exactly what context was used]
```

**Debug Mode**:
- Expandable context sections
- Relevance scores for each chunk
- Token count per chunk
- Manual override: remove/add chunks
- Explain why each chunk was selected

**Settings**:
```json
{
  "omnicontext.invisibleContext.enabled": true,
  "omnicontext.invisibleContext.strategy": "system",  // "system" | "user" | "hybrid"
  "omnicontext.invisibleContext.tokenBudget": 50000,
  "omnicontext.invisibleContext.showInChat": false,   // Show enriched prompt
  "omnicontext.invisibleContext.showBadge": true,     // Show token count badge
  "omnicontext.invisibleContext.showDebugInfo": false, // Show relevance scores
  "omnicontext.invisibleContext.preFetchEnabled": true,
  "omnicontext.invisibleContext.preFetchTTL": 300     // 5 minutes
}
```

### 6.10 Competitive Advantage

**What makes OmniContext better than Augment**:

1. **Local-First**: Augment uses cloud backend, OmniContext runs entirely local
2. **Privacy**: Code never leaves machine (vs Augment's cloud processing)
3. **Transparent**: Show exactly what context was used (vs Augment's black box)
4. **Agent-Agnostic**: Works with ANY MCP agent (vs Augment's proprietary agent)
5. **Open Source**: Community can audit, extend, customize
6. **Pluggable**: Bring your own reranker, embedding model, graph algorithms

**Marketing message**: "All the power of Augment Code's Context Engine, with the privacy and transparency of local-first architecture."

---

## 7. Implementation Roadmap

### Phase 0: Critical Bug Fixes (Week 1-2)

| Task | Priority | Effort | Details |
|------|----------|--------|---------|
| Fix `get_file_summary` paths | P0 | 2h | Normalize UNC paths in MCP layer |
| Fix 100% embedding coverage | P0 | 4h | Ensure every chunk gets vector |
| Fix dependency graph population | P0 | 8h | Implement extract_imports for all languages |
| Fix search score discrimination | P0 | 4h | Wire structural weight boost to RRF |
| Fix FQN construction | P0 | 6h | Build module-qualified FQNs |
| **Make Engine thread-safe** | **P0** | **8h** | **Replace Mutex with RwLock for concurrent access** |

### Phase 1: Search Quality + Concurrency (Week 3-5)

| Task | Priority | Effort | Details |
|------|----------|--------|---------|
| **Convert MCP tools to async** | **P1** | **12h** | **Enable parallel tool execution** |
| Query expansion/rewriting | P1 | 12h | Expand NL queries into keyword variants |
| Contextual chunk enrichment | P1 | 16h | Add imports, parent scope, siblings |
| Graph-boosted search ranking | P1 | 8h | Wire DependencyGraph.distance() into scoring |
| BM25 tuning for code | P1 | 4h | Tune FTS5 for snake_case, CamelCase |
| Result deduplication | P1 | 4h | Deduplicate overlapping chunks |
| **Add batch MCP tools** | **P1** | **8h** | **batch_get_symbols, batch_get_files** |

### Phase 2: Knowledge Graph (Week 6-9)

| Task | Priority | Effort | Details |
|------|----------|--------|---------|
| Import resolution engine | P1 | 24h | Resolve use/import statements to symbols |
| Call graph construction | P2 | 20h | Extract function calls from AST |
| Type hierarchy extraction | P2 | 12h | Extract impl Trait, class inheritance |
| Community detection | P2 | 16h | Implement Louvain algorithm |
| Temporal edges | P2 | 12h | Co-change coupling from git log |

### Phase 3: Reranking & Context Assembly (Week 10-12)

| Task | Priority | Effort | Details |
|------|----------|--------|---------|
| Cross-encoder reranker | P2 | 20h | Integrate ONNX cross-encoder model |
| Token-budget context assembly | P1 | 8h | Pack maximum relevant context |
| Context compression | P2 | 12h | Summarize low-relevance chunks |
| Multi-file context windows | P1 | 8h | Return related files, not just chunks |

### Phase 4: Pre-Flight Delivery + Invisible Context (Week 13-18)

| Task | Priority | Effort | Details |
|------|----------|--------|---------|
| **Persistent daemon process** | **P0** | **16h** | **Long-running with IPC (Unix socket)** |
| **VS Code context interceptor** | **P0** | **24h** | **Intercept chat, inject context before LLM** |
| **Context assembly engine** | **P0** | **16h** | **Token-budget-aware packing with priorities** |
| **Intent classification** | **P1** | **8h** | **Classify query intent for context strategy** |
| MCP `context_window` tool | P1 | 8h | Pre-assembled context window tool (fallback) |
| **Speculative pre-fetch** | **P1** | **12h** | **Cache likely contexts based on IDE state** |
| **Real-time index streaming** | **P1** | **12h** | **Stream file changes, incremental updates** |
| Cursor-aware context | P2 | 8h | Use active file + cursor to bias retrieval |
| Agent system prompt templates | P2 | 8h | MCP resource with instructions |
| **Context debug UI** | **P2** | **8h** | **Show which files included, relevance scores** |

### Phase 5: Scale & Polish (Week 17-20)

| Task | Priority | Effort | Details |
|------|----------|--------|---------|
| Quantized vectors | P1 | 16h | Scalar quantization + hybrid precision |
| Multi-repo indexing | P2 | 20h | Cross-repo symbol resolution |
| Incremental re-embedding | P1 | 8h | Hash-based dirty tracking |
| Benchmark suite | P1 | 12h | Automated MRR, Recall@K, NDCG |
| Additional languages | P2 | 24h | Java, C/C++, C# support |

---

## 6. Target Architecture

### 6.1 System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: Ingestion                                          │
│  File Watcher → Language Router → tree-sitter Parser →     │
│  cAST Chunker → Contextual Enricher → Batch Embedder       │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 2: Storage                                            │
│  • Quantized HNSW Vector Index (usearch)                   │
│  • SQLite Index (FTS5 + Metadata)                          │
│  • GraphRAG Store (petgraph + SQLite communities)          │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 3: Retrieval                                          │
│  Query Analyzer → Query Expander → Parallel Retriever →    │
│  (Vector + BM25 + Symbol + Graph) → ColBERT Reranker →     │
│  Context Assembler                                          │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 4: Delivery                                           │
│  • MCP Server (stdio/SSE)                                   │
│  • Pre-Flight Injector (LSP/Extension) ← PRIMARY           │
│  • REST API (Enterprise)                                    │
└─────────────────────────────────────────────────────────────┘
```

### 6.2 Technology Stack

| Component | Current | Target | 2025 SOTA |
|-----------|---------|--------|-----------|
| Embedding Model | jina-embeddings-v2-base-code | Same | jina-code-embeddings-1.5b (1.54B params) |
| Reranker | None | ms-marco-MiniLM-L-6-v2 | GTE-ModernColBERT (late-interaction) |
| Vector Store | Custom vectors.bin | HNSW (usearch) | Quantized HNSW (uint8) |
| Knowledge Graph | petgraph (empty) | petgraph + SQLite | GraphRAG with communities |
| Chunking | Fixed AST | Contextual enrichment | cAST with overlap |
| Context Delivery | Pull (MCP tools) | Push (pre-flight) | Push with speculative pre-fetch |

---

## 7. Success Metrics

### 7.1 Performance Targets

| Metric | Current | Phase 1 | Phase 3 | Phase 5 |
|--------|---------|---------|---------|---------|
| **MRR@5** | ~0.15 | 0.35 | 0.55 | 0.75 |
| **Recall@10** | ~0.20 | 0.40 | 0.65 | 0.85 |
| **NDCG@10** | ~0.10 | 0.25 | 0.50 | 0.70 |
| **Embedding Coverage** | 13.5% | 100% | 100% | 100% |
| **Graph Edges** | 0 | 200+ | 1000+ | 5000+ |
| **Indexing (10k files)** | <60s | <60s | <45s | <30s |
| **Search Latency (p95)** | <500ms | <400ms | <300ms | <200ms |
| **Memory (100k chunks)** | ~150MB | ~150MB | ~80MB | ~40MB |
| **Parallel Tool Speedup** | 1x | 3x | 4x | 5x |
| **Concurrent Agents** | 1 | 4 | 8 | 16 |
| **Context Assembly Latency** | **N/A** | **N/A** | **<150ms** | **<100ms** |
| **Zero-Tool-Call Rate** | **0%** | **0%** | **60%** | **80%** |
| **Pre-Fetch Cache Hit Rate** | **N/A** | **N/A** | **50%** | **60%** |

### 7.2 Validation Commands

```bash
# Search relevance
cargo bench --bench search_bench

# Embedding coverage
cargo run -p omni-cli -- status

# Indexing performance
cargo bench --bench indexing_bench

# Memory usage
cargo run -p omni-cli -- status | grep "Memory"

# Integration tests
cargo test --test tool_integration

# Parallel tool performance
cargo bench --bench parallel_tools

# Concurrent agent load test
cargo test --test concurrent_agents -- --ignored
```

### 7.3 Competitive Positioning

**What Makes OmniContext Superior**:

1. **Local-First + Privacy**: Code never leaves machine (vs Augment's cloud backend)
2. **Transparent Context**: Show exactly what was selected and why (vs Cursor's black box)
3. **Agent-Agnostic**: Works with ANY MCP agent (vs vendor lock-in)
4. **Pluggable Models**: Bring your own reranker, fine-tune on your codebase
5. **Open Source**: Community-driven patterns and improvements
6. **Invisible Context Injection**: Augment-style zero-tool-call responses with local privacy
7. **Parallel Execution**: 3-5x faster than sequential tool calls

---

## Appendix A: File-Level Flaw Inventory

| File | Flaw | Severity | Fix |
|------|------|----------|-----|
| search/mod.rs | Graph boost not wired | High | Wire DependencyGraph into scoring |
| search/mod.rs:204-208 | final_score = rrf_score only | High | Add weight boost |
| pipeline/mod.rs:317-362 | All edges use DependencyKind::Calls | Medium | Differentiate edge types |
| parser/mod.rs:68-75 | extract_imports returns empty | High | Implement per language |
| chunker/mod.rs:417-423 | Token estimation len/4 inaccurate | Medium | Use tokenizers crate |
| embedder/mod.rs | Only 13.5% coverage | Critical | Fix validation + retry |
| graph/mod.rs | Graph never queried by search | High | Integrate into SearchEngine |

## Appendix B: Research References

**2025 Papers & Resources**:
1. **ColBERT**: Token-level late interaction (100x faster than cross-encoders)
2. **GTE-ModernColBERT**: State-of-the-art late-interaction model (2025)
3. **cAST**: Chunking via Abstract Syntax Trees (arXiv 2506.15655v1)
4. **GNN-Coder**: GNN+Transformer for code retrieval (arXiv 2502.15202)
5. **GraphRAG**: Microsoft's graph-augmented retrieval architecture
6. **Jina Code Embeddings**: Purpose-built 1.5B param code model (2025)
7. **Vector Quantization**: Qdrant, Azure AI Search techniques
8. **LeSeR**: Lexical Reranking of Semantic Retrieval

**Access**: Search arXiv, ACL Anthology, or vendor blogs for implementation details.

---

**Document Status**: Living document, update as implementation progresses
**Next Review**: After Phase 0 completion
