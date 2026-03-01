# Phase 2 Implementation Plan: Knowledge Graph

## Status: Ready to Start
**Duration**: 4 weeks (Week 6-9)  
**Priority**: P1 (Critical for graph-based features)  
**Depends On**: Phase 1 (Concurrency) ✅ COMPLETE

## Overview

Phase 2 builds a dense semantic knowledge graph with 5000+ edges for 10k files, enabling graph-based search, dependency analysis, and architectural understanding. Currently, the graph has 0 edges despite petgraph infrastructure existing.

## Current State

**Problems**:
- Dependency graph has 0 edges (completely empty)
- `get_dependencies` MCP tool returns no results
- Import extraction returns empty Vec by default
- No call graph, type hierarchy, or temporal edges
- Graph-based search features are non-functional

**Root Causes**:
1. `extract_imports()` not implemented for most languages
2. Call site extraction not wired into pipeline
3. Type hierarchy extraction missing
4. No git commit analysis for temporal edges
5. Import resolution logic incomplete

## Success Criteria

- [x] Phase 1 complete (thread-safe concurrent access)
- [ ] Graph has 5000+ edges for 10k files
- [ ] Import resolution works for all supported languages
- [ ] Call graph populated from AST analysis
- [ ] Type hierarchy extracted (implements, extends, inherits)
- [ ] Temporal edges from git co-change analysis
- [ ] `get_dependencies` tool returns meaningful results
- [ ] Graph-based search ranking improves MRR by 20%+
- [ ] Community detection identifies architectural modules
- [ ] All tests passing

## Tasks

### Task 1: Import Resolution Engine (Week 6) - P1
**Goal**: Resolve import/use statements to actual symbols in the codebase

**Current State**:
- `parser::parse_imports()` returns empty Vec for most languages
- Import paths not resolved to actual file paths
- No symbol lookup from import statements

**Implementation**:

1. **Enhance `crates/omni-core/src/parser/mod.rs`**:
```rust
/// Multi-strategy import resolution
pub fn resolve_import(
    index: &MetadataIndex,
    import_path: &str,
    imported_name: &str,
    source_file: &Path,
    language: Language,
) -> Option<i64> {
    // Strategy 1: Direct FQN match
    if let Ok(Some(symbol)) = index.get_symbol_by_fqn(&format!("{}::{}", import_path, imported_name)) {
        return Some(symbol.id);
    }
    
    // Strategy 2: File path resolution
    let resolved_path = resolve_import_path(import_path, source_file, language);
    if let Ok(Some(file)) = index.get_file_by_path(&resolved_path) {
        // Find exported symbol in that file
        if let Ok(symbols) = index.get_symbols_for_file(file.id) {
            for sym in symbols {
                if sym.name == imported_name {
                    return Some(sym.id);
                }
            }
        }
    }
    
    // Strategy 3: Fuzzy name search
    if let Ok(symbols) = index.search_symbols_by_name(imported_name, 5) {
        // Return first match (could be improved with scoring)
        return symbols.first().map(|s| s.id);
    }
    
    None
}

/// Resolve import path to file path
fn resolve_import_path(import_path: &str, source_file: &Path, language: Language) -> PathBuf {
    match language {
        Language::Rust => {
            // crate::module::submodule → src/module/submodule.rs
            let path = import_path.replace("::", "/").replace("crate", "src");
            source_file.parent().unwrap().join(format!("{}.rs", path))
        }
        Language::Python => {
            // from package.module import X → package/module.py
            let path = import_path.replace(".", "/");
            source_file.parent().unwrap().join(format!("{}.py", path))
        }
        Language::TypeScript | Language::JavaScript => {
            // import X from './module' → ./module.ts
            if import_path.starts_with('.') {
                source_file.parent().unwrap().join(import_path)
            } else {
                // node_modules resolution (simplified)
                PathBuf::from("node_modules").join(import_path)
            }
        }
        _ => PathBuf::from(import_path),
    }
}
```

2. **Implement per-language import extraction**:

Update each language parser in `crates/omni-core/src/parser/languages/*.rs`:

