//! File-level dependency graph with AST-derived structural edges.
//!
//! This module implements the architectural dependency graph as described in
//! the CodeCompass research (2026). It tracks file-to-file relationships
//! derived from AST analysis:
//!
//! - IMPORTS: File A imports from file B
//! - INHERITS: Class in A inherits from class in B
//! - CALLS: Function in A calls function in B
//! - INSTANTIATES: File A creates instance of class from B
//!
//! ## Performance Target
//! - 1-hop queries: <10ms
//! - N-hop queries: <50ms for N<=3
//!
//! ## Expected Impact
//! - 23.2% improvement on architectural tasks (per CodeCompass research)
//! - Enables graph-based navigation for AI agents
//! - Complements semantic search with structural understanding

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::error::{OmniError, OmniResult};

/// Type of dependency edge between files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeType {
    /// File A imports from file B (import/require/use statement)
    Imports,
    /// Class in A inherits from class in B (extends/implements)
    Inherits,
    /// Function in A calls function in B (function call)
    Calls,
    /// File A creates instance of class from B (new/instantiation)
    Instantiates,
    /// Files A and B frequently change together (historical co-change)
    HistoricalCoChange,
}

impl EdgeType {
    /// Convert edge type to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeType::Imports => "imports",
            EdgeType::Inherits => "inherits",
            EdgeType::Calls => "calls",
            EdgeType::Instantiates => "instantiates",
            EdgeType::HistoricalCoChange => "historical_co_change",
        }
    }

    /// Parse edge type from string representation.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "imports" => Some(EdgeType::Imports),
            "inherits" => Some(EdgeType::Inherits),
            "calls" => Some(EdgeType::Calls),
            "instantiates" => Some(EdgeType::Instantiates),
            "historical_co_change" => Some(EdgeType::HistoricalCoChange),
            _ => None,
        }
    }
}

/// A directed edge in the file dependency graph.
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// Source file path.
    pub source: PathBuf,
    /// Target file path.
    pub target: PathBuf,
    /// Type of dependency relationship.
    pub edge_type: EdgeType,
    /// Edge weight (importance score).
    pub weight: f32,
}

/// File node in the dependency graph.
#[derive(Debug, Clone)]
pub struct FileNode {
    /// File path.
    pub path: PathBuf,
    /// Programming language.
    pub language: String,
    /// Importance score computed from in-degree and PageRank.
    pub importance: f32,
}

/// Architectural context for a file: all structurally connected files.
#[derive(Debug, Clone)]
pub struct ArchitecturalContext {
    /// The focal file being analyzed.
    pub focal_file: PathBuf,
    /// Neighboring files with dependency relationships.
    pub neighbors: Vec<NeighborFile>,
    /// Total number of files in the graph.
    pub total_files: usize,
    /// Maximum hops used for neighbor discovery.
    pub max_hops: usize,
}

/// A neighboring file in the architectural context.
#[derive(Debug, Clone)]
pub struct NeighborFile {
    /// File path.
    pub path: PathBuf,
    /// Distance in hops from focal file.
    pub distance: usize,
    /// Types of dependency edges connecting to this file.
    pub edge_types: Vec<EdgeType>,
    /// Importance score.
    pub importance: f32,
}

/// Thread-safe file-level dependency graph.
pub struct FileDependencyGraph {
    inner: RwLock<GraphInner>,
}

struct GraphInner {
    /// File nodes indexed by path
    nodes: HashMap<PathBuf, FileNode>,
    /// Adjacency list: source -> [(target, edge_type, weight)]
    outgoing: HashMap<PathBuf, Vec<(PathBuf, EdgeType, f32)>>,
    /// Reverse adjacency list: target -> [(source, edge_type, weight)]
    incoming: HashMap<PathBuf, Vec<(PathBuf, EdgeType, f32)>>,
}

