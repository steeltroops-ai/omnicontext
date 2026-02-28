# OmniContext Embedding Model Evaluation

## Problem

The original spec proposed `all-MiniLM-L6-v2` as the primary embedding model. This model is trained on natural language (NLI + STS datasets) and is **not optimized for code retrieval**.

Code has fundamentally different semantics from prose:

- Variable names carry semantic meaning
- Syntax patterns (loops, conditionals, error handling) are meaningful
- Cross-language concept similarity (Python `try/except` == Rust `Result`)
- Import paths encode architectural relationships

## Candidate Models

| Model                                 | Dimensions | Size  | Speed (CPU) | Code-Trained | ONNX         |
| ------------------------------------- | ---------- | ----- | ----------- | ------------ | ------------ |
| `all-MiniLM-L6-v2`                    | 384        | 80MB  | ~1000/s     | No           | Yes          |
| `microsoft/codebert-base`             | 768        | 420MB | ~300/s      | Yes          | Yes          |
| `Salesforce/codet5p-110m-embedding`   | 256        | 440MB | ~250/s      | Yes          | Needs export |
| `jinaai/jina-embeddings-v2-base-code` | 768        | 550MB | ~200/s      | Yes          | Yes          |
| `nomic-ai/nomic-embed-text-v1.5`      | 768        | 550MB | ~180/s      | Partial      | Yes          |
| `BAAI/bge-base-en-v1.5`               | 768        | 420MB | ~300/s      | Partial      | Yes          |

## Evaluation Protocol

### Dataset

Use CodeSearchNet benchmark (6 languages, ~2M code-query pairs):

- Python subset: 100k pairs
- JavaScript subset: 100k pairs
- Go subset: 50k pairs
- Ruby subset: 50k pairs

### Metrics

1. **MRR@10** (Mean Reciprocal Rank): Primary metric
2. **NDCG@10**: Secondary metric
3. **Recall@100**: Ceiling metric
4. **Inference latency** (P50, P99): Operational constraint
5. **Memory footprint**: Operational constraint

### Evaluation Script

```bash
# Located at scripts/eval_embedding_model.py
python scripts/eval_embedding_model.py \
    --model jinaai/jina-embeddings-v2-base-code \
    --dataset codesearchnet \
    --languages python,javascript \
    --output results/jina_code_eval.json
```

## Recommendation

**Default model**: To be determined after Phase 1 benchmarking.

**Strategy**: Ship with a model download step on first run:

1. `omnicontext init` downloads the default model (~500MB)
2. User can switch models via config:
   ```toml
   [embedding]
   model = "jinaai/jina-embeddings-v2-base-code"  # or path to custom ONNX
   dimensions = 768
   ```
3. Changing model triggers full re-embedding (warn user)

## Architecture Implication

The embedding dimension is not fixed at compile time. The vector index and search engine must accept configurable dimensions:

```rust
pub struct EmbeddingConfig {
    pub model_path: PathBuf,
    pub dimensions: usize,  // 256, 384, 768, or 1024
    pub batch_size: usize,  // default 32
    pub max_tokens: usize,  // model's max sequence length
}
```

This means:

- usearch index must be created with the correct dimension
- Changing model = rebuilding the vector index
- Backward compatibility: store dimension in `state.json`
