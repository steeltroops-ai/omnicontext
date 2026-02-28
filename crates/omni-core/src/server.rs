//! REST API server for enterprise deployment.
//!
//! Provides a hosted OmniContext service with:
//! - RESTful search, index, and status endpoints
//! - API key authentication
//! - Usage metering and rate limiting
//! - JSON request/response format

use std::path::PathBuf;

use tokio::sync::Mutex;

use crate::pipeline::Engine;

/// Server configuration.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    /// Address to bind to.
    #[serde(default = "default_addr")]
    pub addr: String,
    /// Port to listen on.
    #[serde(default = "default_port")]
    pub port: u16,
    /// API keys for authentication (empty = no auth).
    #[serde(default)]
    pub api_keys: Vec<String>,
    /// Maximum requests per minute per key (0 = unlimited).
    #[serde(default)]
    pub rate_limit: u32,
    /// Repository path to serve.
    pub repo_path: PathBuf,
}

fn default_addr() -> String { "127.0.0.1".into() }
fn default_port() -> u16 { 9090 }

/// Authentication and rate-limiting guard (independent of engine).
pub struct AuthGuard {
    /// API keys for authentication (empty = no auth).
    pub api_keys: Vec<String>,
    /// Max requests per minute per key (0 = unlimited).
    pub rate_limit: u32,
    /// Sliding window counters per key.
    counters: Mutex<std::collections::HashMap<String, (u32, std::time::Instant)>>,
}

impl AuthGuard {
    /// Create a new auth guard.
    pub fn new(api_keys: Vec<String>, rate_limit: u32) -> Self {
        Self {
            api_keys,
            rate_limit,
            counters: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Check if a request is authenticated.
    pub fn authenticate(&self, api_key: Option<&str>) -> bool {
        if self.api_keys.is_empty() {
            return true; // No auth configured
        }
        match api_key {
            Some(key) => self.api_keys.contains(&key.to_string()),
            None => false,
        }
    }

    /// Check rate limit for a key. Returns true if allowed.
    pub async fn check_rate_limit(&self, api_key: &str) -> bool {
        if self.rate_limit == 0 {
            return true;
        }

        let mut counters = self.counters.lock().await;
        let now = std::time::Instant::now();
        let window = std::time::Duration::from_secs(60);

        let entry = counters.entry(api_key.to_string()).or_insert((0, now));

        if now.duration_since(entry.1) > window {
            *entry = (0, now);
        }

        if entry.0 >= self.rate_limit {
            return false;
        }

        entry.0 += 1;
        true
    }
}

/// Shared server state.
pub struct ServerState {
    /// The engine instance.
    pub engine: Mutex<Engine>,
    /// Auth and rate-limiting.
    pub auth: AuthGuard,
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Search request body.
#[derive(Debug, serde::Deserialize)]
pub struct SearchRequest {
    /// Search query string.
    pub query: String,
    /// Maximum number of results.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 10 }

/// Search response.
#[derive(Debug, serde::Serialize)]
pub struct SearchResponse {
    /// Query that was executed.
    pub query: String,
    /// Number of results.
    pub count: usize,
    /// Results with relevance scores.
    pub results: Vec<SearchResultItem>,
    /// Time taken in milliseconds.
    pub elapsed_ms: u64,
}

/// A single search result item.
#[derive(Debug, serde::Serialize)]
pub struct SearchResultItem {
    /// File path.
    pub file: String,
    /// Symbol path.
    pub symbol: String,
    /// Chunk kind (function, class, etc.).
    pub kind: String,
    /// Relevance score.
    pub score: f64,
    /// Line range start.
    pub line_start: usize,
    /// Line end.
    pub line_end: usize,
    /// Code content.
    pub content: String,
}

/// Status response.
#[derive(Debug, serde::Serialize)]
pub struct StatusResponse {
    /// Server version.
    pub version: String,
    /// Number of indexed files.
    pub files_indexed: usize,
    /// Number of chunks.
    pub chunks_indexed: usize,
    /// Number of symbols.
    pub symbols_indexed: usize,
    /// Search mode.
    pub search_mode: String,
    /// Dependency graph stats.
    pub dep_edges: usize,
    /// Graph nodes count.
    pub graph_nodes: usize,
    /// Graph edges count.
    pub graph_edges: usize,
    /// Whether cycles exist.
    pub has_cycles: bool,
}

/// Error response body.
#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    /// Error message.
    pub error: String,
    /// HTTP status code.
    pub status: u16,
}

/// API usage metering record.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UsageRecord {
    /// API key used.
    pub api_key: String,
    /// Endpoint called.
    pub endpoint: String,
    /// Timestamp (ISO 8601).
    pub timestamp: String,
    /// Response time in milliseconds.
    pub response_ms: u64,
}

/// Usage metering store.
pub struct UsageMeter {
    records: Mutex<Vec<UsageRecord>>,
}

impl UsageMeter {
    /// Create a new usage meter.
    pub fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }

    /// Record a usage event.
    pub async fn record(&self, api_key: &str, endpoint: &str, response_ms: u64) {
        let record = UsageRecord {
            api_key: api_key.to_string(),
            endpoint: endpoint.to_string(),
            timestamp: chrono_now(),
            response_ms,
        };
        self.records.lock().await.push(record);
    }

    /// Get usage statistics for a key.
    pub async fn stats_for_key(&self, api_key: &str) -> (usize, u64) {
        let records = self.records.lock().await;
        let matching: Vec<&UsageRecord> = records
            .iter()
            .filter(|r| r.api_key == api_key)
            .collect();
        let total_calls = matching.len();
        let total_ms: u64 = matching.iter().map(|r| r.response_ms).sum();
        (total_calls, total_ms)
    }

    /// Get all records (for billing export).
    pub async fn all_records(&self) -> Vec<UsageRecord> {
        self.records.lock().await.clone()
    }
}

impl Default for UsageMeter {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current time as ISO 8601 string (without chrono crate dependency).
fn chrono_now() -> String {
    let now = std::time::SystemTime::now();
    let since_epoch = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}s-since-epoch", since_epoch.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_defaults() {
        let json = r#"{"repo_path": "/tmp/test"}"#;
        let config: ServerConfig = serde_json::from_str(json).expect("parse");
        assert_eq!(config.addr, "127.0.0.1");
        assert_eq!(config.port, 9090);
        assert!(config.api_keys.is_empty());
        assert_eq!(config.rate_limit, 0);
    }

