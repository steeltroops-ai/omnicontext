//! Semantic Reasoning Graph — Logic Propagation Engine.
//!
//! Extends the structural dependency graph with semantic edges that enable
//! intelligent context retrieval beyond textual similarity.
//!
//! ## Edge Types
//!
//! - **DataFlow**: Value propagation across function boundaries
//!   `let x = a(); b(x);` creates DataFlow(a → b)
//!
//! - **ErrorFlow**: Error/Result propagation chains
//!   `a()?.process()` creates ErrorFlow(a → caller)
//!
//! - **TypeFlow**: Generic type parameter binding
//!   `Vec<MyType>` creates TypeFlow(MyType → Vec)
//!
//! ## Reasoning Queries
//!
//! - `data_flows_to(sym)` — Follow data flow edges to find downstream consumers
//! - `error_handled_by(sym)` — Trace error propagation to find handlers
//! - `impact_analysis(sym)` — Combine structural + semantic edges for blast radius
//! - `reasoning_neighborhood(sym, hops)` — BFS over all edge types with decay
//!
//! ## Graph-Augmented Retrieval (GAR)
//!
//! The `reasoning_neighborhood` query is the core of GAR:
//! 1. Start from anchor search results
//! 2. Walk N hops over ALL edge types (structural + semantic)
//! 3. Weight by edge type and hop distance
//! 4. Return ranked symbol IDs for context injection

use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::OmniResult;
use crate::types::DependencyKind;

use super::DependencyGraph;

/// Weight multipliers for different edge types during graph walks.
///
/// Semantic edges get higher weights because they carry richer meaning.
/// Structural edges are lower but still valuable for proximity.
#[derive(Debug, Clone)]
pub struct EdgeWeights {
    /// Weight for `Imports` edges (e.g., `use foo::bar`).
    pub imports: f64,
    /// Weight for `Calls` edges (function/method invocations).
    pub calls: f64,
    /// Weight for `Extends` edges (class/trait inheritance).
    pub extends: f64,
    /// Weight for `Implements` edges (trait/interface implementations).
    pub implements: f64,
    /// Weight for `UsesType` edges (type references in signatures).
    pub uses_type: f64,
    /// Weight for `Instantiates` edges (constructor calls, `::new()`).
    pub instantiates: f64,
    /// Weight for `FieldAccess` edges (struct field reads/writes).
    pub field_access: f64,
    /// Weight for `DataFlow` edges (value propagation across boundaries).
    pub data_flow: f64,
    /// Weight for `ErrorFlow` edges (error/Result propagation chains).
    pub error_flow: f64,
    /// Weight for `TypeFlow` edges (generic type parameter binding).
    pub type_flow: f64,
    /// Weight for `HistoricalCoChange` edges (files that historically change together).
    pub historical_co_change: f64,
}

impl Default for EdgeWeights {
    fn default() -> Self {
        Self {
            // Structural edges — moderate weight
            imports: 0.3,
            calls: 0.6,
            extends: 0.7,
            implements: 0.7,
            uses_type: 0.4,
            instantiates: 0.5,
            field_access: 0.3,
            // Semantic edges — higher weight
            data_flow: 0.8,
            error_flow: 0.9,
            type_flow: 0.6,
            historical_co_change: 0.5,
        }
    }
}

impl EdgeWeights {
    /// Get the weight for a specific edge type.
    pub fn weight_for(&self, kind: &DependencyKind) -> f64 {
        match kind {
            DependencyKind::Imports => self.imports,
            DependencyKind::Calls => self.calls,
            DependencyKind::Extends => self.extends,
            DependencyKind::Implements => self.implements,
            DependencyKind::UsesType => self.uses_type,
            DependencyKind::Instantiates => self.instantiates,
            DependencyKind::FieldAccess => self.field_access,
            DependencyKind::DataFlow => self.data_flow,
            DependencyKind::ErrorFlow => self.error_flow,
            DependencyKind::TypeFlow => self.type_flow,
            DependencyKind::HistoricalCoChange => self.historical_co_change,
        }
    }
}

