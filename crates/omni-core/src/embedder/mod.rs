//! ONNX-based local embedding engine with automatic model management.
//!
//! This module runs embedding inference locally using ONNX Runtime.
//! No network calls during inference, no API keys. The model file is
//! automatically downloaded on first use and cached permanently.
//!
//! ## Model: jina-embeddings-v2-base-code
//!
//! The default model is specifically trained on code retrieval tasks:
//! - Code-to-text and code-to-code search
//! - 768 dimensions, 8192 token context window
//! - Understands variable names, syntax patterns, cross-language concepts
//!
//! ## First-Run Behavior
//!
//! On the first invocation, the engine will:
//! 1. Detect that the model is not cached
//! 2. Download model.onnx (~550MB) and tokenizer.json from HuggingFace
//! 3. Cache them in `~/.omnicontext/models/jina-embeddings-v2-base-code/`
//! 4. Proceed with indexing
//!
//! Subsequent runs use the cached model instantly.
//!
//! ## Failure Handling
//!
//! If the model fails to download or load, the system operates in keyword-only
//! mode. Individual embedding failures (OOM, timeout) are logged and the chunk
//! is indexed without a vector (keyword search still finds it).
//!
//! ## Architecture
//!
//! The embedder has two modes:
//! 1. **Full mode**: ONNX model loaded, produces real embeddings
//! 2. **Degraded mode**: Model unavailable, returns errors gracefully
//!
//! The pipeline checks `is_available()` and skips embedding when degraded.

pub mod model_manager;

use ort::session::Session;

use crate::config::EmbeddingConfig;
use crate::error::{OmniError, OmniResult};

pub use model_manager::{DEFAULT_MODEL, FALLBACK_MODEL, ModelSpec};

/// Embedding engine that uses ONNX Runtime for local inference.
pub struct Embedder {
    config: EmbeddingConfig,
    /// The ONNX runtime session. None if model couldn't be loaded.
    /// Stored in a Mutex because Session::run requires &mut self.
    session: Option<std::sync::Mutex<Session>>,
    /// Tokenizer for the embedding model. None if tokenizer couldn't be loaded.
    tokenizer: Option<tokenizers::Tokenizer>,
}

