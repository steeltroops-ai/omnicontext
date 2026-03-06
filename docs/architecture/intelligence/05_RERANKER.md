# Subsystem 5: Re-ranking Engine

**Status**: ARCHITECTURAL BUG -- using a bi-encoder as a cross-encoder. Reranker is producing meaningless scores.
**Priority**: P1. Fix before enabling reranker in production.

---

## Current Implementation Audit

### What Exists

`crates/omni-core/src/reranker/mod.rs` (274 lines)

```rust
impl Reranker {
    pub fn new(config: &RerankerConfig) -> OmniResult<Self> {
        // Uses the same model as the embedder
        let model_spec = model_manager::resolve_model_spec();
        // -> loads jina-embeddings-v2-base-code
    }

    fn tokenize_pairs(&self, query: &str, documents: &[&str], max_len: usize)
        -> OmniResult<(Vec<i64>, Vec<i64>, Vec<i64>)>
    {
        for doc in documents {
            let encoding = tokenizer.encode(
                tokenizers::EncodeInput::Dual(query.into(), (*doc).into()),
                true,
            )?;
            // Encodes query + document as a PAIR
        }
    }
}
```

### The Fundamental Architecture Bug

The reranker tokenizes query-document pairs (`EncodeInput::Dual`) and feeds them through `jina-embeddings-v2-base-code`. This is **architecturally wrong** for two reasons:

**Reason 1: Bi-encoders are not cross-encoders**

`jina-embeddings-v2-base-code` is a **bi-encoder** (also called a dual-encoder or sentence transformer). It is designed to encode text independently and then compute similarity via dot product. It was NOT trained to process query-document pairs jointly.

When you feed it a `[query, document]` pair, the model processes the concatenated input but its internal attention patterns do not "compare" query to document the way a cross-encoder does. The output is just the embedding of the concatenated sequence -- this is meaningless for re-ranking.

A **cross-encoder** (also called a sequence-pair classifier) jointly processes the query and document with full cross-attention between them. Every token in the query attends to every token in the document. This is why cross-encoders are dramatically better at relevance scoring than bi-encoders.

```
Bi-encoder (current, WRONG for reranking):
[CLS] query tokens [SEP]  →  encode  →  q_vec (768d)
[CLS] doc tokens [SEP]   →  encode  →  d_vec (768d)
similarity = dot(q_vec, d_vec)

Cross-encoder (CORRECT for reranking):
[CLS] query tokens [SEP] doc tokens [SEP]  →  encode jointly  →  [CLS] representation
relevance = linear(cls_vec)  (trained on positive/negative pairs)
```

**Reason 2: The output does not represent relevance**

The bi-encoder output (after mean pooling as done in `run_inference()`) represents the semantic center of the combined text, not a relevance score between query and document. Normalizing this between 0 and 1 produces numbers that look like scores but have no meaning.

**Impact**: The current reranker is actively making search results WORSE by introducing nonsense scores that partially override the correct RRF fusion scores.

---

## What a Real Cross-Encoder Reranker Should Look Like

### Cross-Encoder Architecture

A cross-encoder is trained with:

- Input: `[CLS] query [SEP] document [SEP]`
- Output: a single relevance score (sigmoid output of [CLS] classification head)
- Training: binary (relevant=1, not-relevant=0) or listwise on MS MARCO or similar

The output dimension is `[batch_size, 1]` or `[batch_size, 2]` (logits), not `[batch_size, seq_len, hidden]`.

### Problem with the Current run_inference() for Cross-Encoders

```rust
// Current: designed for bi-encoder (embedding model)
if dims.len() == 3 {
    // [batch, seq_len, hidden_dim] -> mean pool with attention mask
    ...
} else if dims.len() == 2 {
    // [batch, hidden_dim] -> already pooled
    ...
}
```

This code handles the RIGHT output shapes for a cross-encoder (`[batch, 1]` or `[batch, 2]`), but the model being fed is wrong. The shapes will "accidentally" work because `dims.len() == 2` catches the mean-pooled output, but it is not a relevance score.

---

## The Fix: Dedicated Cross-Encoder Model

### Option A: `cross-encoder/ms-marco-MiniLM-L-6-v2` (Recommended)

| Property      | Value                                 |
| ------------- | ------------------------------------- |
| Parameters    | 22M                                   |
| ONNX          | Yes, official HuggingFace             |
| ONNX size     | ~90MB                                 |
| Input         | `[query, document]` pair              |
| Output        | `[batch, 1]` relevance logit          |
| Latency       | ~1ms per pair on CPU                  |
| Training data | MS MARCO passage ranking              |
| Purpose       | Re-ranking top-k retrieval candidates |

This model is purpose-built for exactly what OmniContext's reranker is trying to do. It is small, fast, and has native ONNX support.

**HuggingFace**: `cross-encoder/ms-marco-MiniLM-L-6-v2`
**ONNX URL**: `https://huggingface.co/cross-encoder/ms-marco-MiniLM-L-6-v2/resolve/main/onnx/model.onnx`

**Caveat**: Trained on general passage ranking (MS MARCO), not code-specific. Performance on code retrieval will be less than ideal but still far better than the current bi-encoder misuse.

