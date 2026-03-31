//! NDCG@10 search quality evaluator.
//!
//! Loads a golden query dataset, runs each query through a real Engine::search(),
//! and reports per-query and aggregate NDCG@10, MRR, and Recall@10.
//!
//! Run:
//!   cargo run --package omni-core --bin eval -- --repo . --queries crates/omni-core/tests/fixtures/eval_queries.json
//!
//! Exit code 1 when aggregate NDCG@10 < 0.5 (minimum quality bar).

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::doc_markdown)]

use std::collections::HashMap;
use std::path::PathBuf;

use omni_core::config::Config;
use omni_core::Engine;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Golden dataset types (mirror the JSON schema)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GoldenDataset {
    queries: Vec<GoldenQuery>,
}

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
    relevance: u32,
    #[allow(dead_code)]
    reason: String,
}

// ---------------------------------------------------------------------------
// CLI argument parsing (manual — avoids pulling in clap)
// ---------------------------------------------------------------------------

struct Args {
    repo: PathBuf,
    queries: PathBuf,
}

fn parse_args() -> Result<Args, Box<dyn std::error::Error>> {
    let all: Vec<String> = std::env::args().skip(1).collect();
    let mut repo: Option<PathBuf> = None;
    let mut queries: Option<PathBuf> = None;
    let mut i = 0;

    while i < all.len() {
        match all[i].as_str() {
            "--repo" => {
                i += 1;
                repo = Some(all.get(i).ok_or("--repo requires a value")?.into());
            }
            "--queries" => {
                i += 1;
                queries = Some(all.get(i).ok_or("--queries requires a value")?.into());
            }
            other => {
                return Err(format!("unknown argument: {other}").into());
            }
        }
        i += 1;
    }

    Ok(Args {
        repo: repo.unwrap_or_else(|| PathBuf::from(".")),
        queries: queries
            .unwrap_or_else(|| PathBuf::from("crates/omni-core/tests/fixtures/eval_queries.json")),
    })
}

// ---------------------------------------------------------------------------
// Metric helpers
// ---------------------------------------------------------------------------

fn dcg_at_k(relevance_scores: &[u32], k: usize) -> f64 {
    relevance_scores
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, &rel)| {
            let gain = (2_u64.pow(rel) - 1) as f64;
            let discount = (2.0 + i as f64).ln() / std::f64::consts::LN_2;
            gain / discount
        })
        .sum()
}

fn ndcg_at_k(result_symbols: &[String], relevance_map: &HashMap<String, u32>, k: usize) -> f64 {
    let retrieved: Vec<u32> = result_symbols
        .iter()
        .take(k)
        .map(|s| relevance_map.get(s).copied().unwrap_or(0))
        .collect();
    let dcg = dcg_at_k(&retrieved, k);

    let mut ideal: Vec<u32> = relevance_map.values().copied().collect();
    ideal.sort_by(|a, b| b.cmp(a));
    let idcg = dcg_at_k(&ideal, k);

    if idcg == 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

fn reciprocal_rank(result_symbols: &[String], relevance_map: &HashMap<String, u32>) -> f64 {
    for (i, sym) in result_symbols.iter().enumerate() {
        if relevance_map.contains_key(sym) {
            return 1.0 / (i as f64 + 1.0);
        }
    }
    0.0
}

fn recall_at_k(result_symbols: &[String], relevance_map: &HashMap<String, u32>, k: usize) -> f64 {
    let total = relevance_map.len();
    if total == 0 {
        return 0.0;
    }
    let found = result_symbols
        .iter()
        .take(k)
        .filter(|s| relevance_map.contains_key(*s))
        .count();
    found as f64 / total as f64
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;

    println!("=== OmniContext NDCG@10 Evaluation ===");
    println!("  repo:    {}", args.repo.display());
    println!("  queries: {}", args.queries.display());
    println!();

    let json = std::fs::read_to_string(&args.queries)
        .map_err(|e| format!("cannot read {}: {e}", args.queries.display()))?;
    let dataset: GoldenDataset = serde_json::from_str(&json)
        .map_err(|e| format!("cannot parse {}: {e}", args.queries.display()))?;
    println!("  loaded {} queries", dataset.queries.len());
    println!();

    let config = Config::load(&args.repo)?;
    let engine = Engine::with_config(config)?;

    const K: usize = 10;
    let mut total_ndcg = 0.0_f64;
    let mut total_mrr = 0.0_f64;
    let mut total_recall = 0.0_f64;

    for q in &dataset.queries {
        let results = engine.search(&q.query, K).unwrap_or_default();

        let result_symbols: Vec<String> = results
            .iter()
            .map(|r| r.chunk.symbol_path.clone())
            .collect();

        let relevance_map: HashMap<String, u32> = q
            .expected_results
            .iter()
            .map(|e| (e.symbol_path.clone(), e.relevance))
            .collect();

        let ndcg = ndcg_at_k(&result_symbols, &relevance_map, K);
        let rr = reciprocal_rank(&result_symbols, &relevance_map);
        let recall = recall_at_k(&result_symbols, &relevance_map, K);

        total_ndcg += ndcg;
        total_mrr += rr;
        total_recall += recall;

        println!(
            "  {} [{}] {:45} NDCG@{K}: {ndcg:.4}  RR: {rr:.4}  Recall: {recall:.4}",
            q.id,
            q.intent,
            format!("\"{}\"", q.query),
        );
    }

    let n = dataset.queries.len() as f64;
    let mean_ndcg = total_ndcg / n;
    let mean_mrr = total_mrr / n;
    let mean_recall = total_recall / n;

    println!();
    println!("  -------------------------------------------------------");
    println!("  Mean NDCG@{K}:   {mean_ndcg:.4}");
    println!("  Mean MRR:       {mean_mrr:.4}");
    println!("  Mean Recall@{K}: {mean_recall:.4}");
    println!("  -------------------------------------------------------");
    println!();

    let threshold = 0.5;
    if mean_ndcg < threshold {
        eprintln!("FAIL: mean NDCG@{K} ({mean_ndcg:.4}) below minimum threshold ({threshold:.4})");
        std::process::exit(1);
    }

    println!("PASS: mean NDCG@{K} ({mean_ndcg:.4}) >= threshold ({threshold:.4})");
    println!("EVAL_RESULT: ndcg10={mean_ndcg:.4} mrr={mean_mrr:.4} recall10={mean_recall:.4}");
    Ok(())
}