impl Embedder {
    /// Create a new embedder with the given configuration.
    ///
    /// This will automatically download the embedding model if it's not
    /// already cached. On failure, returns Ok with the embedder in
    /// degraded mode (keyword-only search).
    pub fn new(config: &EmbeddingConfig) -> OmniResult<Self> {
        // Resolve model spec and auto-download if needed
        let (model_path, tokenizer_path) = Self::resolve_model_files(config)?;

        // Try to load the ONNX model
        let session = if model_path.exists() {
            match Session::builder() {
                Ok(builder) => {
                    match builder.commit_from_file(&model_path) {
                        Ok(session) => {
                            tracing::info!(
                                model = %model_path.display(),
                                "loaded ONNX embedding model"
                            );
                            Some(std::sync::Mutex::new(session))
                        }
                        Err(e) => {
                            tracing::warn!(
                                model = %model_path.display(),
                                error = %e,
                                "failed to load embedding model, operating in keyword-only mode"
                            );
                            None
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "failed to create ONNX session builder, operating in keyword-only mode.\n\
                         Hint: ONNX Runtime may not be installed. The engine will use keyword-only search."
                    );
                    None
                }
            }
        } else {
            tracing::warn!(
                model = %model_path.display(),
                "embedding model not found after download attempt, operating in keyword-only mode"
            );
            None
        };

        // Try to load the tokenizer
        let tokenizer = if tokenizer_path.exists() {
            match tokenizers::Tokenizer::from_file(&tokenizer_path) {
                Ok(t) => Some(t),
                Err(e) => {
                    tracing::warn!(
                        tokenizer = %tokenizer_path.display(),
                        error = %e,
                        "failed to load tokenizer"
                    );
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            config: config.clone(),
            session,
            tokenizer,
        })
    }

    /// Resolve model file paths, auto-downloading if needed.
    ///
    /// Strategy:
    /// 1. If config.model_path points to an existing file, use it directly (manual override)
    /// 2. If model is already cached, use the cached version
    /// 3. If OMNI_SKIP_MODEL_DOWNLOAD is set, skip download (CI/testing/offline)
    /// 4. Otherwise, download the model from HuggingFace
    fn resolve_model_files(config: &EmbeddingConfig) -> OmniResult<(std::path::PathBuf, std::path::PathBuf)> {
        // Check if the user has manually specified a model path that exists
        if config.model_path.exists() {
            let tokenizer_path = config.model_path.with_file_name("tokenizer.json");
            tracing::debug!(
                model = %config.model_path.display(),
                "using user-specified model path"
            );
            return Ok((config.model_path.clone(), tokenizer_path));
        }

        let spec = model_manager::resolve_model_spec();

        // Check if already cached -- fast path, no network
        if model_manager::is_model_ready(spec) {
            return Ok((
                model_manager::model_path(spec),
                model_manager::tokenizer_path(spec),
            ));
        }

        // Skip download if explicitly disabled via env var.
        // This env var is also set by CI, integration tests, and offline environments.
        if std::env::var("OMNI_SKIP_MODEL_DOWNLOAD").is_ok() {
            tracing::info!("OMNI_SKIP_MODEL_DOWNLOAD set, operating in keyword-only mode");
            return Ok((
                model_manager::model_path(spec),
                model_manager::tokenizer_path(spec),
            ));
        }

        // Belt-and-suspenders: skip in unit test context within this crate
        #[cfg(test)]
        {
            tracing::debug!("skipping model download in test environment");
            return Ok((
                model_manager::model_path(spec),
                model_manager::tokenizer_path(spec),
            ));
        }

        // Production path: auto-download the model
        #[cfg(not(test))]
        {
            match model_manager::ensure_model(spec) {
                Ok((model, tokenizer)) => Ok((model, tokenizer)),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "model auto-download failed, will operate in keyword-only mode"
                    );
                    Ok((
                        model_manager::model_path(spec),
                        model_manager::tokenizer_path(spec),
                    ))
                }
            }
        }
    }

    /// Create an embedder in degraded mode (for testing without a model).
    pub fn degraded(config: &EmbeddingConfig) -> Self {
        Self {
            config: config.clone(),
            session: None,
            tokenizer: None,
        }
    }

    /// Whether the embedding model is loaded and operational.
    pub fn is_available(&self) -> bool {
        self.session.is_some()
    }

    /// Embed a batch of text chunks.
    ///
    /// Returns a vector where each element corresponds to an input chunk.
    /// If a chunk successfully embeds, `Some(embedding)` is returned.
    /// If embedding fails (e.g., ONNX failure, chunk too large), `None` is returned.
    pub fn embed_batch(&self, chunks: &[&str]) -> Vec<Option<Vec<f32>>> {
        let session_mutex = match self.session.as_ref() {
            Some(s) => s,
            None => return vec![None; chunks.len()],
        };

        let mut session = match session_mutex.lock() {
            Ok(s) => s,
            Err(_) => return vec![None; chunks.len()],
        };

        let mut all_embeddings = Vec::with_capacity(chunks.len());

        // Process in batches
        for batch in chunks.chunks(self.config.batch_size) {
            match self.run_inference(&mut session, batch) {
                Ok(batch_embeddings) => {
                    for emb in batch_embeddings {
                        all_embeddings.push(Some(emb));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "batch inference failed; falling back to individual chunks"
                    );
                    // Fall back to processing chunks one by one
                    for text in batch {
                        match self.run_inference(&mut session, &[*text]) {
                            Ok(mut single_emb) => {
                                all_embeddings.push(Some(single_emb.remove(0)));
                            }
                            Err(chunk_err) => {
                                tracing::warn!(
                                    error = %chunk_err,
                                    "chunk inference failed; skipping this chunk"
                                );
                                all_embeddings.push(None);
                            }
                        }
                    }
                }
            }
        }

        all_embeddings
    }