**Python** (`python.rs`):
```rust
pub fn extract_imports(source: &[u8]) -> Vec<ImportStatement> {
    let mut imports = Vec::new();
    let mut cursor = tree.walk();
    
    for node in cursor.node().children(&mut cursor) {
        match node.kind() {
            "import_statement" => {
                // import module
                let module = node.child_by_field_name("name").unwrap();
                imports.push(ImportStatement {
                    import_path: get_node_text(module, source),
                    imported_names: vec!["*".to_string()],
                    kind: DependencyKind::Imports,
                });
            }
            "import_from_statement" => {
                // from module import X, Y
                let module = node.child_by_field_name("module_name").unwrap();
                let names = extract_imported_names(node, source);
                imports.push(ImportStatement {
                    import_path: get_node_text(module, source),
                    imported_names: names,
                    kind: DependencyKind::Imports,
                });
            }
            _ => {}
        }
    }
    imports
}
```

**Rust** (`rust.rs`):
```rust
pub fn extract_imports(source: &[u8]) -> Vec<ImportStatement> {
    let mut imports = Vec::new();
    let mut cursor = tree.walk();
    
    for node in cursor.node().children(&mut cursor) {
        if node.kind() == "use_declaration" {
            // use crate::module::{X, Y};
            let path = extract_use_path(node, source);
            let names = extract_use_names(node, source);
            imports.push(ImportStatement {
                import_path: path,
                imported_names: names,
                kind: DependencyKind::Imports,
            });
        }
    }
    imports
}
```

**TypeScript/JavaScript** (`typescript.rs`, `javascript.rs`):
```rust
pub fn extract_imports(source: &[u8]) -> Vec<ImportStatement> {
    let mut imports = Vec::new();
    let mut cursor = tree.walk();
    
    for node in cursor.node().children(&mut cursor) {
        match node.kind() {
            "import_statement" => {
                // import { X, Y } from 'module'
                let source_node = node.child_by_field_name("source").unwrap();
                let import_path = get_node_text(source_node, source)
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_string();
                let names = extract_import_specifiers(node, source);
                imports.push(ImportStatement {
                    import_path,
                    imported_names: names,
                    kind: DependencyKind::Imports,
                });
            }
            _ => {}
        }
    }
    imports
}
```

3. **Wire into pipeline** (`crates/omni-core/src/pipeline/mod.rs`):

Already implemented in Phase 0! Lines 317-362 handle import resolution.

**Testing**:
```bash
cargo test -p omni-core parser::tests::test_extract_imports
cargo test -p omni-core graph::tests::test_import_resolution
```

**Validation**:
```bash
# Index a test repo
cargo run -p omni-cli -- index tests/fixtures/python_project

# Check graph edges
cargo run -p omni-cli -- status | grep "Graph edges"
# Should show: Graph edges: 50+ (was 0)
```

### Task 2: Call Graph Construction (Week 7) - P2
**Goal**: Extract function calls from AST and build call edges

**Implementation**:

1. **Enhance `crates/omni-core/src/graph/mod.rs`**:

Already implemented! `build_call_edges()` method exists (lines in pipeline.rs).

2. **Add call site extraction to language parsers**:

Update `extract_element()` in each language parser to populate `references` field:

**Python**:
```rust
fn extract_references(node: Node, source: &[u8]) -> Vec<String> {
    let mut refs = Vec::new();
    let mut cursor = node.walk();
    
    for child in node.children(&mut cursor) {
        if child.kind() == "call" {
            if let Some(func) = child.child_by_field_name("function") {
                refs.push(get_node_text(func, source));
            }
        }
    }
    refs
}
```

**Rust**:
```rust
fn extract_references(node: Node, source: &[u8]) -> Vec<String> {
    let mut refs = Vec::new();
    let mut cursor = node.walk();
    
    for child in node.children(&mut cursor) {
        if child.kind() == "call_expression" {
            if let Some(func) = child.child_by_field_name("function") {
                refs.push(get_node_text(func, source));
            }
        }
    }
    refs
}
```

**Testing**:
```bash
cargo test -p omni-core graph::tests::test_call_graph
```

### Task 3: Type Hierarchy Extraction (Week 7) - P2
**Goal**: Extract implements, extends, inherits relationships

**Implementation**:

1. **Enhance `crates/omni-core/src/graph/mod.rs`**:

Already implemented! `build_type_edges()` method exists.

2. **Add type hierarchy extraction to language parsers**:

**Python**:
```rust
fn extract_type_hierarchy(node: Node, source: &[u8]) -> Vec<String> {
    let mut bases = Vec::new();
    
    if node.kind() == "class_definition" {
        if let Some(bases_node) = node.child_by_field_name("superclasses") {
            for base in bases_node.children(&mut bases_node.walk()) {
                if base.kind() == "identifier" {
                    bases.push(get_node_text(base, source));
                }
            }
        }
    }
    bases
}
```

