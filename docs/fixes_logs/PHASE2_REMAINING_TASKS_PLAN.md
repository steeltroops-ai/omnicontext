# Phase 2 Remaining Tasks - Implementation Plan

## Status: Tasks 1-5 Complete ✅

Tasks 4 (Community Detection) and 5 (Temporal Edges) are now complete. This document provides detailed implementation plans for the remaining tasks.

---

## Priority 2: Full Embedding Coverage (CRITICAL)

**Current**: 13.5% embedding coverage  
**Target**: 100% coverage  
**Status**: Partially implemented (TF-IDF fallback exists but may not be triggered)

### Root Cause Analysis

The embedder already has TF-IDF fallback in `embed_batch()`, but coverage is still low. Possible causes:

1. **Embedder not available** - Model fails to load, degraded mode returns errors instead of fallbacks
2. **Batch processing failures** - Entire batches fail without individual retry
3. **Tokenization errors** - Special characters cause tokenizer to fail

### Implementation Steps

**File**: `crates/omni-core/src/embedder/mod.rs`

1. **Verify TF-IDF fallback is always triggered**:
   ```rust
   // In embed_batch(), ensure degraded mode returns TF-IDF vectors
   if !self.is_available() {
       return chunks.iter().map(|c| Some(self.tfidf_fallback(c))).collect();
   }
   ```

2. **Add coverage logging**:
   ```rust
   // After embedding, log coverage stats
   let coverage = all_embeddings.iter().filter(|e| e.is_some()).count();
   tracing::info!(
       "embedding coverage: {}/{} ({:.1}%)",
       coverage,
       all_embeddings.len(),
       (coverage as f64 / all_embeddings.len() as f64) * 100.0
   );
   ```

3. **Test with real repository**:
   ```bash
   cargo run -p omni-cli -- index .
   cargo run -p omni-cli -- status
   # Check embedding_coverage_percent field
   ```

### Validation

```bash
# Should show 100% coverage
cargo run -p omni-cli -- status | grep embedding_coverage
```

---

## Priority 3: Populated Dependency Graph (HIGH)

**Current**: 202 edges for 100 nodes  
**Target**: 5000+ edges for 10k files  
**Status**: Import resolution exists but coverage is low

### Root Cause Analysis

The graph has low edge count because:

1. **Import resolution misses many imports** - Multi-strategy resolution may not cover all patterns
2. **Call graph extraction is incomplete** - References may not be fully extracted from AST
3. **Type hierarchy edges are sparse** - Not all extends/implements relationships captured

### Implementation Steps

**File**: `crates/omni-core/src/parser/languages/python.rs` (and other language parsers)

1. **Improve import extraction**:
   ```rust
   // Add more import patterns
   // - Relative imports: from ..module import X
   // - Star imports: from module import *
   // - Aliased imports: import module as alias
   ```

2. **Enhance reference extraction**:
   ```rust
   // In extract_references(), capture:
   // - Method calls: obj.method()
   // - Function calls: function()
   // - Constructor calls: new Class()
   // - Type annotations: def foo(x: Type)
   ```

3. **Add cross-file resolution**:
   ```rust
   // In DependencyGraph::resolve_import()
   // - Check file path patterns (e.g., ./utils -> utils.py)
   // - Handle package imports (e.g., package.module)
   // - Support language-specific conventions
   ```

### Validation

```bash
# Should show 5000+ edges
cargo run -p omni-cli -- index .
cargo run -p omni-cli -- status | grep graph_edges
```

---

## Task 6: Graph-Augmented Search (HIGH PRIORITY)

**Goal**: Propagate relevance scores through the dependency graph  
**Status**: Not started  
**Impact**: Improves search recall by including related code

### Algorithm

```
1. Execute initial search → get results R with scores
2. For each result r in top-K:
   a. Find graph neighbors N(r) via dep_graph.upstream() and downstream()
   b. For each neighbor n:
      - Calculate propagated score: score(n) = alpha × score(r) × edge_weight(r,n)
      - Add to candidate set if not already in R
3. Merge R and propagated neighbors
4. Re-rank combined set by final score
5. Return top-limit results
```

### Implementation

**File**: `crates/omni-core/src/search/mod.rs`

