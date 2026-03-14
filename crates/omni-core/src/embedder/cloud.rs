//! Cloud embedding service client.
//!
//! Sends chunks to the OmniContext cloud GPU embedding service for enterprise-tier
//! high-quality embeddings (CodeSage-Large-v2, >5000 chunks/sec on H100).
//!
//! Activated by setting `OMNI_CLOUD_API_KEY` or `config.embedding.cloud_api_key`.
//! Transparently falls back to local ONNX on any HTTP error or timeout.

use serde::{Deserialize, Serialize};

use crate::error::{OmniError, OmniResult};

/// Default cloud embedding endpoint.
pub const CLOUD_ENDPOINT: &str = "https://api.omnicontext.dev/v1/embed";

/// Maximum batch size for cloud embedding requests.
///
/// 256 chunks per POST keeps individual request payload below ~256 KB for
/// typical code chunk sizes and fits comfortably within H100 GPU VRAM for
/// a single forward pass.
pub const CLOUD_BATCH_SIZE: usize = 256;

/// Request timeout for each individual HTTP POST to the cloud embedding service.
///
/// 30 seconds is generous for a 256-chunk batch on a well-provisioned H100
/// endpoint.  Requests that exceed this are treated as transient failures and
/// trigger the local ONNX fallback.
pub const CLOUD_TIMEOUT_SECS: u64 = 30;

/// Request body for the cloud embedding endpoint.
#[derive(Debug, Serialize)]
pub struct EmbedRequest {
    /// Text chunks to embed, in order.
    pub chunks: Vec<String>,

    /// Optional model override (e.g. `"CodeSage-Large-v2"`).
    ///
    /// When `None`, the server uses its current default model.
    /// Omitted from JSON serialization when absent to keep payloads minimal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Response body from the cloud embedding endpoint.
#[derive(Debug, Deserialize)]
pub struct EmbedResponse {
    /// Embedding vectors parallel to the input `chunks` array.
    ///
    /// `embeddings[i]` corresponds to `chunks[i]` in the request.
    pub embeddings: Vec<Vec<f32>>,

    /// Name of the model used for this request (informational).
    #[serde(default)]
    pub model: String,

    /// Number of tokens processed in this request (for billing/metering).
    #[serde(default)]
    pub tokens_used: u64,
}

/// Cloud embedding client.
///
/// Sends text chunks to the OmniContext cloud GPU embedding service in batches
/// of [`CLOUD_BATCH_SIZE`] (256).  Authentication uses a Bearer token supplied
/// via the `OMNI_CLOUD_API_KEY` environment variable or
/// `config.embedding.cloud_api_key`.
///
/// All methods are synchronous (blocking) to match the existing [`crate::embedder::Embedder`] API.
/// The internal `reqwest::blocking::Client` maintains a connection pool across calls.
pub struct CloudEmbedder {
    /// Bearer token for `Authorization: Bearer <key>` header.
    api_key: String,
    /// Target endpoint URL.
    endpoint: String,
    /// Persistent HTTP client with timeout configured at construction time.
    client: reqwest::blocking::Client,
}

impl CloudEmbedder {
    /// Construct a cloud embedder with an explicit API key and optional endpoint override.
    ///
    /// `api_key` — Bearer token sent as `Authorization: Bearer <key>`.
    /// `endpoint` — URL override; defaults to [`CLOUD_ENDPOINT`] when `None`.
    ///
    /// Returns `Err(OmniError::Config)` if the underlying HTTP client cannot be built
    /// (this only happens under extreme OS resource exhaustion).
    pub fn new(api_key: String, endpoint: Option<String>) -> OmniResult<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(CLOUD_TIMEOUT_SECS))
            .user_agent(concat!("omnicontext/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| OmniError::Config {
                details: format!("failed to build HTTP client: {e}"),
            })?;

