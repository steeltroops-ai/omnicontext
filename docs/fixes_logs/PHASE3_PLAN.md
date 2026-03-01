# Phase 3 Implementation Plan: Invisible Context Injection & Performance

**Status**: Starting  
**Date**: March 1, 2026  
**Phase 2 Completion**: 100% (all 10 tasks complete)

## Overview

Phase 3 focuses on implementing the "Augment Code" model - invisible context injection that eliminates the need for explicit MCP tool calls. This dramatically improves user experience by automatically providing relevant context to the LLM before it responds.

## Core Concept: Pre-Flight Context Delivery

**Current Model (Pull-based)**:
```
User: "Fix the auth bug"
→ Agent calls search_code("authentication bug")
→ Agent calls get_file_summary("src/auth/middleware.ts")
→ Agent calls get_dependencies("AuthMiddleware")
→ Agent responds with fix
```

**Target Model (Push-based)**:
```
User: "Fix the auth bug"
→ Extension intercepts message
→ Daemon assembles relevant context (auth files, dependencies, tests)
→ Context injected into system/user prompt
→ Agent responds immediately with fix (no tool calls needed)
```

**Benefits**:
- 80% reduction in tool calls (zero-tool-call rate target: 80%)
- 3-5x faster responses (no round-trip latency)
- Better context quality (intent-based selection)
- Transparent to user (can show/hide context)

## Phase 3 Tasks

### Task 1: Daemon Architecture (Week 1-2)

**Goal**: Create persistent background process with IPC

**Implementation**:
1. Enhance `omni-daemon` to support RPC calls (not just file watching)
2. Add Unix socket (Linux/Mac) and named pipe (Windows) IPC
3. Implement `get_context_window` RPC endpoint
4. Add daemon lifecycle management (start, stop, restart, health check)

**Files to modify**:
- `crates/omni-daemon/src/main.rs` - Add RPC server
- `crates/omni-daemon/src/ipc.rs` - Implement IPC transport
- `crates/omni-daemon/src/protocol.rs` - Define RPC protocol

**New RPC Methods**:
```rust
pub enum DaemonRequest {
    GetContextWindow {
        prompt: String,
        active_file: Option<PathBuf>,
        cursor_position: Option<Position>,
        token_budget: u32,
    },
    GetStatus,
    Shutdown,
}

pub enum DaemonResponse {
    ContextWindow {
        context: ContextWindow,
        latency_ms: u64,
    },
    Status {
        indexed_files: usize,
        memory_mb: f64,
        uptime_secs: u64,
    },
    Ok,
}
```

**Success Criteria**:
- Daemon starts on system boot
- IPC latency <10ms for simple requests
- Graceful shutdown without data loss
- Auto-restart on crash

### Task 2: Context Assembly Engine (Week 3-4)

**Goal**: Build token-budget-aware context packing with priorities

**Implementation**:
1. Create `ContextAssembler` in `omni-core/src/search/`
2. Implement intent classification (Explain, Edit, Debug, Refactor, Generate)
3. Add priority-based packing algorithm
4. Implement context compression for low-priority chunks

**Files to create/modify**:
- `crates/omni-core/src/search/context_assembler.rs` (new)
- `crates/omni-core/src/search/intent.rs` (new)
- `crates/omni-core/src/types.rs` - Add `ContextWindow`, `ContextEntry`, `ChunkPriority`

**Context Assembly Algorithm**:
```rust
pub struct ContextAssembler {
    token_budget: u32,
}

impl ContextAssembler {
    pub fn assemble(
        &self,
        query: &str,
        search_results: Vec<SearchResult>,
        active_file: Option<PathBuf>,
        dep_graph: &DependencyGraph,
    ) -> ContextWindow {
        // 1. Classify intent
        let intent = QueryIntent::classify(query);
        let strategy = intent.context_strategy();
        
        // 2. Prioritize chunks
        let mut prioritized = self.prioritize_chunks(
            search_results,
            active_file,
            strategy,
        );
        
        // 3. Pack within token budget
        let mut context = ContextWindow::new(self.token_budget);
        let mut tokens_used = 0;
        
        for (chunk, priority) in prioritized {
            if tokens_used + chunk.token_count <= self.token_budget {
                context.add_chunk(chunk, priority);
                tokens_used += chunk.token_count;
            } else if priority == ChunkPriority::Critical {
                // Compress and include critical chunks
                let compressed = self.compress_chunk(chunk);
                if tokens_used + compressed.token_count <= self.token_budget {
                    context.add_chunk(compressed, ChunkPriority::Low);
                    tokens_used += compressed.token_count;
                }
            }
        }
        
        // 4. Add architectural summary if space remains
        if tokens_used < self.token_budget * 9 / 10 {
            context.architecture_overview = self.generate_summary(&chunks);
        }
        
        context
    }
}
```

