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

use crate::types::{DependencyEdge, DependencyKind};
use crate::error::OmniResult;

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
        let mut inner = self.inner.write().map_err(|e| {
            crate::error::OmniError::Internal(format!("graph lock poisoned: {e}"))
        })?;

        if !inner.symbol_to_node.contains_key(&symbol_id) {
            let idx = inner.graph.add_node(symbol_id);
            inner.symbol_to_node.insert(symbol_id, idx);
        }

        Ok(())
    }

    /// Add a dependency edge between two symbols.
    pub fn add_edge(&self, edge: &DependencyEdge) -> OmniResult<()> {
        let mut inner = self.inner.write().map_err(|e| {
            crate::error::OmniError::Internal(format!("graph lock poisoned: {e}"))
        })?;

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
        let inner = self.inner.read().map_err(|e| {
            crate::error::OmniError::Internal(format!("graph lock poisoned: {e}"))
        })?;

        let Some(&node) = inner.symbol_to_node.get(&symbol_id) else {
            return Ok(Vec::new());
        };

        // BFS along outgoing edges (what this symbol depends on)
        bfs_collect(&inner.graph, node, depth, Direction::Outgoing)
    }

    /// Get all symbols that depend on the given symbol (downstream / incoming edges).
    /// BFS traversal up to `depth` hops.
    pub fn downstream(&self, symbol_id: i64, depth: usize) -> OmniResult<Vec<i64>> {
        let inner = self.inner.read().map_err(|e| {
            crate::error::OmniError::Internal(format!("graph lock poisoned: {e}"))
        })?;

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
        let inner = self.inner.read().map_err(|e| {
            crate::error::OmniError::Internal(format!("graph lock poisoned: {e}"))
        })?;

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
        let inner = self.inner.read().map_err(|e| {
            crate::error::OmniError::Internal(format!("graph lock poisoned: {e}"))
        })?;

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
        graph.add_edge(&DependencyEdge {
            source_id: 1,
            target_id: 2,
            kind: DependencyKind::Calls,
        }).expect("add edge");

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
        graph.add_edge(&DependencyEdge {
            source_id: 1,
            target_id: 2,
            kind: DependencyKind::Calls,
        }).expect("add edge 1->2");
        graph.add_edge(&DependencyEdge {
            source_id: 3,
            target_id: 2,
            kind: DependencyKind::Imports,
        }).expect("add edge 3->2");

        let downstream = graph.downstream(2, 1).expect("downstream of 2");
        assert_eq!(downstream.len(), 2);
        assert!(downstream.contains(&1));
        assert!(downstream.contains(&3));
    }

    #[test]
    fn test_cycle_detection() {
        let graph = DependencyGraph::new();
        graph.add_edge(&DependencyEdge {
            source_id: 1,
            target_id: 2,
            kind: DependencyKind::Imports,
        }).expect("edge");
        graph.add_edge(&DependencyEdge {
            source_id: 2,
            target_id: 3,
            kind: DependencyKind::Imports,
        }).expect("edge");
        graph.add_edge(&DependencyEdge {
            source_id: 3,
            target_id: 1,
            kind: DependencyKind::Imports,
        }).expect("edge");

        assert!(graph.has_cycles());
        let cycles = graph.find_cycles().expect("find cycles");
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn test_no_cycles() {
        let graph = DependencyGraph::new();
        graph.add_edge(&DependencyEdge {
            source_id: 1,
            target_id: 2,
            kind: DependencyKind::Imports,
        }).expect("edge");
        graph.add_edge(&DependencyEdge {
            source_id: 2,
            target_id: 3,
            kind: DependencyKind::Imports,
        }).expect("edge");

        assert!(!graph.has_cycles());
        let cycles = graph.find_cycles().expect("find cycles");
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_distance() {
        let graph = DependencyGraph::new();
        graph.add_edge(&DependencyEdge {
            source_id: 1,
            target_id: 2,
            kind: DependencyKind::Calls,
        }).expect("edge");
        graph.add_edge(&DependencyEdge {
            source_id: 2,
            target_id: 3,
            kind: DependencyKind::Calls,
        }).expect("edge");

        assert_eq!(graph.distance(1, 3).expect("dist"), Some(2));
        assert_eq!(graph.distance(1, 2).expect("dist"), Some(1));
        assert_eq!(graph.distance(1, 99).expect("dist"), None);
    }
}