    /// Embed a single text string.
    pub fn embed_single(&self, text: &str) -> OmniResult<Vec<f32>> {
        if !self.is_available() {
            return Err(OmniError::ModelUnavailable {
                reason: format!("model not loaded: {}", self.config.model_path.display()),
            });
        }
        let mut results = self.embed_batch(&[text]);
        if let Some(Some(emb)) = results.pop() {
            Ok(emb)
        } else {
            Err(OmniError::Internal("embed_batch failed or returned None".into()))
        }
    }

    /// Returns the embedding dimensions.
    pub fn dimensions(&self) -> usize {
        self.config.dimensions
    }

    /// Run ONNX inference on a batch of texts.
    fn run_inference(
        &self,
        session: &mut Session,
        texts: &[&str],
    ) -> OmniResult<Vec<Vec<f32>>> {
        let batch_size = texts.len();
        let max_len = self.config.max_seq_length;

        // Tokenize
        let (input_ids, attention_mask, token_type_ids) = self.tokenize_batch(texts, max_len)?;

        // Create ort tensors using (shape, data) tuple API
        let shape = vec![batch_size as i64, max_len as i64];

        let ids_value = ort::value::Tensor::from_array(
            (shape.clone(), input_ids)
        ).map_err(|e| OmniError::Internal(format!("ONNX tensor error: {e}")))?;

        let mask_value = ort::value::Tensor::from_array(
            (shape.clone(), attention_mask.clone())
        ).map_err(|e| OmniError::Internal(format!("ONNX tensor error: {e}")))?;

        // Build inputs dynamically based on what the model expects
        use std::borrow::Cow;
        let mut inputs: Vec<(Cow<'_, str>, ort::session::SessionInputValue<'_>)> = vec![
            (Cow::Borrowed("input_ids"), ort::session::SessionInputValue::from(ids_value)),
            (Cow::Borrowed("attention_mask"), ort::session::SessionInputValue::from(mask_value)),
        ];

        // Only add token_type_ids if the model expects it (Jina doesn't, BGE might)
        let expects_token_type = session.inputs().iter().any(|i| i.name() == "token_type_ids");
        if expects_token_type {
            let type_value = ort::value::Tensor::from_array(
                (shape.clone(), token_type_ids)
            ).map_err(|e| OmniError::Internal(format!("ONNX tensor error (token_type_ids): {e}")))?;
            inputs.push((Cow::Borrowed("token_type_ids"), ort::session::SessionInputValue::from(type_value)));
        }

        // Get output name before running (session.outputs() borrows &self)
        let output_name = session.outputs().first()
            .map(|o| o.name().to_string())
            .ok_or_else(|| OmniError::Internal("model has no outputs".into()))?;

        // Run inference (requires &mut self)
        let outputs = session.run(inputs)
            .map_err(|e| OmniError::Internal(format!("ONNX inference error: {e}")))?;

        // Extract first output tensor
        let output_value = outputs.get(&output_name)
            .ok_or_else(|| OmniError::Internal("no output tensor found".into()))?;

        let (output_shape, output_data) = output_value.try_extract_tensor::<f32>()
            .map_err(|e| OmniError::Internal(format!("output extraction error: {e}")))?;

        let dims: Vec<usize> = output_shape.iter().map(|&d| d as usize).collect();
        let mut embeddings = Vec::with_capacity(batch_size);

        if dims.len() == 3 {
            // [batch, seq_len, hidden_dim] -> mean pool with attention mask
            let seq_len = dims[1];
            let hidden_dim = dims[2];

            for b in 0..batch_size {
                let mut pooled = vec![0.0f32; hidden_dim];
                let mut mask_sum = 0.0f32;

                for s in 0..seq_len {
                    let mask_val = attention_mask[b * max_len + s] as f32;
                    mask_sum += mask_val;
                    let offset = b * seq_len * hidden_dim + s * hidden_dim;
                    for d in 0..hidden_dim {
                        pooled[d] += output_data[offset + d] * mask_val;
                    }
                }

                if mask_sum > 0.0 {
                    for d in &mut pooled {
                        *d /= mask_sum;
                    }
                }

                crate::vector::l2_normalize(&mut pooled);
                embeddings.push(pooled);
            }
        } else if dims.len() == 2 {
            // [batch, hidden_dim] -> already pooled
            let hidden_dim = dims[1];
            for b in 0..batch_size {
                let offset = b * hidden_dim;
                let mut vec = output_data[offset..offset + hidden_dim].to_vec();
                crate::vector::l2_normalize(&mut vec);
                embeddings.push(vec);
            }
        } else {
            return Err(OmniError::Internal(format!(
                "unexpected output tensor shape: {dims:?}"
            )));
        }

        Ok(embeddings)
    }