**Priority Levels**:
- `Critical`: Active file, cursor context, direct dependencies
- `High`: Search results with score >0.8, test files
- `Medium`: Search results with score 0.5-0.8, related files
- `Low`: Architectural context, documentation, distant dependencies

**Success Criteria**:
- Token budget utilization >90%
- Context assembly latency <100ms
- Intent classification accuracy >80%
- Relevant context in top 3 chunks >90%

### Task 3: VS Code Extension (Week 5-6)

**Goal**: Intercept chat messages and inject context before LLM call

**Implementation**:
1. Create TypeScript extension in `editors/vscode/`
2. Hook into chat provider API
3. Communicate with daemon via IPC
4. Inject context into system/user prompt

**Files to create**:
- `editors/vscode/src/extension.ts` - Main extension entry
- `editors/vscode/src/daemonClient.ts` - IPC client
- `editors/vscode/src/contextInjector.ts` - Prompt enrichment
- `editors/vscode/package.json` - Extension manifest

**Extension Architecture**:
```typescript
export async function activate(context: vscode.ExtensionContext) {
    const daemonClient = new DaemonClient();
    
    // Register chat participant
    const participant = vscode.chat.createChatParticipant(
        'omnicontext',
        async (request, context, stream, token) => {
            // Get active file and cursor
            const activeFile = vscode.window.activeTextEditor?.document.uri.fsPath;
            const cursorPos = vscode.window.activeTextEditor?.selection.active;
            
            // Request context from daemon
            const contextWindow = await daemonClient.getContextWindow({
                prompt: request.prompt,
                activeFile,
                cursorPosition: cursorPos,
                tokenBudget: 50000,
            });
            
            // Inject context into prompt
            const enrichedPrompt = injectContext(
                request.prompt,
                contextWindow,
                config.get('contextInjection.strategy')
            );
            
            // Forward to LLM with enriched prompt
            return forwardToLLM(enrichedPrompt, stream, token);
        }
    );
    
    context.subscriptions.push(participant);
}
```

**Context Injection Strategies**:
1. **System Prompt** (invisible): Add context to system message
2. **User Prompt** (transparent): Append context to user message
3. **Hybrid**: System for structure, user for key context

**Configuration**:
```json
{
  "omnicontext.contextInjection.enabled": true,
  "omnicontext.contextInjection.strategy": "system",
  "omnicontext.contextInjection.tokenBudget": 50000,
  "omnicontext.contextInjection.showInChat": false,
  "omnicontext.contextInjection.showBadge": true,
  "omnicontext.contextInjection.preFetchEnabled": true
}
```

**Success Criteria**:
- Extension activates without errors
- Context injection latency <150ms
- Zero-tool-call rate >60% for common queries
- User can toggle visibility of injected context

### Task 4: Parallel Tool Execution (Week 7-8)

**Goal**: Enable concurrent MCP tool calls for 3-5x speedup

**Implementation**:
1. Convert MCP tools to async (currently synchronous)
2. Add batch operations for common patterns
3. Enable parallel execution in MCP server

**Files to modify**:
- `crates/omni-mcp/src/tools.rs` - Convert to async
- `crates/omni-mcp/src/main.rs` - Add async runtime
- `crates/omni-core/src/pipeline/mod.rs` - Ensure thread-safety

**Async Tool Signature**:
```rust
pub async fn search_code(
    engine: Arc<RwLock<Engine>>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let engine = engine.read().await;
    engine.search(&query, limit.unwrap_or(10))
        .map_err(|e| e.to_string())
}
```

**Batch Operations**:
```rust
pub async fn batch_get_symbols(
    engine: Arc<RwLock<Engine>>,
    names: Vec<String>,
) -> Result<Vec<Option<Symbol>>, String> {
    let engine = engine.read().await;
    let mut results = Vec::with_capacity(names.len());
    
    for name in names {
        results.push(engine.get_symbol(&name).ok());
    }
    
    Ok(results)
}

pub async fn batch_get_files(
    engine: Arc<RwLock<Engine>>,
    paths: Vec<PathBuf>,
) -> Result<Vec<Option<FileSummary>>, String> {
    let engine = engine.read().await;
    let mut results = Vec::with_capacity(paths.len());
    
    for path in paths {
        results.push(engine.get_file_summary(&path).ok());
    }
    
    Ok(results)
}
```

**Success Criteria**:
- All MCP tools are async
- Parallel execution of 4 tools: 3-4x speedup
- No race conditions or deadlocks
- Backward compatible with sequential calls

### Task 5: Speculative Pre-Fetch (Week 9-10)

