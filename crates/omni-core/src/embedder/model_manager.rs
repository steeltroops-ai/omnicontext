//! Automatic embedding model management.
//!
//! Downloads and caches the ONNX embedding model and tokenizer on first use.
//! Models are stored in `~/.omnicontext/models/<model-name>/`.
//!
//! ## Model Selection
//!
//! Default model: `jinaai/jina-embeddings-v2-base-code`
//! - Specifically trained on code retrieval (code-to-text, code-to-code)
//! - 768 dimensions, 8192 max sequence length
//! - ONNX-compatible, ~550MB download
//!
//! ## Zero-Hassle Philosophy
//!
//! Enterprise users should never manually download models. The engine
//! auto-detects missing models and downloads them with progress reporting.
//! After download, the model path is stable and cached forever.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{OmniError, OmniResult};

/// Metadata for a supported embedding model.
#[derive(Debug, Clone)]
pub struct ModelSpec {
    /// Human-readable model name.
    pub name: &'static str,
    /// HuggingFace model ID (e.g., "jinaai/jina-embeddings-v2-base-code").
    pub hf_repo: &'static str,
    /// URL to the ONNX model file.
    pub model_url: &'static str,
    /// URL to the tokenizer.json file.
    pub tokenizer_url: &'static str,
    /// Output embedding dimensions.
    pub dimensions: usize,
    /// Maximum sequence length the model supports.
    pub max_seq_length: usize,
    /// Approximate download size in bytes (for progress display).
    pub approx_size_bytes: u64,
}

/// Default model: Jina Code v2 -- specifically trained for code retrieval.
///
/// Why this model:
/// - Trained on code-to-text and code-to-code retrieval tasks
/// - Understands variable names, syntax patterns, cross-language concepts
/// - 768 dimensions provides high-quality embeddings
/// - 8192 token context window (much larger than MiniLM's 256)
/// - ONNX available directly from HuggingFace
pub const DEFAULT_MODEL: ModelSpec = ModelSpec {
    name: "jina-embeddings-v2-base-code",
    hf_repo: "jinaai/jina-embeddings-v2-base-code",
    model_url: "https://huggingface.co/jinaai/jina-embeddings-v2-base-code/resolve/main/onnx/model.onnx",
    tokenizer_url: "https://huggingface.co/jinaai/jina-embeddings-v2-base-code/resolve/main/tokenizer.json",
    dimensions: 768,
    max_seq_length: 8192,
    approx_size_bytes: 550_000_000, // ~550MB
};

/// Fallback model: BGE Small -- for constrained environments or fast indexing.
pub const FALLBACK_MODEL: ModelSpec = ModelSpec {
    name: "bge-small-en-v1.5",
    hf_repo: "BAAI/bge-small-en-v1.5",
    model_url: "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx",
    tokenizer_url: "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/tokenizer.json",
    dimensions: 384,
    max_seq_length: 512,
    approx_size_bytes: 130_000_000, // ~130MB
};

/// Get the models directory: `~/.omnicontext/models/`
fn models_base_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("omnicontext")
        .join("models")
}

/// Get the directory for a specific model: `~/.omnicontext/models/<name>/`
pub fn model_dir(spec: &ModelSpec) -> PathBuf {
    models_base_dir().join(spec.name)
}

/// Get the path to the ONNX model file for a given spec.
pub fn model_path(spec: &ModelSpec) -> PathBuf {
    model_dir(spec).join("model.onnx")
}

/// Get the path to the tokenizer file for a given spec.
pub fn tokenizer_path(spec: &ModelSpec) -> PathBuf {
    model_dir(spec).join("tokenizer.json")
}

/// Check if the model files exist and are valid.
pub fn is_model_ready(spec: &ModelSpec) -> bool {
    let model = model_path(spec);
    let tokenizer = tokenizer_path(spec);

    if !model.exists() || !tokenizer.exists() {
        return false;
    }

    // Verify model file is not empty/corrupted (basic size check)
    if let Ok(meta) = std::fs::metadata(&model) {
        if meta.len() < 1_000_000 {
            // Model file is suspiciously small (< 1MB), likely corrupted
            return false;
        }
    }

    true
}

/// Ensure the model is available, downloading if necessary.
///
/// This is the main entry point for auto-model-management.
/// Call this before creating the ONNX session.
///
/// Returns the paths to (model.onnx, tokenizer.json).
pub fn ensure_model(spec: &ModelSpec) -> OmniResult<(PathBuf, PathBuf)> {
    let model = model_path(spec);
    let tokenizer = tokenizer_path(spec);

    if is_model_ready(spec) {
        tracing::debug!(
            model = spec.name,
            path = %model.display(),
            "embedding model already cached"
        );
        return Ok((model, tokenizer));
    }

    // Create the model directory
    let dir = model_dir(spec);
    std::fs::create_dir_all(&dir)?;

    tracing::info!(
        model = spec.name,
        repo = spec.hf_repo,
        "downloading embedding model (first-time setup)"
    );

    // Download model file
    if !model.exists() || std::fs::metadata(&model).map(|m| m.len() < 1_000_000).unwrap_or(true) {
        download_file(
            spec.model_url,
            &model,
            &format!("Downloading {} model", spec.name),
            Some(spec.approx_size_bytes),
        )?;
    }

    // Download tokenizer
    if !tokenizer.exists() {
        download_file(
            spec.tokenizer_url,
            &tokenizer,
            &format!("Downloading {} tokenizer", spec.name),
            None,
        )?;
    }

    // Write a metadata file for tracking
    let meta_path = dir.join("meta.json");
    let meta = serde_json::json!({
        "model": spec.name,
        "hf_repo": spec.hf_repo,
        "dimensions": spec.dimensions,
        "max_seq_length": spec.max_seq_length,
        "downloaded_at": chrono_now_iso(),
    });
    if let Ok(content) = serde_json::to_string_pretty(&meta) {
        let _ = std::fs::write(&meta_path, content);
    }

    tracing::info!(
        model = spec.name,
        path = %model.display(),
        "embedding model ready"
    );

    Ok((model, tokenizer))
}

