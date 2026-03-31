//! Automatic embedding model management.
//!
//! Downloads and caches the ONNX embedding model and tokenizer on first use.
//! Models are stored in `~/.omnicontext/models/<model-name>/`.
//!
//! ## Model Selection
//!
//! Default model: `nomic-ai/CodeRankEmbed`
//! - 137M parameter bi-encoder trained on CoRNStack with InfoNCE contrastive loss
//! - Initialized from Arctic-Embed-M-Long; outperforms CodeSage-Large (1.3B) on CodeSearchNet
//! - 768 dimensions, 2048 max sequence length
//! - Apache-2.0 license — safe for commercial distribution
//! - ONNX export: ~521MB download
//!
//! Enterprise / GPU tier: `Qwen/Qwen3-Embedding-8B` (Apache-2.0, 75.22 MTEB English,
//! instruction-aware, 100+ languages) — selectable via `OMNI_EMBEDDING_MODEL=qwen3`.
//!
//! ## Zero-Hassle Philosophy
//!
//! Enterprise users should never manually download models. The engine
//! auto-detects missing models and downloads them with progress reporting.
//! After download, the model path is stable and cached forever.

use sha2::Digest as _;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{OmniError, OmniResult};

/// Metadata for a supported embedding model.
#[derive(Debug, Clone)]
pub struct ModelSpec {
    /// Human-readable model name.
    pub name: &'static str,
    /// HuggingFace model ID (e.g., "nomic-ai/CodeRankEmbed").
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
    /// Expected SHA-256 hex digest of the ONNX model file.
    ///
    /// When `Some`, the downloaded file is verified against this digest after
    /// the atomic rename.  A mismatch causes the file to be deleted and an
    /// error to be returned, preventing a corrupted or tampered model from
    /// being loaded into the ONNX runtime.
    ///
    /// `None` skips the verification step entirely (safe — no false positives).
    pub sha256: Option<&'static str>,
}

/// Primary embedding model: `nomic-ai/CodeRankEmbed` (Apache-2.0).
///
/// Architecture: bi-encoder, 137M parameters, initialized from Arctic-Embed-M-Long.
/// Training: CoRNStack dataset with dual-consistency filtering + InfoNCE contrastive loss
/// with curriculum hard negatives (code-to-code and text-to-code retrieval tasks).
///
/// Why this model:
/// - Apache-2.0 license — safe for all commercial distribution (replaces CC-BY-NC jina)
/// - 768 dimensions matches the previous model; no schema migration required
/// - Outperforms CodeSage-Large (1.3B) on CodeSearchNet despite being 10× smaller
/// - ONNX export available directly from HuggingFace: ~521MB
/// - 2048 token context sufficient for typical code chunks (512–1024 tokens)
///
/// Query prefix: "Represent this code snippet for searching relevant code: "
/// (applied automatically in `embed_query()`)
///
/// NO FALLBACK: This is the only embedding model. If it fails to load,
/// the system retries with exponential backoff and self-heals to keyword-only mode.
pub const DEFAULT_MODEL: ModelSpec = ModelSpec {
    name: "CodeRankEmbed",
    hf_repo: "nomic-ai/CodeRankEmbed",
    model_url: "https://huggingface.co/nomic-ai/CodeRankEmbed/resolve/main/onnx/model.onnx",
    tokenizer_url: "https://huggingface.co/nomic-ai/CodeRankEmbed/resolve/main/tokenizer.json",
    dimensions: 768,
    max_seq_length: 2048,
    approx_size_bytes: 521_000_000, // ~521MB ONNX export
    sha256: None, // TODO: pin once canonical HF ONNX digest is published
};

/// Enterprise GPU-tier embedding model: `Qwen/Qwen3-Embedding-8B` (Apache-2.0).
///
/// 75.22 MTEB English score, surpasses proprietary models including Gemini Embedding Medium.
/// Instruction-aware architecture, 100+ languages, 8B parameters.
/// Selectable via `OMNI_EMBEDDING_MODEL=qwen3`.
/// Not used by default — requires GPU and significant VRAM (>16GB).
pub const QWEN3_EMBEDDING_MODEL: ModelSpec = ModelSpec {
    name: "Qwen3-Embedding-8B",
    hf_repo: "Qwen/Qwen3-Embedding-8B",
    model_url: "https://huggingface.co/Qwen/Qwen3-Embedding-8B/resolve/main/onnx/model.onnx",
    tokenizer_url: "https://huggingface.co/Qwen/Qwen3-Embedding-8B/resolve/main/tokenizer.json",
    dimensions: 4096,
    max_seq_length: 32768,
    approx_size_bytes: 16_000_000_000, // ~16GB (GPU tier)
    sha256: None, // TODO: pin once canonical HF ONNX digest is published
};