**Goal**: Predict and cache likely queries based on IDE state

**Implementation**:
1. Monitor IDE events (file open, cursor move, edit)
2. Pre-fetch likely contexts
3. Cache with TTL (5 minutes)
4. Measure cache hit rate

**Files to create**:
- `crates/omni-daemon/src/prefetch.rs` (new)
- `crates/omni-core/src/search/cache.rs` (new)

**Pre-Fetch Triggers**:
```rust
pub enum PreFetchTrigger {
    FileOpened(PathBuf),
    CursorMoved(Position),
    FileEdited(PathBuf),
    CommentStarted,
    ErrorDetected(Diagnostic),
    TestFailed(TestResult),
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
            let context = self.engine.get_context_window(query.clone(), ...).await;
            self.cache.insert(query, context, Duration::from_secs(300));
        }
    }
}
```

**Success Criteria**:
- Cache hit rate >50% for common queries
- Pre-fetch latency doesn't impact IDE responsiveness
- Memory usage <50MB for cache
- TTL prevents stale context

### Task 6: Quantized Vector Search (Week 11-12)

**Goal**: Reduce memory usage by 4x with uint8 quantization

**Implementation**:
1. Implement scalar quantization (f32 → uint8)
2. Hybrid approach: quantized for recall, full precision for scoring
3. Update usearch integration

**Files to modify**:
- `crates/omni-core/src/vector/mod.rs` - Add quantization
- `crates/omni-core/src/embedder/mod.rs` - Generate quantized vectors

**Quantization Algorithm**:
```rust
pub struct QuantizedVector {
    quantized: Vec<u8>,
    min: f32,
    max: f32,
}

impl QuantizedVector {
    pub fn from_f32(vec: &[f32]) -> Self {
        let min = vec.iter().copied().fold(f32::INFINITY, f32::min);
        let max = vec.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;
        
        let quantized = vec.iter()
            .map(|&v| ((v - min) / range * 255.0) as u8)
            .collect();
        
        Self { quantized, min, max }
    }
    
    pub fn to_f32(&self) -> Vec<f32> {
        let range = self.max - self.min;
        self.quantized.iter()
            .map(|&q| (q as f32 / 255.0) * range + self.min)
            .collect()
    }
}
```

**Success Criteria**:
- Memory usage: 40MB for 100k chunks (vs 150MB current)
- Search quality degradation <5% (NDCG@10)
- Indexing time increase <10%
- Backward compatible (can load old indexes)

## Performance Targets

| Metric | Current | Phase 3 Target | Validation |
|--------|---------|----------------|------------|
| Zero-Tool-Call Rate | 0% | 60% | Manual testing with common queries |
| Context Assembly Latency | N/A | <100ms | Benchmark |
| Parallel Tool Speedup | 1x | 3-4x | Benchmark |
| Memory (100k chunks) | ~150MB | ~40MB | Status command |
| Pre-Fetch Cache Hit Rate | N/A | >50% | Daemon metrics |
| MRR@5 | 0.15 | 0.55 | Search benchmark |
| NDCG@10 | 0.10 | 0.50 | Search benchmark |

## Success Criteria

**Phase 3 is complete when**:
1. ✅ Daemon runs persistently with IPC
2. ✅ VS Code extension injects context automatically
3. ✅ Context assembly latency <100ms
4. ✅ Zero-tool-call rate >60%
5. ✅ Parallel tool execution works (3x speedup)
6. ✅ Quantized vectors reduce memory by 4x
7. ✅ All tests pass
8. ✅ Documentation updated

## Risk Mitigation

**Risk 1: IPC Complexity**
- Mitigation: Use battle-tested libraries (tokio, serde)
- Fallback: HTTP server if IPC fails

**Risk 2: VS Code API Changes**
- Mitigation: Pin to stable API version
- Fallback: Manual context injection via command

**Risk 3: Context Quality**
- Mitigation: A/B testing with/without injection
- Fallback: User can disable feature

**Risk 4: Performance Regression**
- Mitigation: Benchmark before/after each change
- Fallback: Feature flags to disable expensive operations

## Timeline

- **Week 1-2**: Daemon + IPC
- **Week 3-4**: Context Assembly Engine
- **Week 5-6**: VS Code Extension
- **Week 7-8**: Parallel Tool Execution
- **Week 9-10**: Speculative Pre-Fetch
- **Week 11-12**: Quantized Vectors

**Total Duration**: 12 weeks  
**Target Completion**: May 24, 2026

## Next Actions

1. Start with Task 1 (Daemon Architecture)
2. Implement IPC protocol
3. Test daemon lifecycle management
4. Move to Task 2 (Context Assembly)

