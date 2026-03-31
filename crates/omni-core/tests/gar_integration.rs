//! GAR (Graph-Augmented Retrieval) integration tests for the search pipeline.
//!
//! These tests verify that the reasoning engine's graph traversal correctly
//! discovers neighbours, applies exponential hop-decay, and responds to
//! incremental graph mutations — all without requiring a real embedding model.
//!
//! ## Test design
//!
//! Every test builds a minimal but realistic fixture:
//!   - A `MetadataIndex` backed by a temp SQLite database
//!   - A `DependencyGraph` wired with explicit edges
//!   - A `ReasoningEngine` with default weights (hop_decay = 0.6, max_hops = 4)
//!   - A `SearchEngine` operating in keyword-only mode (degraded embedder)
//!   - A `VectorIndex` opened on a temp path (empty, no real vectors)
//!
//! The FTS5 keyword search is used as the "anchor" signal; because the
//! embedder is in degraded mode it contributes no vectors.  GAR then walks
//! the dependency graph from each anchor symbol and injects neighbour scores
//! into the result map returned by `search_with_gar`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::missing_docs_in_private_items,
    clippy::doc_markdown,
    clippy::items_after_statements,
    clippy::too_many_lines,
    clippy::ignore_without_reason
)]

use std::path::PathBuf;

use omni_core::config::EmbeddingConfig;
use omni_core::embedder::Embedder;
use omni_core::graph::reasoning::ReasoningEngine;
use omni_core::graph::DependencyGraph;
use omni_core::index::MetadataIndex;
use omni_core::search::SearchEngine;
use omni_core::types::{
    Chunk, ChunkKind, DependencyEdge, DependencyKind, FileInfo, Language, Symbol, Visibility,
};
use omni_core::vector::VectorIndex;

// ---------------------------------------------------------------------------
// Test fixture helpers
// ---------------------------------------------------------------------------

/// All the resources that compose a self-contained test fixture.
struct Fixture {
    /// Temporary directory – kept alive so SQLite and the vector index remain
    /// valid for the lifetime of the test.
    _tmpdir: tempfile::TempDir,
    index: MetadataIndex,
    vector_index: VectorIndex,
    embedder: Embedder,
    graph: DependencyGraph,
    engine: ReasoningEngine,
    search: SearchEngine,
}

impl Fixture {
    /// Create a fresh fixture backed by a temporary directory.
    fn new() -> Self {
        let tmpdir = tempfile::tempdir().expect("create tmpdir");
        let db_path = tmpdir.path().join("test.db");
        let vec_path = tmpdir.path().join("vectors");

        let index = MetadataIndex::open(&db_path).expect("open index");
        let vector_index = VectorIndex::open(&vec_path, 384).expect("open vector index");

        let embed_cfg = EmbeddingConfig {
            model_path: PathBuf::from("/nonexistent/model.onnx"),
            dimensions: 384,
            batch_size: 32,
            max_seq_length: 256,
            enable_sparse_retrieval: false,
            cloud_api_key: None,
            quantization_mode: omni_core::embedder::quantization::QuantizationMode::None,
        };
        let embedder = Embedder::degraded(&embed_cfg);

        // Default: hop_decay = 0.6, max_hops = 4
        let engine = ReasoningEngine::default();

        // rrf_k = 60, generous token budget so we never hit the cap in tests
        let search = SearchEngine::new(60, 1_000_000);

        Fixture {
            _tmpdir: tmpdir,
            index,
            vector_index,
            embedder,
            graph: DependencyGraph::new(),
            engine,
            search,
        }
    }
}

/// Insert a file record, one chunk and one symbol that all share the same FQN.
///
/// Returns `(file_id, chunk_id, symbol_id)`.
fn insert_symbol_with_chunk(index: &MetadataIndex, fqn: &str, content: &str) -> (i64, i64, i64) {
    let file_info = FileInfo {
        id: 0,
        path: PathBuf::from(format!("src/{fqn}.rs")),
        language: Language::Rust,
        content_hash: format!("hash_{fqn}"),
        size_bytes: content.len() as u64,
    };
    let file_id = index.upsert_file(&file_info).expect("upsert file");

    let chunk = Chunk {
        id: 0,
        file_id,
        symbol_path: fqn.to_string(),
        kind: ChunkKind::Function,
        visibility: Visibility::Public,
        line_start: 1,
        line_end: 10,
        content: content.to_string(),
        doc_comment: None,
        token_count: 20,
        weight: 1.0,
        vector_id: None,
        is_summary: false,
        content_hash: 0,
    };
    let chunk_id = index.insert_chunk(&chunk).expect("insert chunk");

    let symbol = Symbol {
        id: 0,
        name: fqn.split("::").last().unwrap_or(fqn).to_string(),
        fqn: fqn.to_string(),
        kind: ChunkKind::Function,
        file_id,
        line: 1,
        chunk_id: Some(chunk_id),
    };
    let symbol_id = index.insert_symbol(&symbol).expect("insert symbol");

    (file_id, chunk_id, symbol_id)
}