```rust
/// Apply graph-based relevance propagation to search results.
///
/// For each top result, propagate relevance to graph neighbors
/// (upstream dependencies and downstream dependents).
fn apply_graph_boost(
    &self,
    results: &mut Vec<ScoredChunk>,
    dep_graph: &crate::graph::DependencyGraph,
    index: &MetadataIndex,
    alpha: f64,  // propagation factor (0.3)
    max_depth: usize,  // neighbor hops (2)
    top_k: usize,  // how many top results to propagate from (10)
) -> OmniResult<()> {
    use std::collections::HashMap;
    
    // Track propagated scores
    let mut propagated_scores: HashMap<i64, f64> = HashMap::new();
    
    // For each top-K result, propagate to neighbors
    for scored in results.iter().take(top_k) {
        // Get the symbol_id for this chunk
        let chunk = match self.get_chunk_by_id(index, scored.chunk_id) {
            Some(c) => c,
            None => continue,
        };
        
        // Find symbol for this chunk
        let symbol = match index.get_symbol_by_fqn(&chunk.symbol_path)? {
            Some(s) => s,
            None => continue,
        };
        
        // Get upstream dependencies (what this depends on)
        let upstream = dep_graph.upstream(symbol.id, max_depth)?;
        
        // Get downstream dependents (what depends on this)
        let downstream = dep_graph.downstream(symbol.id, max_depth)?;
        
        // Propagate score to neighbors
        for neighbor_symbol_id in upstream.iter().chain(downstream.iter()) {
            // Find chunks for this neighbor symbol
            let neighbor_symbol = match index.get_symbol_by_id(*neighbor_symbol_id)? {
                Some(s) => s,
                None => continue,
            };
            
            if let Some(chunk_id) = neighbor_symbol.chunk_id {
                // Calculate propagated score
                let prop_score = alpha * scored.final_score;
                
                // Accumulate (a chunk might be reached from multiple paths)
                *propagated_scores.entry(chunk_id).or_insert(0.0) += prop_score;
            }
        }
    }
    
    // Add propagated chunks to results if not already present
    let existing_ids: std::collections::HashSet<i64> = 
        results.iter().map(|r| r.chunk_id).collect();
    
    for (chunk_id, prop_score) in propagated_scores {
        if !existing_ids.contains(&chunk_id) {
            // Add as new result with propagated score
            results.push(ScoredChunk {
                chunk_id,
                final_score: prop_score,
                breakdown: ScoreBreakdown {
                    dependency_boost: prop_score,
                    ..Default::default()
                },
            });
        } else {
            // Boost existing result
            if let Some(result) = results.iter_mut().find(|r| r.chunk_id == chunk_id) {
                result.final_score += prop_score;
                result.breakdown.dependency_boost += prop_score;
            }
        }
    }
    
    // Re-sort by final score
    results.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap());
    
    Ok(())
}
```

**Integration point**: Call after RRF fusion, before reranking:

```rust
// In search() method, after fused = self.fuse_results(...)
if let Some(graph) = dep_graph {
    self.apply_graph_boost(&mut fused, graph, index, 0.3, 2, 10)?;
}
```

### Parameters

- `alpha = 0.3` - Propagation factor (30% of original score)
- `max_depth = 2` - Traverse up to 2 hops in the graph
- `top_k = 10` - Propagate from top 10 results only

### Validation

```bash
# Test search with graph boost
cargo run -p omni-cli -- search "authentication" --limit 20
# Should include related auth functions even if not directly matching
```

---

## Task 7: Cross-Encoder Reranking (HIGH PRIORITY)

**Goal**: Two-stage retrieval for precision improvement  
**Status**: Reranker module exists but needs cross-encoder model  
**Target**: MRR@5 ≥ 0.75, NDCG@10 ≥ 0.70

### Current State

The reranker module (`crates/omni-core/src/reranker/mod.rs`) exists and is integrated into search, but it's using a placeholder model.

### Implementation Steps

**File**: `crates/omni-core/src/embedder/model_manager.rs`

1. **Add cross-encoder model spec**:
   ```rust
   pub const CROSS_ENCODER_MODEL: ModelSpec = ModelSpec {
       name: "ms-marco-MiniLM-L-6-v2",
       repo: "cross-encoder/ms-marco-MiniLM-L-6-v2",
       model_file: "model.onnx",
       tokenizer_file: "tokenizer.json",
       dimensions: 1,  // Cross-encoder outputs a single score
   };
   ```

2. **Update reranker to use cross-encoder**:
   ```rust
   // In reranker/mod.rs
   pub fn rerank(&self, query: &str, texts: &[&str]) -> Vec<Option<f32>> {
       // For each text, compute cross-encoder score with query
       // Input: [CLS] query [SEP] text [SEP]
       // Output: single relevance score
   }
   ```

