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

use petgraph::graph::{DiGraph, NodeIndex};
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

    /// Get all symbols that the given symbol depends on (upstream).
    pub fn upstream(&self, symbol_id: i64, depth: usize) -> OmniResult<Vec<i64>> {
        let inner = self.inner.read().map_err(|e| {
            crate::error::OmniError::Internal(format!("graph lock poisoned: {e}"))
        })?;

        let Some(&node) = inner.symbol_to_node.get(&symbol_id) else {
            return Ok(Vec::new());
        };

        // BFS up to `depth` hops along outgoing edges
        let mut visited = Vec::new();
        let mut frontier = vec![node];

        for _ in 0..depth {
            let mut next_frontier = Vec::new();
            for n in &frontier {
                for neighbor in inner.graph.neighbors(*n) {
                    let sym_id = inner.graph[neighbor];
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

    /// Returns the total number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.inner.read().map(|i| i.graph.node_count()).unwrap_or(0)
    }

    /// Returns the total number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.inner.read().map(|i| i.graph.edge_count()).unwrap_or(0)
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
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
}
