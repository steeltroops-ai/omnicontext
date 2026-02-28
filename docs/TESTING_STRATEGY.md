# OmniContext Testing Strategy

## Test Pyramid

```
                    /\
                   /  \
                  / E2E \        <- 5-10 tests (MCP client simulation)
                 /--------\
                / Integr.   \    <- 30-50 tests (cross-crate, real fixtures)
               /--------------\
              /   Unit Tests    \ <- 200+ tests (per-module, isolated)
             /--------------------\
            /   Property-Based     \ <- 50+ tests (proptest, parser/chunker)
           /------------------------\
          /     Benchmarks            \ <- 20+ benches (criterion, regression guard)
         /------------------------------\
```

## Unit Tests

### Location

Every module has tests in `#[cfg(test)] mod tests` at the bottom of the file.

### Coverage Targets

| Crate                 | Target Coverage |
| --------------------- | --------------- |
| `omni-core::parser`   | >= 90%          |
| `omni-core::chunker`  | >= 90%          |
| `omni-core::embedder` | >= 80%          |
| `omni-core::index`    | >= 85%          |
| `omni-core::search`   | >= 85%          |
| `omni-core::graph`    | >= 80%          |
| `omni-mcp`            | >= 75%          |
| `omni-cli`            | >= 60%          |

### Naming Convention

```rust
#[test]
fn <function_name>_<scenario>_<expected_behavior>() { }

// Examples:
fn parse_python_function_returns_correct_chunk_kind() { }
fn search_empty_index_returns_empty_results() { }
fn embed_large_chunk_splits_and_embeds_both() { }
```

## Property-Based Tests (proptest)

Critical for parser and chunker correctness:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn chunker_never_exceeds_max_tokens(
        content in "[a-zA-Z0-9\n ]{1,10000}"
    ) {
        let chunks = chunk_content(&content, 512);
        for chunk in &chunks {
            prop_assert!(chunk.token_count <= 512);
        }
    }

    #[test]
    fn chunker_covers_all_content(
        content in "[a-zA-Z0-9\n ]{1,10000}"
    ) {
        let chunks = chunk_content(&content, 512);
        let reassembled: String = chunks.iter()
            .map(|c| c.content.as_str())
            .collect();
        // All content must be in some chunk
        for line in content.lines() {
            prop_assert!(reassembled.contains(line.trim()));
        }
    }

    #[test]
    fn parser_never_panics(
        source in "[a-zA-Z0-9(){}\n; ]{1,5000}"
    ) {
        // Parser must handle any input without panic
        let _ = parse_source(&source, "python");
    }
}
```

## Integration Tests

### Fixture Repositories

Located in `tests/fixtures/`:

```
tests/fixtures/
  python_project/       # Flask app, 50 files
  typescript_project/   # Next.js app, 80 files
  rust_project/         # CLI tool, 30 files
  mixed_project/        # Polyglot: Python + TS + Rust
  monorepo/             # Multi-package workspace
  edge_cases/           # Pathological inputs
    deeply_nested.py    # 20 levels of nesting
    huge_file.ts        # 10k lines, single file
    unicode_heavy.rs    # Non-ASCII identifiers
    no_functions.py     # Module-level code only
    circular_imports/   # A imports B, B imports A
```

### Integration Test Examples

```rust
#[tokio::test]
async fn test_full_indexing_pipeline() {
    let repo = fixture_repo("python_project");
    let engine = OmniEngine::new_test(&repo).await;
    engine.index_all().await.unwrap();

    assert!(engine.file_count() > 0);
    assert!(engine.chunk_count() > 0);
    assert!(engine.symbol_count() > 0);
}

#[tokio::test]
async fn test_search_relevance() {
    let repo = fixture_repo("python_project");
    let engine = OmniEngine::new_test(&repo).await;
    engine.index_all().await.unwrap();

    let results = engine.search("error handling", 10).await.unwrap();

    // At least one result should contain error-related code
    assert!(results.iter().any(|r|
        r.content.contains("Error") ||
        r.content.contains("except") ||
        r.content.contains("raise")
    ));
}

#[tokio::test]
async fn test_incremental_update() {
    let repo = fixture_repo("python_project");
    let engine = OmniEngine::new_test(&repo).await;
    engine.index_all().await.unwrap();

    let initial_count = engine.chunk_count();

    // Modify a file
    modify_fixture_file(&repo, "app.py", "# new comment\n");
    engine.reindex_file("app.py").await.unwrap();

    // Chunk count should be similar (maybe +1 for new comment)
    assert!((engine.chunk_count() as i64 - initial_count as i64).abs() <= 2);
}
```

## Benchmark Suite (criterion)

Located in `benches/`:

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_parse_python(c: &mut Criterion) {
    let source = include_str!("../tests/fixtures/python_project/app.py");

    c.bench_function("parse_python_500_lines", |b| {
        b.iter(|| parse_source(source, "python"));
    });
}

fn bench_embed_batch(c: &mut Criterion) {
    let chunks: Vec<String> = (0..100)
        .map(|i| format!("def function_{i}(): return {i}"))
        .collect();

    let embedder = Embedder::new_test();

    c.bench_function("embed_100_chunks", |b| {
        b.iter(|| embedder.embed_batch(&chunks));
    });
}

fn bench_search(c: &mut Criterion) {
    let engine = setup_indexed_engine("python_project");

    c.bench_function("search_semantic", |b| {
        b.iter(|| engine.search_blocking("error handling", 10));
    });
}

criterion_group!(benches, bench_parse_python, bench_embed_batch, bench_search);
criterion_main!(benches);
```

### Performance Regression Detection

CI runs benchmarks on every PR and compares against `main`:

- If any benchmark regresses by > 10%, the PR is blocked
- Benchmark results are stored as CI artifacts for trend analysis

## Search Relevance Evaluation

### NDCG Benchmark

A curated set of (query, expected_results) pairs:

```json
{
  "benchmark": "python_project_v1",
  "queries": [
    {
      "query": "error handling",
      "relevant_chunks": [
        "app.error_handler",
        "utils.CustomError",
        "middleware.catch_exceptions"
      ],
      "irrelevant_chunks": ["models.User", "config.Settings"]
    },
    {
      "query": "database connection",
      "relevant_chunks": ["db.connect", "db.pool", "models.Base"],
      "irrelevant_chunks": ["auth.login", "views.homepage"]
    }
  ]
}
```

Target: **NDCG@10 > 0.75** on this benchmark.

## CI Pipeline

```yaml
# Runs on every PR
1. cargo fmt --check
2. cargo clippy -- -D warnings
3. cargo test --workspace (unit + integration)
4. cargo bench --workspace (regression check)
5. cargo audit (security)
```
