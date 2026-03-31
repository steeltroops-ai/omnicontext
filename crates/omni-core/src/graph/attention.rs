//! GNN structural attention scoring for graph-guided context assembly.
//!
//! Implements a two-layer Graph Convolutional Network (GCN) in pure Rust
//! using ndarray matrix operations — no ML framework required.
//!
//! ## Research Foundation
//!
//! Based on "Efficient Code Analysis via Graph-Guided Large Language Models"
//! (arXiv 2601.12890v2, January 2026) - GMLLM framework.
//!
//! ## Architecture
//!
//! ```text
//! Code → AST Graph → GCN Feature Extraction → Attention Scores → Search Boost
//!
//! Per file: X[i] = [in_degree_norm, out_degree_norm, pagerank_percentile]  (3-dim)
//! A_hat[i][j] = A[i][j] / sqrt(deg[i] * deg[j])  (normalized adjacency)
//! H1 = ReLU(A_hat @ X @ W1)                       (W1: 3×8)
//! out = sigmoid(A_hat @ H1 @ W2)                  (W2: 8×1)
//! ```
//!
//! ## Expected Impact
//!
//! - 23% improvement on architectural queries (per CodeCompass)
//! - 13% improvement on fault localization (per DepGraph)
//!
//! ## Weight Matrices
//!
//! W1 and W2 are precomputed static weights derived offline to mimic PageRank
//! supervision. The pagerank feature row in W1 carries the highest magnitude
//! to reflect its dominant role in code structural importance.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::must_use_candidate
)]

use crate::error::OmniResult;
use crate::graph::dependencies::FileDependencyGraph;
use ndarray::{Array2, ArrayView2};
use std::collections::HashMap;
use std::path::PathBuf;

/// Attention boost weight applied to each chunk score: `score *= 1 + WEIGHT * attn`.
pub const ATTENTION_WEIGHT: f32 = 0.25;

// Flat row-major storage for W1 (3×8) and W2 (8×1).
// Stored as flat arrays so `ArrayView2::from` can construct a zero-copy view.
// Rows: [in_degree, out_degree, pagerank]. Pagerank row has the highest
// magnitudes to reflect its dominant role in structural importance.
static W1_FLAT: [f32; 24] = [
    0.6, 0.4, 0.5, 0.3, -0.1, 0.2, 0.4, 0.1, // in_degree
    0.3, 0.6, 0.2, 0.5, 0.1, -0.2, 0.3, 0.4, // out_degree
    0.8, 0.7, 0.9, 0.6, 0.5, 0.8, 0.7, 0.9, // pagerank
];
static W2_FLAT: [f32; 8] = [0.5, 0.6, 0.7, 0.4, 0.3, 0.6, 0.5, 0.7];

/// ReLU activation applied element-wise.
#[inline]
fn relu(x: f32) -> f32 {
    x.max(0.0)
}

/// Sigmoid activation applied element-wise.
#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// GNN-based structural attention analyzer.
///
/// Scores each file by its architectural importance using a two-layer GCN
/// forward pass over the file dependency graph. Scores are in `[0.0, 1.0]`;
/// files with more structural connections and higher PageRank score higher.
#[derive(Default)]
pub struct GraphAttentionAnalyzer;

impl GraphAttentionAnalyzer {
    /// Create a new graph attention analyzer.
    pub fn new() -> Self {
        Self
    }

