# Phase 2 Implementation Status

## Overview

Phase 2 focuses on building a dense, semantically-rich knowledge graph with community detection and temporal analysis.

**Target**: Graph with 5000+ edges for 10k files (currently: 202 edges, 100 nodes)

## Task Status

### ✅ Task 1: Import Resolution (COMPLETE - Phase 0)

**Status**: Already implemented in Phase 0

**Implementation**: `crates/omni-core/src/graph/mod.rs`
- Multi-strategy import resolution: exact FQN match → suffix match → name-only fallback
- Handles both `::` (Rust) and `.` (Python/TS) separators
- Shortest FQN preference for ambiguous matches

**Evidence**: `DependencyGraph::resolve_import()` method fully implemented

---

### ✅ Task 2: Call Graph Construction (COMPLETE - Phase 0)

**Status**: Already implemented in Phase 0

**Implementation**: `crates/omni-core/src/graph/mod.rs`
- `DependencyGraph::build_call_edges()` method extracts function calls from AST
- Resolves references to target symbols (local file first, then global)
- Creates `DependencyKind::Calls` edges
- Wired into `Engine::process_file()` pipeline

**Evidence**: Graph has 202 edges (verified with `cargo run -p omni-cli -- status`)

---

### ✅ Task 3: Type Hierarchy Edges (COMPLETE - Phase 0)

**Status**: Already implemented in Phase 0

**Implementation**: `crates/omni-core/src/graph/mod.rs`
- `DependencyGraph::build_type_edges()` method extracts extends/implements relationships
- Creates `DependencyKind::Extends` and `DependencyKind::Implements` edges
- Wired into `Engine::process_file()` pipeline

**Evidence**: Type edges are being created during indexing

---

### ✅ Task 4: Community Detection (COMPLETE)

**Status**: Fully implemented and integrated

**Implementation**:
- **Algorithm**: `crates/omni-core/src/graph/community.rs`
  - Louvain algorithm with iterative modularity optimization
  - Detects cohesive architectural modules in the codebase
  - Returns communities with modularity scores (>0.3 = good structure)
- **Graph Integration**: `crates/omni-core/src/graph/mod.rs`
  - `DependencyGraph::detect_communities()` method
- **Persistence**: `crates/omni-core/src/index/mod.rs`
  - `MetadataIndex::store_communities()` - atomic storage
  - `MetadataIndex::get_communities()` - retrieval
  - `MetadataIndex::community_count()` - count
- **Pipeline Integration**: `crates/omni-core/src/pipeline/mod.rs`
  - Community detection runs after indexing completes
  - Results stored in SQLite (`communities` and `community_members` tables)
  - `EngineStatus::community_count` field added

**Database Schema**: `crates/omni-core/src/index/schema.sql`
```sql
CREATE TABLE IF NOT EXISTS communities (
    id INTEGER PRIMARY KEY,
    modularity REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS community_members (
    community_id INTEGER NOT NULL,
    symbol_id INTEGER NOT NULL,
    FOREIGN KEY (community_id) REFERENCES communities(id) ON DELETE CASCADE,
    FOREIGN KEY (symbol_id) REFERENCES symbols(id) ON DELETE CASCADE,
    PRIMARY KEY (community_id, symbol_id)
);
```

**Testing**: Unit tests in `crates/omni-core/src/graph/community.rs`
- Empty graph handling
- Single node community
- Two-cluster detection
- Modularity calculation

**Verification**:
```bash
cargo build -p omni-core  # ✅ Compiles successfully
cargo test -p omni-core graph::community::tests  # ✅ All tests pass
cargo run -p omni-cli -- status  # Shows community_count field
```

---

### ✅ Task 5: Temporal Edges from Git Co-Change Analysis (COMPLETE)

**Status**: Fully implemented

**Implementation**:
- **Co-Change Extraction**: `crates/omni-core/src/commits.rs`
  - `CommitEngine::extract_cochange_coupling()` method
  - Analyzes git history to find files that frequently change together
  - Returns coupling strength (0.0 to 1.0) normalized by total commits
  - Filters weak couplings (<15% threshold)
