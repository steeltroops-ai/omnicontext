//! Minimal benchmark suite for OmniContext core operations.
//!
//! Benchmarks critical paths that are easy to test:
//! - Graph queries (in-memory, no dependencies)
//! - Keyword search (FTS5)
//!
//! Run with: `cargo bench --package omni-core`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use omni_core::graph::dependencies::{DependencyEdge, EdgeType, FileDependencyGraph};
use std::path::PathBuf;

/// Benchmark graph query latency.
fn bench_graph_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_queries");

    let graph = FileDependencyGraph::new();

    // Build a sample graph with 100 files
    for i in 0..100 {
        let file = PathBuf::from(format!("src/file_{}.rs", i));
        let _ = graph.add_file(file.clone(), "rust".to_string());

        // Add some edges
        if i > 0 {
            let prev_file = PathBuf::from(format!("src/file_{}.rs", i - 1));
            let edge = DependencyEdge {
                source: file.clone(),
                target: prev_file,
                edge_type: EdgeType::Imports,
                weight: 1.0,
            };
            let _ = graph.add_edge(edge);
        }
    }

    let focal_file = PathBuf::from("src/file_50.rs");

    for hops in [1, 2, 3].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_hops", hops)),
            hops,
            |b, hops| {
                b.iter(|| {
                    let neighbors = graph
                        .get_neighbors(black_box(&focal_file), black_box(*hops))
                        .unwrap();
                    black_box(neighbors)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_graph_queries);
criterion_main!(benches);
