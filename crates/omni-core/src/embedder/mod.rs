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
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::doc_markdown,
    clippy::items_after_statements,
    clippy::must_use_candidate
)]

pub mod model_manager;
pub mod session_pool;

use ort::session::Session;

use crate::config::EmbeddingConfig;
use crate::error::{OmniError, OmniResult};

pub use model_manager::{ModelSpec, DEFAULT_MODEL, FALLBACK_MODEL};

/// Embedding engine that uses ONNX Runtime for local inference.
pub struct Embedder {
    config: EmbeddingConfig,
    /// The ONNX runtime session. None if model couldn't be loaded.
    /// Stored in a Mutex because Session::run requires &mut self.
    /// Used for single-session mode (pool_size=1 or pool unavailable).
    session: Option<std::sync::Mutex<Session>>,
    /// Tokenizer for the embedding model. None if tokenizer couldn't be loaded.
    tokenizer: Option<tokenizers::Tokenizer>,
    /// Model fingerprint for staleness detection.
    /// Format: "{model_name}:{dimensions}:{max_seq_length}".
    /// Used to detect when vectors were produced by a different model.
    model_fingerprint: String,
    /// Optional session pool for parallel inference (pool_size > 1).
    /// When available, `embed_batch_parallel` uses this instead of the single session.
    pool: Option<session_pool::SessionPool>,
}