/// A symbol discovered during graph reasoning, with a relevance score.
#[derive(Debug, Clone)]
pub struct ReasoningHit {
    /// Symbol ID in the dependency graph.
    pub symbol_id: i64,
    /// Accumulated relevance score (higher = more relevant).
    pub score: f64,
    /// Number of hops from the seed symbol.
    pub depth: usize,
    /// The edge types traversed to reach this symbol.
    pub edge_path: Vec<DependencyKind>,
}

/// Semantic Reasoning Engine — performs intelligent graph traversals.
///
/// Wraps the `DependencyGraph` and adds higher-level reasoning queries
/// that combine structural and semantic edges.
pub struct ReasoningEngine {
    /// Edge weights for scoring during graph walks.
    weights: EdgeWeights,
    /// Decay factor per hop (score *= decay^hop). Default: 0.6
    hop_decay: f64,
    /// Maximum hops for any traversal.
    max_hops: usize,
}

impl Default for ReasoningEngine {
    fn default() -> Self {
        Self {
            weights: EdgeWeights::default(),
            hop_decay: 0.6,
            max_hops: 4,
        }
    }
}

impl ReasoningEngine {
    /// Create a new reasoning engine with custom settings.
    pub fn new(weights: EdgeWeights, hop_decay: f64, max_hops: usize) -> Self {
        Self {
            weights,
            hop_decay,
            max_hops,
        }
    }

    /// Get the maximum hops this engine will traverse.
    pub fn max_hops(&self) -> usize {
        self.max_hops
    }

    /// Find all symbols that receive data from `symbol_id` via DataFlow edges.
    ///
    /// Follows only `DataFlow` edges, returning symbols that consume
    /// values produced by the given symbol.
    pub fn data_flows_to(
        &self,
        graph: &DependencyGraph,
        symbol_id: i64,
        max_depth: usize,
    ) -> OmniResult<Vec<ReasoningHit>> {
        self.walk_filtered(
            graph,
            symbol_id,
            max_depth.min(self.max_hops),
            &[DependencyKind::DataFlow],
            true, // forward direction
        )
    }

    /// Find all symbols that produce data consumed by `symbol_id`.
    ///
    /// Follows `DataFlow` edges backwards (upstream data sources).
    pub fn data_flows_from(
        &self,
        graph: &DependencyGraph,
        symbol_id: i64,
        max_depth: usize,
    ) -> OmniResult<Vec<ReasoningHit>> {
        self.walk_filtered(
            graph,
            symbol_id,
            max_depth.min(self.max_hops),
            &[DependencyKind::DataFlow],
            false, // backward direction
        )
    }

    /// Trace error propagation from `symbol_id` to find where errors are handled.
    ///
    /// Follows `ErrorFlow` edges forward (error propagation direction).
    pub fn error_handled_by(
        &self,
        graph: &DependencyGraph,
        symbol_id: i64,
    ) -> OmniResult<Vec<ReasoningHit>> {
        self.walk_filtered(
            graph,
            symbol_id,
            self.max_hops,
            &[DependencyKind::ErrorFlow],
            true,
        )
    }

