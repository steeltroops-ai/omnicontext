//! Comprehensive benchmark suite for `OmniContext`.
//!
//! Measures:
//! 1. Low-level performance (vector search, `SQLite` operations)
//! 2. Embedding coverage
//! 3. Reranker performance
//! 4. End-to-end indexing and search
//!
//! Run with: `cargo run --package omni-core --bin benchmark [repo_path]`

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::ptr_arg)]

use std::path::PathBuf;
use std::time::Instant;

use omni_core::vector::{l2_normalize, VectorIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OmniContext Comprehensive Benchmark Suite ===\n");

    // Get optional repository path for high-level benchmarks
    let repo_path = std::env::args().nth(1).map(PathBuf::from);

    // Run low-level benchmarks (always)
    println!("--- Part 1: Low-Level Performance ---\n");
    bench_vector_operations();
    bench_index_operations()?;

    // Run high-level benchmarks (if repo provided)
    if let Some(ref path) = repo_path {
        if path.exists() {
            println!("\n--- Part 2: High-Level Benchmarks ---\n");
            println!("Repository: {}\n", path.display());
            bench_embedding_coverage(path)?;
            bench_reranker_performance()?;
        } else {
            println!("\n⚠️  Repository not found: {}", path.display());
            println!("Skipping high-level benchmarks");
        }
    } else {
        println!("\n⚠️  No repository path provided");
        println!("Usage: cargo run --bin benchmark [repo_path]");
        println!("Skipping high-level benchmarks");
    }

    println!("\n=== Benchmark Complete ===");
    Ok(())
}

// ============================================================================
// Part 1: Low-Level Performance Benchmarks
// ============================================================================

fn bench_vector_operations() {
    println!("Vector Search Performance:");
    for &n in &[1_000, 10_000, 50_000] {
        let ms = bench_vector_search(n, 384, 10);
        println!(
            "  {:>6} vectors, dim=384, k=10: {:.3}ms/query ({:.0} qps)",
            n,
            ms,
            1000.0 / ms
        );
    }

    println!("\nVector Insert Performance:");
    for &n in &[1_000, 10_000, 50_000] {
        let ms = bench_vector_insert(n, 384);
        println!(
            "  {:>6} vectors, dim=384: {:.1}ms total ({:.0} inserts/sec)",
            n,
            ms,
            (n as f64) / (ms / 1000.0)
        );
    }
    println!();
}

fn bench_vector_search(n: usize, dim: usize, k: usize) -> f64 {
    let mut index = VectorIndex::in_memory(dim);
    for i in 0..n {
        let v = random_vector(dim, i as u64);
        index.add(i as u64, &v).unwrap_or_else(|e| {
            eprintln!("Failed to add vector: {e}");
        });
    }

    let query = random_vector(dim, 42);
    let start = Instant::now();
    let iters = 100;
    for _ in 0..iters {
        let _ = index.search(&query, k);
    }
    let elapsed = start.elapsed();
    elapsed.as_secs_f64() / f64::from(iters) * 1000.0
}

fn bench_vector_insert(n: usize, dim: usize) -> f64 {
    let mut index = VectorIndex::in_memory(dim);
    let vectors: Vec<Vec<f32>> = (0..n).map(|i| random_vector(dim, i as u64)).collect();

    let start = Instant::now();
    for (i, v) in vectors.iter().enumerate() {
        index.add(i as u64, v).unwrap_or_else(|e| {
            eprintln!("Failed to add vector {i}: {e}");
        });
    }
    let elapsed = start.elapsed();
    elapsed.as_secs_f64() * 1000.0
}

fn bench_index_operations() -> Result<(), Box<dyn std::error::Error>> {
    use omni_core::index::MetadataIndex;
    use omni_core::types::{Chunk, ChunkKind, FileInfo, Language, Symbol, Visibility};

    println!("SQLite Index Performance:");

    let db_path = std::env::temp_dir().join(format!("omni_bench_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&db_path);
    let index = MetadataIndex::open(&db_path)?;

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
    let upsert_ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(iters);

    // Prep chunks for keyword search
    let file_id = index.upsert_file(&file)?;
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
            line: c.line_start,
            chunk_id: None,
        })
        .collect();

    index.reindex_file(&file, &chunks, &symbols)?;

    // Bench keyword search
    let start = Instant::now();
    let search_iters = 100;
    for _ in 0..search_iters {
        let _ = index.keyword_search("func", 10);
    }
    let search_ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(search_iters);

    println!(
        "  File upsert:     {:.3}ms/op ({:.0} ops/sec)",
        upsert_ms,
        1000.0 / upsert_ms
    );
    println!(
        "  Keyword search:  {:.3}ms/query ({:.0} qps)",
        search_ms,
        1000.0 / search_ms
    );

    // Cleanup
    let _ = std::fs::remove_file(&db_path);

    // Print summary for CI
    let search_10k = bench_vector_search(10_000, 384, 10);
    println!(
        "\nBENCHMARK_RESULT: vector_search_10k={:.3}ms keyword_search={:.3}ms",
        search_10k, search_ms
    );

    Ok(())
}