- **Edge Creation**: `crates/omni-core/src/pipeline/mod.rs`
  - `Engine::build_temporal_edges()` method
  - Creates bidirectional `DependencyKind::CoChanges` edges
  - Stores in both SQLite and in-memory graph
- **Type System**: `crates/omni-core/src/types.rs`
  - Added `DependencyKind::CoChanges` variant
  - Updated `as_str()` and `from_str_lossy()` methods

**Algorithm**:
1. Parse git log for last N commits (configurable, default 1000)
2. For each commit, track which files changed together
3. Calculate coupling strength: `co_changes / total_commits`
4. Filter pairs with strength > 0.15 (strong coupling)
5. Create bidirectional CoChanges edges between file symbols

**Usage**:
```rust
// Call after indexing to add temporal edges
engine.build_temporal_edges(1000)?;
```

**Verification**:
```bash
cargo build -p omni-core  # ✅ Compiles successfully
cargo build -p omni-mcp   # ✅ Compiles successfully
```

---

## Next Steps (Phase 2 Remaining Tasks)

### Task 6: Graph-Augmented Search

**Goal**: Propagate relevance scores through the dependency graph

**Implementation Plan**:
1. Modify `crates/omni-core/src/search/mod.rs`
2. After initial search, get graph neighbors for top results
3. Propagate relevance: `score(neighbor) += alpha × score(result) × edge_weight`
4. Re-rank combined results
5. Add `graph_boost` field to `ScoreBreakdown`

**Parameters**:
- `alpha = 0.3` (propagation factor)
- `max_depth = 2` (neighbor hops)

---

### Task 7: Cross-Encoder Reranking

**Goal**: Two-stage retrieval for precision improvement

**Implementation Plan**:
1. Add ONNX cross-encoder model to `embedder/model_manager.rs`
2. Stage 1: Fast recall via HNSW + BM25 → top-100 candidates
3. Stage 2: Cross-encoder scores query-chunk pairs → rerank to top-10
4. Integrate into `search/mod.rs` as post-processing

**Target**: MRR@5 ≥ 0.75, NDCG@10 ≥ 0.70

---

### Task 8: Overlapping Chunking

**Goal**: Prevent context loss at chunk boundaries

**Implementation Plan**:
1. Modify `crates/omni-core/src/chunker/mod.rs`
2. Implement CAST algorithm with 100-200 token overlap
3. Ensure functions include surrounding module context
4. Add overlap configuration to `config.rs`

---

## Performance Metrics (Current vs Target)

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Graph Edges (10k files) | 202 | 5000+ | 🔴 Need more edges |
| Embedding Coverage | 13.5% | 100% | 🔴 Fix chunker validation |
| MRR@5 | 0.15 | 0.75 | 🔴 Need cross-encoder |
| NDCG@10 | 0.10 | 0.70 | 🔴 Need cross-encoder |
| Community Count | TBD | >10 | ✅ Implemented |
| Temporal Edges | TBD | >100 | ✅ Implemented |

---

## Validation Commands

```bash
# Build and test
cargo build --workspace
cargo test -p omni-core

# Check graph statistics
cargo run -p omni-cli -- status

# Run MCP server
cargo run -p omni-mcp -- --repo .

# Test community detection
cargo test -p omni-core graph::community::tests

# Test temporal edges (requires git repo)
cargo run -p omni-cli -- index .
# Check dep_edges count in status output
```

---

## Notes

- Tasks 1-3 were already completed in Phase 0 (import resolution, call graph, type hierarchy)
- Tasks 4-5 are now complete (community detection, temporal edges)
- Graph edge count is still low (202) - need to improve import resolution coverage
- Next priority: Fix embedding coverage (Task from competitive-advantage.md)
- Then implement graph-augmented search (Task 6) and cross-encoder reranking (Task 7)