    /// Compute the full reasoning neighborhood of a symbol.
    ///
    /// This is the core of **Graph-Augmented Retrieval (GAR)**:
    /// 1. BFS from `symbol_id` over ALL edge types
    /// 2. Score each discovered symbol by edge weight * hop decay
    /// 3. Return top-N symbols by accumulated score
    ///
    /// Used to inject "shadow context" — structurally relevant code
    /// that didn't match the text query but is architecturally important.
    pub fn reasoning_neighborhood(
        &self,
        graph: &DependencyGraph,
        symbol_id: i64,
        max_depth: usize,
        top_n: usize,
    ) -> OmniResult<Vec<ReasoningHit>> {
        let depth = max_depth.min(self.max_hops);
        let mut scores: HashMap<i64, (f64, usize, Vec<DependencyKind>)> = HashMap::new();
        let mut visited: HashSet<i64> = HashSet::new();
        let mut queue: VecDeque<(i64, usize, Vec<DependencyKind>)> = VecDeque::new();

        visited.insert(symbol_id);
        queue.push_back((symbol_id, 0, Vec::new()));

        while let Some((current, current_depth, path)) = queue.pop_front() {
            if current_depth >= depth {
                continue;
            }

            // Get all edges for current symbol (both outgoing and incoming)
            if let Ok(edges) = graph.get_edges_for_symbol(current) {
                for (neighbor_id, kind, direction) in &edges {
                    // Only follow outgoing edges for forward traversal
                    if *direction != "outgoing" {
                        continue;
                    }
                    let edge_weight = self.weights.weight_for(kind);
                    let hop_score = edge_weight * self.hop_decay.powi((current_depth + 1) as i32);

                    let mut new_path = path.clone();
                    new_path.push(*kind);

                    // Accumulate score for this neighbor.
                    // Update depth to minimum and keep the highest-scoring path.
                    let entry = scores.entry(*neighbor_id).or_insert((
                        0.0,
                        current_depth + 1,
                        new_path.clone(),
                    ));
                    entry.0 += hop_score;
                    // Track the shortest path (minimum depth) for this node
                    if current_depth + 1 < entry.1 {
                        entry.1 = current_depth + 1;
                        entry.2.clone_from(&new_path);
                    }

                    // Only enqueue if not yet visited
                    if visited.insert(*neighbor_id) {
                        queue.push_back((*neighbor_id, current_depth + 1, new_path));
                    }
                }

                // Also follow incoming edges (upstream — bidirectional walk with weaker signal)
                // Use actual edge kind and weight, but apply a 0.5× discount since
                // incoming edges carry less directional relevance than outgoing.
                for (neighbor_id, kind, direction) in &edges {
                    if *direction != "incoming" {
                        continue;
                    }
                    if visited.contains(neighbor_id) {
                        continue;
                    }
                    let edge_weight = self.weights.weight_for(kind) * 0.5; // incoming discount
                    let hop_score = edge_weight * self.hop_decay.powi((current_depth + 1) as i32);
                    let mut new_path = path.clone();
                    new_path.push(*kind);

                    let entry = scores.entry(*neighbor_id).or_insert((
                        0.0,
                        current_depth + 1,
                        new_path.clone(),
                    ));
                    entry.0 += hop_score;
                    if current_depth + 1 < entry.1 {
                        entry.1 = current_depth + 1;
                        entry.2.clone_from(&new_path);
                    }

                    if visited.insert(*neighbor_id) {
                        queue.push_back((*neighbor_id, current_depth + 1, new_path));
                    }
                }
            }
        }

        // Sort by score, return top-N
        let mut hits: Vec<ReasoningHit> = scores
            .into_iter()
            .map(|(sym_id, (score, depth, path))| ReasoningHit {
                symbol_id: sym_id,
                score,
                depth,
                edge_path: path,
            })
            .collect();

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(top_n);

        Ok(hits)
    }

    /// Combined impact analysis: structural + semantic + historical.
    ///
    /// Returns all symbols that would be affected if `symbol_id` changes,
    /// scored by impact severity.
    pub fn impact_analysis(
        &self,
        graph: &DependencyGraph,
        symbol_id: i64,
        max_depth: usize,
    ) -> OmniResult<Vec<ReasoningHit>> {
        // Start with downstream structural dependents
        let mut combined = self.reasoning_neighborhood(graph, symbol_id, max_depth, 100)?;

        // Boost symbols connected by semantic/flow/historical edges.
        // Use a single combined boost factor to avoid multiplicative stacking.
        // Flow edges (DataFlow/ErrorFlow/TypeFlow) carry the richest signal → highest boost.
        // HistoricalCoChange is semantic but not a flow edge → moderate boost.
        // Structural-only edges → no additional boost.
        for hit in &mut combined {
            let has_flow = hit.edge_path.iter().any(|e| e.is_flow_edge());
            let has_semantic = hit.edge_path.iter().any(|e| e.is_semantic());

            let boost = if has_flow {
                1.8 // Flow edges: data/error/type dependencies → strongest impact signal
            } else if has_semantic {
                1.4 // Semantic non-flow (e.g. HistoricalCoChange) → moderate boost
            } else {
                1.0 // Structural only → base score preserved
            };
            hit.score *= boost;
        }

        // Re-sort after boosting
        combined.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(combined)
    }

