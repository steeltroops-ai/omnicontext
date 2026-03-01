//! OmniContext Search Relevance Benchmark Suite.
//!
//! Automated testing of search quality using golden test queries
//! with expected results. Measures MRR@K, Recall@K, and NDCG.

use std::path::Path;

/// A golden test case: query + expected relevant results.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GoldenQuery {
    /// The search query to execute.
    pub query: String,
    /// Expected relevant symbols/file paths (ground truth).
    pub expected: Vec<String>,
    /// Intent classification for this query.
    pub intent: Option<String>,
    /// Description of what this test validates.
    pub description: Option<String>,
}

/// Metrics from a benchmark run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BenchmarkResults {
    /// Total queries evaluated.
    pub total_queries: usize,
    /// Mean Reciprocal Rank at K.
    pub mrr_at_5: f64,
    /// Mean Reciprocal Rank at K=10.
    pub mrr_at_10: f64,
    /// Recall at K=5 (fraction of relevant results found in top 5).
    pub recall_at_5: f64,
    /// Recall at K=10.
    pub recall_at_10: f64,
    /// Normalized Discounted Cumulative Gain at K=10.
    pub ndcg_at_10: f64,
    /// Per-query details.
    pub per_query: Vec<QueryResult>,
}

/// Results for a single benchmark query.
#[derive(Debug, Clone, serde::Serialize)]
pub struct QueryResult {
    /// The query text.
    pub query: String,
    /// Rank of the first relevant result (1-indexed, 0 = not found).
    pub first_relevant_rank: usize,
    /// How many of the expected results appeared in top K.
    pub relevant_found: usize,
    /// Total expected relevant results.
    pub relevant_total: usize,
    /// Reciprocal rank (1/rank of first relevant, 0 if not found).
    pub reciprocal_rank: f64,
    /// Recall at K for this query.
    pub recall: f64,
    /// Whether the query was a pass (found at least one relevant result in top K).
    pub pass: bool,
    /// Elapsed time in milliseconds.
    pub elapsed_ms: u64,
}

/// Run the benchmark suite against an engine.
pub fn run_benchmark(
    engine: &omni_core::Engine,
    golden_queries: &[GoldenQuery],
    k: usize,
) -> BenchmarkResults {
    let mut per_query = Vec::with_capacity(golden_queries.len());
    let mut mrr_5_sum = 0.0;
    let mut mrr_10_sum = 0.0;
    let mut recall_5_sum = 0.0;
    let mut recall_10_sum = 0.0;
    let mut ndcg_10_sum = 0.0;

    for golden in golden_queries {
        let start = std::time::Instant::now();
        let results = engine.search(&golden.query, k).unwrap_or_default();
        let elapsed_ms = start.elapsed().as_millis() as u64;

        // Find which results are in the expected set
        let result_symbols: Vec<String> = results
            .iter()
            .map(|r| {
                format!(
                    "{}::{}",
                    r.file_path.display(),
                    r.chunk.symbol_path
                )
            })
            .collect();

        let mut first_relevant_rank = 0;
        let mut relevant_found = 0;
        let mut relevance_vec = Vec::new();

        for (i, sym) in result_symbols.iter().enumerate() {
            let is_relevant = golden.expected.iter().any(|exp| {
                sym.contains(exp) || exp.contains(sym.as_str())
            });
            relevance_vec.push(if is_relevant { 1.0 } else { 0.0 });

            if is_relevant {
                relevant_found += 1;
                if first_relevant_rank == 0 {
                    first_relevant_rank = i + 1;
                }
            }
        }

        let reciprocal_rank = if first_relevant_rank > 0 {
            1.0 / first_relevant_rank as f64
        } else {
            0.0
        };

        let recall = if golden.expected.is_empty() {
            1.0
        } else {
            relevant_found as f64 / golden.expected.len() as f64
        };

        // MRR@5
        let mrr_5 = if first_relevant_rank > 0 && first_relevant_rank <= 5 {
            1.0 / first_relevant_rank as f64
        } else {
            0.0
        };

        // MRR@10
        let mrr_10 = if first_relevant_rank > 0 && first_relevant_rank <= 10 {
            1.0 / first_relevant_rank as f64
        } else {
            0.0
        };

        // Recall@5
        let relevant_in_5 = relevance_vec.iter().take(5).filter(|&&v| v > 0.0).count();
        let recall_5 = if golden.expected.is_empty() {
            1.0
        } else {
            relevant_in_5 as f64 / golden.expected.len() as f64
        };

        // Recall@10
        let relevant_in_10 = relevance_vec.iter().take(10).filter(|&&v| v > 0.0).count();
        let recall_10 = if golden.expected.is_empty() {
            1.0
        } else {
            relevant_in_10 as f64 / golden.expected.len() as f64
        };

        // NDCG@10
        let ndcg_10 = compute_ndcg(&relevance_vec, 10);

        mrr_5_sum += mrr_5;
        mrr_10_sum += mrr_10;
        recall_5_sum += recall_5;
        recall_10_sum += recall_10;
        ndcg_10_sum += ndcg_10;

        per_query.push(QueryResult {
            query: golden.query.clone(),
            first_relevant_rank,
            relevant_found,
            relevant_total: golden.expected.len(),
            reciprocal_rank,
            recall,
            pass: relevant_found > 0,
            elapsed_ms,
        });
    }

    let n = golden_queries.len() as f64;
    BenchmarkResults {
        total_queries: golden_queries.len(),
        mrr_at_5: mrr_5_sum / n,
        mrr_at_10: mrr_10_sum / n,
        recall_at_5: recall_5_sum / n,
        recall_at_10: recall_10_sum / n,
        ndcg_at_10: ndcg_10_sum / n,
        per_query,
    }
}

/// Compute Normalized Discounted Cumulative Gain at K.
fn compute_ndcg(relevance: &[f64], k: usize) -> f64 {
    let dcg = relevance
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, &rel)| rel / (2.0_f64 + i as f64).log2())
        .sum::<f64>();

    // Ideal DCG: all relevant results at the top
    let mut ideal: Vec<f64> = relevance.to_vec();
    ideal.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let idcg = ideal
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, &rel)| rel / (2.0_f64 + i as f64).log2())
        .sum::<f64>();

    if idcg == 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

/// Load golden queries from a JSON file.
pub fn load_golden_queries(path: &Path) -> Result<Vec<GoldenQuery>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let queries: Vec<GoldenQuery> = serde_json::from_str(&content)?;
    Ok(queries)
}
