//! Search quality benchmarks using golden query dataset.
//!
//! Measures MRR, NDCG, Recall@K, and Precision@K against expected results.
//!
//! Run with: cargo test --test search_quality_bench -- --nocapture --ignored

use omni_core::config::Config;
use omni_core::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Golden query dataset structure.
#[derive(Debug, Deserialize)]
struct GoldenDataset {
    version: String,
    description: String,
    queries: Vec<GoldenQuery>,
    metadata: DatasetMetadata,
}

#[derive(Debug, Deserialize)]
struct DatasetMetadata {
    total_queries: usize,
    intents: HashMap<String, usize>,
    coverage: HashMap<String, usize>,
}

/// A single golden query with expected results.
#[derive(Debug, Deserialize)]
struct GoldenQuery {
    id: String,
    intent: String,
    query: String,
    expected_results: Vec<ExpectedResult>,
}

#[derive(Debug, Deserialize)]
struct ExpectedResult {
    symbol_path: String,
    relevance: u32, // 3=highly relevant, 2=relevant, 1=marginally relevant
    reason: String,
}

/// Benchmark results for a single query.
#[derive(Debug, Serialize)]
struct QueryResult {
    query_id: String,
    query: String,
    intent: String,
    reciprocal_rank: f64,
    ndcg_at_10: f64,
    recall_at_10: f64,
    precision_at_10: f64,
    found_count: usize,
    expected_count: usize,
}

/// Aggregate benchmark results.
#[derive(Debug, Serialize)]
struct BenchmarkResults {
    mrr: f64,             // Mean Reciprocal Rank
    ndcg_at_10: f64,      // Normalized Discounted Cumulative Gain @ 10
    recall_at_10: f64,    // Recall @ 10
    precision_at_10: f64, // Precision @ 10
    total_queries: usize,
    per_query: Vec<QueryResult>,
}

/// Run search quality benchmarks.
///
/// This test is marked as #[ignore] because it requires a fully indexed repository.
/// Run with: cargo test --test search_quality_bench -- --nocapture --ignored
#[test]
#[ignore]
fn benchmark_search_quality() {
    let repo_path = std::env::var("OMNI_TEST_REPO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    println!(
        "Running search quality benchmarks on: {}",
        repo_path.display()
    );
    println!("{}", "=".repeat(80));

    let results = run_benchmarks(&repo_path).expect("Benchmarks failed");

    // Print results
    println!("\n{}", "=".repeat(80));
    println!("BENCHMARK RESULTS");
    println!("{}", "=".repeat(80));
    println!("MRR (Mean Reciprocal Rank):     {:.4}", results.mrr);
    println!("NDCG@10:                         {:.4}", results.ndcg_at_10);
    println!(
        "Recall@10:                       {:.4}",
        results.recall_at_10
    );
    println!(
        "Precision@10:                    {:.4}",
        results.precision_at_10
    );
    println!("Total Queries:                   {}", results.total_queries);
    println!("{}", "=".repeat(80));

    // Print per-query results
    println!("\nPER-QUERY RESULTS:");
    println!("{}", "-".repeat(80));
    for query_result in &results.per_query {
        println!(
            "{} [{}]: RR={:.3}, NDCG={:.3}, Recall={:.3}, Precision={:.3} ({}/{})",
            query_result.query_id,
            query_result.intent,
            query_result.reciprocal_rank,
            query_result.ndcg_at_10,
            query_result.recall_at_10,
            query_result.precision_at_10,
            query_result.found_count,
            query_result.expected_count
        );
    }

    // Save results to JSON
    let output_path = PathBuf::from("tests/bench/results.json");
    let results_json = serde_json::to_string_pretty(&results).expect("JSON serialization failed");
    std::fs::write(&output_path, results_json).expect("Failed to write results");
    println!("\nResults saved to: {}", output_path.display());

    // Check if we meet targets
    println!("\n{}", "=".repeat(80));
    println!("TARGET COMPARISON:");
    println!("{}", "=".repeat(80));

    let mrr_target = 0.75;
    let ndcg_target = 0.70;
    let recall_target = 0.85;

    print_target_status("MRR", results.mrr, mrr_target);
    print_target_status("NDCG@10", results.ndcg_at_10, ndcg_target);
    print_target_status("Recall@10", results.recall_at_10, recall_target);

    // Assert minimum thresholds (50% of target)
    assert!(
        results.mrr >= mrr_target * 0.5,
        "MRR too low: {:.4} < {:.4}",
        results.mrr,
        mrr_target * 0.5
    );
    assert!(
        results.ndcg_at_10 >= ndcg_target * 0.5,
        "NDCG@10 too low: {:.4} < {:.4}",
        results.ndcg_at_10,
        ndcg_target * 0.5
    );
}

