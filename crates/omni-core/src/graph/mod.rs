//! Dependency graph construction and traversal using petgraph.
//!
//! The dependency graph tracks relationships between symbols:
//! imports, calls, extends, implements, type usage, etc.
//!
//! Used for:
//! - Dependency proximity boosting in search
//! - get_dependencies MCP tool
//! - Impact analysis ("what breaks if I change this?")
//! - Circular dependency detection
#![allow(
    clippy::doc_markdown,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::manual_let_else,
    clippy::map_entry,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::unnecessary_wraps
)]

pub mod attention;
pub mod dependencies;
pub mod edge_extractor;
pub mod historical;
pub mod queries;

use crate::error::OmniResult;
use crate::types::{DependencyEdge, DependencyKind};

use petgraph::algo::is_cyclic_directed;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use std::collections::HashMap;
use std::sync::RwLock;

/// Thread-safe dependency graph.
pub struct DependencyGraph {
    /// The underlying directed graph. Protected by RwLock.
    inner: RwLock<GraphInner>,
}

struct GraphInner {
    graph: DiGraph<i64, DependencyKind>,
    symbol_to_node: HashMap<i64, NodeIndex>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(GraphInner {
                graph: DiGraph::new(),
                symbol_to_node: HashMap::new(),
            }),
        }
    }

    /// Add a symbol node to the graph. Returns the node index.
    pub fn add_symbol(&self, symbol_id: i64) -> OmniResult<()> {
        let mut inner = self
            .inner
            .write()
            .map_err(|e| crate::error::OmniError::Internal(format!("graph lock poisoned: {e}")))?;

        if !inner.symbol_to_node.contains_key(&symbol_id) {
            let idx = inner.graph.add_node(symbol_id);
            inner.symbol_to_node.insert(symbol_id, idx);
        }

        Ok(())
    }

    /// Add a dependency edge between two symbols.
    pub fn add_edge(&self, edge: &DependencyEdge) -> OmniResult<()> {
        let mut inner = self
            .inner
            .write()
            .map_err(|e| crate::error::OmniError::Internal(format!("graph lock poisoned: {e}")))?;

        // Ensure source node exists
        if !inner.symbol_to_node.contains_key(&edge.source_id) {
            let idx = inner.graph.add_node(edge.source_id);
            inner.symbol_to_node.insert(edge.source_id, idx);
        }
        // Ensure target node exists
        if !inner.symbol_to_node.contains_key(&edge.target_id) {
            let idx = inner.graph.add_node(edge.target_id);
            inner.symbol_to_node.insert(edge.target_id, idx);
        }

        let source = inner.symbol_to_node[&edge.source_id];
        let target = inner.symbol_to_node[&edge.target_id];

        inner.graph.add_edge(source, target, edge.kind);
        Ok(())
    }

    /// Get all symbols that the given symbol depends on (upstream / outgoing edges).
    /// BFS traversal up to `depth` hops.
    pub fn upstream(&self, symbol_id: i64, depth: usize) -> OmniResult<Vec<i64>> {
        let inner = self
            .inner
            .read()
            .map_err(|e| crate::error::OmniError::Internal(format!("graph lock poisoned: {e}")))?;

        let Some(&node) = inner.symbol_to_node.get(&symbol_id) else {
            return Ok(Vec::new());
        };

        // BFS along outgoing edges (what this symbol depends on)
        bfs_collect(&inner.graph, node, depth, Direction::Outgoing)
    }

    /// Get all symbols that depend on the given symbol (downstream / incoming edges).
    /// BFS traversal up to `depth` hops.
    pub fn downstream(&self, symbol_id: i64, depth: usize) -> OmniResult<Vec<i64>> {
        let inner = self
            .inner
            .read()
            .map_err(|e| crate::error::OmniError::Internal(format!("graph lock poisoned: {e}")))?;

        let Some(&node) = inner.symbol_to_node.get(&symbol_id) else {
            return Ok(Vec::new());
        };

        // BFS along incoming edges (what depends on this symbol)
        bfs_collect(&inner.graph, node, depth, Direction::Incoming)
    }

    /// Check if the dependency graph has any cycles.
    pub fn has_cycles(&self) -> bool {
        self.inner
            .read()
            .map(|inner| is_cyclic_directed(&inner.graph))
            .unwrap_or(false)
    }

    /// Find all strongly connected components with more than one node (cycles).
    /// Returns groups of symbol IDs that form circular dependencies.
    pub fn find_cycles(&self) -> OmniResult<Vec<Vec<i64>>> {
        let inner = self
            .inner
            .read()
            .map_err(|e| crate::error::OmniError::Internal(format!("graph lock poisoned: {e}")))?;

        let sccs = petgraph::algo::tarjan_scc(&inner.graph);
        let cycles: Vec<Vec<i64>> = sccs
            .into_iter()
            .filter(|scc| scc.len() > 1)
            .map(|scc| scc.into_iter().map(|n| inner.graph[n]).collect())
            .collect();

        Ok(cycles)
    }

    /// Compute the shortest graph distance between two symbols.
    /// Returns None if they are not connected.
    pub fn distance(&self, from: i64, to: i64) -> OmniResult<Option<usize>> {
        let inner = self
            .inner
            .read()
            .map_err(|e| crate::error::OmniError::Internal(format!("graph lock poisoned: {e}")))?;

        let (Some(&from_node), Some(&to_node)) = (
            inner.symbol_to_node.get(&from),
            inner.symbol_to_node.get(&to),
        ) else {
            return Ok(None);
        };

        // BFS to find shortest path (unweighted)
        use std::collections::VecDeque;
        let mut visited = HashMap::new();
        let mut queue = VecDeque::new();
        visited.insert(from_node, 0usize);
        queue.push_back(from_node);

        while let Some(current) = queue.pop_front() {
            let dist = visited[&current];

            if current == to_node {
                return Ok(Some(dist));
            }

            // Check both directions (undirected distance)
            for direction in [Direction::Outgoing, Direction::Incoming] {
                for neighbor in inner.graph.neighbors_directed(current, direction) {
                    if !visited.contains_key(&neighbor) {
                        visited.insert(neighbor, dist + 1);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        Ok(None)
    }

    /// Returns the total number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.inner.read().map(|i| i.graph.node_count()).unwrap_or(0)
    }

    /// Returns the total number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.inner.read().map(|i| i.graph.edge_count()).unwrap_or(0)
    }

    /// Clear the entire graph.
    pub fn clear(&self) {
        if let Ok(mut inner) = self.inner.write() {
            inner.graph.clear();
            inner.symbol_to_node.clear();
        }
    }

    /// Get the in-degree (number of incoming edges) for a symbol.
    ///
    /// High in-degree means many other symbols depend on this one --
    /// it is structurally important (e.g., a core utility function).
    pub fn in_degree(&self, symbol_id: i64) -> usize {
        self.inner
            .read()
            .ok()
            .and_then(|inner| {
                inner.symbol_to_node.get(&symbol_id).map(|&node| {
                    inner
                        .graph
                        .neighbors_directed(node, Direction::Incoming)
                        .count()
                })
            })
            .unwrap_or(0)
    }

    /// Compute the blast radius for a given symbol.
    ///
    /// Returns all symbol IDs that would be transitively affected if the given
    /// symbol changes. This is the full downstream transitive closure -- it
    /// answers "what breaks if I modify this?"
    ///
    /// The result is sorted by distance from the source (closest first).
    /// Each entry is `(symbol_id, distance)`.
    pub fn blast_radius(&self, symbol_id: i64, max_depth: usize) -> OmniResult<Vec<(i64, usize)>> {
        let inner = self
            .inner
            .read()
            .map_err(|e| crate::error::OmniError::Internal(format!("graph lock poisoned: {e}")))?;

        let Some(&node) = inner.symbol_to_node.get(&symbol_id) else {
            return Ok(Vec::new());
        };

        // BFS along incoming edges (what depends on this symbol) with depth tracking
        use std::collections::VecDeque;
        let mut visited: HashMap<NodeIndex, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        visited.insert(node, 0);
        queue.push_back((node, 0usize));

        let mut results = Vec::new();

        while let Some((current, dist)) = queue.pop_front() {
            if dist >= max_depth {
                continue;
            }
            let next_dist = dist + 1;
            for neighbor in inner.graph.neighbors_directed(current, Direction::Incoming) {
                if !visited.contains_key(&neighbor) {
                    visited.insert(neighbor, next_dist);
                    let sym_id = inner.graph[neighbor];
                    results.push((sym_id, next_dist));
                    queue.push_back((neighbor, next_dist));
                }
            }
        }

        // Sort by distance ascending (closest affected first)
        results.sort_by_key(|&(_, d)| d);
        Ok(results)
    }

    /// Get all typed edges for a specific symbol.
    ///
    /// Returns `(target_symbol_id, edge_kind, direction_label)` tuples.
    /// direction_label is "outgoing" or "incoming".
    /// Used by the call graph MCP tool.
    pub fn get_edges_for_symbol(
        &self,
        symbol_id: i64,
    ) -> OmniResult<Vec<(i64, DependencyKind, &'static str)>> {
        let inner = self
            .inner
            .read()
            .map_err(|e| crate::error::OmniError::Internal(format!("graph lock poisoned: {e}")))?;

        let Some(&node) = inner.symbol_to_node.get(&symbol_id) else {
            return Ok(Vec::new());
        };

        let mut edges = Vec::new();

        // Outgoing edges (what this symbol depends on / calls)
        for neighbor in inner.graph.neighbors_directed(node, Direction::Outgoing) {
            if let Some(edge_idx) = inner.graph.find_edge(node, neighbor) {
                let kind = inner.graph[edge_idx];
                let target_id = inner.graph[neighbor];
                edges.push((target_id, kind, "outgoing"));
            }
        }

        // Incoming edges (what depends on / calls this symbol)
        for neighbor in inner.graph.neighbors_directed(node, Direction::Incoming) {
            if let Some(edge_idx) = inner.graph.find_edge(neighbor, node) {
                let kind = inner.graph[edge_idx];
                let source_id = inner.graph[neighbor];
                edges.push((source_id, kind, "incoming"));
            }
        }

        Ok(edges)
    }

    /// Resolve an import statement to a target symbol ID.
    ///
    /// Multi-strategy resolution:
    /// 1. Exact FQN match (e.g., `crate::config::Config`)
    /// 2. FQN suffix match (e.g., `config::Config` matches `crate::config::Config`)
    /// 3. Name-only fallback with shortest FQN preference
    ///
    /// Returns `None` if the import cannot be resolved.
    pub fn resolve_import(
        index: &crate::index::MetadataIndex,
        import_path: &str,
        imported_name: &str,
    ) -> Option<i64> {
        // Strategy 1: Exact FQN match
        // Try: import_path::imported_name (e.g., "crate::config" + "Config" -> "crate::config::Config")
        let fqn_candidate = if import_path.is_empty() {
            imported_name.to_string()
        } else {
            format!("{import_path}::{imported_name}")
        };

        if let Ok(Some(sym)) = index.get_symbol_by_fqn(&fqn_candidate) {
            return Some(sym.id);
        }

        // Also try with dot separator (Python/TS style)
        let fqn_dot = if import_path.is_empty() {
            imported_name.to_string()
        } else {
            format!("{import_path}.{imported_name}")
        };
        if let Ok(Some(sym)) = index.get_symbol_by_fqn(&fqn_dot) {
            return Some(sym.id);
        }

        // Strategy 2: FQN suffix match
        // Try matching any symbol whose FQN ends with the import path
        let suffix = if imported_name.is_empty() {
            import_path.to_string()
        } else {
            format!("::{imported_name}")
        };
        if let Ok(matches) = index.search_symbols_by_fqn_suffix(&suffix, 5) {
            if matches.len() == 1 {
                return Some(matches[0].id);
            }
            // If multiple matches, prefer the one whose FQN contains the import path
            if !import_path.is_empty() {
                for m in &matches {
                    if m.fqn.contains(import_path) {
                        return Some(m.id);
                    }
                }
            }
            // Fall through to name-only if ambiguous
            if !matches.is_empty() {
                return Some(matches[0].id);
            }
        }

        // Strategy 3: Name-only fallback (shortest FQN wins)
        if let Ok(matches) = index.search_symbols_by_name(imported_name, 5) {
            if !matches.is_empty() {
                return Some(matches[0].id);
            }
        }

        None
    }

    /// Build call graph edges from element references.
    ///
    /// For each symbol in the file, resolve its `references` to target symbols
    /// and add `Calls` edges to the graph.
    pub fn build_call_edges(
        &self,
        index: &crate::index::MetadataIndex,
        file_id: i64,
        elements: &[crate::parser::StructuralElement],
    ) -> Vec<DependencyEdge> {
        let mut edges = Vec::new();

        // Get all symbols in this file
        let file_symbols = match index.get_all_symbols_for_file(file_id) {
            Ok(s) => s,
            Err(_) => return edges,
        };

        // Build a map from element name -> symbol_id for this file
        let mut name_to_symbol: HashMap<String, i64> = HashMap::new();
        for sym in &file_symbols {
            name_to_symbol.insert(sym.name.clone(), sym.id);
        }

        // For each element with references, try to resolve the references
        for elem in elements {
            if elem.references.is_empty() {
                continue;
            }

            // Find the symbol_id for this element
            let source_id = match name_to_symbol.get(&elem.name) {
                Some(&id) => id,
                None => continue,
            };

            for ref_name in &elem.references {
                // Skip self-references
                if ref_name == &elem.name {
                    continue;
                }

                // Try to resolve: first check local file symbols, then global
                let target_id = if let Some(&local_id) = name_to_symbol.get(ref_name) {
                    if local_id != source_id {
                        Some(local_id)
                    } else {
                        None
                    }
                } else {
                    // Global resolution via index
                    index
                        .search_symbols_by_name(ref_name, 1)
                        .ok()
                        .and_then(|v| v.into_iter().next())
                        .map(|s| s.id)
                };

                if let Some(target) = target_id {
                    let edge = DependencyEdge {
                        source_id,
                        target_id: target,
                        kind: DependencyKind::Calls,
                    };
                    edges.push(edge.clone());
                    let _ = self.add_edge(&edge);
                }
            }
        }

        edges
    }

    /// Build type hierarchy edges (Extends, Implements) from element structure.
    pub fn build_type_edges(
        &self,
        index: &crate::index::MetadataIndex,
        file_id: i64,
        elements: &[crate::parser::StructuralElement],
    ) -> Vec<DependencyEdge> {
        let mut edges = Vec::new();

        let file_symbols = match index.get_all_symbols_for_file(file_id) {
            Ok(s) => s,
            Err(_) => return edges,
        };

        let mut name_to_symbol: HashMap<String, i64> = HashMap::new();
        for sym in &file_symbols {
            name_to_symbol.insert(sym.name.clone(), sym.id);
        }

        for elem in elements {
            if elem.extends.is_empty() && elem.implements.is_empty() {
                continue;
            }

            let source_id = match name_to_symbol.get(&elem.name) {
                Some(&id) => id,
                None => continue,
            };

            for type_name in &elem.extends {
                let target_id = name_to_symbol.get(type_name).copied().or_else(|| {
                    index
                        .search_symbols_by_name(type_name, 1)
                        .ok()
                        .and_then(|v| v.into_iter().next())
                        .map(|s| s.id)
                });

                if let Some(target) = target_id {
                    let edge = DependencyEdge {
                        source_id,
                        target_id: target,
                        kind: DependencyKind::Extends,
                    };
                    edges.push(edge.clone());
                    let _ = self.add_edge(&edge);
                }
            }

            for type_name in &elem.implements {
                let target_id = name_to_symbol.get(type_name).copied().or_else(|| {
                    index
                        .search_symbols_by_name(type_name, 1)
                        .ok()
                        .and_then(|v| v.into_iter().next())
                        .map(|s| s.id)
                });

                if let Some(target) = target_id {
                    let edge = DependencyEdge {
                        source_id,
                        target_id: target,
                        kind: DependencyKind::Implements,
                    };
                    edges.push(edge.clone());
                    let _ = self.add_edge(&edge);
                }
            }
        }

        edges
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// BFS helper: collect symbol IDs reachable within `depth` hops.
fn bfs_collect(
    graph: &DiGraph<i64, DependencyKind>,
    start: NodeIndex,
    depth: usize,
    direction: Direction,
) -> OmniResult<Vec<i64>> {
    let mut visited = Vec::new();
    let mut frontier = vec![start];

    for _ in 0..depth {
        let mut next_frontier = Vec::new();
        for &n in &frontier {
            for neighbor in graph.neighbors_directed(n, direction) {
                let sym_id = graph[neighbor];
                if !visited.contains(&sym_id) {
                    visited.push(sym_id);
                    next_frontier.push(neighbor);
                }
            }
        }
        frontier = next_frontier;
    }

    Ok(visited)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_query_dependencies() {
        let graph = DependencyGraph::new();
        graph.add_symbol(1).expect("add symbol 1");
        graph.add_symbol(2).expect("add symbol 2");
        graph
            .add_edge(&DependencyEdge {
                source_id: 1,
                target_id: 2,
                kind: DependencyKind::Calls,
            })
            .expect("add edge");

        let upstream = graph.upstream(1, 1).expect("query upstream");
        assert_eq!(upstream, vec![2]);
    }

    #[test]
    fn test_upstream_unknown_symbol() {
        let graph = DependencyGraph::new();
        let upstream = graph.upstream(999, 1).expect("query unknown");
        assert!(upstream.is_empty());
    }

    #[test]
    fn test_downstream_dependencies() {
        let graph = DependencyGraph::new();
        graph
            .add_edge(&DependencyEdge {
                source_id: 1,
                target_id: 2,
                kind: DependencyKind::Calls,
            })
            .expect("add edge 1->2");
        graph
            .add_edge(&DependencyEdge {
                source_id: 3,
                target_id: 2,
                kind: DependencyKind::Imports,
            })
            .expect("add edge 3->2");

        let downstream = graph.downstream(2, 1).expect("downstream of 2");
        assert_eq!(downstream.len(), 2);
        assert!(downstream.contains(&1));
        assert!(downstream.contains(&3));
    }

    #[test]
    fn test_cycle_detection() {
        let graph = DependencyGraph::new();
        graph
            .add_edge(&DependencyEdge {
                source_id: 1,
                target_id: 2,
                kind: DependencyKind::Imports,
            })
            .expect("edge");
        graph
            .add_edge(&DependencyEdge {
                source_id: 2,
                target_id: 3,
                kind: DependencyKind::Imports,
            })
            .expect("edge");
        graph
            .add_edge(&DependencyEdge {
                source_id: 3,
                target_id: 1,
                kind: DependencyKind::Imports,
            })
            .expect("edge");

        assert!(graph.has_cycles());
        let cycles = graph.find_cycles().expect("find cycles");
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn test_no_cycles() {
        let graph = DependencyGraph::new();
        graph
            .add_edge(&DependencyEdge {
                source_id: 1,
                target_id: 2,
                kind: DependencyKind::Imports,
            })
            .expect("edge");
        graph
            .add_edge(&DependencyEdge {
                source_id: 2,
                target_id: 3,
                kind: DependencyKind::Imports,
            })
            .expect("edge");

        assert!(!graph.has_cycles());
        let cycles = graph.find_cycles().expect("find cycles");
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_distance() {
        let graph = DependencyGraph::new();
        graph
            .add_edge(&DependencyEdge {
                source_id: 1,
                target_id: 2,
                kind: DependencyKind::Calls,
            })
            .expect("edge");
        graph
            .add_edge(&DependencyEdge {
                source_id: 2,
                target_id: 3,
                kind: DependencyKind::Calls,
            })
            .expect("edge");

        assert_eq!(graph.distance(1, 3).expect("dist"), Some(2));
        assert_eq!(graph.distance(1, 2).expect("dist"), Some(1));
        assert_eq!(graph.distance(1, 99).expect("dist"), None);
    }

    #[test]
    fn test_blast_radius() {
        // Build: 1 -> 2 -> 3, and 4 -> 2
        // Blast radius of 2 = {1, 4} at depth 1
        // Blast radius of 3 = {2} at depth 1, {1, 4} at depth 2
        let graph = DependencyGraph::new();
        graph
            .add_edge(&DependencyEdge {
                source_id: 1,
                target_id: 2,
                kind: DependencyKind::Calls,
            })
            .expect("edge");
        graph
            .add_edge(&DependencyEdge {
                source_id: 2,
                target_id: 3,
                kind: DependencyKind::Calls,
            })
            .expect("edge");
        graph
            .add_edge(&DependencyEdge {
                source_id: 4,
                target_id: 2,
                kind: DependencyKind::Imports,
            })
            .expect("edge");

        let radius = graph.blast_radius(2, 5).expect("blast radius");
        assert_eq!(radius.len(), 2);
        let ids: Vec<i64> = radius.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&4));
        // All at depth 1
        assert!(radius.iter().all(|(_, d)| *d == 1));

        // Blast radius of 3 (depth 2 should reach 1 and 4)
        let radius3 = graph.blast_radius(3, 5).expect("blast radius");
        assert_eq!(radius3.len(), 3); // 2 at depth 1, then 1 and 4 at depth 2
        assert_eq!(radius3[0].1, 1); // First result at depth 1

        // Blast radius with max_depth=1 should only get depth 1
        let radius_shallow = graph.blast_radius(3, 1).expect("shallow blast");
        assert_eq!(radius_shallow.len(), 1);
        assert_eq!(radius_shallow[0].0, 2);
    }

    #[test]
    fn test_get_edges_for_symbol() {
        let graph = DependencyGraph::new();
        graph
            .add_edge(&DependencyEdge {
                source_id: 1,
                target_id: 2,
                kind: DependencyKind::Calls,
            })
            .expect("edge");
        graph
            .add_edge(&DependencyEdge {
                source_id: 3,
                target_id: 1,
                kind: DependencyKind::Imports,
            })
            .expect("edge");

        let edges = graph.get_edges_for_symbol(1).expect("edges");
        assert_eq!(edges.len(), 2);

        // Should have one outgoing (1->2) and one incoming (3->1)
        let outgoing: Vec<_> = edges.iter().filter(|(_, _, d)| *d == "outgoing").collect();
        let incoming: Vec<_> = edges.iter().filter(|(_, _, d)| *d == "incoming").collect();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(incoming.len(), 1);
        assert_eq!(outgoing[0].0, 2); // calls symbol 2
        assert_eq!(incoming[0].0, 3); // symbol 3 imports us
    }

    #[test]
    fn test_blast_radius_empty() {
        let graph = DependencyGraph::new();
        graph.add_symbol(1).expect("add");
        let radius = graph.blast_radius(1, 5).expect("blast radius");
        assert!(radius.is_empty());

        let radius_unknown = graph.blast_radius(999, 5).expect("blast radius");
        assert!(radius_unknown.is_empty());
    }
}