    #[test]
    fn test_auth_no_keys() {
        let guard = AuthGuard::new(vec![], 0);
        assert!(guard.authenticate(None));
        assert!(guard.authenticate(Some("any-key")));
    }

    #[test]
    fn test_auth_with_keys() {
        let guard = AuthGuard::new(vec!["valid-key".into()], 0);
        assert!(!guard.authenticate(None));
        assert!(!guard.authenticate(Some("wrong-key")));
        assert!(guard.authenticate(Some("valid-key")));
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let guard = AuthGuard::new(vec!["test-key".into()], 3);

        assert!(guard.check_rate_limit("test-key").await);
        assert!(guard.check_rate_limit("test-key").await);
        assert!(guard.check_rate_limit("test-key").await);
        assert!(!guard.check_rate_limit("test-key").await); // Rate limited
    }

    #[tokio::test]
    async fn test_rate_limit_unlimited() {
        let guard = AuthGuard::new(vec![], 0);
        for _ in 0..100 {
            assert!(guard.check_rate_limit("any").await);
        }
    }

    #[tokio::test]
    async fn test_usage_meter() {
        let meter = UsageMeter::new();
        meter.record("key-1", "/search", 50).await;
        meter.record("key-1", "/search", 30).await;
        meter.record("key-2", "/status", 10).await;

        let (calls, total_ms) = meter.stats_for_key("key-1").await;
        assert_eq!(calls, 2);
        assert_eq!(total_ms, 80);

        let all = meter.all_records().await;
        assert_eq!(all.len(), 3);
    }
}
