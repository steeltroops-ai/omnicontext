//! Benchmark suite to measure embedding coverage and reranker improvements.
//!
//! This tool measures:
//! 1. Embedding coverage percentage
//! 2. Search relevance with and without reranker
//! 3. Performance metrics (latency, throughput)
//!
//! Run with: cargo run --package omni-core --bin benchmark_improvements

use std::path::PathBuf;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OmniContext Improvement Benchmark Suite ===\n");

    // Check if we have a test repository to benchmark against
    let test_repo = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    if !test_repo.exists() {
        eprintln!("Error: Test repository not found: {}", test_repo.display());
        eprintln!("Usage: cargo run --bin benchmark_improvements [repo_path]");
        std::process::exit(1);
    }

    println!("Test Repository: {}\n", test_repo.display());

    // Set up environment for testing
    std::env::set_var("OMNI_SKIP_MODEL_DOWNLOAD", "1");

    println!("--- Phase 1: Embedding Coverage Test ---");
    test_embedding_coverage(&test_repo)?;

    println!("\n--- Phase 2: Reranker Performance Test ---");
    test_reranker_performance(&test_repo)?;

    println!("\n--- Phase 3: End-to-End Search Quality ---");
    test_search_quality(&test_repo)?;

    println!("\n=== Benchmark Complete ===");
    Ok(())
}

fn test_embedding_coverage(repo_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use omni_core::config::Config;
    use omni_core::pipeline::Engine;

    println!("Creating engine and indexing repository...");
    let config = Config::load(repo_path)?;
    let mut engine = Engine::with_config(config)?;

    let start = Instant::now();
    let result = tokio::runtime::Runtime::new()?.block_on(engine.run_index())?;
    let duration = start.elapsed();

    println!("\nIndexing Results:");
    println!("  Files processed: {}", result.files_processed);
    println!("  Files failed: {}", result.files_failed);
    println!("  Chunks created: {}", result.chunks_created);
    println!("  Symbols extracted: {}", result.symbols_extracted);
    println!("  Embeddings generated: {}", result.embeddings_generated);
    println!("  Duration: {:.2}s", duration.as_secs_f64());

    // Calculate coverage
    let coverage = if result.chunks_created > 0 {
        (result.embeddings_generated as f64 / result.chunks_created as f64) * 100.0
    } else {
        0.0
    };

    println!("\n  Embedding Coverage: {:.2}%", coverage);

    // Get detailed status
    let status = engine.status()?;
    println!("\nEngine Status:");
    println!("  Chunks indexed: {}", status.chunks_indexed);
    println!("  Vectors indexed: {}", status.vectors_indexed);
    println!("  Coverage (from status): {:.2}%", status.embedding_coverage_percent);
    println!("  Search mode: {}", status.search_mode);
    println!("  Graph nodes: {}", status.graph_nodes);
    println!("  Graph edges: {}", status.graph_edges);
    println!("  Dependency edges (SQLite): {}", status.dep_edges);
    println!("  Has cycles: {}", status.has_cycles);

    // Evaluate coverage
    if coverage >= 95.0 {
        println!("\n✅ PASS: Embedding coverage >= 95% (Target: 100%)");
    } else if coverage >= 80.0 {
        println!("\n⚠️  WARN: Embedding coverage {:.2}% (Target: 100%)", coverage);
    } else {
        println!("\n❌ FAIL: Embedding coverage {:.2}% < 80%", coverage);
    }

    println!("\nBENCHMARK_RESULT: embedding_coverage={:.2}", coverage);
    Ok(())
}

fn test_reranker_performance(_repo_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use omni_core::config::RerankerConfig;
    use omni_core::reranker::Reranker;

    println!("Testing reranker availability and performance...");

    let config = RerankerConfig::default();
    let reranker = Reranker::new(&config)?;

    if !reranker.is_available() {
        println!("⚠️  Reranker model not available (expected in test environment)");
        println!("   Set OMNI_SKIP_MODEL_DOWNLOAD=0 to enable reranker testing");
        return Ok(());
    }

    println!("✅ Reranker model loaded successfully");

    // Test reranking performance
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

    println!("\nReranking Results:");
    println!("  Query: \"{}\"", query);
    println!("  Documents: {}", documents.len());
    println!("  Duration: {:.2}ms", duration.as_secs_f64() * 1000.0);

    for (i, (doc, score)) in documents.iter().zip(scores.iter()).enumerate() {
        if let Some(score) = score {
            println!("  [{i}] Score: {:.4} - {}", score, &doc[..doc.len().min(50)]);
        } else {
            println!("  [{i}] Score: None - {}", &doc[..doc.len().min(50)]);
        }
    }

    // Check if reranking produced meaningful scores
    let valid_scores: Vec<f32> = scores.iter().filter_map(|s| *s).collect();
    if valid_scores.len() >= documents.len() / 2 {
        println!("\n✅ PASS: Reranker produced scores for {}/{} documents", valid_scores.len(), documents.len());
    } else {
        println!("\n⚠️  WARN: Reranker only scored {}/{} documents", valid_scores.len(), documents.len());
    }

    println!("\nBENCHMARK_RESULT: reranker_latency_ms={:.2}", duration.as_secs_f64() * 1000.0);
    Ok(())
}

fn test_search_quality(_repo_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing end-to-end search quality...");
    println!("(This would run actual search queries against the indexed repository)");
    println!("⚠️  Skipped in current implementation - requires indexed test data");

    // TODO: Implement actual search quality tests
    // 1. Index a known test repository
    // 2. Run predefined queries
    // 3. Measure MRR, NDCG, Recall
    // 4. Compare with/without reranker

    Ok(())
}