**Rust**:
```rust
fn extract_type_hierarchy(node: Node, source: &[u8]) -> Vec<String> {
    let mut traits = Vec::new();
    
    if node.kind() == "impl_item" {
        if let Some(trait_node) = node.child_by_field_name("trait") {
            traits.push(get_node_text(trait_node, source));
        }
    }
    traits
}
```

**Testing**:
```bash
cargo test -p omni-core graph::tests::test_type_hierarchy
```

### Task 4: Community Detection (Week 8) - P2
**Goal**: Implement Louvain algorithm to detect architectural modules

**Implementation**:

1. **Add `crates/omni-core/src/graph/community.rs`**:
```rust
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

pub struct Community {
    pub id: usize,
    pub nodes: Vec<NodeIndex>,
    pub modularity: f64,
}

pub fn detect_communities(graph: &DependencyGraph) -> Vec<Community> {
    // Louvain algorithm implementation
    let mut communities = initialize_communities(graph);
    let mut improved = true;
    
    while improved {
        improved = false;
        
        for node in graph.graph.node_indices() {
            let best_community = find_best_community(node, &communities, graph);
            if best_community != communities[&node] {
                move_node_to_community(node, best_community, &mut communities);
                improved = true;
            }
        }
    }
    
    aggregate_communities(communities)
}

fn modularity(graph: &DependencyGraph, communities: &HashMap<NodeIndex, usize>) -> f64 {
    // Calculate modularity score
    let m = graph.edge_count() as f64;
    let mut q = 0.0;
    
    for edge in graph.graph.edge_indices() {
        let (src, dst) = graph.graph.edge_endpoints(edge).unwrap();
        if communities[&src] == communities[&dst] {
            let k_i = graph.graph.edges(src).count() as f64;
            let k_j = graph.graph.edges(dst).count() as f64;
            q += 1.0 - (k_i * k_j) / (2.0 * m);
        }
    }
    
    q / (2.0 * m)
}
```

2. **Store communities in SQLite**:

Add to `crates/omni-core/src/index/schema.sql`:
```sql
CREATE TABLE IF NOT EXISTS communities (
    id INTEGER PRIMARY KEY,
    modularity REAL NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS community_members (
    community_id INTEGER NOT NULL,
    symbol_id INTEGER NOT NULL,
    FOREIGN KEY (community_id) REFERENCES communities(id),
    FOREIGN KEY (symbol_id) REFERENCES symbols(id),
    PRIMARY KEY (community_id, symbol_id)
);
```

**Testing**:
```bash
cargo test -p omni-core graph::community::tests
```

### Task 5: Temporal Edges from Git (Week 8-9) - P2
**Goal**: Add co-change coupling edges from git commit history

**Implementation**:

1. **Enhance `crates/omni-core/src/commits.rs`**:
```rust
use gix::Repository;
use std::collections::HashMap;

pub fn extract_cochange_edges(repo_path: &Path) -> Vec<(PathBuf, PathBuf, f64)> {
    let repo = Repository::open(repo_path).unwrap();
    let mut cochanges: HashMap<(PathBuf, PathBuf), usize> = HashMap::new();
    
    // Analyze last 1000 commits
    for commit in repo.head().unwrap().peeled_to_commit().unwrap().ancestors().take(1000) {
        let commit = commit.unwrap();
        let tree = commit.tree().unwrap();
        let parent = commit.parent(0).ok().and_then(|p| p.tree().ok());
        
        if let Some(parent_tree) = parent {
            let changes = tree.changes().unwrap().for_each_to_obtain_tree(&parent_tree, |change| {
                // Track which files changed together
                change
            });
            
            // For each pair of files in this commit, increment co-change count
            let changed_files: Vec<PathBuf> = changes.collect();
            for i in 0..changed_files.len() {
                for j in (i+1)..changed_files.len() {
                    let pair = (changed_files[i].clone(), changed_files[j].clone());
                    *cochanges.entry(pair).or_insert(0) += 1;
                }
            }
        }
    }
    
    // Convert to edges with coupling strength
    cochanges.into_iter()
        .map(|((a, b), count)| (a, b, count as f64 / 1000.0))
        .filter(|(_, _, strength)| *strength > 0.1) // Only strong couplings
        .collect()
}
```

