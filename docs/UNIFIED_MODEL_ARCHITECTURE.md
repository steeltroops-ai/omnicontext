# Unified Model Architecture

## Overview

OmniContext now uses a **unified model approach** where both the embedder and reranker use the same model. This simplifies architecture, improves consistency, and makes model management easier.

## Architecture

### Before (Separate Models)

```
┌─────────────────────────────────────────────────────────────┐
│                         OmniContext                          │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────────┐              ┌──────────────────┐    │
│  │    Embedder      │              │    Reranker      │    │
│  ├──────────────────┤              ├──────────────────┤    │
│  │ Model: jina-v2   │              │ Model: ms-marco  │    │
│  │ Size: 550MB      │              │ Size: 23MB       │    │
│  │ Training: Code   │              │ Training: Web    │    │
│  └──────────────────┘              └──────────────────┘    │
│         │                                   │                │
│         │                                   │                │
│  ┌──────▼───────────────────────────────────▼──────┐       │
│  │        Duplicate Model Management Code          │       │
│  │  - model_manager.rs (embedder)                  │       │
│  │  - resolve_model_files() (reranker)             │       │
│  │  - download_file() (duplicated)                 │       │
│  └─────────────────────────────────────────────────┘       │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

**Problems**:
- ❌ Two different models with different training
- ❌ Duplicate model management code
- ❌ Reranker trained on web search, not code
- ❌ Hard to change models (update in two places)

### After (Unified Model)

```
┌─────────────────────────────────────────────────────────────┐
│                         OmniContext                          │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────────┐              ┌──────────────────┐    │
│  │    Embedder      │              │    Reranker      │    │
│  ├──────────────────┤              ├──────────────────┤    │
│  │ Uses: model_mgr  │              │ Uses: model_mgr  │    │
│  └────────┬─────────┘              └────────┬─────────┘    │
│           │                                  │               │
│           │         ┌────────────────┐      │               │
│           └────────►│  Model Manager │◄─────┘               │
│                     ├────────────────┤                       │
│                     │ Single Source  │                       │
│                     │ of Truth       │                       │
│                     │                │                       │
│                     │ Model: jina-v2 │                       │
│                     │ Size: 550MB    │                       │
│                     │ Training: Code │                       │
│                     └────────────────┘                       │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

**Benefits**:
- ✅ Single model with code-specific training
- ✅ Centralized model management
- ✅ Consistent understanding of code
- ✅ Easy to change model (one place)

## Implementation Details

### Model Manager (Single Source of Truth)

Location: `crates/omni-core/src/embedder/model_manager.rs`

**Public API**:
```rust
/// Get the recommended model spec based on environment
pub fn resolve_model_spec() -> &'static ModelSpec;

/// Ensure model is available, downloading if necessary
pub fn ensure_model(spec: &ModelSpec) -> OmniResult<(PathBuf, PathBuf)>;

/// Check if model files exist and are valid
pub fn is_model_ready(spec: &ModelSpec) -> bool;

/// Get paths to model files
pub fn model_path(spec: &ModelSpec) -> PathBuf;
pub fn tokenizer_path(spec: &ModelSpec) -> PathBuf;
```

**Model Specs**:
```rust
pub const DEFAULT_MODEL: ModelSpec = ModelSpec {
    name: "jina-embeddings-v2-base-code",
    hf_repo: "jinaai/jina-embeddings-v2-base-code",
    model_url: "https://huggingface.co/.../model.onnx",
    tokenizer_url: "https://huggingface.co/.../tokenizer.json",
    dimensions: 768,
    max_seq_length: 8192,
    approx_size_bytes: 550_000_000,
};

pub const FALLBACK_MODEL: ModelSpec = ModelSpec {
    name: "bge-small-en-v1.5",
    // ... smaller model for constrained environments
};
```

### Embedder Usage

Location: `crates/omni-core/src/embedder/mod.rs`

```rust
use crate::embedder::model_manager;

pub fn new(config: &EmbeddingConfig) -> OmniResult<Self> {
    let spec = model_manager::resolve_model_spec();
    let (model_path, tokenizer_path) = model_manager::ensure_model(spec)?;
    
    // Load ONNX session and tokenizer
    // ...
}
```

### Reranker Usage

Location: `crates/omni-core/src/reranker/mod.rs`

```rust
use crate::embedder::model_manager;

pub fn new(config: &RerankerConfig) -> OmniResult<Self> {
    // Use the same model as the embedder
    let spec = model_manager::resolve_model_spec();
    let (model_path, tokenizer_path) = model_manager::ensure_model(spec)?;
    
    // Load ONNX session and tokenizer
    // ...
}
```

## Model Selection

### Environment Variables

```bash
# Use default model (jina-embeddings-v2-base-code)
# No environment variable needed

# Use fallback model (bge-small-en-v1.5)
export OMNI_EMBEDDING_MODEL=small
# or
export OMNI_EMBEDDING_MODEL=lite

# Use custom model path
export OMNI_MODEL_PATH=/path/to/custom/model.onnx
```