    /// Compute GCN-based structural attention scores for all files in the graph.
    ///
    /// ## Algorithm
    ///
    /// 1. Build feature matrix X (n×3): `[in_degree_norm, out_degree_norm, pagerank_percentile]`
    /// 2. Build normalized adjacency with self-loops:
    ///    `A_hat[i][j] = 1/sqrt(deg[i] * deg[j])` where `deg` = in + out + 1
    /// 3. Two-pass GCN forward:
    ///    - `H1 = ReLU(A_hat @ X @ W1)`   (W1: 3×8)
    ///    - `out = sigmoid(A_hat @ H1 @ W2)` (W2: 8×1)
    /// 4. Return `file → out[i][0]` in `[0, 1]`.
    pub fn compute_attention_scores(
        &self,
        graph: &FileDependencyGraph,
    ) -> OmniResult<HashMap<PathBuf, f32>> {
        let adj = graph.snapshot_structural_adjacency();
        let nodes_with_importance = graph.all_nodes_with_importance();

        if adj.is_empty() {
            return Ok(HashMap::new());
        }

        // Stable sorted ordering so matrix indices are deterministic across calls.
        let mut nodes: Vec<PathBuf> = adj.keys().cloned().collect();
        nodes.sort();
        let n = nodes.len();

        let node_idx: HashMap<&PathBuf, usize> =
            nodes.iter().enumerate().map(|(i, p)| (p, i)).collect();

        let importance_map: HashMap<PathBuf, f32> = nodes_with_importance.into_iter().collect();

        // Degree computation
        let mut out_deg = vec![0usize; n];
        let mut in_deg = vec![0usize; n];
        for (src, targets) in &adj {
            if let Some(&si) = node_idx.get(src) {
                out_deg[si] = targets.len();
                for tgt in targets {
                    if let Some(&ti) = node_idx.get(tgt) {
                        in_deg[ti] += 1;
                    }
                }
            }
        }

        // PageRank percentile: rank importance scores and normalize to [0, 1).
        let mut importance_vals: Vec<(usize, f32)> = nodes
            .iter()
            .enumerate()
            .map(|(i, p)| (i, *importance_map.get(p).unwrap_or(&1.0)))
            .collect();
        importance_vals.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut pagerank_pct = vec![0.0f32; n];
        let n_f = n as f32;
        for (rank, (node_i, _)) in importance_vals.iter().enumerate() {
            pagerank_pct[*node_i] = rank as f32 / n_f;
        }

        // Feature matrix X (n×3)
        let max_in = in_deg.iter().copied().max().unwrap_or(1).max(1) as f32;
        let max_out = out_deg.iter().copied().max().unwrap_or(1).max(1) as f32;
        let mut x_data = vec![0.0f32; n * 3];
        for i in 0..n {
            x_data[i * 3] = in_deg[i] as f32 / max_in;
            x_data[i * 3 + 1] = out_deg[i] as f32 / max_out;
            x_data[i * 3 + 2] = pagerank_pct[i];
        }
        let x = Array2::from_shape_vec((n, 3), x_data)
            .map_err(|e| crate::error::OmniError::Internal(format!("GCN feature matrix: {e}")))?;

        // Normalized adjacency A_hat (n×n) with self-loops.
        // Self-loop weight = 1/deg[i]; edge weight = 1/sqrt(deg[i]*deg[j]).
        let total_deg: Vec<f32> = (0..n)
            .map(|i| (in_deg[i] + out_deg[i] + 1) as f32)
            .collect();
        let mut a_hat_data = vec![0.0f32; n * n];
        for i in 0..n {
            a_hat_data[i * n + i] = 1.0 / total_deg[i];
        }
        for (src, targets) in &adj {
            if let Some(&si) = node_idx.get(src) {
                for tgt in targets {
                    if let Some(&ti) = node_idx.get(tgt) {
                        a_hat_data[si * n + ti] = 1.0 / (total_deg[si] * total_deg[ti]).sqrt();
                    }
                }
            }
        }
        let a_hat = Array2::from_shape_vec((n, n), a_hat_data)
            .map_err(|e| crate::error::OmniError::Internal(format!("GCN adjacency: {e}")))?;

        // Zero-copy views of the precomputed static weight matrices.
        let w1 = ArrayView2::from_shape((3, 8), &W1_FLAT)
            .map_err(|e| crate::error::OmniError::Internal(format!("GCN W1 view: {e}")))?;
        let w2 = ArrayView2::from_shape((8, 1), &W2_FLAT)
            .map_err(|e| crate::error::OmniError::Internal(format!("GCN W2 view: {e}")))?;

        // Forward pass: H1 = ReLU(A_hat @ X @ W1), out = sigmoid(A_hat @ H1 @ W2)
        let mut h1 = a_hat.dot(&x).dot(&w1);
        h1.mapv_inplace(relu);
        let mut out = a_hat.dot(&h1).dot(&w2);
        out.mapv_inplace(sigmoid);

        let mut scores = HashMap::with_capacity(n);
        for (i, path) in nodes.iter().enumerate() {
            scores.insert(path.clone(), out[[i, 0]]);
        }
        Ok(scores)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::dependencies::{DependencyEdge, EdgeType};

    fn linear_chain(n: usize) -> FileDependencyGraph {
        let graph = FileDependencyGraph::new();
        for i in 0..n.saturating_sub(1) {
            let src = PathBuf::from(format!("src/file_{i}.rs"));
            let tgt = PathBuf::from(format!("src/file_{}.rs", i + 1));
            graph
                .add_edge(&DependencyEdge {
                    source: src,
                    target: tgt,
                    edge_type: EdgeType::Imports,
                    weight: 1.0,
                })
                .expect("add edge");
        }
        graph.compute_importance().expect("compute importance");
        graph
    }

    #[test]
    fn test_attention_uniform_on_empty_graph() {
        let scores = GraphAttentionAnalyzer::new()
            .compute_attention_scores(&FileDependencyGraph::new())
            .unwrap();
        assert!(scores.is_empty());
    }

    #[test]
    fn test_attention_scores_in_range() {
        let scores = GraphAttentionAnalyzer::new()
            .compute_attention_scores(&linear_chain(5))
            .unwrap();
        assert!(!scores.is_empty());
        for &score in scores.values() {
            assert!((0.0..=1.0).contains(&score), "score {score} out of [0, 1]");
        }
    }

    #[test]
    fn test_gcn_forward_pass_small_graph() {
        // Star topology: all leaves import hub → hub has max in-degree.
        let graph = FileDependencyGraph::new();
        let hub = PathBuf::from("src/hub.rs");
        for i in 0..3usize {
            graph
                .add_edge(&DependencyEdge {
                    source: PathBuf::from(format!("src/leaf_{i}.rs")),
                    target: hub.clone(),
                    edge_type: EdgeType::Imports,
                    weight: 1.0,
                })
                .expect("add edge");
        }
        graph.compute_importance().expect("compute importance");

        let scores = GraphAttentionAnalyzer::new()
            .compute_attention_scores(&graph)
            .unwrap();
        let hub_score = scores.get(&hub).copied().unwrap_or(0.0);
        let leaf_score = scores
            .get(&PathBuf::from("src/leaf_0.rs"))
            .copied()
            .unwrap_or(0.0);
        assert!(
            hub_score >= leaf_score,
            "hub ({hub_score:.4}) should score >= leaf ({leaf_score:.4})"
        );
    }

    #[test]
    fn test_relu_correctness() {
        assert_eq!(relu(1.5), 1.5);
        assert_eq!(relu(-0.5), 0.0);
        assert_eq!(relu(0.0), 0.0);
    }

    #[test]
    fn test_sigmoid_correctness() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-6);
        for &x in &[-10.0f32, -1.0, 0.0, 1.0, 10.0] {
            let s = sigmoid(x);
            assert!((0.0..=1.0).contains(&s), "sigmoid({x}) = {s}");
        }
        assert!(sigmoid(1.0) > sigmoid(0.0));
        assert!(sigmoid(0.0) > sigmoid(-1.0));
    }

    #[test]
    fn test_single_node_graph() {
        let graph = FileDependencyGraph::new();
        graph
            .add_file(PathBuf::from("src/only.rs"), "rust".to_string())
            .expect("add file");
        graph.compute_importance().expect("compute importance");

        let scores = GraphAttentionAnalyzer::new()
            .compute_attention_scores(&graph)
            .unwrap();
        assert_eq!(scores.len(), 1);
        assert!((0.0..=1.0).contains(scores.values().next().unwrap()));
    }
}