// ---------------------------------------------------------------------------
// Test 1 – GAR neighbours are included in results when graph has edges
// ---------------------------------------------------------------------------

/// Verify that `search_with_gar` populates the GAR neighbour map with chunks
/// that are graph-adjacent to the anchor (but may not match the text query).
///
/// Graph layout
/// ────────────
///   anchor_fn  --[Calls]-->  neighbour_fn
///                            (content has no keyword overlap with query)
///
/// The keyword search will return `anchor_fn` because its content contains the
/// query token.  GAR should then walk the edge and expose `neighbour_fn` in the
/// returned `gar_neighbors` map.
#[test]
fn gar_neighbours_included_when_edge_connects_anchor_to_chunk() {
    let fx = Fixture::new();

    // Insert anchor (keyword match) and neighbour (graph-only match)
    let (_, anchor_chunk_id, anchor_sym_id) = insert_symbol_with_chunk(
        &fx.index,
        "mylib::anchor_fn",
        "pub fn anchor_fn() { /* gar_unique_token_xyz */ }",
    );
    let (_, neighbour_chunk_id, neighbour_sym_id) = insert_symbol_with_chunk(
        &fx.index,
        "mylib::neighbour_fn",
        "pub fn neighbour_fn() { /* totally unrelated content */ }",
    );

    // Wire the graph: anchor → neighbour via Calls edge
    fx.graph
        .add_edge(&DependencyEdge {
            source_id: anchor_sym_id,
            target_id: neighbour_sym_id,
            kind: DependencyKind::Calls,
        })
        .expect("add Calls edge");

    let (results, gar_neighbors) = fx
        .search
        .search_with_gar(
            "gar_unique_token_xyz",
            20,
            &fx.index,
            &fx.vector_index,
            &fx.embedder,
            Some(&fx.graph),
            Some(&fx.engine),
            None,
            None,
            &[],  // open_files
            &[],  // sparse_results (no BGE-M3 in test fixture)
            None, // file_dep_graph
        )
        .expect("search_with_gar");

    // Anchor chunk should appear in search results
    assert!(
        results.iter().any(|r| r.chunk.id == anchor_chunk_id),
        "anchor chunk (id={anchor_chunk_id}) must appear in search results"
    );

    // Neighbour chunk should appear in GAR map (graph-discovered, not text-matched)
    assert!(
        gar_neighbors.contains_key(&neighbour_chunk_id),
        "neighbour chunk (id={neighbour_chunk_id}) must be in GAR neighbors map; \
         got keys: {:?}",
        gar_neighbors.keys().collect::<Vec<_>>()
    );

    // GAR score for the neighbour must be positive
    let &gar_score = gar_neighbors.get(&neighbour_chunk_id).unwrap();
    assert!(
        gar_score > 0.0,
        "GAR score for neighbour must be > 0, got {gar_score}"
    );
}

// ---------------------------------------------------------------------------
// Test 2 – Exponential hop-decay: 1-hop score > 2-hop score
// ---------------------------------------------------------------------------