/// BGE-M3 multi-function embedding model: `BAAI/bge-m3` (Apache-2.0).
///
/// BGE-M3 produces three vector types from a single pass:
/// - Dense vectors (1024-dim) for semantic similarity
/// - Sparse SPLADE-style vectors (top-K token_id→weight pairs)
/// - ColBERT multi-vectors for late interaction retrieval
///
/// Only the sparse track is used here (see `Embedder::embed_sparse()`).
/// Dense retrieval continues to use CodeRankEmbed for code-specific quality.
///
/// Downloaded only when `config.embedding.enable_sparse_retrieval = true`.
/// License: Apache-2.0 — safe for commercial distribution.
/// IMPORTANT: Verify the specific ONNX export license before production distribution.
pub const BGE_M3_MODEL: ModelSpec = ModelSpec {
    name: "bge-m3",
    hf_repo: "BAAI/bge-m3",
    model_url: "https://huggingface.co/BAAI/bge-m3/resolve/main/onnx/model.onnx",
    tokenizer_url: "https://huggingface.co/BAAI/bge-m3/resolve/main/tokenizer.json",
    dimensions: 1024,
    max_seq_length: 8192,
    approx_size_bytes: 560_000_000, // ~560MB ONNX export
    sha256: None, // TODO: pin once canonical HF ONNX digest is published
};

