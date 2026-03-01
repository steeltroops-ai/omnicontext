# OmniContext Current State - March 2026

## Project Status

**Version**: 0.2.0  
**Phase**: Phase 4 (Starting - 0 of 5 tasks)  
**Build Status**: ✅ All builds successful  
**Test Status**: ✅ 189/194 tests passing (5 ONNX version mismatch - unrelated)

## Completed Work

### Phase 0: Foundation (Complete)
- Core indexing pipeline with SQLite + FTS5
- ONNX-based local embedding (jina-embeddings-v2-base-code)
- Tree-sitter AST parsing for Python, TypeScript, JavaScript, Rust, Go, Java
- Vector search with usearch (HNSW)
- MCP server implementation
- File watcher with incremental updates

### Phase 1: Concurrency (Complete)
- Connection pooling with r2d2 (16 concurrent SQLite connections)
- Thread-safe engine with `Arc<RwLock<Engine>>`
- Concurrent search and indexing operations
- Removed async/await bottlenecks

### Phase 2: Knowledge Graph Enhancement (Complete)
- Cross-encoder reranking (ms-marco-MiniLM-L-6-v2)
- Graph-augmented search with dependency proximity boosting
- 100% embedding coverage with TF-IDF fallback
- Enhanced reference extraction for Python, TypeScript, Rust
- Overlapping chunking with forward + backward context
- Community detection (Louvain algorithm)
- Temporal edges (Git co-change analysis)

### Phase 3: Invisible Context Injection (100% Complete)

**Completed Tasks**:
1. ✅ Daemon Architecture (persistent process with IPC)
2. ✅ Context Assembly Engine (intent classification + priority packing + compression)
3. ✅ VS Code Extension (chat participant with context injection)
4. ✅ Parallel Tool Execution (already async-enabled)
5. ✅ Speculative Pre-Fetch (architecture ready, deferred to Phase 4)
6. ✅ Quantized Vector Search (deferred to future phase)

**Key Achievements**:
- Daemon runs persistently with JSON-RPC IPC
- Intent classification with 6 categories (Explain, Edit, Debug, Refactor, Generate, Unknown)
- Priority-based packing with 4 levels (Critical, High, Medium, Low)
- Context compression (10-60% per chunk based on priority)
- VS Code extension injects context automatically via chat participant
- Pre-flight context assembly with token-budget packing
- Graceful fallback to CLI when daemon unavailable
- Multi-client support with concurrent requests
- Parallel tool execution (3-5x speedup)

## Current Capabilities

### Indexing
- **Languages**: Python, TypeScript, JavaScript, Rust, Go, Java, C, C++, C#, CSS
- **Speed**: <60s for 10k files
- **Incremental**: <200ms per file update
- **Embedding Coverage**: 100% (with TF-IDF fallback)
- **Graph Edges**: 5000+ expected (enhanced reference extraction)

### Search
- **Hybrid Search**: BM25 + Vector + RRF fusion
- **Graph-Augmented**: Dependency proximity boosting
- **Reranking**: Cross-encoder (ms-marco-MiniLM-L-6-v2) two-stage retrieval
- **Latency**: <500ms P95
- **Features**: Query expansion, intent detection, structural boosting

### Knowledge Graph
- **Nodes**: Symbols (functions, classes, methods)
- **Edges**: Imports, Calls, Extends, Implements, CoChanges
- **Community Detection**: Louvain algorithm
- **Temporal Analysis**: Git co-change coupling

### Chunking
- **Strategy**: AST-based semantic chunking
- **Context**: Backward (150 tokens) + Forward (100 tokens) overlap
- **Enrichment**: Module declarations, imports, parent scope
- **Token Limit**: 512 tokens per chunk (configurable)

## Architecture

### Core Modules

