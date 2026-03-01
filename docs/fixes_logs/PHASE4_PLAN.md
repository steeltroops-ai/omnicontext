# Phase 4 Plan: Performance Optimization & Benchmarking

**Start Date**: March 1, 2026  
**Target Completion**: May 10, 2026 (10 weeks)  
**Status**: Starting

## Overview

Phase 4 focuses on performance optimization, comprehensive benchmarking, and expanding language support. This phase will validate that OmniContext meets its performance targets and provides production-ready quality.

## Goals

1. Establish automated benchmark suite for continuous quality monitoring
2. Optimize indexing and search performance to meet targets
3. Expand language support to cover more ecosystems
4. Implement speculative pre-fetch for improved UX
5. Optimize memory usage with quantized vectors

## Task Breakdown

### Task 1: Benchmark Suite (Week 1-2) ⏳ STARTING

**Objective**: Create automated benchmark suite for MRR, NDCG, Recall@K validation

**Subtasks**:

1.1. **Golden Query Dataset** (Day 1-2)
- Create `tests/bench/golden_queries.json` with 50+ queries
- Each query has:
  - Query text
  - Expected relevant chunks (symbol paths)
  - Relevance scores (3=highly relevant, 2=relevant, 1=marginally)
- Cover all query intents: Explain, Edit, Debug, Refactor, Generate
- Include edge cases: ambiguous queries, multi-file queries, cross-module queries

1.2. **Benchmark Runner** (Day 3-4)
- Create `tests/bench/search_quality.rs`
- Implement MRR (Mean Reciprocal Rank) calculation
- Implement NDCG@K (Normalized Discounted Cumulative Gain)
- Implement Recall@K
- Implement Precision@K
- Output results in JSON format for tracking

1.3. **Baseline Measurement** (Day 5)
- Run benchmarks on current implementation
- Record baseline metrics:
  - MRR@5: Current value
  - NDCG@10: Current value
  - Recall@10: Current value
  - Precision@10: Current value
- Store in `tests/bench/baseline.json`

1.4. **CI Integration** (Day 6-7)
- Add benchmark step to CI pipeline
- Fail if metrics regress by >10%
- Generate performance report on each PR
- Track metrics over time

**Success Criteria**:
- ✅ 50+ golden queries covering all intents
- ✅ Automated benchmark runner
- ✅ Baseline metrics recorded
- ✅ CI integration complete
- ✅ Performance regression detection working

**Files to Create**:
- `tests/bench/golden_queries.json`
- `tests/bench/search_quality.rs`
- `tests/bench/baseline.json`
- `.github/workflows/benchmark.yml`

---

### Task 2: Performance Optimization (Week 3-4) ⏳ NOT STARTED

**Objective**: Optimize indexing and search to meet performance targets

**Subtasks**:

2.1. **Profile Current Performance** (Day 1-2)
- Use `cargo flamegraph` to profile indexing
- Use `cargo flamegraph` to profile search
- Identify hot paths (>10% CPU time)
- Document bottlenecks

2.2. **Optimize Indexing** (Day 3-5)
- Target: <30s for 10k files (currently ~60s)
- Potential optimizations:
  - Batch SQLite inserts (use transactions)
  - Parallel file parsing (rayon)
  - Reduce allocations in chunker
  - Optimize tree-sitter queries
- Benchmark after each optimization

2.3. **Optimize Search** (Day 6-8)
- Target: <200ms P95 (currently ~500ms)
- Potential optimizations:
  - Cache query embeddings (LRU cache)
  - Optimize RRF fusion (reduce allocations)
  - Parallel signal retrieval (tokio::join!)
  - Optimize graph traversal (cache distances)
- Benchmark after each optimization

2.4. **Memory Profiling** (Day 9-10)
- Use `valgrind --tool=massif` or `heaptrack`
- Identify memory leaks
- Reduce peak memory usage
- Document memory usage per component

**Success Criteria**:
- ✅ Indexing: <30s for 10k files
- ✅ Search P95: <200ms
- ✅ Memory: <100MB for 10k files
- ✅ No memory leaks detected
- ✅ Flamegraphs show no obvious bottlenecks