### Option B: `jinaai/jina-reranker-v2-base-multilingual` (Better for code)

| Property      | Value                        |
| ------------- | ---------------------------- |
| Parameters    | 278M (Jina BERT base)        |
| ONNX          | Yes                          |
| ONNX size     | ~560MB                       |
| Training data | Code + multilingual passages |
| Latency       | ~5ms per pair on CPU         |

This is a purpose-built cross-encoder reranker from the same family as the current embedding model. Better alignment with code semantics.

**Tradeoff**: 560MB additional download. Combined with the 550MB embedder, total model size becomes ~1.1GB -- still manageable.

### Option C: Use BGE-Reranker (Best quality, largest)

`BAAI/bge-reranker-v2-m3` -- cross-encoder, multilingual, 568M params, ONNX available.

For now, **Option A is recommended** as a drop-in fix that is immediately correct. Option B or C can be evaluated as quality improvements.

---

## Implementation Fix

### 1. Add cross-encoder ModelSpec

```rust
// In model_manager.rs

pub const RERANKER_MODEL: ModelSpec = ModelSpec {
    name: "ms-marco-MiniLM-L-6-v2",
    hf_repo: "cross-encoder/ms-marco-MiniLM-L-6-v2",
    model_url: "https://huggingface.co/cross-encoder/ms-marco-MiniLM-L-6-v2/resolve/main/onnx/model.onnx",
    tokenizer_url: "https://huggingface.co/cross-encoder/ms-marco-MiniLM-L-6-v2/resolve/main/tokenizer.json",
    dimensions: 1,  // output is a single relevance score
    max_seq_length: 512,
    approx_size_bytes: 90_000_000,
};
```

### 2. Fix Reranker::new() to load the correct model

```rust
impl Reranker {
    pub fn new(config: &RerankerConfig) -> OmniResult<Self> {
        // Load dedicated cross-encoder, not the embedding model
        let reranker_spec = &model_manager::RERANKER_MODEL;
        let (model_path, tokenizer_path) = model_manager::ensure_model(reranker_spec)?;
        // ...rest of session creation is unchanged
    }
}
```

### 3. Fix run_inference() output extraction

```rust
fn run_inference(&self, session: &mut Session, query: &str, documents: &[&str])
    -> OmniResult<Vec<f32>>
{
    // ... tokenization is correct (Dual encoding) ...

    let (output_shape, output_data) = output_value
        .try_extract_tensor::<f32>()?;

    let dims: Vec<usize> = output_shape.iter().map(|&d| d as usize).collect();

    // Cross-encoder output: [batch_size, 1] logits
    // Apply sigmoid to get [0, 1] relevance scores
    let mut scores = Vec::with_capacity(batch_size);
    for b in 0..batch_size {
        let logit = match dims.as_slice() {
            [_, 1] => output_data[b],
            [_, 2] => output_data[b * 2 + 1], // take positive class logit
            [_]    => output_data[b],
            _ => return Err(OmniError::Internal(format!("unexpected shape: {dims:?}"))),
        };
        // Sigmoid activation: score = 1/(1+exp(-logit))
        let score = 1.0 / (1.0 + (-logit).exp());
        scores.push(score);
    }
    Ok(scores)
}
```

---

## What Good Re-ranking Looks Like

After the fix, the reranker pipeline becomes:

```
RRF fusion output (top-20 candidates, unordered by true relevance)
    |
    v
Cross-encoder Reranker:
    Input:  [("query", chunk1_content), ("query", chunk2_content), ...]
    Model:  ms-marco-MiniLM-L-6-v2 (joint attention, true relevance scoring)
    Output: [0.94, 0.12, 0.87, 0.03, 0.76, ...] (relevance probabilities)
    |
    v
Final merge: final_score = 0.3 * rrf_score + 0.7 * reranker_score
    |
    v
Top-5 results with high true relevance
```

**Expected quality gain**: 15-25% improvement in MRR@5 (Mean Reciprocal Rank at 5) compared to RRF-only, based on published benchmarks for cross-encoder reranking on code search tasks.

---

## Flows with Problems

```
Current (BROKEN) Reranker Flow:
RRF top-20 → Reranker::rerank(query, docs)
    -> tokenize_pairs([query, doc]) using jina-v2-base-code tokenizer
    -> run_inference() with EMBEDDING model (bi-encoder)
    -> model sees [query][SEP][doc] pair
    -> computes mean-pooled embedding of the concatenation  (MEANINGLESS for ranking)
    -> normalizes to [0,1] range
    -> multiplied into final score (CORRUPTS the RRF score)

Correct Flow:
RRF top-20 → Reranker::rerank(query, docs)
    -> tokenize_pairs([query, doc]) using cross-encoder tokenizer
    -> run_inference() with CROSS-ENCODER model (ms-marco-MiniLM)
    -> model jointly attends across all query-doc token pairs
    -> outputs relevance logit [batch, 1]
    -> sigmoid → relevance probability [0, 1]
    -> combined with RRF score: final = 0.3*rrf + 0.7*xenc
    -> results ordered by true semantic relevance
```