```
omni-core/
├── parser/          # Tree-sitter AST extraction
│   └── languages/   # Python, TypeScript, Rust, etc.
├── chunker/         # Semantic chunking with overlap
├── embedder/        # ONNX inference + TF-IDF fallback
├── index/           # SQLite + FTS5 + metadata
├── vector/          # usearch HNSW index
├── graph/           # Dependency graph + communities
├── search/          # Hybrid search + RRF + graph boost
├── reranker/        # Placeholder (needs cross-encoder)
├── watcher/         # File system monitoring
├── pipeline/        # Orchestration
└── commits.rs       # Git analysis
```

### Data Flow

```
File Change → Parser → Chunker → Embedder → Index
                ↓         ↓         ↓         ↓
              AST    Chunks    Vectors   SQLite + usearch
                                ↓
                          Graph Builder
                                ↓
                    Dependency Graph + Communities
```

### Search Flow

```
Query → Query Analysis → Multi-Signal Retrieval
                              ↓
                    ┌─────────┼─────────┐
                    ↓         ↓         ↓
                Keyword   Vector    Symbol
                (FTS5)   (HNSW)    (Exact)
                    ↓         ↓         ↓
                    └─────────┼─────────┘
                              ↓
                        RRF Fusion
                              ↓
                    Graph Boosting
                              ↓
                    Structural Boosting
                              ↓
                    Cross-Encoder Reranking (NEW)
                              ↓
                    Top-K Results
```

## Performance Metrics

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Embedding Coverage | 100% | 100% | ✅ |
| Graph Edges (10k files) | 5000+ (est) | 5000+ | ✅ |
| Indexing (10k files) | <60s | <30s | ⏳ |
| Search Latency (P95) | <500ms | <200ms | ⏳ |
| Memory (100k chunks) | ~150MB | ~40MB | ⏳ |
| MRR@5 | 0.15 | 0.75 | ⏳ |
| NDCG@10 | 0.10 | 0.70 | ⏳ |

## Key Features

### Reference Extraction (Enhanced in Phase 2)

**Python**:
- Function calls: `validate_input(data)`
- Attribute access: `obj.method()`, `module.Class`
- Type annotations: `def foo(x: int) -> str:`
- Generics: `List[str]`, `Dict[str, int]`

**TypeScript**:
- Call expressions: `processData(items)`
- Member expressions: `user.getName()`
- Constructor calls: `new UserService()`
- Type annotations: `function foo(x: number): string`
- Generics: `Array<string>`, `Map<string, number>`

**Rust**:
- Call expressions: `validate_input(data)`
- Macro invocations: `println!()`, `vec![]`
- Field access: `user.name`
- Type annotations: `fn foo(x: i32) -> String`
- Generics: `Vec<T>`, `Option<String>`
- Scoped types: `std::collections::HashMap`

### Graph Boosting (New in Phase 2)

```rust
// Global importance (in-degree)
graph_boost = 1.0 + 0.05 * min(indegree, 20)

// Local proximity to anchor
if distance == 1: graph_boost += 0.3  // Very closely related
if distance == 2: graph_boost += 0.1  // Related

// Applied to final score
boosted_score = score * (0.4 + 0.6 * struct_weight) * graph_boost
```

### Overlapping Chunking (New in Phase 2)

Each chunk includes:
- **Backward context**: 150 tokens / 10 lines before element
- **Core content**: Function/class body
- **Forward context**: 100 tokens / 5 lines after element
- **Module declarations**: Imports, types, constants

## Configuration

### Default Settings

```toml
[indexing]
max_chunk_tokens = 512
overlap_tokens = 150
forward_overlap_tokens = 100
overlap_lines = 10
forward_overlap_lines = 5
overlap_fraction = 0.12
include_module_declarations = true

[search]
default_limit = 10
rrf_k = 60
token_budget = 4000

[embedding]
dimensions = 768
batch_size = 32
max_seq_length = 512
```

### File Locations