3. **Integrate into search pipeline**:
   - Stage 1: Fast recall via HNSW + BM25 → top-100 candidates
   - Stage 2: Cross-encoder scores query-chunk pairs → rerank to top-10

### Validation

```bash
# Run benchmarks to measure MRR and NDCG
cargo bench --bench search_bench
# Should show MRR@5 ≥ 0.75, NDCG@10 ≥ 0.70
```

---

## Task 8: Overlapping Chunking (MEDIUM PRIORITY)

**Goal**: Prevent context loss at chunk boundaries  
**Status**: Backward overlap exists, need forward overlap  
**Impact**: Improves LLM understanding of chunk context

### Current State

The chunker already implements backward overlap (`compute_backward_context()`), but it doesn't include forward overlap for context continuity.

### Implementation Steps

**File**: `crates/omni-core/src/chunker/mod.rs`

1. **Add forward overlap configuration**:
   ```rust
   // In Config
   pub struct IndexingConfig {
       // ... existing fields
       pub forward_overlap_tokens: u32,  // default: 100
       pub forward_overlap_lines: usize,  // default: 5
   }
   ```

2. **Implement forward overlap**:
   ```rust
   fn compute_forward_context(
       source_lines: &[&str],
       end_line_idx: usize,
       target_tokens: u32,
       max_lines: usize,
   ) -> String {
       // Similar to backward context, but grab lines AFTER the element
       let latest = (end_line_idx + max_lines).min(source_lines.len());
       let mut selected_end = end_line_idx;
       let mut accumulated_tokens: u32 = 0;
       
       for idx in end_line_idx..latest {
           let line_tokens = estimate_tokens(source_lines[idx]);
           if accumulated_tokens + line_tokens > target_tokens {
               break;
           }
           accumulated_tokens += line_tokens;
           selected_end = idx + 1;
       }
       
       if selected_end > end_line_idx {
           source_lines[end_line_idx..selected_end].join("\n")
       } else {
           String::new()
       }
   }
   ```

3. **Apply to chunk creation**:
   ```rust
   // In element_to_chunk()
   let forward_context = compute_forward_context(
       &source_lines,
       elem.line_end as usize,
       config.indexing.forward_overlap_tokens,
       config.indexing.forward_overlap_lines,
   );
   
   let content = format!("{}{}\n{}", context_header, elem.content, forward_context);
   ```

### Validation

```bash
# Check that chunks include surrounding context
cargo run -p omni-cli -- index .
# Inspect chunk content in database to verify overlap
```

---

## Implementation Priority Order

1. **Priority 2: Full Embedding Coverage** (1-2 hours)
   - Critical for search quality
   - Quick fix: ensure TF-IDF fallback always triggers

2. **Task 6: Graph-Augmented Search** (2-3 hours)
   - High impact on search recall
   - Algorithm is well-defined

3. **Priority 3: Populated Dependency Graph** (3-4 hours)
   - Improves graph-augmented search effectiveness
   - Requires parser improvements across multiple languages

4. **Task 7: Cross-Encoder Reranking** (4-5 hours)
   - High impact on search precision
   - Requires model integration and testing

5. **Task 8: Overlapping Chunking** (2-3 hours)
   - Medium impact on context quality
   - Relatively straightforward implementation

**Total estimated time**: 12-17 hours

---

## Testing Strategy

After each task:

1. **Unit tests**: Add tests to verify the feature works in isolation
2. **Integration tests**: Test with real repository
3. **Benchmarks**: Measure performance impact
4. **Status check**: Verify metrics improve

```bash
# Standard validation workflow
cargo build --workspace
cargo test -p omni-core
cargo run -p omni-cli -- index .
cargo run -p omni-cli -- status
cargo bench --bench search_bench
```

---

## Success Criteria

| Metric | Current | Target | Task |
|--------|---------|--------|------|
| Embedding Coverage | 13.5% | 100% | Priority 2 |
| Graph Edges (10k files) | 202 | 5000+ | Priority 3 |
| MRR@5 | 0.15 | 0.75 | Task 7 |
| NDCG@10 | 0.10 | 0.70 | Task 7 |
| Search Recall | Low | High | Task 6 |
| Context Quality | Good | Excellent | Task 8 |

---

## Next Steps

1. Start with Priority 2 (Full Embedding Coverage) - quick win
2. Implement Task 6 (Graph-Augmented Search) - high impact
3. Continue with remaining tasks in priority order
4. Run full benchmark suite after each task
5. Update PHASE2_STATUS.md as tasks complete