    /// Tokenize a batch of texts with padding and truncation.
    fn tokenize_batch(
        &self,
        texts: &[&str],
        max_len: usize,
    ) -> OmniResult<(Vec<i64>, Vec<i64>, Vec<i64>)> {
        let tokenizer = self.tokenizer.as_ref().ok_or_else(|| {
            OmniError::Internal("tokenizer not loaded".into())
        })?;

        let mut all_input_ids = Vec::with_capacity(texts.len() * max_len);
        let mut all_attention_mask = Vec::with_capacity(texts.len() * max_len);
        let mut all_token_type_ids = Vec::with_capacity(texts.len() * max_len);

        for text in texts {
            let encoding = tokenizer.encode(*text, true)
                .map_err(|e| OmniError::Internal(format!("tokenization error: {e}")))?;

            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let type_ids = encoding.get_type_ids();

            let actual_len = ids.len().min(max_len);

            // Copy tokens up to max_len
            for i in 0..actual_len {
                all_input_ids.push(ids[i] as i64);
                all_attention_mask.push(mask[i] as i64);
                all_token_type_ids.push(type_ids[i] as i64);
            }

            // Pad to max_len
            for _ in actual_len..max_len {
                all_input_ids.push(0);
                all_attention_mask.push(0);
                all_token_type_ids.push(0);
            }
        }

        Ok((all_input_ids, all_attention_mask, all_token_type_ids))
    }
}

/// Format a chunk for embedding.
///
/// Under OmniContext v2, chunks are enriched at chunking time (the context header
/// is directly included in `chunk.content`). This method serves as a no-op 
/// wrapper, kept only for backwards compatibility or future dynamic formatting.
pub fn format_chunk_for_embedding(
    _language: &str,
    _symbol_path: &str,
    _kind: &str,
    content: &str,
) -> String {
    content.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_chunk_for_embedding_pass_through() {
        let result = format_chunk_for_embedding(
            "python",
            "app.routes.login",
            "function",
            "def login(request):\n    pass",
        );
        assert_eq!(result, "def login(request):\n    pass");
    }

    #[test]
    fn test_embedder_degraded_mode() {
        let config = EmbeddingConfig {
            model_path: "/nonexistent/model.onnx".into(),
            dimensions: 384,
            batch_size: 32,
            max_seq_length: 256,
        };
        // Use degraded() directly to avoid triggering download
        let embedder = Embedder::degraded(&config);
        assert!(!embedder.is_available());
    }

    #[test]
    fn test_embedder_degraded_returns_correct_error() {
        let config = EmbeddingConfig {
            model_path: "/nonexistent/model.onnx".into(),
            dimensions: 384,
            batch_size: 32,
            max_seq_length: 256,
        };
        let embedder = Embedder::degraded(&config);
        let result = embedder.embed_single("test text");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, OmniError::ModelUnavailable { .. }),
            "should be ModelUnavailable, got: {err:?}"
        );
    }

    #[test]
    fn test_embedder_dimensions() {
        let config = EmbeddingConfig {
            model_path: "/nonexistent/model.onnx".into(),
            dimensions: 768,
            batch_size: 16,
            max_seq_length: 512,
        };
        let embedder = Embedder::degraded(&config);
        assert_eq!(embedder.dimensions(), 768);
    }

    #[test]
    fn test_format_multiple_languages() {
        let rust = format_chunk_for_embedding("rust", "lib::Config::new", "function", "pub fn new() {}");
        assert_eq!(rust, "pub fn new() {}");

        let ts = format_chunk_for_embedding("typescript", "UserService.getUser", "function", "getUser() {}");
        assert_eq!(ts, "getUser() {}");
    }
}
