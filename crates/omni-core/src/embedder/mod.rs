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

use crate::config::EmbeddingConfig;
use crate::error::{OmniError, OmniResult};

/// Embedding engine that uses ONNX Runtime for local inference.
pub struct Embedder {
    config: EmbeddingConfig,
    // session: Option<ort::Session>, // will be initialized when ONNX model is available
}

impl Embedder {
    /// Create a new embedder with the given configuration.
    ///
    /// If the model file doesn't exist, returns `Ok` with the embedder in
    /// degraded mode (no embedding, keyword-only search).
    pub fn new(config: &EmbeddingConfig) -> OmniResult<Self> {
        if !config.model_path.exists() {
            tracing::warn!(
                model_path = %config.model_path.display(),
                "embedding model not found, operating in keyword-only mode"
            );
        }

        Ok(Self {
            config: config.clone(),
        })
    }

    /// Whether the embedding model is loaded and operational.
    pub fn is_available(&self) -> bool {
        self.config.model_path.exists()
    }

    /// Embed a batch of text chunks.
    ///
    /// Returns a vector of embedding vectors, one per input chunk.
    /// Each embedding is L2-normalized.
    pub fn embed_batch(&self, _chunks: &[&str]) -> OmniResult<Vec<Vec<f32>>> {
        if !self.is_available() {
            return Err(OmniError::ModelUnavailable {
                reason: format!("model not found: {}", self.config.model_path.display()),
            });
        }

        // TODO: Implement ONNX inference
        // 1. Tokenize each chunk (model's tokenizer)
        // 2. Pad/truncate to max_seq_length
        // 3. Run ONNX inference in batches of batch_size
        // 4. L2-normalize output vectors
        // 5. Return Vec<Vec<f32>>
        Err(OmniError::Internal("embedding not yet implemented".into()))
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
}
