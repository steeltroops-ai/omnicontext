# Phase 1 Implementation Plan: Async & Concurrency

## Status: Ready to Start
**Duration**: 7 weeks  
**Priority**: P0 (Blocks Phase 2+)

## Overview

Phase 1 makes OmniContext thread-safe and enables parallel tool execution, achieving 3-5x speedup for multi-tool agent queries.

## Tasks

### Task 1: Connection Pooling (Week 1) - P0
**Goal**: Make MetadataIndex thread-safe using r2d2 connection pool

**Changes**:
1. Add dependencies to `Cargo.toml`:
   ```toml
   r2d2 = "0.8"
   r2d2_sqlite = "0.26"
   ```

2. Update `crates/omni-core/src/index/mod.rs`:
   ```rust
   pub struct MetadataIndex {
       pool: Pool<SqliteConnectionManager>,
       db_path: PathBuf,
   }
   
   impl MetadataIndex {
       pub fn open(db_path: &Path) -> OmniResult<Self> {
           let manager = SqliteConnectionManager::file(db_path);
           let pool = Pool::builder()
               .max_size(16)  // Support 16 concurrent connections
               .build(manager)?;
           
           // Initialize schema on one connection
           let conn = pool.get()?;
           Self::ensure_schema(&conn)?;
           
           Ok(Self { pool, db_path: db_path.to_path_buf() })
       }
       
       // Update all methods to use pool.get()?
       pub fn upsert_file(&self, file: &FileInfo) -> OmniResult<i64> {
           let conn = self.pool.get()?;
           // ... existing logic
       }
   }
   ```

3. Update all methods in MetadataIndex to acquire connection from pool

**Testing**:
```bash
cargo test -p omni-core index
```

### Task 2: Async Tool Handlers (Week 2) - P0
**Goal**: Convert all MCP tool handlers to async

**Changes**:
1. Update `crates/omni-mcp/src/tools.rs`:
   ```rust
   // Before
   fn search_code(params: SearchParams) -> Result<SearchResults>
   
   // After
   async fn search_code(params: SearchParams) -> Result<SearchResults>
   ```

2. Wrap Engine in Arc for sharing across async tasks:
   ```rust
   pub struct McpServer {
       engine: Arc<Engine>,  // Shared across async handlers
   }
   ```

3. Update all tool handlers to be async

**Testing**:
```bash
cargo test -p omni-mcp
cargo run -p omni-mcp -- --repo .
```

### Task 3: Parallel Tool Execution (Week 3) - P0
**Goal**: Enable tokio::join! for parallel tool calls

**Changes**:
1. Update MCP server to handle parallel requests:
   ```rust
   // Agent can now call multiple tools simultaneously
   let (search_results, symbol_info, deps) = tokio::join!(
       search_code(search_params),
       get_symbol(symbol_params),
       get_dependencies(deps_params)
   );
   ```

2. Add batch tools for common patterns:
   ```rust
   async fn batch_get_symbols(symbols: Vec<String>) -> Vec<SymbolInfo>
   async fn batch_get_files(paths: Vec<String>) -> Vec<FileContent>
   ```

**Testing**:
```bash
# Test parallel execution
cargo test -p omni-mcp --test parallel_tools
```

### Task 4: Wrap Engine in Arc<RwLock> (Week 4) - P1
**Goal**: Allow concurrent reads, exclusive writes

**Changes**:
1. Update `crates/omni-core/src/pipeline/mod.rs`:
   ```rust
   // Wrap Engine in Arc<RwLock> for shared access
   pub type SharedEngine = Arc<RwLock<Engine>>;
   
   impl Engine {
       pub fn shared(self) -> SharedEngine {
           Arc::new(RwLock::new(self))
       }
   }
   ```

2. Update MCP server to use SharedEngine:
   ```rust
   pub struct McpServer {
       engine: SharedEngine,
   }
   
   async fn search_code(&self, params: SearchParams) -> Result<SearchResults> {
       let engine = self.engine.read().await;
       engine.search(&params.query, params.limit)
   }
   ```

**Testing**:
```bash
cargo test --workspace
```

### Task 5: Concurrent Agent Load Test (Week 5) - P1
**Goal**: Verify 16 concurrent agents can query simultaneously

**Changes**:
1. Create `crates/omni-mcp/tests/concurrent_agents.rs`:
   ```rust
   #[tokio::test]
   async fn test_16_concurrent_agents() {
       let engine = Engine::new(test_repo()).unwrap().shared();
       
       let handles: Vec<_> = (0..16).map(|i| {
           let engine = Arc::clone(&engine);
           tokio::spawn(async move {
               let results = engine.read().await.search(&format!("query {}", i), 10);
               assert!(results.is_ok());
           })
       }).collect();
       
       for handle in handles {
           handle.await.unwrap();
       }
   }
   ```

**Testing**:
```bash
cargo test -p omni-mcp --test concurrent_agents -- --ignored
```

### Task 6: Performance Benchmarks (Week 6) - P1
**Goal**: Measure parallel speedup

**Changes**:
1. Create `benches/parallel_tools.rs`:
   ```rust
   fn bench_sequential_tools(c: &mut Criterion) {
       c.bench_function("sequential_3_tools", |b| {
           b.iter(|| {
               let r1 = search_code();
               let r2 = get_symbol();
               let r3 = get_dependencies();
           })
       });
   }
   
   fn bench_parallel_tools(c: &mut Criterion) {
       c.bench_function("parallel_3_tools", |b| {
           b.iter(|| {
               tokio::runtime::Runtime::new().unwrap().block_on(async {
                   tokio::join!(
                       search_code(),
                       get_symbol(),
                       get_dependencies()
                   )
               })
           })
       });
   }
   ```

**Testing**:
```bash
cargo bench --bench parallel_tools
```

### Task 7: Documentation (Week 7) - P2
**Goal**: Document async patterns and concurrency model

**Changes**:
1. Update `docs/architecture/CONCURRENCY_ARCHITECTURE.md`
2. Add examples to `docs/guides/MCP_USAGE.md`
3. Update `README.md` with parallel tool examples

## Success Criteria

- [ ] MetadataIndex uses connection pool (16 connections)
- [ ] All MCP tools are async
- [ ] tokio::join! works for parallel tools
- [ ] Engine wrapped in Arc<RwLock<Engine>>
- [ ] 16 concurrent agents can query simultaneously
- [ ] 3-5x speedup measured for 3-tool queries
- [ ] All tests passing
- [ ] Documentation updated

## Performance Targets

| Metric | Current | Target | Validation |
|--------|---------|--------|------------|
| Concurrent agents | 1 | 16 | Load test |
| Parallel tool speedup | 1x | 3-5x | Benchmark |
| Search latency (p95) | <500ms | <400ms | Benchmark |

## Dependencies

- r2d2 0.8
- r2d2_sqlite 0.26
- tokio (already in workspace)

## Risks

1. **Connection pool exhaustion**: Mitigated by 16 connection limit
2. **Deadlocks**: Mitigated by RwLock and careful lock ordering
3. **Performance regression**: Mitigated by benchmarks before/after

## Next Phase

After Phase 1 completes, proceed to Phase 2: Cross-Encoder Reranking