impl FileDependencyGraph {
    /// Create a new empty file dependency graph.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(GraphInner {
                nodes: HashMap::new(),
                outgoing: HashMap::new(),
                incoming: HashMap::new(),
            }),
        }
    }

    /// Add a file node to the graph.
    pub fn add_file(&self, path: PathBuf, language: String) -> OmniResult<()> {
        let mut inner = self.inner.write().map_err(|e| {
            OmniError::Internal(format!("file dependency graph lock poisoned: {e}"))
        })?;

        inner.nodes.entry(path.clone()).or_insert(FileNode {
            path,
            language,
            importance: 1.0,
        });

        Ok(())
    }

    /// Add a dependency edge between two files.
    pub fn add_edge(&self, edge: &DependencyEdge) -> OmniResult<()> {
        let mut inner = self.inner.write().map_err(|e| {
            OmniError::Internal(format!("file dependency graph lock poisoned: {e}"))
        })?;

        // Ensure both nodes exist
        inner
            .nodes
            .entry(edge.source.clone())
            .or_insert_with(|| FileNode {
                path: edge.source.clone(),
                language: String::new(),
                importance: 1.0,
            });
        inner
            .nodes
            .entry(edge.target.clone())
            .or_insert_with(|| FileNode {
                path: edge.target.clone(),
                language: String::new(),
                importance: 1.0,
            });

        // Add to outgoing adjacency list
        inner
            .outgoing
            .entry(edge.source.clone())
            .or_default()
            .push((edge.target.clone(), edge.edge_type, edge.weight));

        // Add to incoming adjacency list
        inner
            .incoming
            .entry(edge.target.clone())
            .or_default()
            .push((edge.source.clone(), edge.edge_type, edge.weight));

        Ok(())
    }

    /// Get N-hop neighborhood of a file.
    ///
    /// Returns all files within `max_hops` of the focal file, with their
    /// distances and edge types.
    ///
    /// Performance target: <10ms for 1-hop, <50ms for 3-hop.
    ///
    /// # Panics
    /// May panic if importance scores contain NaN values during sorting.
    pub fn get_neighbors(&self, file: &Path, max_hops: usize) -> OmniResult<Vec<NeighborFile>> {
        let inner = self.inner.read().map_err(|e| {
            OmniError::Internal(format!("file dependency graph lock poisoned: {e}"))
        })?;

        let file_path = file.to_path_buf();

        if !inner.nodes.contains_key(&file_path) {
            return Ok(Vec::new());
        }

        // BFS to find all neighbors within max_hops
        let mut visited: HashMap<PathBuf, (usize, Vec<EdgeType>)> = HashMap::new();
        let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();

        visited.insert(file_path.clone(), (0, Vec::new()));
        queue.push_back((file_path.clone(), 0));

        while let Some((current, dist)) = queue.pop_front() {
            if dist >= max_hops {
                continue;
            }

            let next_dist = dist + 1;

            // Explore outgoing edges
            if let Some(neighbors) = inner.outgoing.get(&current) {
                for (neighbor, edge_type, _weight) in neighbors {
                    visited
                        .entry(neighbor.clone())
                        .and_modify(|(_, types)| {
                            if !types.contains(edge_type) {
                                types.push(*edge_type);
                            }
                        })
                        .or_insert_with(|| {
                            queue.push_back((neighbor.clone(), next_dist));
                            (next_dist, vec![*edge_type])
                        });
                }
            }

            // Explore incoming edges
            if let Some(neighbors) = inner.incoming.get(&current) {
                for (neighbor, edge_type, _weight) in neighbors {
                    visited
                        .entry(neighbor.clone())
                        .and_modify(|(_, types)| {
                            if !types.contains(edge_type) {
                                types.push(*edge_type);
                            }
                        })
                        .or_insert_with(|| {
                            queue.push_back((neighbor.clone(), next_dist));
                            (next_dist, vec![*edge_type])
                        });
                }
            }
        }

        // Convert to NeighborFile structs (exclude focal file)
        let mut neighbors: Vec<NeighborFile> = visited
            .into_iter()
            .filter(|(path, _)| path != &file_path)
            .map(|(path, (distance, edge_types))| {
                let importance = inner.nodes.get(&path).map(|n| n.importance).unwrap_or(1.0);
                NeighborFile {
                    path,
                    distance,
                    edge_types,
                    importance,
                }
            })
            .collect();

        // Sort by distance (closest first), then by importance (highest first)
        neighbors.sort_by(|a, b| {
            a.distance.cmp(&b.distance).then_with(|| {
                b.importance
                    .partial_cmp(&a.importance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        Ok(neighbors)
    }

    /// Get architectural context for a file.
    ///
    /// Returns all structurally connected files with edge types and distances.
    /// This is the primary API for the `get_architectural_context` MCP tool.
    pub fn get_architectural_context(
        &self,
        file: &Path,
        max_hops: Option<usize>,
    ) -> OmniResult<ArchitecturalContext> {
        let max_hops = max_hops.unwrap_or(2); // Default: 2-hop neighborhood
        let neighbors = self.get_neighbors(file, max_hops)?;

        let inner = self.inner.read().map_err(|e| {
            OmniError::Internal(format!("file dependency graph lock poisoned: {e}"))
        })?;

        Ok(ArchitecturalContext {
            focal_file: file.to_path_buf(),
            neighbors,
            total_files: inner.nodes.len(),
            max_hops,
        })
    }

    /// Compute importance scores for all files using PageRank-style algorithm.
    ///
    /// Files with high in-degree (many files depend on them) get higher scores.
    /// This is used to prioritize architecturally important files in search results.
    pub fn compute_importance(&self) -> OmniResult<()> {
        let mut inner = self.inner.write().map_err(|e| {
            OmniError::Internal(format!("file dependency graph lock poisoned: {e}"))
        })?;

        let num_nodes = inner.nodes.len();
        if num_nodes == 0 {
            return Ok(());
        }

        // Initialize all nodes with equal importance
        let initial_score = 1.0 / num_nodes as f32;
        for node in inner.nodes.values_mut() {
            node.importance = initial_score;
        }

        // PageRank iterations (10 iterations is sufficient)
        let damping = 0.85;
        for _ in 0..10 {
            let mut new_scores: HashMap<PathBuf, f32> = HashMap::new();

            for path in inner.nodes.keys() {
                let mut score = (1.0 - damping) / num_nodes as f32;

                // Add contributions from incoming edges
                if let Some(incoming) = inner.incoming.get(path) {
                    for (source, _, _) in incoming {
                        if let Some(source_node) = inner.nodes.get(source) {
                            let out_degree =
                                inner.outgoing.get(source).map(|v| v.len()).unwrap_or(1);
                            score += damping * source_node.importance / out_degree as f32;
                        }
                    }
                }

                new_scores.insert(path.clone(), score);
            }

            // Update scores
            for (path, score) in new_scores {
                if let Some(node) = inner.nodes.get_mut(&path) {
                    node.importance = score;
                }
            }
        }

        Ok(())
    }

    /// Get the number of files in the graph.
    pub fn node_count(&self) -> usize {
        self.inner
            .read()
            .map(|inner| inner.nodes.len())
            .unwrap_or(0)
    }

    /// Get the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.inner
            .read()
            .map(|inner| inner.outgoing.values().map(|v| v.len()).sum())
            .unwrap_or(0)
    }

    /// Clear the entire graph.
    pub fn clear(&self) {
        if let Ok(mut inner) = self.inner.write() {
            inner.nodes.clear();
            inner.outgoing.clear();
            inner.incoming.clear();
        }
    }

    /// Return a snapshot of all (path, importance) pairs after `compute_importance()` has run.
    ///
    /// Acquires a single read lock and clones only the fields needed by the caller.
    /// Returns an empty vec if the lock is poisoned.
    pub fn all_nodes_with_importance(&self) -> Vec<(PathBuf, f32)> {
        self.inner
            .read()
            .map(|g| {
                g.nodes
                    .values()
                    .map(|n| (n.path.clone(), n.importance))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Return all outgoing edges whose source matches `path`.
    ///
    /// Used by the pipeline to snapshot per-file edges for SQLite persistence
    /// immediately after a file is re-indexed.  Acquires a single read lock.
    pub fn outgoing_edges_for(&self, path: &std::path::Path) -> Vec<DependencyEdge> {
        let path_buf = path.to_path_buf();
        self.inner
            .read()
            .map(|g| {
                g.outgoing
                    .get(&path_buf)
                    .map(|edges| {
                        edges
                            .iter()
                            .map(|(target, edge_type, weight)| DependencyEdge {
                                source: path_buf.clone(),
                                target: target.clone(),
                                edge_type: *edge_type,
                                weight: *weight,
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Count edges grouped by edge type.
    ///
    /// Returns a map of `EdgeType -> count` covering all five edge categories.
    /// This is the authoritative source for `graph/get_metrics` IPC data.
    pub fn count_by_edge_type(&self) -> std::collections::HashMap<EdgeType, usize> {
        let mut counts: std::collections::HashMap<EdgeType, usize> =
            std::collections::HashMap::new();
        // Pre-seed all variants so callers always get a complete map
        for et in [
            EdgeType::Imports,
            EdgeType::Inherits,
            EdgeType::Calls,
            EdgeType::Instantiates,
            EdgeType::HistoricalCoChange,
        ] {
            counts.insert(et, 0);
        }

        if let Ok(inner) = self.inner.read() {
            for edges in inner.outgoing.values() {
                for (_, edge_type, _) in edges {
                    *counts.entry(*edge_type).or_insert(0) += 1;
                }
            }
        }

        counts
    }

    /// Snapshot the adjacency list for offline graph algorithms.
    ///
    /// Returns a map of `source → Vec<target>` using only the `Imports`,
    /// `Inherits`, `Calls`, and `Instantiates` edge types (structural edges).
    /// `HistoricalCoChange` edges are excluded because they are undirected
    /// in practice and would produce spurious cycles.
    pub fn snapshot_structural_adjacency(&self) -> HashMap<PathBuf, Vec<PathBuf>> {
        let inner = match self.inner.read() {
            Ok(g) => g,
            Err(_) => return HashMap::new(),
        };

        let mut adj: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

        // Ensure every node appears as a key even if it has no outgoing edges.
        for path in inner.nodes.keys() {
            adj.entry(path.clone()).or_default();
        }

        for (src, edges) in &inner.outgoing {
            let targets = adj.entry(src.clone()).or_default();
            for (tgt, edge_type, _weight) in edges {
                if !matches!(edge_type, EdgeType::HistoricalCoChange) {
                    targets.push(tgt.clone());
                }
            }
        }

        adj
    }

    /// Return all edges across the entire graph that match `filter`.
    ///
    /// Acquires a single read lock and collects every `(source, target, weight)`
    /// triple where the edge type equals `filter`.  Used by the pipeline to
    /// snapshot `HistoricalCoChange` edges for SQLite persistence after
    /// `HistoricalGraphEnhancer::enhance_graph()` runs.
    pub fn all_edges_of_type(&self, filter: EdgeType) -> Vec<DependencyEdge> {
        self.inner
            .read()
            .map(|g| {
                let mut result = Vec::new();
                for (src, edges) in &g.outgoing {
                    for (tgt, et, w) in edges {
                        if *et == filter {
                            result.push(DependencyEdge {
                                source: src.clone(),
                                target: tgt.clone(),
                                edge_type: *et,
                                weight: *w,
                            });
                        }
                    }
                }
                result
            })
            .unwrap_or_default()
    }
}

impl Default for FileDependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_file_and_edge() {
        let graph = FileDependencyGraph::new();
        let file_a = PathBuf::from("src/a.rs");
        let file_b = PathBuf::from("src/b.rs");

        graph
            .add_file(file_a.clone(), "rust".to_string())
            .expect("add file a");
        graph
            .add_file(file_b.clone(), "rust".to_string())
            .expect("add file b");

        graph
            .add_edge(&DependencyEdge {
                source: file_a.clone(),
                target: file_b.clone(),
                edge_type: EdgeType::Imports,
                weight: 1.0,
            })
            .expect("add edge");

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_get_neighbors_1_hop() {
        let graph = FileDependencyGraph::new();
        let file_a = PathBuf::from("src/a.rs");
        let file_b = PathBuf::from("src/b.rs");
        let file_c = PathBuf::from("src/c.rs");

        graph
            .add_edge(&DependencyEdge {
                source: file_a.clone(),
                target: file_b.clone(),
                edge_type: EdgeType::Imports,
                weight: 1.0,
            })
            .expect("add edge a->b");

        graph
            .add_edge(&DependencyEdge {
                source: file_a.clone(),
                target: file_c.clone(),
                edge_type: EdgeType::Calls,
                weight: 1.0,
            })
            .expect("add edge a->c");

        let neighbors = graph.get_neighbors(&file_a, 1).expect("get neighbors");
        assert_eq!(neighbors.len(), 2);

        let paths: Vec<&PathBuf> = neighbors.iter().map(|n| &n.path).collect();
        assert!(paths.contains(&&file_b));
        assert!(paths.contains(&&file_c));

        // All should be at distance 1
        assert!(neighbors.iter().all(|n| n.distance == 1));
    }

    #[test]
    fn test_get_neighbors_2_hop() {
        let graph = FileDependencyGraph::new();
        let file_a = PathBuf::from("src/a.rs");
        let file_b = PathBuf::from("src/b.rs");
        let file_c = PathBuf::from("src/c.rs");

        graph
            .add_edge(&DependencyEdge {
                source: file_a.clone(),
                target: file_b.clone(),
                edge_type: EdgeType::Imports,
                weight: 1.0,
            })
            .expect("add edge a->b");

        graph
            .add_edge(&DependencyEdge {
                source: file_b.clone(),
                target: file_c.clone(),
                edge_type: EdgeType::Calls,
                weight: 1.0,
            })
            .expect("add edge b->c");

        let neighbors = graph.get_neighbors(&file_a, 2).expect("get neighbors");
        assert_eq!(neighbors.len(), 2);

        // file_b at distance 1, file_c at distance 2
        let b_neighbor = neighbors.iter().find(|n| n.path == file_b).unwrap();
        assert_eq!(b_neighbor.distance, 1);

        let c_neighbor = neighbors.iter().find(|n| n.path == file_c).unwrap();
        assert_eq!(c_neighbor.distance, 2);
    }

    #[test]
    fn test_get_neighbors_empty() {
        let graph = FileDependencyGraph::new();
        let file_a = PathBuf::from("src/a.rs");

        let neighbors = graph.get_neighbors(&file_a, 1).expect("get neighbors");
        assert!(neighbors.is_empty());
    }

    #[test]
    fn test_architectural_context() {
        let graph = FileDependencyGraph::new();
        let file_a = PathBuf::from("src/a.rs");
        let file_b = PathBuf::from("src/b.rs");

        graph
            .add_edge(&DependencyEdge {
                source: file_a.clone(),
                target: file_b.clone(),
                edge_type: EdgeType::Imports,
                weight: 1.0,
            })
            .expect("add edge");

        let context = graph
            .get_architectural_context(&file_a, Some(1))
            .expect("get context");

        assert_eq!(context.focal_file, file_a);
        assert_eq!(context.neighbors.len(), 1);
        assert_eq!(context.max_hops, 1);
        assert_eq!(context.total_files, 2);
    }

    #[test]
    fn test_compute_importance() {
        let graph = FileDependencyGraph::new();
        let file_a = PathBuf::from("src/a.rs");
        let file_b = PathBuf::from("src/b.rs");
        let file_c = PathBuf::from("src/c.rs");

        // Both a and c depend on b (b is more important)
        graph
            .add_edge(&DependencyEdge {
                source: file_a.clone(),
                target: file_b.clone(),
                edge_type: EdgeType::Imports,
                weight: 1.0,
            })
            .expect("add edge");

        graph
            .add_edge(&DependencyEdge {
                source: file_c.clone(),
                target: file_b.clone(),
                edge_type: EdgeType::Imports,
                weight: 1.0,
            })
            .expect("add edge");

        graph.compute_importance().expect("compute importance");

        let inner = graph.inner.read().unwrap();
        let b_importance = inner.nodes.get(&file_b).unwrap().importance;
        let a_importance = inner.nodes.get(&file_a).unwrap().importance;

        // b should have higher importance (more incoming edges)
        assert!(b_importance > a_importance);
    }

    #[test]
    fn test_edge_type_conversion() {
        assert_eq!(EdgeType::Imports.as_str(), "imports");
        assert_eq!(EdgeType::parse("imports"), Some(EdgeType::Imports));
        assert_eq!(EdgeType::parse("invalid"), None);
    }

    #[test]
    fn test_count_by_edge_type() {
        let graph = FileDependencyGraph::new();
        let file_a = PathBuf::from("src/a.rs");
        let file_b = PathBuf::from("src/b.rs");
        let file_c = PathBuf::from("src/c.rs");

        graph
            .add_edge(&DependencyEdge {
                source: file_a.clone(),
                target: file_b.clone(),
                edge_type: EdgeType::Imports,
                weight: 1.0,
            })
            .unwrap();
        graph
            .add_edge(&DependencyEdge {
                source: file_a.clone(),
                target: file_c.clone(),
                edge_type: EdgeType::Calls,
                weight: 1.0,
            })
            .unwrap();
        graph
            .add_edge(&DependencyEdge {
                source: file_b.clone(),
                target: file_c.clone(),
                edge_type: EdgeType::Inherits,
                weight: 1.0,
            })
            .unwrap();

        let counts = graph.count_by_edge_type();
        assert_eq!(*counts.get(&EdgeType::Imports).unwrap(), 1);
        assert_eq!(*counts.get(&EdgeType::Calls).unwrap(), 1);
        assert_eq!(*counts.get(&EdgeType::Inherits).unwrap(), 1);
        assert_eq!(*counts.get(&EdgeType::Instantiates).unwrap(), 0);
        assert_eq!(*counts.get(&EdgeType::HistoricalCoChange).unwrap(), 0);
    }
}