/// Verify that symbols discovered at greater graph distance receive lower GAR
/// scores, consistent with the `hop_decay^depth` formula.
///
/// Graph layout
/// ────────────
///   anchor  --[Calls]-->  hop1  --[Calls]-->  hop2
///
/// `hop1` is 1 hop from the anchor; `hop2` is 2 hops.
/// Expected: score(hop1) > score(hop2)
#[test]
fn gar_hop_decay_score_decreases_with_distance() {
    let fx = Fixture::new();

    let (_, _anchor_chunk_id, anchor_sym_id) = insert_symbol_with_chunk(
        &fx.index,
        "decay::anchor",
        "pub fn anchor() { /* decay_unique_token */ }",
    );
    let (_, _hop1_chunk_id, hop1_sym_id) =
        insert_symbol_with_chunk(&fx.index, "decay::hop1", "pub fn hop1() {}");
    let (_, hop2_chunk_id, hop2_sym_id) =
        insert_symbol_with_chunk(&fx.index, "decay::hop2", "pub fn hop2() {}");

    // anchor -> hop1 -> hop2
    for (src, tgt) in [(anchor_sym_id, hop1_sym_id), (hop1_sym_id, hop2_sym_id)] {
        fx.graph
            .add_edge(&DependencyEdge {
                source_id: src,
                target_id: tgt,
                kind: DependencyKind::Calls,
            })
            .expect("add edge");
    }

    // Use the ReasoningEngine directly – this is the canonical GAR primitive
    let hits = fx
        .engine
        .reasoning_neighborhood(&fx.graph, anchor_sym_id, 4, 50)
        .expect("reasoning_neighborhood");

    let hop1_hit = hits
        .iter()
        .find(|h| h.symbol_id == hop1_sym_id)
        .expect("hop1 must be in neighbourhood");
    let hop2_hit = hits
        .iter()
        .find(|h| h.symbol_id == hop2_sym_id)
        .expect("hop2 must be in neighbourhood");

    assert_eq!(hop1_hit.depth, 1, "hop1 should be at depth 1");
    assert_eq!(hop2_hit.depth, 2, "hop2 should be at depth 2");

    assert!(
        hop1_hit.score > hop2_hit.score,
        "1-hop score ({}) must be greater than 2-hop score ({})",
        hop1_hit.score,
        hop2_hit.score
    );

    // Verify the exact decay ratio matches the default hop_decay (0.6).
    // score(hop1) = calls_weight * 0.6^1 = 0.6 * 0.6 = 0.36
    // score(hop2) = calls_weight * 0.6^2 = 0.6 * 0.36 = 0.216
    // ratio = 0.6 (the hop_decay constant)
    let ratio = hop2_hit.score / hop1_hit.score;
    let expected_decay = 0.6_f64;
    assert!(
        (ratio - expected_decay).abs() < 1e-9,
        "score ratio hop2/hop1 should equal hop_decay ({expected_decay}), got {ratio:.6}"
    );

    // The GAR map in search_with_gar should NOT include hop2's chunk (it is
    // only reachable at depth 2, but still within max_hops=4).
    // We just confirm hop2 IS accessible – the above depth/score checks suffice.
    let _ = hop2_chunk_id; // suppress unused warning
}

// ---------------------------------------------------------------------------
// Test 3 – remove_edges_for_symbols enables incremental graph updates
// ---------------------------------------------------------------------------