    /// Walk the graph following only specific edge types.
    ///
    /// `forward = true`: follow outgoing edges FROM symbol (downstream)
    /// `forward = false`: follow incoming edges TO symbol (upstream)
    fn walk_filtered(
        &self,
        graph: &DependencyGraph,
        symbol_id: i64,
        max_depth: usize,
        allowed_kinds: &[DependencyKind],
        forward: bool,
    ) -> OmniResult<Vec<ReasoningHit>> {
        let allowed: HashSet<String> = allowed_kinds
            .iter()
            .map(|k| k.as_str().to_string())
            .collect();
        let target_direction = if forward { "outgoing" } else { "incoming" };
        let mut visited: HashSet<i64> = HashSet::new();
        // Accumulate scores per symbol: (total_score, min_depth, best_path)
        let mut scores: HashMap<i64, (f64, usize, Vec<DependencyKind>)> = HashMap::new();
        let mut queue: VecDeque<(i64, usize, Vec<DependencyKind>)> = VecDeque::new();

        visited.insert(symbol_id);
        queue.push_back((symbol_id, 0, Vec::new()));

        while let Some((current, depth, path)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            if let Ok(edges) = graph.get_edges_for_symbol(current) {
                for (neighbor_id, kind, direction) in &edges {
                    if *direction != target_direction {
                        continue;
                    }
                    if !allowed.contains(kind.as_str()) {
                        continue;
                    }
                    let mut new_path = path.clone();
                    new_path.push(*kind);
                    let hop_score =
                        self.weights.weight_for(kind) * self.hop_decay.powi((depth + 1) as i32);

                    // Accumulate score across paths
                    let entry =
                        scores
                            .entry(*neighbor_id)
                            .or_insert((0.0, depth + 1, new_path.clone()));
                    entry.0 += hop_score;
                    if depth + 1 < entry.1 {
                        entry.1 = depth + 1;
                        entry.2.clone_from(&new_path);
                    }

                    // Only enqueue if not yet visited (BFS)
                    if visited.insert(*neighbor_id) {
                        queue.push_back((*neighbor_id, depth + 1, new_path));
                    }
                }
            }
        }

        let mut results: Vec<ReasoningHit> = scores
            .into_iter()
            .map(|(sym_id, (score, depth, path))| ReasoningHit {
                symbol_id: sym_id,
                score,
                depth,
                edge_path: path,
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DependencyEdge;

    fn setup_test_graph() -> DependencyGraph {
        let graph = DependencyGraph::new();

        // Create a chain: A -> B -> C with mixed edge types
        graph.add_symbol(1).unwrap(); // A
        graph.add_symbol(2).unwrap(); // B
        graph.add_symbol(3).unwrap(); // C
        graph.add_symbol(4).unwrap(); // D (error handler)
        graph.add_symbol(5).unwrap(); // E (data consumer)

        // Structural edges
        graph
            .add_edge(&DependencyEdge {
                source_id: 1,
                target_id: 2,
                kind: DependencyKind::Calls,
            })
            .unwrap();
        graph
            .add_edge(&DependencyEdge {
                source_id: 2,
                target_id: 3,
                kind: DependencyKind::Calls,
            })
            .unwrap();

        // Semantic edges
        graph
            .add_edge(&DependencyEdge {
                source_id: 1,
                target_id: 5,
                kind: DependencyKind::DataFlow,
            })
            .unwrap();
        graph
            .add_edge(&DependencyEdge {
                source_id: 1,
                target_id: 4,
                kind: DependencyKind::ErrorFlow,
            })
            .unwrap();
        graph
            .add_edge(&DependencyEdge {
                source_id: 3,
                target_id: 5,
                kind: DependencyKind::DataFlow,
            })
            .unwrap();

        graph
    }

    #[test]
    fn test_reasoning_neighborhood() {
        let graph = setup_test_graph();
        let engine = ReasoningEngine::default();

        let hits = engine.reasoning_neighborhood(&graph, 1, 2, 10).unwrap();

        // Should find B (calls), D (error_flow), E (data_flow), and C (2-hop)
        assert!(!hits.is_empty());

        // E should be found (data_flow from 1)
        assert!(hits.iter().any(|h| h.symbol_id == 5));
        // D should be found (error_flow from 1)
        assert!(hits.iter().any(|h| h.symbol_id == 4));
        // B should be found (calls from 1)
        assert!(hits.iter().any(|h| h.symbol_id == 2));
    }

    #[test]
    fn test_data_flows_to() {
        let graph = setup_test_graph();
        let engine = ReasoningEngine::default();

        let hits = engine.data_flows_to(&graph, 1, 2).unwrap();

        // Symbol 1 has DataFlow to 5
        assert!(hits.iter().any(|h| h.symbol_id == 5));
        // Should NOT include B (that's a Calls edge, not DataFlow)
        assert!(!hits.iter().any(|h| h.symbol_id == 2));
    }

    #[test]
    fn test_error_handled_by() {
        let graph = setup_test_graph();
        let engine = ReasoningEngine::default();

        let hits = engine.error_handled_by(&graph, 1).unwrap();

        // Symbol 1 has ErrorFlow to 4
        assert!(hits.iter().any(|h| h.symbol_id == 4));
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_impact_analysis() {
        let graph = setup_test_graph();
        let engine = ReasoningEngine::default();

        let hits = engine.impact_analysis(&graph, 1, 3).unwrap();

        // All reachable symbols should appear
        assert!(hits.len() >= 3);

        // Symbols connected by flow edges should score higher (1.8× vs 1.0×)
        let data_flow_hit = hits.iter().find(|h| h.symbol_id == 5);
        let call_hit = hits.iter().find(|h| h.symbol_id == 2);
        if let (Some(df), Some(c)) = (data_flow_hit, call_hit) {
            // DataFlow hit should have higher score due to flow-edge boost (1.8×)
            assert!(
                df.score >= c.score,
                "DataFlow hit ({}) should score >= Calls hit ({})",
                df.score,
                c.score
            );
        }
    }

    #[test]
    fn test_edge_weights_default() {
        let weights = EdgeWeights::default();
        // Semantic edges should have higher weights than structural
        assert!(weights.data_flow > weights.imports);
        assert!(weights.error_flow > weights.calls);
    }

    #[test]
    fn test_dependency_kind_semantic_classification() {
        assert!(DependencyKind::DataFlow.is_semantic());
        assert!(DependencyKind::ErrorFlow.is_semantic());
        assert!(DependencyKind::TypeFlow.is_semantic());
        assert!(DependencyKind::HistoricalCoChange.is_semantic());
        assert!(!DependencyKind::Calls.is_semantic());
        assert!(!DependencyKind::Imports.is_semantic());
    }

    #[test]
    fn test_dependency_kind_flow_classification() {
        assert!(DependencyKind::DataFlow.is_flow_edge());
        assert!(DependencyKind::ErrorFlow.is_flow_edge());
        assert!(DependencyKind::TypeFlow.is_flow_edge());
        assert!(!DependencyKind::HistoricalCoChange.is_flow_edge());
        assert!(!DependencyKind::Calls.is_flow_edge());
    }
}
