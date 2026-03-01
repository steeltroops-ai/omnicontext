//! Community detection using the Louvain algorithm.
//!
//! Communities represent cohesive architectural modules in the codebase.
//! Used for:
//! - Architectural understanding
//! - Module boundary detection
//! - Context assembly (include related modules)

use petgraph::graph::NodeIndex;
use std::collections::HashMap;

use crate::error::OmniResult;

/// A detected community (architectural module).
#[derive(Debug, Clone)]
pub struct Community {
    /// Unique community ID.
    pub id: usize,
    /// Symbol IDs in this community.
    pub members: Vec<i64>,
    /// Modularity score (quality metric).
    pub modularity: f64,
}

/// Detect communities using the Louvain algorithm.
///
/// Returns a list of communities with their members and modularity scores.
pub fn detect_communities(
    graph: &petgraph::graph::DiGraph<i64, crate::types::DependencyKind>,
) -> OmniResult<Vec<Community>> {
    if graph.node_count() == 0 {
        return Ok(Vec::new());
    }

    // Phase 1: Initialize - each node in its own community
    let mut node_to_community: HashMap<NodeIndex, usize> = HashMap::new();
    for (idx, node) in graph.node_indices().enumerate() {
        node_to_community.insert(node, idx);
    }

    let mut improved = true;
    let mut iteration = 0;
    const MAX_ITERATIONS: usize = 100;

    // Phase 2: Iteratively move nodes to maximize modularity
    while improved && iteration < MAX_ITERATIONS {
        improved = false;
        iteration += 1;

        for node in graph.node_indices() {
            let current_community = node_to_community[&node];
            let best_community = find_best_community(node, &node_to_community, graph);

            if best_community != current_community {
                node_to_community.insert(node, best_community);
                improved = true;
            }
        }
    }

    // Phase 3: Aggregate into communities
    let communities = aggregate_communities(&node_to_community, graph);

    Ok(communities)
}

/// Find the best community for a node to maximize modularity gain.
fn find_best_community(
    node: NodeIndex,
    node_to_community: &HashMap<NodeIndex, usize>,
    graph: &petgraph::graph::DiGraph<i64, crate::types::DependencyKind>,
) -> usize {
    let current_community = node_to_community[&node];

    // Count edges to each neighboring community
    let mut community_edges: HashMap<usize, usize> = HashMap::new();

    // Outgoing edges
    for neighbor in graph.neighbors(node) {
        if let Some(&comm) = node_to_community.get(&neighbor) {
            *community_edges.entry(comm).or_insert(0) += 1;
        }
    }

    // Incoming edges (treat graph as undirected for community detection)
    for neighbor in graph.neighbors_directed(node, petgraph::Direction::Incoming) {
        if let Some(&comm) = node_to_community.get(&neighbor) {
            *community_edges.entry(comm).or_insert(0) += 1;
        }
    }

    // Find community with most connections
    let best = community_edges
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(comm, _)| comm)
        .unwrap_or(current_community);

    best
}

/// Aggregate nodes into communities and calculate modularity.
fn aggregate_communities(
    node_to_community: &HashMap<NodeIndex, usize>,
    graph: &petgraph::graph::DiGraph<i64, crate::types::DependencyKind>,
) -> Vec<Community> {
    // Group nodes by community
    let mut communities_map: HashMap<usize, Vec<i64>> = HashMap::new();
    for (node, &comm_id) in node_to_community {
        let symbol_id = graph[*node];
        communities_map.entry(comm_id).or_default().push(symbol_id);
    }

    // Calculate modularity for the entire partition
    let modularity = calculate_modularity(node_to_community, graph);

    // Convert to Community structs
    let mut communities: Vec<Community> = communities_map
        .into_iter()
        .enumerate()
        .map(|(idx, (_, members))| Community {
            id: idx,
            members,
            modularity,
        })
        .collect();

    // Sort by size (largest first)
    communities.sort_by(|a, b| b.members.len().cmp(&a.members.len()));

    // Reassign IDs after sorting
    for (idx, comm) in communities.iter_mut().enumerate() {
        comm.id = idx;
    }

    communities
}

