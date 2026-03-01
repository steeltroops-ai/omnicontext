#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::items_after_statements,
    clippy::manual_let_else,
    clippy::map_unwrap_or,
    clippy::missing_errors_doc
)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ort::session::Session;

use crate::error::{OmniError, OmniResult};

#[derive(Debug, Clone)]
pub struct ModelSpec {
    pub name: &'static str,
    pub model_url: &'static str,
    pub tokenizer_url: &'static str,
}

pub const DEFAULT_MODEL: ModelSpec = ModelSpec {
    name: "ms-marco-MiniLM-L-6-v2",
    model_url: "https://huggingface.co/Xenova/ms-marco-MiniLM-L-6-v2/resolve/main/onnx/model.onnx",
    tokenizer_url: "https://huggingface.co/Xenova/ms-marco-MiniLM-L-6-v2/resolve/main/tokenizer.json",
};

pub struct Reranker {
    session: Option<Mutex<Session>>,
    tokenizer: Option<tokenizers::Tokenizer>,
    max_seq_length: usize,
    #[allow(dead_code)]
    batch_size: usize,
}

impl Reranker {
    /// Create a new reranker using settings from `RerankerConfig`.
    pub fn new(config: &crate::config::RerankerConfig) -> OmniResult<Self> {
        if std::env::var("OMNI_DISABLE_RERANKER").is_ok() {
            return Ok(Self::disabled(config));
        }

        let (model_path, tokenizer_path) = match resolve_model_files() {
            Ok(paths) => paths,
            Err(e) => {
                tracing::warn!(error = %e, "reranker model resolution failed");
                return Ok(Self::disabled(config));
            }
        };

        let session = if model_path.exists() {
            match Session::builder() {
                Ok(builder) => match builder.commit_from_file(&model_path) {
                    Ok(session) => Some(Mutex::new(session)),
                    Err(e) => {
                        tracing::warn!(model = %model_path.display(), error = %e, "failed to load reranker model");
                        None
                    }
                },
                Err(e) => {
                    tracing::warn!(error = %e, "failed to create reranker session");
                    None
                }
            }
        } else {
            None
        };

        let tokenizer = if tokenizer_path.exists() {
            match tokenizers::Tokenizer::from_file(&tokenizer_path) {
                Ok(t) => Some(t),
                Err(e) => {
                    tracing::warn!(tokenizer = %tokenizer_path.display(), error = %e, "failed to load reranker tokenizer");
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            session,
            tokenizer,
            max_seq_length: config.max_seq_length,
            batch_size: config.batch_size,
        })
    }

    fn disabled(config: &crate::config::RerankerConfig) -> Self {
        Self {
            session: None,
            tokenizer: None,
            max_seq_length: config.max_seq_length,
            batch_size: config.batch_size,
        }
    }

    pub fn is_available(&self) -> bool {
        self.session.is_some() && self.tokenizer.is_some()
    }

    pub fn rerank(&self, query: &str, documents: &[&str]) -> Vec<Option<f32>> {
        if !self.is_available() {
            return vec![None; documents.len()];
        }

        let session_mutex = match self.session.as_ref() {
            Some(s) => s,
            None => return vec![None; documents.len()],
        };

        let mut session = match session_mutex.lock() {
            Ok(s) => s,
            Err(_) => return vec![None; documents.len()],
        };

        let mut scores = Vec::with_capacity(documents.len());

        for batch in documents.chunks(self.batch_size) {
            match self.run_inference(&mut session, query, batch) {
                Ok(batch_scores) => {
                    for score in batch_scores {
                        scores.push(Some(score));
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "reranker batch inference failed");
                    for _ in batch {
                        scores.push(None);
                    }
                }
            }
        }

        scores
    }

    fn run_inference(
        &self,
        session: &mut Session,
        query: &str,
        documents: &[&str],
    ) -> OmniResult<Vec<f32>> {
        let batch_size = documents.len();
        let max_len = self.max_seq_length;

        let (input_ids, attention_mask, token_type_ids) =
            self.tokenize_pairs(query, documents, max_len)?;

        let shape = vec![batch_size as i64, max_len as i64];

        let ids_value = ort::value::Tensor::from_array((shape.clone(), input_ids))
            .map_err(|e| OmniError::Internal(format!("ONNX tensor error: {e}")))?;

        let mask_value = ort::value::Tensor::from_array((shape.clone(), attention_mask.clone()))
            .map_err(|e| OmniError::Internal(format!("ONNX tensor error: {e}")))?;

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

        let expects_token_type = session
            .inputs()
            .iter()
            .any(|i| i.name() == "token_type_ids");
        if expects_token_type {
            let type_value = ort::value::Tensor::from_array((shape.clone(), token_type_ids))
                .map_err(|e| OmniError::Internal(format!("ONNX tensor error: {e}")))?;
            inputs.push((
                Cow::Borrowed("token_type_ids"),
                ort::session::SessionInputValue::from(type_value),
            ));
        }

        let output_name = session
            .outputs()
            .first()
            .map(|o| o.name().to_string())
            .ok_or_else(|| OmniError::Internal("reranker model has no outputs".into()))?;

        let outputs = session
            .run(inputs)
            .map_err(|e| OmniError::Internal(format!("ONNX inference error: {e}")))?;

        let output_value = outputs
            .get(&output_name)
            .ok_or_else(|| OmniError::Internal("no output tensor found".into()))?;

        let (output_shape, output_data) = output_value
            .try_extract_tensor::<f32>()
            .map_err(|e| OmniError::Internal(format!("output extraction error: {e}")))?;

        let dims: Vec<usize> = output_shape.iter().map(|&d| d as usize).collect();
        if dims.is_empty() {
            return Err(OmniError::Internal("unexpected output shape".into()));
        }

        let mut scores = Vec::with_capacity(batch_size);
        if dims.len() == 2 {
            let labels = dims[1];
            for b in 0..batch_size {
                let offset = b * labels;
                let score = if labels == 1 {
                    output_data[offset]
                } else {
                    output_data[offset + labels - 1]
                };
                scores.push(score);
            }
        } else if dims.len() == 1 {
            scores.extend_from_slice(&output_data[..batch_size.min(output_data.len())]);
        } else {
            return Err(OmniError::Internal(format!(
                "unexpected output tensor shape: {dims:?}"
            )));
        }

        Ok(scores)
    }

    fn tokenize_pairs(
        &self,
        query: &str,
        documents: &[&str],
        max_len: usize,
    ) -> OmniResult<(Vec<i64>, Vec<i64>, Vec<i64>)> {
        let tokenizer = self
            .tokenizer
            .as_ref()
            .ok_or_else(|| OmniError::Internal("reranker tokenizer not loaded".into()))?;

        let mut all_input_ids = Vec::with_capacity(documents.len() * max_len);
        let mut all_attention_mask = Vec::with_capacity(documents.len() * max_len);
        let mut all_token_type_ids = Vec::with_capacity(documents.len() * max_len);

        for doc in documents {
            let encoding = tokenizer
                .encode(
                    tokenizers::EncodeInput::Dual(query.into(), (*doc).into()),
                    true,
                )
                .map_err(|e| OmniError::Internal(format!("tokenization error: {e}")))?;

            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let type_ids = encoding.get_type_ids();

            let actual_len = ids.len().min(max_len);

            for i in 0..actual_len {
                all_input_ids.push(ids[i] as i64);
                all_attention_mask.push(mask[i] as i64);
                all_token_type_ids.push(type_ids[i] as i64);
            }

            for _ in actual_len..max_len {
                all_input_ids.push(0);
                all_attention_mask.push(0);
                all_token_type_ids.push(0);
            }
        }

        Ok((all_input_ids, all_attention_mask, all_token_type_ids))
    }
}

fn resolve_model_files() -> OmniResult<(PathBuf, PathBuf)> {
    if let Ok(model_path) = std::env::var("OMNI_RERANKER_MODEL_PATH") {
        let model_path = PathBuf::from(model_path);
        if model_path.exists() {
            let tokenizer_path = std::env::var("OMNI_RERANKER_TOKENIZER_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| model_path.with_file_name("tokenizer.json"));
            return Ok((model_path, tokenizer_path));
        }
    }

    let spec = &DEFAULT_MODEL;
    if is_model_ready(spec) {
        return Ok((model_path(spec), tokenizer_path(spec)));
    }

    if std::env::var("OMNI_SKIP_MODEL_DOWNLOAD").is_ok() {
        return Ok((model_path(spec), tokenizer_path(spec)));
    }

    ensure_model(spec)
}

fn models_base_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("omnicontext")
        .join("models")
}

fn model_dir(spec: &ModelSpec) -> PathBuf {
    models_base_dir().join(spec.name)
}

fn model_path(spec: &ModelSpec) -> PathBuf {
    model_dir(spec).join("model.onnx")
}

fn tokenizer_path(spec: &ModelSpec) -> PathBuf {
    model_dir(spec).join("tokenizer.json")
}

fn is_model_ready(spec: &ModelSpec) -> bool {
    let model = model_path(spec);
    let tokenizer = tokenizer_path(spec);
    if !model.exists() || !tokenizer.exists() {
        return false;
    }
    std::fs::metadata(&model)
        .map(|m| m.len() > 1_000_000)
        .unwrap_or(false)
}

fn ensure_model(spec: &ModelSpec) -> OmniResult<(PathBuf, PathBuf)> {
    let model = model_path(spec);
    let tokenizer = tokenizer_path(spec);

    if is_model_ready(spec) {
        return Ok((model, tokenizer));
    }

    let dir = model_dir(spec);
    std::fs::create_dir_all(&dir)?;

    if !model.exists()
        || std::fs::metadata(&model)
            .map(|m| m.len() < 1_000_000)
            .unwrap_or(true)
    {
        download_file(spec.model_url, &model)?;
    }

    if !tokenizer.exists() {
        download_file(spec.tokenizer_url, &tokenizer)?;
    }

    Ok((model, tokenizer))
}

fn download_file(url: &str, dest: &Path) -> OmniResult<()> {
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| OmniError::Internal(format!("HTTP client error: {e}")))?
        .get(url)
        .send()
        .map_err(|e| OmniError::Internal(format!("download failed: {e}")))?;

    if !response.status().is_success() {
        return Err(OmniError::Internal(format!(
            "download failed: HTTP {}",
            response.status()
        )));
    }

    let temp_path = dest.with_extension("downloading");
    let bytes = response
        .bytes()
        .map_err(|e| OmniError::Internal(format!("download stream error: {e}")))?;
    std::fs::write(&temp_path, bytes)?;
    std::fs::rename(&temp_path, dest)?;
    Ok(())
}