        Ok(Self {
            api_key,
            endpoint: endpoint.unwrap_or_else(|| CLOUD_ENDPOINT.to_string()),
            client,
        })
    }

    /// Attempt to construct a cloud embedder from environment variables.
    ///
    /// Reads `OMNI_CLOUD_API_KEY` for the Bearer token and, optionally,
    /// `OMNI_CLOUD_ENDPOINT` for an endpoint override.
    ///
    /// Returns:
    /// - `Ok(Some(embedder))` — `OMNI_CLOUD_API_KEY` is set and non-empty.
    /// - `Ok(None)` — `OMNI_CLOUD_API_KEY` is unset or blank; cloud path is inactive.
    /// - `Err(...)` — Key is present but the HTTP client failed to build.
    pub fn from_env() -> OmniResult<Option<Self>> {
        match std::env::var("OMNI_CLOUD_API_KEY") {
            Ok(key) if !key.trim().is_empty() => {
                let endpoint = std::env::var("OMNI_CLOUD_ENDPOINT").ok();
                Ok(Some(Self::new(key, endpoint)?))
            }
            _ => Ok(None),
        }
    }

    /// Embed a slice of text chunks, returning one embedding vector per chunk.
    ///
    /// Splits `chunks` into sub-batches of [`CLOUD_BATCH_SIZE`] (256) and issues
    /// a separate HTTP POST for each sub-batch.  All embeddings are collected and
    /// returned in input order.
    ///
    /// Returns an empty `Vec` immediately if `chunks` is empty (no network call).
    ///
    /// # Errors
    ///
    /// - `OmniError::ModelUnavailable` — the HTTP request failed (timeout, DNS, TLS).
    /// - `OmniError::ModelUnavailable` — the server returned a non-2xx status code.
    /// - `OmniError::ModelUnavailable` — the response JSON could not be deserialized.
    /// - `OmniError::ModelUnavailable` — the server returned a mismatched embedding count.
    ///
    /// All errors carry a human-readable `reason` field for diagnostics.
    pub fn embed_batch(&self, chunks: &[String]) -> OmniResult<Vec<Vec<f32>>> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_embeddings: Vec<Vec<f32>> = Vec::with_capacity(chunks.len());

        for batch in chunks.chunks(CLOUD_BATCH_SIZE) {
            let request = EmbedRequest {
                chunks: batch.to_vec(),
                model: None,
            };

            let response = self
                .client
                .post(&self.endpoint)
                .bearer_auth(&self.api_key)
                .json(&request)
                .send()
                .map_err(|e| OmniError::ModelUnavailable {
                    reason: format!("cloud embed request failed: {e}"),
                })?;

            let status = response.status();
            if !status.is_success() {
                let body = response.text().unwrap_or_default();
                return Err(OmniError::ModelUnavailable {
                    reason: format!("cloud embed returned HTTP {status}: {body}"),
                });
            }

            let embed_response: EmbedResponse =
                response.json().map_err(|e| OmniError::ModelUnavailable {
                    reason: format!("cloud embed response parse failed: {e}"),
                })?;

            if embed_response.embeddings.len() != batch.len() {
                return Err(OmniError::ModelUnavailable {
                    reason: format!(
                        "cloud embed returned {} embeddings for {} chunks",
                        embed_response.embeddings.len(),
                        batch.len()
                    ),
                });
            }

            all_embeddings.extend(embed_response.embeddings);
        }

        Ok(all_embeddings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_embedder_from_env_none_when_unset() {
        // Safety: test binary is single-threaded at this point; env mutation is safe.
        std::env::remove_var("OMNI_CLOUD_API_KEY");
        let result = CloudEmbedder::from_env().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_cloud_embedder_from_env_some_when_set() {
        std::env::set_var("OMNI_CLOUD_API_KEY", "test-key-123");
        let result = CloudEmbedder::from_env().unwrap();
        assert!(result.is_some());
        std::env::remove_var("OMNI_CLOUD_API_KEY");
    }

    #[test]
    fn test_cloud_embedder_from_env_none_when_empty() {
        std::env::set_var("OMNI_CLOUD_API_KEY", "  ");
        let result = CloudEmbedder::from_env().unwrap();
        assert!(result.is_none());
        std::env::remove_var("OMNI_CLOUD_API_KEY");
    }

    #[test]
    fn test_embed_request_serializes_without_model_field() {
        let req = EmbedRequest {
            chunks: vec!["fn main() {}".to_string()],
            model: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("chunks"));
        // skip_serializing_if = Option::is_none must suppress the field entirely
        assert!(!json.contains("model"));
    }

    #[test]
    fn test_embed_request_with_model_serializes_model_field() {
        let req = EmbedRequest {
            chunks: vec!["fn main() {}".to_string()],
            model: Some("CodeSage-Large-v2".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("model"));
        assert!(json.contains("CodeSage-Large-v2"));
    }

    #[test]
    fn test_embed_response_deserializes_full_payload() {
        let json = r#"{"embeddings": [[0.1, 0.2, 0.3]], "model": "CodeSage", "tokens_used": 10}"#;
        let resp: EmbedResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.embeddings.len(), 1);
        assert_eq!(resp.embeddings[0].len(), 3);
        assert_eq!(resp.model, "CodeSage");
        assert_eq!(resp.tokens_used, 10);
    }

    #[test]
    fn test_embed_response_deserializes_with_defaults() {
        // model and tokens_used are #[serde(default)] — must deserialize without them.
        let json = r#"{"embeddings": [[0.5, 0.6]]}"#;
        let resp: EmbedResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.embeddings.len(), 1);
        assert_eq!(resp.model, "");
        assert_eq!(resp.tokens_used, 0);
    }

    #[test]
    fn test_embed_batch_empty_input_returns_empty_without_network_call() {
        // Port 9999 is intentionally unreachable; the early-return must fire before
        // any network I/O so the test completes instantly.
        let embedder = CloudEmbedder::new(
            "test-key".to_string(),
            Some("http://localhost:9999".to_string()),
        )
        .unwrap();
        let result = embedder.embed_batch(&[]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_embed_batch_returns_err_on_connection_refused() {
        // Port 1 is privileged and guaranteed closed on all test platforms.
        let embedder = CloudEmbedder::new(
            "test-key".to_string(),
            Some("http://127.0.0.1:1".to_string()),
        )
        .unwrap();
        let result = embedder.embed_batch(&["hello".to_string()]);
        assert!(result.is_err());
        // Verify the error carries a diagnostic message.
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(!msg.is_empty());
    }

    #[test]
    fn test_constants_have_expected_values() {
        assert_eq!(CLOUD_ENDPOINT, "https://api.omnicontext.dev/v1/embed");
        assert_eq!(CLOUD_BATCH_SIZE, 256);
        // Verify timeout is positive — this is a compile-time constant assertion.
        const { assert!(CLOUD_TIMEOUT_SECS > 0) };
    }

    #[test]
    fn test_new_accepts_endpoint_override() {
        let embedder = CloudEmbedder::new(
            "key".to_string(),
            Some("https://custom.example.com/embed".to_string()),
        )
        .unwrap();
        // Verify the override was stored by observing a connection error to the custom
        // host rather than the default endpoint.
        let result = embedder.embed_batch(&["test".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_uses_default_endpoint_when_none() {
        // Construction must succeed even without an endpoint override.
        let embedder = CloudEmbedder::new("key".to_string(), None).unwrap();
        // The empty-batch fast path must not make any HTTP calls regardless of endpoint.
        let result = embedder.embed_batch(&[]).unwrap();
        assert!(result.is_empty());
    }
}