fn run_benchmarks(repo_path: &PathBuf) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
    // Load golden dataset
    let dataset_path = PathBuf::from("tests/bench/golden_queries.json");
    let dataset_json = std::fs::read_to_string(&dataset_path)?;
    let dataset: GoldenDataset = serde_json::from_str(&dataset_json)?;

    println!("Loaded {} golden queries", dataset.queries.len());

    // Initialize engine (assumes repository is already indexed)
    let config = Config::defaults(repo_path);
    let engine = Engine::with_config(config)?;

    println!("Engine initialized, running benchmarks...");

    // Run benchmarks
    let mut query_results = Vec::new();
    let mut total_rr = 0.0;
    let mut total_ndcg = 0.0;
    let mut total_recall = 0.0;
    let mut total_precision = 0.0;

    for golden_query in &dataset.queries {
        println!("Testing query: {}", golden_query.query);

        // Execute search using Engine's search method
        let results = engine.search(&golden_query.query, 10)?;

        // Extract symbol paths from results
        let result_symbols: Vec<String> = results
            .iter()
            .map(|r| r.chunk.symbol_path.clone())
            .collect();

        // Build relevance map
        let mut relevance_map: HashMap<String, u32> = HashMap::new();
        for expected in &golden_query.expected_results {
            relevance_map.insert(expected.symbol_path.clone(), expected.relevance);
        }

        // Calculate metrics
        let rr = calculate_reciprocal_rank(&result_symbols, &relevance_map);
        let ndcg = calculate_ndcg(&result_symbols, &relevance_map, 10);
        let recall = calculate_recall(&result_symbols, &relevance_map, 10);
        let precision = calculate_precision(&result_symbols, &relevance_map, 10);

        let found_count = result_symbols
            .iter()
            .filter(|s| relevance_map.contains_key(*s))
            .count();

        query_results.push(QueryResult {
            query_id: golden_query.id.clone(),
            query: golden_query.query.clone(),
            intent: golden_query.intent.clone(),
            reciprocal_rank: rr,
            ndcg_at_10: ndcg,
            recall_at_10: recall,
            precision_at_10: precision,
            found_count,
            expected_count: golden_query.expected_results.len(),
        });

        total_rr += rr;
        total_ndcg += ndcg;
        total_recall += recall;
        total_precision += precision;
    }

    let n = dataset.queries.len() as f64;
    let results = BenchmarkResults {
        mrr: total_rr / n,
        ndcg_at_10: total_ndcg / n,
        recall_at_10: total_recall / n,
        precision_at_10: total_precision / n,
        total_queries: dataset.queries.len(),
        per_query: query_results,
    };

    Ok(results)
}

/// Calculate Mean Reciprocal Rank (MRR).
///
/// MRR = 1 / rank of first relevant result
fn calculate_reciprocal_rank(results: &[String], relevance_map: &HashMap<String, u32>) -> f64 {
    for (i, symbol) in results.iter().enumerate() {
        if relevance_map.contains_key(symbol) {
            return 1.0 / (i as f64 + 1.0);
        }
    }
    0.0
}