2. **Wire into pipeline**:

Add to `Engine::run_index()` after file processing:
```rust
// Extract temporal edges from git history
if let Ok(cochange_edges) = extract_cochange_edges(&self.config.repo_path) {
    for (file_a, file_b, strength) in cochange_edges {
        // Create temporal edge between files
        if let (Ok(Some(info_a)), Ok(Some(info_b))) = (
            self.index.get_file_by_path(&file_a),
            self.index.get_file_by_path(&file_b)
        ) {
            // Get first symbol from each file
            if let (Ok(Some(sym_a)), Ok(Some(sym_b))) = (
                self.index.get_first_symbol_for_file(info_a.id),
                self.index.get_first_symbol_for_file(info_b.id)
            ) {
                let edge = DependencyEdge {
                    source_id: sym_a.id,
                    target_id: sym_b.id,
                    kind: DependencyKind::CoChanges,
                };
                let _ = self.index.insert_dependency(&edge);
                let _ = self.dep_graph.add_edge(&edge);
            }
        }
    }
}
```

**Testing**:
```bash
cargo test -p omni-core commits::tests::test_cochange_extraction
```

### Task 6: Graph-Based Search Ranking (Week 9) - P1
**Goal**: Use graph distance to boost search scores

**Implementation**:

Already implemented in `crates/omni-core/src/search/mod.rs`! The `search()` method accepts `Option<&DependencyGraph>` and uses it for ranking.

**Enhancement**: Add graph-based relevance propagation:

```rust
fn propagate_relevance(
    results: &[SearchResult],
    graph: &DependencyGraph,
    index: &MetadataIndex,
    alpha: f64,
) -> Vec<SearchResult> {
    let mut scores: HashMap<i64, f64> = HashMap::new();
    
    // Initialize with search scores
    for result in results {
        if let Ok(Some(symbol)) = index.get_symbol_by_fqn(&result.chunk.symbol_path) {
            scores.insert(symbol.id, result.score);
        }
    }
    
    // Propagate scores to neighbors
    for result in results {
        if let Ok(Some(symbol)) = index.get_symbol_by_fqn(&result.chunk.symbol_path) {
            let neighbors = graph.neighbors(symbol.id, 2).unwrap_or_default();
            for neighbor_id in neighbors {
                let propagated_score = alpha * result.score;
                *scores.entry(neighbor_id).or_insert(0.0) += propagated_score;
            }
        }
    }
    
    // Re-rank results
    let mut enhanced_results = results.to_vec();
    for result in &mut enhanced_results {
        if let Ok(Some(symbol)) = index.get_symbol_by_fqn(&result.chunk.symbol_path) {
            if let Some(enhanced_score) = scores.get(&symbol.id) {
                result.score = *enhanced_score;
            }
        }
    }
    
    enhanced_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    enhanced_results
}
```

**Testing**:
```bash
cargo bench --bench search_bench
```

## Performance Targets

| Metric | Current | Target | Validation |
|--------|---------|--------|------------|
| Graph edges (10k files) | 0 | 5000+ | `cargo run -p omni-cli -- status` |
| Import resolution rate | 0% | 90%+ | Integration tests |
| Call graph coverage | 0% | 80%+ | Integration tests |
| Type hierarchy coverage | 0% | 70%+ | Integration tests |
| Community count | 0 | 10-20 | Community detection tests |
| Graph-based MRR improvement | 0% | 20%+ | `cargo bench --bench search_bench` |
| `get_dependencies` success rate | 0% | 95%+ | MCP integration tests |

## Dependencies

- Phase 1 (Concurrency) ✅ COMPLETE
- gix crate (already in Cargo.toml)
- petgraph crate (already in Cargo.toml)
- tree-sitter grammars (already in Cargo.toml)

## Risks & Mitigations

### Risk: Import Resolution Complexity
**Mitigation**: Multi-strategy resolution with fallbacks. Start with simple FQN matching, add fuzzy search as fallback.

### Risk: Graph Size Explosion
**Mitigation**: Limit edge types, filter low-confidence edges, use edge weights to prune weak connections.

### Risk: Community Detection Performance
**Mitigation**: Run community detection offline during indexing, cache results in SQLite.

### Risk: Git History Analysis Slow
**Mitigation**: Limit to last 1000 commits, run asynchronously, cache results.

## Next Phase

After Phase 2 completes, proceed to Phase 3: Cross-Encoder Reranking & Context Assembly