/// Cross-encoder reranker: `BAAI/bge-reranker-v2-m3` (Apache-2.0).
///
/// Cross-encoder architecture: takes (query, passage) pair as input, outputs a single
/// relevance score directly — fundamentally different from bi-encoder embedding models.
///
/// Why this model:
/// - Apache-2.0 license — replaces CC-BY-NC jina reranker; safe for commercial distribution
/// - ONNX export at `mogolloni/bge-reranker-v2-m3-onnx`: ~568MB
/// - Trained on multilingual passage ranking with strong code understanding
/// - Cross-encoder architecture consistently outperforms bi-encoder reranking
/// - Output: single logit per pair; apply sigmoid for [0, 1] relevance probability
///
/// NOTE: `dimensions` is set to 1 because the output is a single score,
/// not an embedding vector.
pub const RERANKER_MODEL: ModelSpec = ModelSpec {
    name: "bge-reranker-v2-m3",
    hf_repo: "mogolloni/bge-reranker-v2-m3-onnx",
    model_url: "https://huggingface.co/mogolloni/bge-reranker-v2-m3-onnx/resolve/main/model.onnx",
    tokenizer_url: "https://huggingface.co/BAAI/bge-reranker-v2-m3/resolve/main/tokenizer.json",
    dimensions: 1, // single relevance score output, not an embedding vector
    max_seq_length: 1024,
    approx_size_bytes: 568_000_000, // ~568MB ONNX export
    sha256: None, // TODO: pin once canonical HF ONNX digest is published
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

    if !model.exists() {
        tracing::debug!(
            path = %model.display(),
            "model file does not exist"
        );
        return false;
    }

    if !tokenizer.exists() {
        tracing::debug!(
            path = %tokenizer.display(),
            "tokenizer file does not exist"
        );
        return false;
    }

    // Verify model file is not truncated or corrupted
    if let Ok(meta) = std::fs::metadata(&model) {
        let size = meta.len();
        let expected = spec.approx_size_bytes;

        // File must be at least 80% of expected size
        if size < (expected * 80 / 100) {
            tracing::error!(
                path = %model.display(),
                actual_bytes = size,
                expected_bytes = expected,
                "model file appears truncated or corrupt (< 80% of expected size), will re-download"
            );
            // Delete corrupt file to force re-download
            let _ = std::fs::remove_file(&model);
            return false;
        }

        // File must not be suspiciously small
        if size < 1_000_000 {
            tracing::error!(
                path = %model.display(),
                actual_bytes = size,
                "model file is too small (< 1MB), likely corrupt"
            );
            let _ = std::fs::remove_file(&model);
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
    if !model.exists()
        || std::fs::metadata(&model)
            .map(|m| m.len() < 1_000_000)
            .unwrap_or(true)
    {
        download_file(
            spec.model_url,
            &model,
            &format!("Downloading {} model", spec.name),
            Some(spec.approx_size_bytes),
            spec.sha256,
        )?;
    }

    // Download tokenizer
    if !tokenizer.exists() {
        download_file(
            spec.tokenizer_url,
            &tokenizer,
            &format!("Downloading {} tokenizer", spec.name),
            None,
            None, // tokenizer is small JSON; no integrity pin needed
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
///
/// Uses `tokio::task::block_in_place` when called from within an async runtime
/// to avoid panics from `reqwest::blocking` nesting a second tokio runtime.
fn download_file(
    url: &str,
    dest: &Path,
    message: &str,
    expected_size: Option<u64>,
    expected_sha256: Option<&str>,
) -> OmniResult<()> {
    // If we're inside a tokio runtime, use block_in_place to allow blocking I/O.
    // reqwest::blocking creates its own internal runtime, which panics if a
    // tokio runtime is already running on this thread.
    if tokio::runtime::Handle::try_current().is_ok() {
        return tokio::task::block_in_place(|| {
            download_file_inner(url, dest, message, expected_size, expected_sha256)
        });
    }
    download_file_inner(url, dest, message, expected_size, expected_sha256)
}

fn download_file_inner(
    url: &str,
    dest: &Path,
    message: &str,
    expected_size: Option<u64>,
    expected_sha256: Option<&str>,
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

    let total_size = response.content_length().or(expected_size).unwrap_or(0);

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
    let bytes = response
        .bytes()
        .map_err(|e| OmniError::Internal(format!("download stream error: {e}")))?;

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

    // SHA-256 integrity check — guards against download corruption and supply-chain tampering.
    // Performed after the rename so any deletion targets the final path, not a temp artefact.
    verify_sha256_after_download(dest, expected_sha256)?;

    Ok(())
}

/// Resolve the active embedding model spec.
///
/// Returns `DEFAULT_MODEL` (`CodeRankEmbed`, Apache-2.0) unless overridden by the
/// `OMNI_EMBEDDING_MODEL` environment variable.
///
/// Supported values for `OMNI_EMBEDDING_MODEL`:
/// - `"default"` / `"coderankeembed"` / `""` → `CodeRankEmbed` (137M, Apache-2.0)
/// - `"qwen3"` / `"qwen3-embedding"` → `Qwen3-Embedding-8B` (8B, GPU-tier, Apache-2.0)
///
/// Self-Healing Architecture:
/// - If model download fails: retry with exponential backoff (max 5 attempts)
/// - If model is corrupted: auto-delete and re-download
/// - If ONNX session fails: circuit breaker opens, system retries after cooldown
/// - Degraded mode: keyword-only search when embedding is unavailable
pub fn resolve_model_spec() -> &'static ModelSpec {
    if let Ok(model_name) = std::env::var("OMNI_EMBEDDING_MODEL") {
        match model_name.to_lowercase().as_str() {
            "default" | "coderankeembed" | "nomic" | "nomic-code" | "" => {}
            "qwen3" | "qwen3-embedding" | "qwen3-8b" => {
                tracing::info!(
                    model = "Qwen3-Embedding-8B",
                    "using enterprise GPU-tier embedding model (OMNI_EMBEDDING_MODEL=qwen3)"
                );
                return &QWEN3_EMBEDDING_MODEL;
            }
            other => {
                tracing::warn!(
                    requested = other,
                    fallback = "CodeRankEmbed",
                    "unrecognized OMNI_EMBEDDING_MODEL value, using CodeRankEmbed (Apache-2.0)"
                );
            }
        }
    }

    &DEFAULT_MODEL
}

/// Verify the SHA-256 digest of a file on disk.
///
/// Called after an atomic rename so that any deletion targets the final path,
/// not a temporary artefact.  When `expected` is `None` the function is a
/// guaranteed no-op and returns `Ok(())` immediately.
///
/// On mismatch the file is deleted so the next startup triggers a clean
/// re-download rather than loading a corrupt or tampered model.
fn verify_sha256_after_download(path: &Path, expected: Option<&str>) -> OmniResult<()> {
    let Some(expected_hex) = expected else {
        return Ok(());
    };

    let data = std::fs::read(path)
        .map_err(|e| OmniError::Internal(format!("failed to read downloaded file for checksum: {e}")))?;
    let digest = sha2::Sha256::digest(&data);
    let actual = hex::encode(digest);

    if actual != expected_hex {
        tracing::warn!(
            path = %path.display(),
            expected = expected_hex,
            actual = %actual,
            "SHA-256 mismatch — deleting file to force re-download on next startup"
        );
        let _ = std::fs::remove_file(path);
        return Err(OmniError::ModelUnavailable {
            reason: "SHA-256 checksum mismatch — possible corruption or supply-chain tampering"
                .into(),
        });
    }

    tracing::debug!(
        path = %path.display(),
        sha256 = %actual,
        "SHA-256 integrity check passed"
    );
    Ok(())
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
        assert!(dir.ends_with("CodeRankEmbed"));

        let model = model_path(&DEFAULT_MODEL);
        assert!(model.ends_with("model.onnx"));

        let tokenizer = tokenizer_path(&DEFAULT_MODEL);
        assert!(tokenizer.ends_with("tokenizer.json"));
    }

    #[test]
    fn test_reranker_model_different_path() {
        let default_dir = model_dir(&DEFAULT_MODEL);
        let reranker_dir = model_dir(&RERANKER_MODEL);
        assert_ne!(default_dir, reranker_dir);
    }

    #[test]
    fn test_resolve_model_default() {
        // Without env var, should return default (CodeRankEmbed)
        std::env::remove_var("OMNI_EMBEDDING_MODEL");
        let spec = resolve_model_spec();
        assert_eq!(spec.dimensions, 768);
        assert_eq!(spec.name, "CodeRankEmbed");
    }

    #[test]
    fn test_resolve_model_qwen3() {
        std::env::set_var("OMNI_EMBEDDING_MODEL", "qwen3");
        let spec = resolve_model_spec();
        assert_eq!(spec.name, "Qwen3-Embedding-8B");
        assert_eq!(spec.dimensions, 4096);
        std::env::remove_var("OMNI_EMBEDDING_MODEL");
    }

    #[test]
    fn test_resolve_model_unknown_falls_back_to_default() {
        std::env::set_var("OMNI_EMBEDDING_MODEL", "some-unknown-model");
        let spec = resolve_model_spec();
        assert_eq!(
            spec.name, "CodeRankEmbed",
            "unknown model must fall back to CodeRankEmbed"
        );
        std::env::remove_var("OMNI_EMBEDDING_MODEL");
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
            sha256: None,
        };
        assert!(!is_model_ready(&dummy));
    }

    #[test]
    fn test_default_model_constants() {
        assert_eq!(DEFAULT_MODEL.dimensions, 768);
        assert_eq!(DEFAULT_MODEL.max_seq_length, 2048);
        assert!(DEFAULT_MODEL.model_url.starts_with("https://"));
        assert!(DEFAULT_MODEL.tokenizer_url.starts_with("https://"));
        // Confirm Apache-2.0-compatible repo (not jinaai)
        assert!(DEFAULT_MODEL.hf_repo.starts_with("nomic-ai/"));
    }

    #[test]
    fn test_reranker_model_constants() {
        assert_eq!(RERANKER_MODEL.dimensions, 1);
        assert_eq!(RERANKER_MODEL.max_seq_length, 1024);
        // Confirm Apache-2.0-compatible repo (not jinaai)
        assert!(!RERANKER_MODEL.hf_repo.starts_with("jinaai/"));
    }

    #[test]
    fn test_qwen3_model_constants() {
        assert_eq!(QWEN3_EMBEDDING_MODEL.dimensions, 4096);
        assert_eq!(QWEN3_EMBEDDING_MODEL.max_seq_length, 32768);
        assert!(QWEN3_EMBEDDING_MODEL.hf_repo.starts_with("Qwen/"));
    }

    /// `sha256: None` must be a no-op — the file must remain on disk and the
    /// function must return `Ok(())`.
    #[test]
    fn test_sha256_verification_skipped_when_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("model.onnx");
        std::fs::write(&dest, b"dummy model content").expect("write");

        // download_file_inner normally does the HTTP fetch, so we test only the
        // post-rename verification path via the public helper.
        let result = verify_sha256_after_download(&dest, None);
        assert!(result.is_ok(), "None sha256 must skip check: {result:?}");
        assert!(dest.exists(), "file must not be deleted when sha256 is None");
    }

    /// When the file's actual digest matches `expected`, the function succeeds.
    #[test]
    fn test_sha256_verification_passes_on_correct_hash() {
        use sha2::Digest as _;

        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("model.onnx");
        let content = b"known content for hash test";
        std::fs::write(&dest, content).expect("write");

        let expected = hex::encode(sha2::Sha256::digest(content));
        let result = verify_sha256_after_download(&dest, Some(expected.as_str()));
        assert!(result.is_ok(), "correct hash must pass: {result:?}");
        assert!(dest.exists(), "file must stay after a passing check");
    }

    /// When the digest does not match, the function must return `Err` AND
    /// delete the file so the next startup triggers a fresh download.
    #[test]
    fn test_sha256_verification_fails_on_wrong_hash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("model.onnx");
        std::fs::write(&dest, b"real content").expect("write");

        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = verify_sha256_after_download(&dest, Some(wrong_hash));
        assert!(result.is_err(), "wrong hash must return Err");
        assert!(
            matches!(result.unwrap_err(), OmniError::ModelUnavailable { .. }),
            "error variant must be ModelUnavailable"
        );
        assert!(!dest.exists(), "corrupt file must be deleted on mismatch");
    }
}