**Files to Modify**:
- `crates/omni-core/src/pipeline/mod.rs` (indexing)
- `crates/omni-core/src/search/mod.rs` (search)
- `crates/omni-core/src/chunker/mod.rs` (chunking)
- `crates/omni-core/src/parser/mod.rs` (parsing)

---

### Task 3: Additional Languages (Week 5-6) ⏳ NOT STARTED

**Objective**: Add Ruby, PHP, Swift, Kotlin support with enhanced reference extraction

**Subtasks**:

3.1. **Ruby Support** (Day 1-3)
- Add `tree-sitter-ruby` dependency
- Create `crates/omni-core/src/parser/languages/ruby.rs`
- Extract: classes, modules, methods, constants
- Extract references: method calls, constant refs, module includes
- Write 10+ unit tests
- Test on real Ruby projects (Rails, Sinatra)

3.2. **PHP Support** (Day 4-6)
- Add `tree-sitter-php` dependency
- Create `crates/omni-core/src/parser/languages/php.rs`
- Extract: classes, functions, traits, interfaces
- Extract references: function calls, class instantiation, trait usage
- Write 10+ unit tests
- Test on real PHP projects (Laravel, Symfony)

3.3. **Swift Support** (Day 7-9)
- Add `tree-sitter-swift` dependency
- Create `crates/omni-core/src/parser/languages/swift.rs`
- Extract: classes, structs, protocols, extensions
- Extract references: method calls, protocol conformance, property access
- Write 10+ unit tests
- Test on real Swift projects (iOS apps)

3.4. **Kotlin Support** (Day 10-12)
- Add `tree-sitter-kotlin` dependency
- Create `crates/omni-core/src/parser/languages/kotlin.rs`
- Extract: classes, objects, interfaces, functions
- Extract references: function calls, class instantiation, interface implementation
- Write 10+ unit tests
- Test on real Kotlin projects (Android apps)

**Success Criteria**:
- ✅ All 4 languages parse correctly
- ✅ Reference extraction works (graph edges increase)
- ✅ All tests pass
- ✅ Tested on real-world projects
- ✅ Documentation updated

**Files to Create**:
- `crates/omni-core/src/parser/languages/ruby.rs`
- `crates/omni-core/src/parser/languages/php.rs`
- `crates/omni-core/src/parser/languages/swift.rs`
- `crates/omni-core/src/parser/languages/kotlin.rs`

**Files to Modify**:
- `crates/omni-core/src/parser/registry.rs` (register languages)
- `crates/omni-core/Cargo.toml` (add dependencies)
- `docs/guides/SUPPORTED_LANGUAGES.md` (update docs)

---

### Task 4: Speculative Pre-Fetch (Week 7-8) ⏳ NOT STARTED

**Objective**: Implement speculative pre-fetch for improved UX

**Subtasks**:

4.1. **Pre-Fetch Module** (Day 1-3)
- Create `crates/omni-daemon/src/prefetch.rs`
- Implement LRU cache with TTL (5 minutes)
- Implement pre-fetch heuristics:
  - File open → pre-fetch file context
  - Cursor move to symbol → pre-fetch symbol dependencies
  - Edit in function → pre-fetch related tests
- Track cache hit rate

4.2. **IDE Event Monitoring** (Day 4-6)
- Extend VS Code extension to send events:
  - `file_opened`
  - `cursor_moved`
  - `text_edited`
- Send events to daemon via IPC
- Implement event debouncing (200ms)

4.3. **Pre-Fetch Integration** (Day 7-9)
- Integrate pre-fetch with context assembly
- Serve cached context when available
- Fall back to on-demand search if cache miss
- Log cache hit rate

4.4. **Evaluation** (Day 10-12)
- Measure cache hit rate on real usage
- Target: >50% cache hit rate
- Measure latency improvement
- Tune heuristics based on data

**Success Criteria**:
- ✅ Pre-fetch module implemented
- ✅ IDE events sent to daemon
- ✅ Cache hit rate >50%
- ✅ Latency improvement measurable
- ✅ No performance degradation