/// Calculate Normalized Discounted Cumulative Gain (NDCG@K).
///
/// NDCG = DCG / IDCG
/// DCG = sum((2^rel - 1) / log2(i + 2)) for i in 0..k
/// IDCG = DCG of perfect ranking
fn calculate_ndcg(results: &[String], relevance_map: &HashMap<String, u32>, k: usize) -> f64 {
    let k = k.min(results.len());

    // Calculate DCG
    let mut dcg = 0.0;
    for (i, symbol) in results.iter().take(k).enumerate() {
        let rel = relevance_map.get(symbol).copied().unwrap_or(0);
        let gain = (2_u32.pow(rel) - 1) as f64;
        let discount = (2.0 + i as f64).log2();
        dcg += gain / discount;
    }

    // Calculate IDCG (ideal DCG with perfect ranking)
    let mut ideal_rels: Vec<u32> = relevance_map.values().copied().collect();
    ideal_rels.sort_by(|a, b| b.cmp(a)); // Sort descending

    let mut idcg = 0.0;
    for (i, &rel) in ideal_rels.iter().take(k).enumerate() {
        let gain = (2_u32.pow(rel) - 1) as f64;
        let discount = (2.0 + i as f64).log2();
        idcg += gain / discount;
    }

    if idcg == 0.0 {
        return 0.0;
    }

    dcg / idcg
}

/// Calculate Recall@K.
///
/// Recall = (relevant results in top-K) / (total relevant results)
fn calculate_recall(results: &[String], relevance_map: &HashMap<String, u32>, k: usize) -> f64 {
    let k = k.min(results.len());
    let found = results
        .iter()
        .take(k)
        .filter(|s| relevance_map.contains_key(*s))
        .count();
    let total = relevance_map.len();

    if total == 0 {
        return 0.0;
    }

    found as f64 / total as f64
}

/// Calculate Precision@K.
///
/// Precision = (relevant results in top-K) / K
fn calculate_precision(results: &[String], relevance_map: &HashMap<String, u32>, k: usize) -> f64 {
    let k = k.min(results.len());
    let found = results
        .iter()
        .take(k)
        .filter(|s| relevance_map.contains_key(*s))
        .count();

    if k == 0 {
        return 0.0;
    }

    found as f64 / k as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reciprocal_rank_first_position() {
        let results = vec!["relevant".to_string(), "other".to_string()];
        let mut relevance = HashMap::new();
        relevance.insert("relevant".to_string(), 3);

        let rr = calculate_reciprocal_rank(&results, &relevance);
        assert!((rr - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_reciprocal_rank_second_position() {
        let results = vec!["other".to_string(), "relevant".to_string()];
        let mut relevance = HashMap::new();
        relevance.insert("relevant".to_string(), 3);

        let rr = calculate_reciprocal_rank(&results, &relevance);
        assert!((rr - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_reciprocal_rank_not_found() {
        let results = vec!["other1".to_string(), "other2".to_string()];
        let mut relevance = HashMap::new();
        relevance.insert("relevant".to_string(), 3);

        let rr = calculate_reciprocal_rank(&results, &relevance);
        assert_eq!(rr, 0.0);
    }

    #[test]
    fn test_recall_perfect() {
        let results = vec!["rel1".to_string(), "rel2".to_string()];
        let mut relevance = HashMap::new();
        relevance.insert("rel1".to_string(), 3);
        relevance.insert("rel2".to_string(), 2);

        let recall = calculate_recall(&results, &relevance, 10);
        assert!((recall - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_recall_partial() {
        let results = vec!["rel1".to_string(), "other".to_string()];
        let mut relevance = HashMap::new();
        relevance.insert("rel1".to_string(), 3);
        relevance.insert("rel2".to_string(), 2);

        let recall = calculate_recall(&results, &relevance, 10);
        assert!((recall - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_precision_perfect() {
        let results = vec!["rel1".to_string(), "rel2".to_string()];
        let mut relevance = HashMap::new();
        relevance.insert("rel1".to_string(), 3);
        relevance.insert("rel2".to_string(), 2);

        let precision = calculate_precision(&results, &relevance, 2);
        assert!((precision - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_precision_partial() {
        let results = vec!["rel1".to_string(), "other".to_string()];
        let mut relevance = HashMap::new();
        relevance.insert("rel1".to_string(), 3);

        let precision = calculate_precision(&results, &relevance, 2);
        assert!((precision - 0.5).abs() < 1e-6);
    }
}

fn print_target_status(metric: &str, actual: f64, target: f64) {
    let status = if actual >= target {
        "✅ PASS"
    } else if actual >= target * 0.8 {
        "⚠️  WARN"
    } else {
        "❌ FAIL"
    };

    let percentage = (actual / target * 100.0).min(100.0);
    println!(
        "{:12} {:.4} / {:.4} ({:5.1}%) {}",
        metric, actual, target, percentage, status
    );
}
