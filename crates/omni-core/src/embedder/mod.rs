//! ONNX-based local embedding engine.
//!
//! This module runs embedding inference locally using ONNX Runtime.
//! No network calls, no API keys. The model file is loaded from disk
//! at startup and kept in memory.
//!
//! ## Failure Handling
//!
//! If the model fails to load, the system operates in keyword-only mode.
//! Individual embedding failures (OOM, timeout) are logged and the chunk
//! is indexed without a vector (keyword search still finds it).
//!
//! ## Architecture
//!
//! The embedder has two modes:
//! 1. **Full mode**: ONNX model loaded, produces real embeddings
//! 2. **Degraded mode**: Model unavailable, returns errors gracefully
//!
//! The pipeline checks `is_available()` and skips embedding when degraded.

use ort::session::Session;

use crate::config::EmbeddingConfig;
use crate::error::{OmniError, OmniResult};

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
    /// If the model file doesn't exist, returns `Ok` with the embedder in
    /// degraded mode (no embedding, keyword-only search).
    pub fn new(config: &EmbeddingConfig) -> OmniResult<Self> {
        let model_path = &config.model_path;
        let tokenizer_path = model_path.with_file_name("tokenizer.json");

        // Try to load the ONNX model
        let session = if model_path.exists() {
            match Session::builder() {
                Ok(builder) => {
                    match builder.commit_from_file(model_path) {
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
                        "failed to create ONNX session builder, operating in keyword-only mode"
                    );
                    None
                }
            }
        } else {
            tracing::warn!(
                model = %model_path.display(),
                "embedding model not found, operating in keyword-only mode"
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
    /// Returns a vector of embedding vectors, one per input chunk.
    /// Each embedding is L2-normalized.
    pub fn embed_batch(&self, chunks: &[&str]) -> OmniResult<Vec<Vec<f32>>> {
        let session_mutex = self.session.as_ref().ok_or_else(|| {
            OmniError::ModelUnavailable {
                reason: format!("model not loaded: {}", self.config.model_path.display()),
            }
        })?;

        let mut session = session_mutex.lock().map_err(|e| {
            OmniError::Internal(format!("session lock poisoned: {e}"))
        })?;

        let mut all_embeddings = Vec::with_capacity(chunks.len());

        // Process in batches
        for batch in chunks.chunks(self.config.batch_size) {
            let batch_embeddings = self.run_inference(&mut session, batch)?;
            all_embeddings.extend(batch_embeddings);
        }

        Ok(all_embeddings)
    }

    /// Embed a single text string.
    pub fn embed_single(&self, text: &str) -> OmniResult<Vec<f32>> {
        let results = self.embed_batch(&[text])?;
        results.into_iter().next().ok_or_else(|| {
            OmniError::Internal("embed_batch returned empty results".into())
        })
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

        let type_value = ort::value::Tensor::from_array(
            (shape, token_type_ids)
        ).map_err(|e| OmniError::Internal(format!("ONNX tensor error: {e}")))?;

        // Build inputs
        use std::borrow::Cow;
        let inputs: Vec<(Cow<'_, str>, ort::session::SessionInputValue<'_>)> = vec![
            (Cow::Borrowed("input_ids"), ort::session::SessionInputValue::from(ids_value)),
            (Cow::Borrowed("attention_mask"), ort::session::SessionInputValue::from(mask_value)),
            (Cow::Borrowed("token_type_ids"), ort::session::SessionInputValue::from(type_value)),
        ];

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

/// Format a chunk for embedding with metadata prefix.
///
/// This prepends language and symbol information so the embedding
/// captures structural context, not just raw code.
pub fn format_chunk_for_embedding(
    language: &str,
    symbol_path: &str,
    kind: &str,
    content: &str,
) -> String {
    format!("[{language}] {symbol_path}: {kind}\n{content}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_chunk_for_embedding() {
        let result = format_chunk_for_embedding(
            "python",
            "app.routes.login",
            "function",
            "def login(request):\n    pass",
        );
        assert!(result.starts_with("[python] app.routes.login: function"));
        assert!(result.contains("def login"));
    }

    #[test]
    fn test_embedder_degraded_mode() {
        let config = EmbeddingConfig {
            model_path: "/nonexistent/model.onnx".into(),
            dimensions: 384,
            batch_size: 32,
            max_seq_length: 256,
        };
        let embedder = Embedder::new(&config).expect("should create in degraded mode");
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
        assert!(rust.starts_with("[rust]"));

        let ts = format_chunk_for_embedding("typescript", "UserService.getUser", "function", "getUser() {}");
        assert!(ts.starts_with("[typescript]"));
    }
}