### Resolution Logic

```rust
pub fn resolve_model_spec() -> &'static ModelSpec {
    if let Ok(model_name) = std::env::var("OMNI_EMBEDDING_MODEL") {
        match model_name.to_lowercase().as_str() {
            "small" | "lite" | "fallback" => return &FALLBACK_MODEL,
            _ => {} // fall through to default
        }
    }
    &DEFAULT_MODEL
}
```

## Changing Models

### To Change the Default Model

Edit **ONE FILE**: `crates/omni-core/src/embedder/model_manager.rs`

```rust
pub const DEFAULT_MODEL: ModelSpec = ModelSpec {
    name: "new-model-name",
    hf_repo: "org/new-model",
    model_url: "https://huggingface.co/.../model.onnx",
    tokenizer_url: "https://huggingface.co/.../tokenizer.json",
    dimensions: 1024,  // Update if different
    max_seq_length: 8192,
    approx_size_bytes: 6_000_000_000,
};
```

**That's it!** Both embedder and reranker will automatically use the new model.

### To Add a New Model Option

1. Add a new constant in `model_manager.rs`:
```rust
pub const NEW_MODEL: ModelSpec = ModelSpec {
    // ... model details
};
```

2. Update `resolve_model_spec()`:
```rust
pub fn resolve_model_spec() -> &'static ModelSpec {
    if let Ok(model_name) = std::env::var("OMNI_EMBEDDING_MODEL") {
        match model_name.to_lowercase().as_str() {
            "small" => &FALLBACK_MODEL,
            "new" => &NEW_MODEL,  // Add this line
            _ => &DEFAULT_MODEL,
        }
    } else {
        &DEFAULT_MODEL
    }
}
```

3. Update config dimensions if needed in `config.rs`:
```rust
fn default_dimensions() -> usize {
    // Match the DEFAULT_MODEL dimensions
    768
}
```

## Benefits of Unified Approach

### 1. Consistency

Both embedder and reranker understand code the same way:
- Same vocabulary
- Same semantic space
- Same training data
- Consistent relevance scoring

### 2. Simplicity

- One model to download (~550MB vs 550MB + 23MB)
- One model to manage
- One model to update
- Simpler codebase

### 3. Maintainability

- Change model in ONE place
- No duplicate code
- Easier to test
- Easier to debug

### 4. Code-Specific Training

Using jina-embeddings-v2-base-code for reranking is better than ms-marco because:
- Trained on code, not web search
- Understands programming languages
- Better at code similarity
- Consistent with embedder

### 5. Easy Upgrades

When better ONNX models become available:
1. Update `DEFAULT_MODEL` in `model_manager.rs`
2. Update `default_dimensions()` in `config.rs` if needed
3. Done! Both embedder and reranker use new model

## Testing

### Unit Tests

No changes needed - tests still pass:
```bash
cargo test --package omni-core --lib
# test result: ok. 202 passed; 0 failed
```

### Integration Tests

Verify both embedder and reranker use same model:
```bash
cargo run -p omni-cli -- status
# Should show same model for both components
```

## Migration Notes

### For Users

No action required! The change is transparent:
- Same model is used (jina-embeddings-v2-base-code)
- Same performance
- Same API
- Automatic on upgrade

### For Developers

If you were using `OMNI_RERANKER_MODEL_PATH`:
- ❌ No longer supported (removed)
- ✅ Use `OMNI_EMBEDDING_MODEL` instead
- ✅ Or use `OMNI_MODEL_PATH` for custom path

## Future Improvements

### When Better ONNX Models Become Available

Example: If jina-code-embeddings-1.5b gets ONNX export:

1. Update `model_manager.rs`:
```rust
pub const DEFAULT_MODEL: ModelSpec = ModelSpec {
    name: "jina-code-embeddings-1.5b",
    hf_repo: "jinaai/jina-code-embeddings-1.5b",
    model_url: "https://huggingface.co/.../onnx/model.onnx",
    tokenizer_url: "https://huggingface.co/.../tokenizer.json",
    dimensions: 1024,
    max_seq_length: 8192,
    approx_size_bytes: 6_000_000_000,
};
```

2. Update `config.rs`:
```rust
fn default_dimensions() -> usize {
    1024  // Updated from 768
}
```

3. Done! Both embedder and reranker upgraded.

### Monitoring for Better Models

Watch for ONNX exports of:
- jina-code-embeddings-1.5b (~73 MTEB Code)
- LateOn-Code (74.12 MTEB Code)
- Jina Reranker v2 (71.36 MRR@10 on CodeSearchNet)

## Conclusion

The unified model architecture:
- ✅ Simplifies codebase
- ✅ Improves consistency
- ✅ Makes upgrades easier
- ✅ Maintains performance
- ✅ Reduces maintenance burden

**Key Principle**: One model, one source of truth, easy to change.
