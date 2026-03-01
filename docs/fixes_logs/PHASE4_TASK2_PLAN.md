# Phase 4 Task 2: Performance Optimization

**Start Date**: March 1, 2026  
**Target Duration**: 2-3 days  
**Status**: Starting

## Objectives

1. Reduce indexing time from ~60s to <30s for 10k files
2. Reduce search latency from ~500ms to <200ms P95
3. Optimize memory usage
4. Identify and fix performance bottlenecks

## Subtasks

### 2.1: Profile Current Performance ⏳ STARTING

**Tools**:
- `cargo flamegraph` for CPU profiling
- `cargo bench` for performance measurement
- `time` command for timing

**Actions**:
1. Profile indexing pipeline
2. Profile search operations
3. Identify hot paths (>10% CPU time)
4. Document bottlenecks

### 2.2: Optimize Indexing

**Target**: <30s for 10k files (currently ~60s)

**Potential Optimizations**:
1. **Batch SQLite Operations**
   - Use transactions for bulk inserts
   - Reduce fsync calls
   - Batch chunk insertions

2. **Parallel File Processing**
   - Use `rayon` for parallel parsing
   - Process multiple files concurrently
   - Maintain order for deterministic results

3. **Reduce Allocations**
   - Reuse buffers in chunker
   - Optimize string operations
   - Use `Cow` where appropriate

4. **Optimize Tree-Sitter Queries**
   - Cache compiled queries
   - Reduce query complexity
   - Minimize AST traversals

### 2.3: Optimize Search

**Target**: <200ms P95 (currently ~500ms)

**Potential Optimizations**:
1. **Cache Query Embeddings**
   - LRU cache for recent queries
   - Cache size: 100 queries
   - TTL: 5 minutes

2. **Optimize RRF Fusion**
   - Reduce allocations in fusion
   - Use iterators instead of collecting
   - Early termination when possible

3. **Parallel Signal Retrieval**
   - Use `tokio::join!` for concurrent retrieval
   - Keyword, vector, and symbol searches in parallel
   - Reduce total latency

4. **Optimize Graph Traversal**
   - Cache distance calculations
   - Use BFS with early termination
   - Limit graph depth

### 2.4: Memory Profiling

**Actions**:
1. Measure memory usage per component
2. Identify memory leaks
3. Reduce peak memory usage
4. Document memory characteristics

## Implementation Strategy

### Phase 1: Measure Baseline
- Run benchmarks to establish baseline
- Profile with flamegraph
- Document current performance

### Phase 2: Low-Hanging Fruit
- Implement easy optimizations first
- Batch SQLite operations
- Add query caching
- Parallel signal retrieval

### Phase 3: Deep Optimizations
- Parallel file processing
- Reduce allocations
- Optimize hot paths

### Phase 4: Validate
- Re-run benchmarks
- Compare against baseline
- Document improvements

## Success Criteria

- ✅ Indexing: <30s for 10k files
- ✅ Search P95: <200ms
- ✅ Memory: <100MB for 10k files
- ✅ No performance regressions
- ✅ All tests still pass

## Files to Modify

1. `crates/omni-core/src/pipeline/mod.rs` - Indexing pipeline
2. `crates/omni-core/src/search/mod.rs` - Search engine
3. `crates/omni-core/src/index/mod.rs` - SQLite operations
4. `crates/omni-core/src/chunker/mod.rs` - Chunking
5. `crates/omni-core/src/parser/mod.rs` - Parsing

## Risks

1. **Complexity**: Parallel processing adds complexity
2. **Correctness**: Must maintain correctness while optimizing
3. **Diminishing Returns**: Some optimizations may not be worth the complexity

## Mitigation

1. Measure before and after each optimization
2. Run full test suite after each change
3. Use benchmarks to validate improvements
4. Document tradeoffs