// ============================================================================
// Part 2: High-Level Benchmarks
// ============================================================================

fn bench_embedding_coverage(repo_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use omni_core::config::Config;
    use omni_core::pipeline::Engine;

    println!("Embedding Coverage Test:");
    println!("  Creating engine and indexing repository...");

    let config = Config::load(repo_path)?;
    let mut engine = Engine::with_config(config)?;

    let start = Instant::now();
    let result = tokio::runtime::Runtime::new()?.block_on(engine.run_index())?;
    let duration = start.elapsed();

    println!("\n  Indexing Results:");
    println!("    Files processed: {}", result.files_processed);
    println!("    Files failed: {}", result.files_failed);
    println!("    Chunks created: {}", result.chunks_created);
    println!("    Symbols extracted: {}", result.symbols_extracted);
    println!("    Embeddings generated: {}", result.embeddings_generated);
    println!("    Duration: {:.2}s", duration.as_secs_f64());

    let coverage = if result.chunks_created > 0 {
        (result.embeddings_generated as f64 / result.chunks_created as f64) * 100.0
    } else {
        0.0
    };

    println!("\n  Embedding Coverage: {coverage:.2}%");

    let status = engine.status()?;
    println!("\n  Engine Status:");
    println!("    Chunks indexed: {}", status.chunks_indexed);
    println!("    Vectors indexed: {}", status.vectors_indexed);
    println!("    Coverage: {:.2}%", status.embedding_coverage_percent);
    println!("    Search mode: {}", status.search_mode);
    println!("    Graph nodes: {}", status.graph_nodes);
    println!("    Graph edges: {}", status.graph_edges);

    if coverage >= 95.0 {
        println!("\n  ✅ PASS: Embedding coverage >= 95%");
    } else if coverage >= 80.0 {
        println!("\n  ⚠️  WARN: Embedding coverage {coverage:.2}% (Target: 100%)");
    } else {
        println!("\n  ❌ FAIL: Embedding coverage {coverage:.2}% < 80%");
    }

    println!("\nBENCHMARK_RESULT: embedding_coverage={coverage:.2}");
    Ok(())
}

fn bench_reranker_performance() -> Result<(), Box<dyn std::error::Error>> {
    use omni_core::config::RerankerConfig;
    use omni_core::reranker::Reranker;

    println!("\nReranker Performance Test:");

    let config = RerankerConfig::default();
    let reranker = Reranker::new(&config)?;

    if !reranker.is_available() {
        println!("  ⚠️  Reranker model not available");
        println!("     Set OMNI_SKIP_MODEL_DOWNLOAD=0 to enable");
        return Ok(());
    }

    println!("  ✅ Reranker model loaded");

    let query = "how to implement error handling";
    let documents = vec![
        "def handle_error(e): raise Exception(e)",
        "class ErrorHandler: pass",
        "import logging; logger = logging.getLogger()",
        "def process_data(x): return x * 2",
        "try: result = compute() except: pass",
    ];

    let start = Instant::now();
    let scores = reranker.rerank(query, &documents);
    let duration = start.elapsed();

    println!("\n  Reranking Results:");
    println!("    Query: \"{query}\"");
    println!("    Documents: {}", documents.len());
    println!("    Duration: {:.2}ms", duration.as_secs_f64() * 1000.0);

    let valid_scores: Vec<f32> = scores.iter().filter_map(|s| *s).collect();
    if valid_scores.len() >= documents.len() / 2 {
        println!(
            "\n  ✅ PASS: Reranker scored {}/{} documents",
            valid_scores.len(),
            documents.len()
        );
    } else {
        println!(
            "\n  ⚠️  WARN: Reranker only scored {}/{} documents",
            valid_scores.len(),
            documents.len()
        );
    }

    println!(
        "\nBENCHMARK_RESULT: reranker_latency_ms={:.2}",
        duration.as_secs_f64() * 1000.0
    );
    Ok(())
}

// ============================================================================
// Utilities
// ============================================================================

/// Generate a deterministic pseudo-random vector.
fn random_vector(dim: usize, seed: u64) -> Vec<f32> {
    let mut vec = Vec::with_capacity(dim);
    let mut state = seed;
    for _ in 0..dim {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        vec.push(((state >> 33) as f32) / (u32::MAX as f32) - 0.5);
    }
    l2_normalize(&mut vec);
    vec
}