/// Download a file from a URL with progress bar.
fn download_file(
    url: &str,
    dest: &Path,
    message: &str,
    expected_size: Option<u64>,
) -> OmniResult<()> {
    // Use a temp file to avoid partial downloads
    let temp_path = dest.with_extension("downloading");

    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600)) // 10 min timeout for large models
        .build()
        .map_err(|e| OmniError::Internal(format!("HTTP client error: {e}")))?
        .get(url)
        .send()
        .map_err(|e| {
            OmniError::Internal(format!(
                "failed to download model from {url}: {e}\n\
                 Hint: Check your internet connection. You can also manually download\n\
                 the model and set OMNI_MODEL_PATH to point to it."
            ))
        })?;

    if !response.status().is_success() {
        return Err(OmniError::Internal(format!(
            "model download failed: HTTP {} from {url}",
            response.status()
        )));
    }

    let total_size = response
        .content_length()
        .or(expected_size)
        .unwrap_or(0);

    // Create progress bar
    let pb = if total_size > 0 {
        let pb = indicatif::ProgressBar::new(total_size);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{msg}\n  [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar())
                .progress_chars("##-"),
        );
        pb.set_message(message.to_string());
        pb
    } else {
        let pb = indicatif::ProgressBar::new_spinner();
        pb.set_message(message.to_string());
        pb
    };

    // Stream download to temp file
    let mut file = std::fs::File::create(&temp_path)?;
    let mut downloaded: u64 = 0;

    // Read in chunks
    let bytes = response.bytes().map_err(|e| {
        OmniError::Internal(format!("download stream error: {e}"))
    })?;

    let chunk_size = 8192;
    for chunk in bytes.chunks(chunk_size) {
        file.write_all(chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    file.flush()?;
    drop(file);

    pb.finish_with_message(format!("{message} -- done"));

    // Atomic rename: temp -> final (prevents corrupt partial files)
    std::fs::rename(&temp_path, dest)?;

    Ok(())
}

/// Get the recommended model spec based on user preference or environment.
///
/// Checks `OMNI_EMBEDDING_MODEL` env var for overrides.
/// - "default" or "jina-code" -> DEFAULT_MODEL (jina-embeddings-v2-base-code)
/// - "small" or "bge-small" -> FALLBACK_MODEL (bge-small-en-v1.5)
/// - Anything else -> DEFAULT_MODEL
pub fn resolve_model_spec() -> &'static ModelSpec {
    if let Ok(model_name) = std::env::var("OMNI_EMBEDDING_MODEL") {
        match model_name.to_lowercase().as_str() {
            "small" | "bge-small" | "bge-small-en" | "lite" => {
                tracing::info!("using lightweight embedding model (bge-small-en-v1.5)");
                return &FALLBACK_MODEL;
            }
            _ => {} // fall through to default
        }
    }

    &DEFAULT_MODEL
}

/// Simple ISO 8601 timestamp without pulling in chrono.
fn chrono_now_iso() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("epoch:{}", now.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_dir_structure() {
        let dir = model_dir(&DEFAULT_MODEL);
        assert!(dir.ends_with("jina-embeddings-v2-base-code"));

        let model = model_path(&DEFAULT_MODEL);
        assert!(model.ends_with("model.onnx"));

        let tokenizer = tokenizer_path(&DEFAULT_MODEL);
        assert!(tokenizer.ends_with("tokenizer.json"));
    }

    #[test]
    fn test_fallback_model_different_path() {
        let default_dir = model_dir(&DEFAULT_MODEL);
        let fallback_dir = model_dir(&FALLBACK_MODEL);
        assert_ne!(default_dir, fallback_dir);
    }

    #[test]
    fn test_resolve_model_default() {
        // Without env var, should return default
        let spec = resolve_model_spec();
        assert_eq!(spec.dimensions, 768);
    }

    #[test]
    fn test_model_not_ready_when_missing() {
        // Non-existent path should not be ready
        let dummy = ModelSpec {
            name: "non-existent-model-xyz-123",
            hf_repo: "fake/repo",
            model_url: "http://fake.com",
            tokenizer_url: "http://fake.com",
            dimensions: 10,
            max_seq_length: 10,
            approx_size_bytes: 10,
        };
        assert!(!is_model_ready(&dummy));
    }

    #[test]
    fn test_default_model_constants() {
        assert_eq!(DEFAULT_MODEL.dimensions, 768);
        assert_eq!(DEFAULT_MODEL.max_seq_length, 8192);
        assert!(DEFAULT_MODEL.model_url.starts_with("https://"));
        assert!(DEFAULT_MODEL.tokenizer_url.starts_with("https://"));
    }

    #[test]
    fn test_fallback_model_constants() {
        assert_eq!(FALLBACK_MODEL.dimensions, 384);
        assert_eq!(FALLBACK_MODEL.max_seq_length, 512);
    }
}