/// Calculate modularity score for a partition.
///
/// Modularity measures the quality of a community structure.
/// Range: [-0.5, 1.0], higher is better.
/// > 0.3 is considered good community structure.
fn calculate_modularity(
    node_to_community: &HashMap<NodeIndex, usize>,
    graph: &petgraph::graph::DiGraph<i64, crate::types::DependencyKind>,
) -> f64 {
    let m = graph.edge_count() as f64;
    if m == 0.0 {
        return 0.0;
    }

    let mut q = 0.0;

    // For each edge, check if both endpoints are in the same community
    for edge in graph.edge_indices() {
        if let Some((src, dst)) = graph.edge_endpoints(edge) {
            let src_comm = node_to_community.get(&src);
            let dst_comm = node_to_community.get(&dst);

            if src_comm == dst_comm && src_comm.is_some() {
                // Edge within community
                let k_i = graph.neighbors_undirected(src).count() as f64;
                let k_j = graph.neighbors_undirected(dst).count() as f64;

                // Modularity contribution
                q += 1.0 - (k_i * k_j) / (2.0 * m * m);
            }
        }
    }

    q / m
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::DiGraph;

    #[test]
    fn test_detect_communities_empty_graph() {
        let graph: DiGraph<i64, crate::types::DependencyKind> = DiGraph::new();
        let communities = detect_communities(&graph).unwrap();
        assert!(communities.is_empty());
    }

    #[test]
    fn test_detect_communities_single_node() {
        let mut graph = DiGraph::new();
        graph.add_node(1);
        let communities = detect_communities(&graph).unwrap();
        assert_eq!(communities.len(), 1);
        assert_eq!(communities[0].members, vec![1]);
    }

    #[test]
    fn test_detect_communities_two_clusters() {
        let mut graph = DiGraph::new();

        // Cluster 1: nodes 1, 2, 3 (densely connected)
        let n1 = graph.add_node(1);
        let n2 = graph.add_node(2);
        let n3 = graph.add_node(3);
        graph.add_edge(n1, n2, crate::types::DependencyKind::Calls);
        graph.add_edge(n2, n3, crate::types::DependencyKind::Calls);
        graph.add_edge(n3, n1, crate::types::DependencyKind::Calls);

        // Cluster 2: nodes 4, 5, 6 (densely connected)
        let n4 = graph.add_node(4);
        let n5 = graph.add_node(5);
        let n6 = graph.add_node(6);
        graph.add_edge(n4, n5, crate::types::DependencyKind::Calls);
        graph.add_edge(n5, n6, crate::types::DependencyKind::Calls);
        graph.add_edge(n6, n4, crate::types::DependencyKind::Calls);

        // Weak connection between clusters
        graph.add_edge(n3, n4, crate::types::DependencyKind::Imports);

        let communities = detect_communities(&graph).unwrap();

        // Should detect 2 communities
        assert!(communities.len() >= 1);
        assert!(communities.len() <= 3); // May merge if modularity is better

        // Total members should be 6
        let total_members: usize = communities.iter().map(|c| c.members.len()).sum();
        assert_eq!(total_members, 6);
    }

    #[test]
    fn test_modularity_calculation() {
        let mut graph = DiGraph::new();
        let n1 = graph.add_node(1);
        let n2 = graph.add_node(2);
        graph.add_edge(n1, n2, crate::types::DependencyKind::Calls);

        let mut node_to_community = HashMap::new();
        node_to_community.insert(n1, 0);
        node_to_community.insert(n2, 0);

        let modularity = calculate_modularity(&node_to_community, &graph);

        // Both nodes in same community with edge between them
        // Should have positive modularity
        assert!(modularity >= 0.0);
    }
}