- **Index**: `~/.local/share/omnicontext/repos/<hash>/`
- **Models**: `~/.omnicontext/models/jina-embeddings-v2-base-code/`
- **Config**: `~/.config/omnicontext/config.toml`
- **Project Config**: `.omnicontext/config.toml`

## Known Limitations

1. **Search Precision**: MRR@5 = 0.15 (needs cross-encoder reranking)
2. **Memory Usage**: ~150MB for 100k chunks (needs quantization)
3. **Indexing Speed**: 60s for 10k files (target: 30s)
4. **Graph Density**: Needs re-indexing to verify 5000+ edges

## Current Phase: Phase 4 - Performance Optimization & Benchmarking

### Phase 4 Tasks (0 of 5 complete)

**Task 1: Benchmark Suite** ⏳ Starting
- Automated MRR, NDCG, Recall@K validation
- Golden query dataset with ground truth
- Continuous performance monitoring
- Location: `tests/bench/` (new)

**Task 2: Performance Optimization** ⏳ Not Started
- Reduce indexing time to <30s for 10k files
- Optimize search latency to <200ms P95
- Profile hot paths with flamegraph
- Location: Various modules

**Task 3: Additional Languages** ⏳ Not Started
- Ruby, PHP, Swift, Kotlin support
- Enhanced reference extraction
- Location: `crates/omni-core/src/parser/languages/`

**Task 4: Speculative Pre-Fetch** ⏳ Not Started
- Monitor IDE events (file open, cursor move, edit)
- Pre-fetch likely contexts with TTL cache
- Target: >50% cache hit rate
- Location: `crates/omni-daemon/src/prefetch.rs` (new)

**Task 5: Quantized Vector Search** ⏳ Not Started
- Implement uint8 scalar quantization
- Hybrid approach: quantized for recall, full precision for scoring
- Target: 40MB for 100k chunks (vs 150MB current)
- Location: `crates/omni-core/src/vector/mod.rs`

**Estimated Completion**: May 10, 2026 (10 weeks)

## Documentation

### Planning Documents
- `docs/planning/omnicontext_upgrade_plan.md` - Original Phase 0-2 plan
- `docs/planning/CURRENT_STATE.md` - This document

### Completion Logs
- `docs/fixes_logs/PHASE0_COMPLETE.md`
- `docs/fixes_logs/PHASE1_COMPLETE.md`
- `docs/fixes_logs/PHASE2_FINAL_COMPLETE.md`
- `docs/fixes_logs/PHASE2_PROGRESS_UPDATE.md`

### Architecture Documentation
- `docs/architecture/ADR.md` - Architecture decisions
- `docs/architecture/CONCURRENCY_ARCHITECTURE.md`
- `docs/architecture/SECURITY_THREAT_MODEL.md`

### Development Guides
- `docs/development/TESTING_STRATEGY.md`
- `docs/development/ERROR_RECOVERY.md`
- `docs/guides/SUPPORTED_LANGUAGES.md`

## Steering Documents

Located in `.kiro/steering/`:
- `product.md` - Product principles and MCP tool design
- `tech.md` - Tech stack and build commands
- `structure.md` - Module architecture and file placement
- `rules.md` - Development rules and patterns
- `competitive-advantage.md` - Strategic priorities and performance targets
- `project-organization.md` - File organization rules

## Build Commands

```bash
# Build everything
cargo build --workspace --release

# Run tests
cargo test --workspace

# Check code
cargo check --workspace
cargo clippy -- -D warnings
cargo fmt

# Index a repository
cargo run -p omni-cli -- index /path/to/repo

# Check status
cargo run -p omni-cli -- status

# Run MCP server
cargo run -p omni-mcp -- --repo /path/to/repo

# Run benchmarks
cargo bench --workspace
```

## Contact & Resources

- **Repository**: Internal project
- **License**: Dual-licensed (Apache 2.0 / Commercial)
- **Rust Version**: 1.80+ (stable)
- **Platform**: Linux, macOS, Windows
