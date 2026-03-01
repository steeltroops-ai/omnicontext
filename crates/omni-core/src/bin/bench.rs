//! Benchmarks for OmniContext core operations.
//!
//! Run with: cargo bench --package omni-core

use std::path::PathBuf;
use std::time::Instant;

use omni_core::vector::{l2_normalize, VectorIndex};

/// Generate a deterministic pseudo-random vector.
fn random_vector(dim: usize, seed: u64) -> Vec<f32> {
    let mut vec = Vec::with_capacity(dim);
    let mut state = seed;
    for _ in 0..dim {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        vec.push(((state >> 33) as f32) / (u32::MAX as f32) - 0.5);
    }
    l2_normalize(&mut vec);
    vec
}

fn bench_vector_search(n: usize, dim: usize, k: usize) -> f64 {
    let mut index = VectorIndex::in_memory(dim);
    for i in 0..n {
        let v = random_vector(dim, i as u64);
        index.add(i as u64, &v).expect("add");
    }

    let query = random_vector(dim, 42);
    let start = Instant::now();
    let iters = 100;
    for _ in 0..iters {
        let _ = index.search(&query, k).expect("search");
    }
    let elapsed = start.elapsed();
    elapsed.as_secs_f64() / iters as f64 * 1000.0 // ms per search
}

fn bench_vector_insert(n: usize, dim: usize) -> f64 {
    let mut index = VectorIndex::in_memory(dim);
    let vectors: Vec<Vec<f32>> = (0..n).map(|i| random_vector(dim, i as u64)).collect();

    let start = Instant::now();
    for (i, v) in vectors.iter().enumerate() {
        index.add(i as u64, v).expect("add");
    }
    let elapsed = start.elapsed();
    elapsed.as_secs_f64() * 1000.0 // total ms
}

fn bench_index_operations() -> (f64, f64) {
    use omni_core::index::MetadataIndex;
    use omni_core::types::*;

    // Use a unique temp path to avoid collisions
    let db_path = std::env::temp_dir().join(format!("omni_bench_{}.db", std::process::id()));
    // Clean up any previous run
    let _ = std::fs::remove_file(&db_path);
    let index = MetadataIndex::open(&db_path).expect("open");

    let file = FileInfo {
        id: 0,
        path: PathBuf::from("bench/test.rs"),
        language: Language::Rust,
        content_hash: "abc123".into(),
        size_bytes: 1000,
    };

    // Bench file upsert
    let start = Instant::now();
    let iters = 1000;
    for _ in 0..iters {
        let _ = index.upsert_file(&file);
    }
    let upsert_ms = start.elapsed().as_secs_f64() * 1000.0 / iters as f64;

    // Prep chunks for keyword search bench
    let file_id = index.upsert_file(&file).expect("upsert");
    let mut chunks = Vec::new();
    for i in 0..500 {
        chunks.push(Chunk {
            id: 0,
            file_id,
            symbol_path: format!("bench::func_{i}"),
            kind: ChunkKind::Function,
            visibility: Visibility::Public,
            line_start: i * 10,
            line_end: i * 10 + 9,
            content: format!("fn func_{i}(x: i32) -> i32 {{ x + {i} }}"),
            doc_comment: Some(format!("Documentation for function {i}")),
            token_count: 20,
            weight: 1.0,
            vector_id: None,
        });
    }
    let symbols: Vec<Symbol> = chunks
        .iter()
        .map(|c| Symbol {
            id: 0,
            name: c.symbol_path.split("::").last().unwrap_or("").into(),
            fqn: c.symbol_path.clone(),
            kind: c.kind,
            file_id,
            line: c.line_start as u32,
            chunk_id: None,
        })
        .collect();

    let _ = index.reindex_file(&file, &chunks, &symbols);

    // Bench keyword search
    let start = Instant::now();
    let search_iters = 100;
    for _ in 0..search_iters {
        let _ = index.keyword_search("func", 10);
    }
    let search_ms = start.elapsed().as_secs_f64() * 1000.0 / search_iters as f64;

    // Cleanup
    let _ = std::fs::remove_file(&db_path);

    (upsert_ms, search_ms)
}

fn main() {
    println!("=== OmniContext Benchmarks ===");
    println!();

    // Vector search benchmarks
    println!("--- Vector Search ---");
    for &n in &[1_000, 10_000, 50_000] {
        let ms = bench_vector_search(n, 384, 10);
        println!(
            "  {n:>6} vectors, dim=384, k=10: {ms:.3}ms/query  ({:.0} qps)",
            1000.0 / ms
        );
    }
    println!();

    // Vector insert benchmarks
    println!("--- Vector Insert ---");
    for &n in &[1_000, 10_000, 50_000] {
        let ms = bench_vector_insert(n, 384);
        println!(
            "  {n:>6} vectors, dim=384: {ms:.1}ms total  ({:.0} inserts/sec)",
            n as f64 / (ms / 1000.0)
        );
    }
    println!();

    // Index benchmarks
    println!("--- SQLite Index ---");
    let (upsert_ms, search_ms) = bench_index_operations();
    println!(
        "  File upsert:     {upsert_ms:.3}ms/op  ({:.0} ops/sec)",
        1000.0 / upsert_ms
    );
    println!(
        "  Keyword search:  {search_ms:.3}ms/query  ({:.0} qps)",
        1000.0 / search_ms
    );
    println!();

    // Print summary line for CI parsing
    let search_10k = bench_vector_search(10_000, 384, 10);
    println!(
        "BENCHMARK_RESULT: vector_search_10k={search_10k:.3}ms keyword_search={search_ms:.3}ms"
    );
}
