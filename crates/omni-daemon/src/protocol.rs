//! JSON-RPC protocol types for daemon IPC.
//!
//! All communication between the VS Code extension and the daemon
//! uses newline-delimited JSON-RPC 2.0 messages over named pipes.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// JSON-RPC 2.0 envelope
// ---------------------------------------------------------------------------

/// A JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// Protocol version, always "2.0".
    pub jsonrpc: String,
    /// Request ID for correlating responses.
    pub id: u64,
    /// Method name.
    pub method: String,
    /// Method parameters (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// A JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Protocol version, always "2.0".
    pub jsonrpc: String,
    /// Request ID this response corresponds to.
    pub id: u64,
    /// Successful result (mutually exclusive with `error`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error result (mutually exclusive with `result`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    /// Error code.
    pub code: i32,
    /// Human-readable error message.
    pub message: String,
    /// Additional error data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Response {
    /// Create a success response.
    pub fn success(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: u64, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Method-specific parameter types
// ---------------------------------------------------------------------------

/// Parameters for the `search` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParams {
    /// The search query.
    pub query: String,
    /// Maximum results.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

/// Parameters for the `context_window` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindowParams {
    /// The search query.
    pub query: String,
    /// Maximum search results to consider.
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Token budget for the context window.
    pub token_budget: Option<u32>,
    /// Active file path (for cursor-aware context).
    pub active_file: Option<String>,
    /// Cursor line in the active file.
    pub cursor_line: Option<u32>,
    /// Currently open files in the editor.
    #[serde(default)]
    pub open_files: Vec<String>,
}

/// Parameters for the `get_module_map` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMapParams {
    /// Maximum depth for the module tree.
    pub max_depth: Option<usize>,
}

/// Parameters for pre-flight context injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightParams {
    /// The user's prompt text.
    pub prompt: String,
    /// Active file path.
    pub active_file: Option<String>,
    /// Cursor line.
    pub cursor_line: Option<u32>,
    /// Open file paths.
    #[serde(default)]
    pub open_files: Vec<String>,
    /// Token budget for injected context.
    #[serde(default = "default_token_budget")]
    pub token_budget: u32,
    /// Intent classification hint (edit, explain, debug, refactor).
    pub intent: Option<String>,
}

/// Pre-flight context injection response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightResponse {
    /// The system prompt to inject before the user's prompt.
    pub system_context: String,
    /// Number of context entries included.
    pub entries_count: usize,
    /// Total tokens consumed by the context.
    pub tokens_used: u32,
    /// Token budget this was assembled for.
    pub token_budget: u32,
    /// Time taken in milliseconds.
    pub elapsed_ms: u64,
    /// Whether this response was served from cache.
    pub from_cache: bool,
}

/// Parameters for IDE event notifications (for pre-fetch).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeEventParams {
    /// Event type: `file_opened`, `cursor_moved`, `text_edited`.
    pub event_type: String,
    /// File path for the event.
    pub file_path: String,
    /// Cursor line (for `cursor_moved` events).
    pub cursor_line: Option<u32>,
    /// Symbol at cursor (if available).
    pub symbol: Option<String>,
}

/// Parameters for getting pre-fetch cache statistics.
#[allow(dead_code)] // TODO: Remove when used in VS Code extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchStatsResponse {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Current cache size.
    pub size: usize,
    /// Hit rate (0.0 to 1.0).
    pub hit_rate: f64,
}

/// Parameters for updating pre-fetch cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfigParams {
    /// New cache capacity (maximum number of entries).
    pub cache_size: Option<usize>,
    /// New cache TTL in seconds.
    pub cache_ttl_seconds: Option<u64>,
}

/// Parameters for clearing the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearIndexParams {
    /// Whether to also clear the vector index.
    #[serde(default = "default_true")]
    pub clear_vectors: bool,
}

fn default_true() -> bool {
    true
}

/// System status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusResponse {
    /// Initialization status: "initializing", "ready", "error".
    pub initialization_status: String,
    /// Connection health: "connected", "disconnected", "reconnecting".
    pub connection_health: String,
    /// Last index time (Unix timestamp in seconds).
    pub last_index_time: Option<u64>,
    /// Daemon uptime in seconds.
    pub daemon_uptime_seconds: u64,
    /// Number of files indexed.
    pub files_indexed: usize,
    /// Number of chunks indexed.
    pub chunks_indexed: usize,
}

/// Performance metrics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetricsResponse {
    /// Search latency P50 (median) in milliseconds.
    pub search_latency_p50_ms: f64,
    /// Search latency P95 in milliseconds.
    pub search_latency_p95_ms: f64,
    /// Search latency P99 in milliseconds.
    pub search_latency_p99_ms: f64,
    /// Embedding coverage percentage (0.0 to 100.0).
    pub embedding_coverage_percent: f64,
    /// Current memory usage in bytes.
    pub memory_usage_bytes: u64,
    /// Peak memory usage in bytes since daemon start.
    pub peak_memory_usage_bytes: u64,
    /// Total number of searches performed.
    pub total_searches: u64,
}

fn default_limit() -> usize {
    10
}

fn default_token_budget() -> u32 {
    8192
}

// ---------------------------------------------------------------------------
// Error codes
// ---------------------------------------------------------------------------

/// Standard JSON-RPC error codes.
pub mod error_codes {
    #![allow(dead_code)]
    /// Invalid JSON was received by the server.
    pub const PARSE_ERROR: i32 = -32700;
    /// The JSON sent is not a valid Request object.
    pub const INVALID_REQUEST: i32 = -32600;
    /// The method does not exist / is not available.
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid method parameter(s).
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal JSON-RPC error.
    pub const INTERNAL_ERROR: i32 = -32603;
    /// Engine-specific error (indexing, search, etc.).
    pub const ENGINE_ERROR: i32 = -32000;
}