impl Embedder {
    /// Create a new embedder with the given configuration.
    ///
    /// This will automatically download the embedding model if it's not
    /// already cached. On failure, returns Ok with the embedder in
    /// degraded mode (keyword-only search).
    pub fn new(config: &EmbeddingConfig) -> OmniResult<Self> {
        // In test environments where ONNX might not be available or
        // we want to skip downloading completely, return a degraded embedder.
        if std::env::var("OMNI_SKIP_MODEL_DOWNLOAD").is_ok() {
            tracing::info!("OMNI_SKIP_MODEL_DOWNLOAD is set, skipping embedding model loading");
            return Ok(Self {
                config: config.clone(),
                session: None,
                tokenizer: None,
                model_fingerprint: format!("skip:{}:{}", config.dimensions, config.max_seq_length,),
                pool: None,
            });
        }

        // Resolve model spec and auto-download if needed
        let (model_path, tokenizer_path) = Self::resolve_model_files(config)?;

        // Try to load the ONNX model
        let session = if model_path.exists() {
            match Session::builder() {
                Ok(builder) => match builder.commit_from_file(&model_path) {
                    Ok(session) => {
                        tracing::info!(
                            model = %model_path.display(),
                            "loaded ONNX embedding model successfully"
                        );
                        Some(std::sync::Mutex::new(session))
                    }
                    Err(e) => {
                        tracing::error!(
                            model = %model_path.display(),
                            error = %e,
                            "CRITICAL: Failed to load embedding model. \
                             Model file may be corrupt. \
                             Delete {} and re-run to re-download. \
                             Semantic search is DISABLED. Operating in keyword-only mode.",
                            model_path.display()
                        );
                        None
                    }
                },
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "CRITICAL: Failed to create ONNX session builder. \
                         ONNX Runtime may not be installed correctly. \
                         Semantic search is DISABLED. Operating in keyword-only mode."
                    );
                    None
                }
            }
        } else {
            tracing::error!(
                model = %model_path.display(),
                "CRITICAL: Embedding model not found after download attempt. \
                 Check internet connection and disk space. \
                 Semantic search is DISABLED. Operating in keyword-only mode."
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

        // Try to create a session pool for parallel inference
        let pool = if session.is_some() {
            let pool_size = session_pool::optimal_pool_size(4);
            if pool_size > 1 {
                match session_pool::SessionPool::new(&model_path, pool_size - 1) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(error = %e, "session pool creation failed, using single session");
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            config: config.clone(),
            session,
            tokenizer,
            model_fingerprint: format!(
                "{}:{}:{}",
                config
                    .model_path
                    .file_name()
                    .map(|f| f.to_string_lossy())
                    .unwrap_or_default(),
                config.dimensions,
                config.max_seq_length,
            ),
            pool,
        })
    }

    /// Resolve model file paths, auto-downloading if needed.
    ///
    /// Strategy:
    /// 1. If config.model_path points to an existing file, use it directly (manual override)
    /// 2. If model is already cached, use the cached version
    /// 3. If OMNI_SKIP_MODEL_DOWNLOAD is set, skip download (CI/testing/offline)
    /// 4. Otherwise, download the model from HuggingFace
    fn resolve_model_files(
        config: &EmbeddingConfig,
    ) -> OmniResult<(std::path::PathBuf, std::path::PathBuf)> {
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
            model_fingerprint: format!("degraded:{}:{}", config.dimensions, config.max_seq_length,),
            pool: None,
        }
    }

    /// Whether the embedding model is loaded and operational.
    pub fn is_available(&self) -> bool {
        self.session.is_some()
    }

    /// Number of ONNX sessions available (1 primary + pool).
    pub fn pool_size(&self) -> usize {
        let base = if self.session.is_some() { 1 } else { 0 };
        base + self.pool.as_ref().map(|p| p.pool_size()).unwrap_or(0)
    }

    /// Embed a batch using the session pool for parallel inference.
    ///
    /// Splits the input into sub-batches and processes them concurrently
    /// using `std::thread::scope`. Each worker checks out a session from
    /// the pool, runs inference, and returns it.
    ///
    /// Falls back to single-session `embed_batch` if no pool is available.
    pub fn embed_batch_parallel(&self, chunks: &[&str]) -> Vec<Option<Vec<f32>>> {
        let pool = match &self.pool {
            Some(p) if p.pool_size() > 0 => p,
            _ => return self.embed_batch(chunks),
        };

        if chunks.len() <= self.config.batch_size {
            // Small batch -- not worth parallel overhead
            return self.embed_batch(chunks);
        }

        let n_workers = (pool.pool_size() + 1).min(chunks.len() / self.config.batch_size + 1);
        let sub_batch_size = (chunks.len() + n_workers - 1) / n_workers;

        // Use scoped threads for safe borrow of &self
        let results: Vec<Vec<Option<Vec<f32>>>> = std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(n_workers);

            for (worker_idx, sub_batch) in chunks.chunks(sub_batch_size).enumerate() {
                let handle = scope.spawn(move || {
                    if worker_idx == 0 {
                        // First worker uses the primary session
                        self.embed_batch(sub_batch)
                    } else {
                        // Other workers use pool sessions
                        match pool.try_checkout() {
                            Some(mut guard) => {
                                self.embed_batch_with_session(guard.session_mut(), sub_batch)
                            }
                            None => {
                                // Pool exhausted, fall back to primary session
                                self.embed_batch(sub_batch)
                            }
                        }
                    }
                });
                handles.push(handle);
            }

            handles
                .into_iter()
                .map(|h| h.join().unwrap_or_default())
                .collect()
        });

        // Flatten sub-batch results
        let mut all_results = Vec::with_capacity(chunks.len());
        for batch_result in results {
            all_results.extend(batch_result);
        }
        all_results
    }

    /// Embed a batch using a specific session (for pool workers).
    fn embed_batch_with_session(
        &self,
        session: &mut Session,
        chunks: &[&str],
    ) -> Vec<Option<Vec<f32>>> {
        let sanitized: Vec<String> = chunks.iter().map(|c| sanitize_for_embedding(c)).collect();
        let sanitized_refs: Vec<&str> = sanitized.iter().map(String::as_str).collect();

        let mut all_embeddings = Vec::with_capacity(chunks.len());

        for batch in sanitized_refs.chunks(self.config.batch_size) {
            match self.run_inference(session, batch) {
                Ok(batch_embeddings) => {
                    for emb in batch_embeddings {
                        all_embeddings.push(Some(emb));
                    }
                }
                Err(e) => {
                    tracing::debug!(error = %e, "pool session batch inference failed");
                    for text in batch {
                        let embedding = self.embed_single_with_retry(text, session);
                        all_embeddings.push(embedding);
                    }
                }
            }
        }

        all_embeddings
    }

    /// Returns the model fingerprint for staleness detection.
    ///
    /// Store this alongside vectors in the database. When the model changes
    /// (upgrade, config change), vectors with a different fingerprint are stale
    /// and should be re-embedded.
    pub fn model_fingerprint(&self) -> &str {
        &self.model_fingerprint
    }

    /// Check if vectors produced with the given fingerprint are stale.
    ///
    /// Returns `true` if the stored fingerprint differs from the current model,
    /// meaning those vectors should be re-embedded.
    pub fn is_stale(&self, stored_fingerprint: &str) -> bool {
        stored_fingerprint != self.model_fingerprint
    }

    /// Embed a batch of text chunks.
    ///
    /// Returns a vector where each element corresponds to an input chunk.
    /// If a chunk successfully embeds, `Some(embedding)` is returned.
    /// If embedding fails (e.g., ONNX failure, chunk too large), `None` is returned.
    ///
    /// This implementation includes:
    /// - Content sanitization to handle special characters
    /// - Automatic retry with truncation for oversized chunks
    /// - Individual fallback when batch processing fails
    /// - Detailed logging for debugging coverage issues
    /// - Aggressive skipping of problematic chunks to prevent hangs
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

        // Sanitize chunks before processing
        let sanitized: Vec<String> = chunks.iter().map(|c| sanitize_for_embedding(c)).collect();
        let sanitized_refs: Vec<&str> = sanitized.iter().map(String::as_str).collect();

        // Process in batches with progress logging
        let total_batches =
            (sanitized_refs.len() + self.config.batch_size - 1) / self.config.batch_size;
        for (batch_idx, batch) in sanitized_refs.chunks(self.config.batch_size).enumerate() {
            // Log progress every 10 batches
            if batch_idx % 10 == 0 {
                tracing::info!(
                    batch = batch_idx + 1,
                    total = total_batches,
                    progress_pct = ((batch_idx + 1) as f64 / total_batches as f64 * 100.0) as u32,
                    "embedding progress"
                );
            }

            match self.run_inference(&mut session, batch) {
                Ok(batch_embeddings) => {
                    for emb in batch_embeddings {
                        all_embeddings.push(Some(emb));
                    }
                }
                Err(e) => {
                    tracing::debug!(
                        error = %e,
                        batch_idx = batch_idx,
                        batch_size = batch.len(),
                        "batch inference failed; falling back to individual chunks"
                    );
                    // Fall back to processing chunks one by one with retry logic
                    for (chunk_idx, text) in batch.iter().enumerate() {
                        // Skip extremely large chunks that might cause hangs
                        if text.len() > 100_000 {
                            tracing::warn!(
                                batch_idx = batch_idx,
                                chunk_idx = chunk_idx,
                                text_len = text.len(),
                                "skipping extremely large chunk (>100KB)"
                            );
                            all_embeddings.push(None);
                            continue;
                        }

                        let embedding = self.embed_single_with_retry(text, &mut session);
                        if embedding.is_none() {
                            tracing::debug!(
                                batch_idx = batch_idx,
                                chunk_idx = chunk_idx,
                                text_len = text.len(),
                                "chunk embedding failed after retry"
                            );
                        }
                        all_embeddings.push(embedding);
                    }
                }
            }
        }

        all_embeddings
    }

    /// Process a large set of chunks through the embedding pipeline with progress reporting.
    ///
    /// This is the production-grade batch API for indexing. It:
    /// 1. Splits the input into macro-batches (4x the ONNX batch size)
    /// 2. Processes each macro-batch through `embed_batch`
    /// 3. Calls `on_progress(completed, total)` after each macro-batch
    /// 4. Returns early if `on_progress` returns `false` (cancellation)
    ///
    /// The macro-batch size controls memory backpressure: at most
    /// `batch_size * 4` chunks are in-flight simultaneously.
    pub fn embed_pipeline<F>(&self, chunks: &[&str], mut on_progress: F) -> Vec<Option<Vec<f32>>>
    where
        F: FnMut(usize, usize) -> bool, // (completed, total) -> should_continue
    {
        let total = chunks.len();
        if total == 0 || !self.is_available() {
            return vec![None; total];
        }

        // Macro-batch size: 4x the ONNX batch size for amortized overhead
        let macro_batch_size = self.config.batch_size.saturating_mul(4).max(32);
        let mut all_results = Vec::with_capacity(total);
        let mut completed = 0;

        for macro_batch in chunks.chunks(macro_batch_size) {
            let batch_results = self.embed_batch(macro_batch);
            completed += macro_batch.len();
            all_results.extend(batch_results);

            // Report progress and check for cancellation
            if !on_progress(completed, total) {
                tracing::info!(completed, total, "embedding pipeline cancelled by caller");
                // Fill remaining with None
                let remaining = total - completed;
                all_results.extend(std::iter::repeat_with(|| None).take(remaining));
                break;
            }
        }

        all_results
    }

    /// Embed a single chunk with automatic retry and truncation.
    fn embed_single_with_retry(&self, text: &str, session: &mut Session) -> Option<Vec<f32>> {
        // Skip empty or extremely large chunks
        if text.trim().is_empty() {
            tracing::trace!("skipping empty chunk");
            return None;
        }

        if text.len() > 50_000 {
            tracing::debug!(text_len = text.len(), "chunk too large, truncating to 50KB");
            // Truncate to 50KB immediately for very large chunks
            let truncated = &text[..50_000.min(text.len())];
            match self.run_inference(session, &[truncated]) {
                Ok(mut embs) => return Some(embs.remove(0)),
                Err(e) => {
                    tracing::trace!(error = %e, "truncated large chunk failed");
                    return None;
                }
            }
        }

        // Try with full text first
        match self.run_inference(session, &[text]) {
            Ok(mut embs) => return Some(embs.remove(0)),
            Err(e) => {
                tracing::trace!(error = %e, "first attempt failed, trying with truncation");
            }
        }

        // If that fails, try truncating to max_seq_length
        let max_chars = self.config.max_seq_length * 4; // ~4 chars per token
        if text.len() > max_chars {
            let truncated = &text[..max_chars];
            match self.run_inference(session, &[truncated]) {
                Ok(mut embs) => {
                    tracing::trace!("embedding succeeded after truncation");
                    return Some(embs.remove(0));
                }
                Err(e) => {
                    tracing::trace!(error = %e, "truncation attempt failed");
                }
            }
        }

        // If still failing, try with just the first 512 characters
        if text.len() > 512 {
            let minimal = &text[..512];
            match self.run_inference(session, &[minimal]) {
                Ok(mut embs) => {
                    tracing::trace!("embedding succeeded with minimal content");
                    return Some(embs.remove(0));
                }
                Err(e) => {
                    tracing::trace!(error = %e, "minimal content attempt failed");
                }
            }
        }

        None
    }

    /// Embed a single text string (passage/chunk mode -- no prefix).
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
            Err(OmniError::Internal(
                "embed_batch failed or returned None".into(),
            ))
        }
    }

    /// Embed a query string with asymmetric instruction prefix.
    ///
    /// For bi-encoder retrieval models, prepending a task-specific instruction
    /// to queries improves retrieval quality by 8-15% (Jina AI docs).
    /// Passages are embedded without prefix (via `embed_single`).
    ///
    /// Prefix: "Represent this sentence for searching relevant passages: "
    pub fn embed_query(&self, query: &str) -> OmniResult<Vec<f32>> {
        const QUERY_PREFIX: &str = "Represent this sentence for searching relevant passages: ";
        let prefixed = format!("{QUERY_PREFIX}{query}");
        self.embed_single(&prefixed)
    }

    /// Returns the embedding dimensions.
    pub fn dimensions(&self) -> usize {
        self.config.dimensions
    }

    /// Run ONNX inference on a batch of texts.
    fn run_inference(&self, session: &mut Session, texts: &[&str]) -> OmniResult<Vec<Vec<f32>>> {
        let batch_size = texts.len();
        let max_len = self.config.max_seq_length;

        // Tokenize
        let (input_ids, attention_mask, token_type_ids) = self.tokenize_batch(texts, max_len)?;

        // Create ort tensors using (shape, data) tuple API
        let shape = vec![batch_size as i64, max_len as i64];

        let ids_value = ort::value::Tensor::from_array((shape.clone(), input_ids))
            .map_err(|e| OmniError::Internal(format!("ONNX tensor error: {e}")))?;

        let mask_value = ort::value::Tensor::from_array((shape.clone(), attention_mask.clone()))
            .map_err(|e| OmniError::Internal(format!("ONNX tensor error: {e}")))?;

        // Build inputs dynamically based on what the model expects
        use std::borrow::Cow;
        let mut inputs: Vec<(Cow<'_, str>, ort::session::SessionInputValue<'_>)> = vec![
            (
                Cow::Borrowed("input_ids"),
                ort::session::SessionInputValue::from(ids_value),
            ),
            (
                Cow::Borrowed("attention_mask"),
                ort::session::SessionInputValue::from(mask_value),
            ),
        ];

        // Only add token_type_ids if the model expects it (Jina doesn't, BGE might)
        let expects_token_type = session
            .inputs()
            .iter()
            .any(|i| i.name() == "token_type_ids");
        if expects_token_type {
            let type_value = ort::value::Tensor::from_array((shape.clone(), token_type_ids))
                .map_err(|e| {
                    OmniError::Internal(format!("ONNX tensor error (token_type_ids): {e}"))
                })?;
            inputs.push((
                Cow::Borrowed("token_type_ids"),
                ort::session::SessionInputValue::from(type_value),
            ));
        }

        // Get output name before running (session.outputs() borrows &self)
        let output_name = session
            .outputs()
            .first()
            .map(|o| o.name().to_string())
            .ok_or_else(|| OmniError::Internal("model has no outputs".into()))?;

        // Run inference (requires &mut self)
        let outputs = session
            .run(inputs)
            .map_err(|e| OmniError::Internal(format!("ONNX inference error: {e}")))?;

        // Extract first output tensor
        let output_value = outputs
            .get(&output_name)
            .ok_or_else(|| OmniError::Internal("no output tensor found".into()))?;

        let (output_shape, output_data) = output_value
            .try_extract_tensor::<f32>()
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
    ///
    /// Handles tokenization errors gracefully by:
    /// - Catching encoding failures
    /// - Providing detailed error context
    /// - Ensuring consistent output dimensions
    fn tokenize_batch(
        &self,
        texts: &[&str],
        max_len: usize,
    ) -> OmniResult<(Vec<i64>, Vec<i64>, Vec<i64>)> {
        let tokenizer = self
            .tokenizer
            .as_ref()
            .ok_or_else(|| OmniError::Internal("tokenizer not loaded".into()))?;

        let mut all_input_ids = Vec::with_capacity(texts.len() * max_len);
        let mut all_attention_mask = Vec::with_capacity(texts.len() * max_len);
        let mut all_token_type_ids = Vec::with_capacity(texts.len() * max_len);

        for (idx, text) in texts.iter().enumerate() {
            // Handle empty text
            if text.trim().is_empty() {
                // Add padding for empty text
                for _ in 0..max_len {
                    all_input_ids.push(0);
                    all_attention_mask.push(0);
                    all_token_type_ids.push(0);
                }
                continue;
            }

            let encoding = tokenizer.encode(*text, true).map_err(|e| {
                OmniError::Internal(format!(
                    "tokenization error at index {}: {} (text length: {}, first 100 chars: {})",
                    idx,
                    e,
                    text.len(),
                    &text.chars().take(100).collect::<String>()
                ))
            })?;

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

/// Sanitize text content for embedding to prevent tokenization failures.
///
/// This function:
/// - Replaces null bytes and other control characters
/// - Normalizes whitespace
/// - Ensures valid UTF-8
/// - Truncates extremely long lines that might cause issues
fn sanitize_for_embedding(text: &str) -> String {
    let mut sanitized = String::with_capacity(text.len());

    for line in text.lines() {
        // Skip extremely long lines (> 10k chars) that might cause tokenizer issues
        if line.len() > 10000 {
            sanitized.push_str(&line[..10000]);
            sanitized.push_str(" [truncated]\n");
            continue;
        }

        // Replace problematic characters
        for ch in line.chars() {
            match ch {
                '\0' => sanitized.push(' '), // null byte
                '\x01'..='\x08' | '\x0B'..='\x0C' | '\x0E'..='\x1F' => {
                    // Control characters (except \t, \n, \r)
                    sanitized.push(' ');
                }
                _ => sanitized.push(ch),
            }
        }
        sanitized.push('\n');
    }

    // Trim excessive whitespace
    sanitized.trim().to_string()
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
        let rust =
            format_chunk_for_embedding("rust", "lib::Config::new", "function", "pub fn new() {}");
        assert_eq!(rust, "pub fn new() {}");

        let ts = format_chunk_for_embedding(
            "typescript",
            "UserService.getUser",
            "function",
            "getUser() {}",
        );
        assert_eq!(ts, "getUser() {}");
    }

    #[test]
    fn test_model_fingerprint_degraded() {
        let config = EmbeddingConfig {
            model_path: "/nonexistent/model.onnx".into(),
            dimensions: 768,
            batch_size: 16,
            max_seq_length: 8192,
        };
        let embedder = Embedder::degraded(&config);
        assert!(
            embedder.model_fingerprint().starts_with("degraded:"),
            "degraded fingerprint should start with 'degraded:', got: {}",
            embedder.model_fingerprint()
        );
        assert!(embedder.model_fingerprint().contains("768"));
    }

    #[test]
    fn test_is_stale_same_fingerprint() {
        let config = EmbeddingConfig {
            model_path: "/nonexistent/model.onnx".into(),
            dimensions: 768,
            batch_size: 16,
            max_seq_length: 8192,
        };
        let embedder = Embedder::degraded(&config);
        let fp = embedder.model_fingerprint().to_string();
        assert!(
            !embedder.is_stale(&fp),
            "same fingerprint should not be stale"
        );
    }

    #[test]
    fn test_is_stale_different_fingerprint() {
        let config = EmbeddingConfig {
            model_path: "/nonexistent/model.onnx".into(),
            dimensions: 768,
            batch_size: 16,
            max_seq_length: 8192,
        };
        let embedder = Embedder::degraded(&config);
        assert!(
            embedder.is_stale("old-model:384:512"),
            "different fingerprint should be stale"
        );
    }

    #[test]
    fn test_embed_query_degraded_returns_error() {
        let config = EmbeddingConfig {
            model_path: "/nonexistent/model.onnx".into(),
            dimensions: 384,
            batch_size: 32,
            max_seq_length: 256,
        };
        let embedder = Embedder::degraded(&config);
        let result = embedder.embed_query("how does caching work?");
        assert!(
            result.is_err(),
            "embed_query on degraded should return error"
        );
    }
}