/// Verify that after calling `remove_edges_for_symbols`, the edge is gone from
/// the graph and GAR no longer discovers the previously-connected chunk.
///
/// This simulates the incremental re-index flow: a file is modified, its stale
/// edges are stripped, and fresh edges are inserted.  Results between the two
/// states must differ.
#[test]
fn gar_remove_edges_changes_neighbourhood() {
    let fx = Fixture::new();

    let (_, _anchor_chunk_id, anchor_sym_id) = insert_symbol_with_chunk(
        &fx.index,
        "incr::anchor",
        "pub fn anchor() { /* incr_unique_token */ }",
    );
    let (_, _, connected_sym_id) =
        insert_symbol_with_chunk(&fx.index, "incr::connected", "pub fn connected() {}");
    let (_, _, isolated_sym_id) =
        insert_symbol_with_chunk(&fx.index, "incr::isolated", "pub fn isolated() {}");

    // Add edges: anchor -> connected, anchor -> isolated
    for tgt in [connected_sym_id, isolated_sym_id] {
        fx.graph
            .add_edge(&DependencyEdge {
                source_id: anchor_sym_id,
                target_id: tgt,
                kind: DependencyKind::Calls,
            })
            .expect("add edge");
    }

    // Before removal – both should be neighbours
    let hits_before = fx
        .engine
        .reasoning_neighborhood(&fx.graph, anchor_sym_id, 3, 50)
        .expect("neighbourhood before");

    let ids_before: Vec<i64> = hits_before.iter().map(|h| h.symbol_id).collect();
    assert!(
        ids_before.contains(&connected_sym_id),
        "connected must be a neighbour before edge removal"
    );
    assert!(
        ids_before.contains(&isolated_sym_id),
        "isolated must be a neighbour before edge removal"
    );

    let edge_count_before = fx.graph.edge_count();
    assert_eq!(edge_count_before, 2, "expected 2 edges before removal");

    // Simulate incremental update: strip the anchor's edges
    let removed = fx.graph.remove_edges_for_symbols(&[anchor_sym_id]);
    assert_eq!(removed, 2, "both edges touching anchor should be removed");
    assert_eq!(
        fx.graph.edge_count(),
        0,
        "no edges should remain after removal"
    );

    // Nodes must still exist (remove_edges_for_symbols preserves nodes)
    assert_eq!(
        fx.graph.node_count(),
        3,
        "all three nodes must survive edge removal"
    );

    // After removal – neighbourhood must be empty
    let hits_after = fx
        .engine
        .reasoning_neighborhood(&fx.graph, anchor_sym_id, 3, 50)
        .expect("neighbourhood after");

    assert!(
        hits_after.is_empty(),
        "neighbourhood must be empty after all edges are removed; \
         still found: {:?}",
        hits_after.iter().map(|h| h.symbol_id).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Test 4 – GAR depth limit: results beyond max_hops are excluded
// ---------------------------------------------------------------------------

/// Verify that the `max_hops` ceiling is respected: symbols reachable only at
/// depth > max_hops must not appear in the neighbourhood.
///
/// Graph layout (chain of 5 hops)
/// ──────────────────────────────
///   s0 -> s1 -> s2 -> s3 -> s4 -> s5
///
/// With max_hops = 3 the engine with custom settings should find s1, s2, s3
/// but must NOT find s4 or s5.
#[test]
fn gar_depth_limit_excludes_deep_results() {
    let fx_base = Fixture::new();

    // Use a custom ReasoningEngine with max_hops = 3
    let engine_3hops =
        ReasoningEngine::new(omni_core::graph::reasoning::EdgeWeights::default(), 0.6, 3);

    // Build a chain: s0 -> s1 -> s2 -> s3 -> s4 -> s5
    let mut sym_ids = Vec::new();
    for i in 0..=5_usize {
        let (_, _, sym_id) = insert_symbol_with_chunk(
            &fx_base.index,
            &format!("chain::s{i}"),
            &format!("pub fn s{i}() {{}}"),
        );
        sym_ids.push(sym_id);
    }

    for w in sym_ids.windows(2) {
        fx_base
            .graph
            .add_edge(&DependencyEdge {
                source_id: w[0],
                target_id: w[1],
                kind: DependencyKind::Calls,
            })
            .expect("add chain edge");
    }

    let hits = engine_3hops
        .reasoning_neighborhood(&fx_base.graph, sym_ids[0], 10, 100)
        .expect("reasoning_neighborhood with 3-hop limit");

    let found_ids: Vec<i64> = hits.iter().map(|h| h.symbol_id).collect();

    // Nodes at depth 1, 2, 3 must be found
    for (i, &sym_id) in sym_ids[1..=3].iter().enumerate() {
        let expected_depth = i + 1;
        assert!(
            found_ids.contains(&sym_id),
            "s{expected_depth} (depth {expected_depth}) should be within 3-hop limit"
        );
    }

    // Nodes beyond max_hops must NOT be found
    for beyond in [4, 5] {
        assert!(
            !found_ids.contains(&sym_ids[beyond]),
            "s{beyond} (depth {beyond}) must be excluded by max_hops=3; \
             found: {found_ids:?}"
        );
    }

    // Also check depth annotation is correct for each found node
    for (i, &sym_id) in sym_ids[1..=3].iter().enumerate() {
        let expected_depth = i + 1;
        let hit = hits.iter().find(|h| h.symbol_id == sym_id).unwrap();
        assert_eq!(
            hit.depth, expected_depth,
            "s{expected_depth} should report depth={expected_depth}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 5 – search_with_gar: GAR map score respects edge-type weight ordering
// ---------------------------------------------------------------------------

/// `DataFlow` edges carry a higher default weight (0.8) than `Calls` (0.6).
/// A symbol reached via `DataFlow` from the anchor should have a higher GAR
/// score than a symbol reached via `Calls`, all else being equal (same depth).
#[test]
fn gar_edge_type_weights_order_scores_correctly() {
    let graph = DependencyGraph::new();
    let engine = ReasoningEngine::default();

    // Three symbols; only sym_anchor is the root
    for id in [100_i64, 200, 300] {
        graph.add_symbol(id).unwrap();
    }

    // anchor --[DataFlow]--> sym_200
    graph
        .add_edge(&DependencyEdge {
            source_id: 100,
            target_id: 200,
            kind: DependencyKind::DataFlow,
        })
        .unwrap();
    // anchor --[Calls]--> sym_300
    graph
        .add_edge(&DependencyEdge {
            source_id: 100,
            target_id: 300,
            kind: DependencyKind::Calls,
        })
        .unwrap();

    let hits = engine.reasoning_neighborhood(&graph, 100, 2, 10).unwrap();

    let data_flow_hit = hits
        .iter()
        .find(|h| h.symbol_id == 200)
        .expect("DataFlow neighbour must be discovered");
    let calls_hit = hits
        .iter()
        .find(|h| h.symbol_id == 300)
        .expect("Calls neighbour must be discovered");

    // Both are 1-hop; DataFlow weight (0.8) > Calls weight (0.6)
    assert_eq!(data_flow_hit.depth, 1);
    assert_eq!(calls_hit.depth, 1);
    assert!(
        data_flow_hit.score > calls_hit.score,
        "DataFlow score ({}) must exceed Calls score ({})",
        data_flow_hit.score,
        calls_hit.score
    );

    // Verify the exact scores match the formula: weight * hop_decay^1
    // DataFlow: 0.8 * 0.6 = 0.48
    // Calls:    0.6 * 0.6 = 0.36
    assert!(
        (data_flow_hit.score - 0.48).abs() < 1e-9,
        "DataFlow 1-hop score should be 0.48, got {}",
        data_flow_hit.score
    );
    assert!(
        (calls_hit.score - 0.36).abs() < 1e-9,
        "Calls 1-hop score should be 0.36, got {}",
        calls_hit.score
    );
}

// ---------------------------------------------------------------------------
// Test 6 – remove_edges_for_symbols: partial removal leaves unrelated edges
// ---------------------------------------------------------------------------

/// After removing edges for a specific symbol, unrelated edges in the graph
/// must remain intact and continue to produce valid GAR results.
#[test]
fn gar_partial_edge_removal_leaves_unrelated_edges_intact() {
    let graph = DependencyGraph::new();
    let engine = ReasoningEngine::default();

    // sym_A -> sym_B  (we will remove this)
    // sym_C -> sym_D  (must survive)
    for id in [10_i64, 20, 30, 40] {
        graph.add_symbol(id).unwrap();
    }
    graph
        .add_edge(&DependencyEdge {
            source_id: 10,
            target_id: 20,
            kind: DependencyKind::Calls,
        })
        .unwrap();
    graph
        .add_edge(&DependencyEdge {
            source_id: 30,
            target_id: 40,
            kind: DependencyKind::DataFlow,
        })
        .unwrap();

    assert_eq!(graph.edge_count(), 2);

    // Remove only sym_A (id=10)'s edges
    let removed = graph.remove_edges_for_symbols(&[10]);
    assert_eq!(removed, 1, "only the 10->20 edge should be removed");
    assert_eq!(graph.edge_count(), 1, "C->D edge must survive");

    // sym_A should now have an empty neighbourhood
    let hits_a = engine.reasoning_neighborhood(&graph, 10, 4, 50).unwrap();
    assert!(
        hits_a.is_empty(),
        "sym_A neighbourhood must be empty after edge removal"
    );

    // sym_C -> sym_D relationship must still produce GAR results
    let hits_c = engine.reasoning_neighborhood(&graph, 30, 4, 50).unwrap();
    assert!(
        hits_c.iter().any(|h| h.symbol_id == 40),
        "sym_D (id=40) must still be reachable from sym_C after partial removal"
    );
}

// ---------------------------------------------------------------------------
// Test 7 – GAR with no graph: search_with_gar returns empty neighbor map
// ---------------------------------------------------------------------------

/// When `dep_graph = None`, `search_with_gar` must still return search results
/// but the GAR neighbor map must be empty (no graph traversal possible).
#[test]
fn gar_no_graph_returns_empty_neighbor_map() {
    let fx = Fixture::new();

    insert_symbol_with_chunk(
        &fx.index,
        "standalone::fn_alpha",
        "pub fn fn_alpha() { /* no_graph_token */ }",
    );

    let (results, gar_neighbors) = fx
        .search
        .search_with_gar(
            "no_graph_token",
            10,
            &fx.index,
            &fx.vector_index,
            &fx.embedder,
            None, // no graph
            None, // no reasoning engine
            None,
            None,
            &[],  // open_files
            &[],  // sparse_results (no BGE-M3 in test fixture)
            None, // file_dep_graph
        )
        .expect("search_with_gar without graph");

    // Search should still find the keyword-matching chunk
    assert!(
        !results.is_empty(),
        "keyword search should still work without a graph"
    );

    // GAR map must be empty — no graph means no neighbours
    assert!(
        gar_neighbors.is_empty(),
        "GAR neighbor map must be empty when no graph is provided"
    );
}