**Files to Create**:
- `crates/omni-daemon/src/prefetch.rs`

**Files to Modify**:
- `crates/omni-daemon/src/main.rs` (integrate pre-fetch)
- `editors/vscode/src/extension.ts` (send events)

---

### Task 5: Quantized Vector Search (Week 9-10) ⏳ NOT STARTED

**Objective**: Reduce memory usage with quantized vectors

**Subtasks**:

5.1. **Scalar Quantization** (Day 1-3)
- Implement uint8 scalar quantization in `crates/omni-core/src/vector/mod.rs`
- Store min/max per vector for dequantization
- Formula: `quantized = ((value - min) / (max - min)) * 255`
- Dequantize for final scoring

5.2. **Hybrid Search** (Day 4-6)
- Use quantized vectors for recall (HNSW search)
- Use full precision vectors for final scoring
- Store both quantized and full precision
- Benchmark accuracy vs memory tradeoff

5.3. **Index Migration** (Day 7-9)
- Implement index migration from full precision to quantized
- Add version field to index schema
- Backward compatibility with old indexes
- Automatic migration on first load

5.4. **Evaluation** (Day 10-12)
- Measure memory usage: target 40MB for 100k chunks
- Measure search quality: should not regress >5%
- Benchmark search latency: should not increase >10%
- Document tradeoffs

**Success Criteria**:
- ✅ Memory usage: 40MB for 100k chunks (vs 150MB)
- ✅ Search quality: <5% regression
- ✅ Search latency: <10% increase
- ✅ Index migration works
- ✅ Backward compatibility maintained

**Files to Modify**:
- `crates/omni-core/src/vector/mod.rs` (quantization)
- `crates/omni-core/src/index/mod.rs` (migration)
- `crates/omni-core/src/index/schema.sql` (version field)

---

## Performance Targets

| Metric | Current | Target | Task |
|--------|---------|--------|------|
| MRR@5 | 0.15 | 0.75 | Task 1 (measure), Task 2 (optimize) |
| NDCG@10 | 0.10 | 0.70 | Task 1 (measure), Task 2 (optimize) |
| Recall@10 | 0.20 | 0.85 | Task 1 (measure), Task 2 (optimize) |
| Indexing (10k files) | ~60s | <30s | Task 2 |
| Search P95 | ~500ms | <200ms | Task 2 |
| Memory (100k chunks) | ~150MB | ~40MB | Task 5 |
| Cache Hit Rate | N/A | >50% | Task 4 |
| Language Support | 10 | 14 | Task 3 |

## Risk Mitigation

### Risk 1: Performance targets not achievable
**Mitigation**: Profile early, optimize incrementally, adjust targets if needed

### Risk 2: Quantization degrades search quality
**Mitigation**: Hybrid approach (quantized for recall, full precision for scoring)

### Risk 3: Pre-fetch increases memory usage
**Mitigation**: LRU cache with TTL, configurable cache size

### Risk 4: New languages have poor reference extraction
**Mitigation**: Test on real projects, iterate on extraction logic

## Dependencies

- Task 2 depends on Task 1 (need benchmarks to validate optimizations)
- Task 4 can run in parallel with Task 3
- Task 5 can run in parallel with Task 3 and 4

## Timeline

```
Week 1-2:  Task 1 (Benchmark Suite)
Week 3-4:  Task 2 (Performance Optimization)
Week 5-6:  Task 3 (Additional Languages)
Week 7-8:  Task 4 (Speculative Pre-Fetch)
Week 9-10: Task 5 (Quantized Vector Search)
```

## Success Criteria (Phase 4 Complete)

- ✅ All 5 tasks complete
- ✅ All performance targets met or documented why not
- ✅ All tests pass
- ✅ No performance regressions
- ✅ Documentation updated
- ✅ CI integration complete

## Next Phase

**Phase 5**: Production Readiness
- Multi-repo indexing
- Enterprise features (SSO, RBAC)
- Hosted API deployment
- Advanced pattern recognition
- Custom model fine-tuning
