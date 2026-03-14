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
/// Enhanced with LSP-resolved symbol metadata for precise context pre-loading.
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
    /// Fully qualified symbol name from LSP.
    pub symbol_fqn: Option<String>,
    /// Symbol kind (Function, Class, Method, etc.).
    pub symbol_kind: Option<String>,
    /// Type signature from LSP hover.
    pub type_signature: Option<String>,
    /// File where symbol is defined (for cross-file pre-fetch).
    pub definition_file: Option<String>,
    /// Line where symbol is defined.
    pub definition_line: Option<u32>,
}

/// Pre-fetch cache statistics response.
#[allow(dead_code)] // Response schema type -- used for documentation/serialization reference
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
    /// Confirmation token. Must be the literal string "CONFIRM_CLEAR" to proceed.
    /// This prevents accidental index destruction from automated/malicious callers.
    #[serde(default)]
    pub confirm: Option<String>,
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
    /// Invalid JSON was received by the server.
    pub const PARSE_ERROR: i32 = -32700;
    /// The JSON sent is not a valid Request object.
    #[allow(dead_code)] // Reserved for future request validation
    pub const INVALID_REQUEST: i32 = -32600;
    /// The method does not exist / is not available.
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid method parameter(s).
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal JSON-RPC error.
    pub const INTERNAL_ERROR: i32 = -32603;
    /// Engine-specific error (indexing, search, etc.).
    pub const ENGINE_ERROR: i32 = -32000;
    /// Server overloaded (503 Service Unavailable equivalent).
    pub const SERVER_OVERLOADED: i32 = -32001;
    /// Feature not yet implemented.
    pub const NOT_IMPLEMENTED: i32 = -32002;
}

/// Parameters for search intent classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIntentParams {
    /// The search query to classify.
    pub query: String,
}

// ---------------------------------------------------------------------------
// Resilience monitoring types
// ---------------------------------------------------------------------------

/// Parameters for resilience status request (no parameters needed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResilienceStatusParams {}

/// Resilience status response with circuit breaker and health monitoring data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResilienceStatusResponse {
    /// Circuit breaker states by subsystem.
    pub circuit_breakers: std::collections::HashMap<String, CircuitBreakerState>,
    /// Health status by subsystem.
    pub health_status: std::collections::HashMap<String, HealthStatus>,
    /// Event deduplication statistics.
    pub deduplication: DeduplicationMetrics,
    /// Backpressure monitoring statistics.
    pub backpressure: BackpressureMetrics,
}

/// Circuit breaker state for a subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerState {
    /// Current state: "closed", "open", "half_open".
    pub state: String,
    /// Number of consecutive failures.
    pub failure_count: usize,
    /// Timestamp of last failure (Unix timestamp in seconds).
    pub last_failure_time: Option<u64>,
    /// Timestamp when circuit will attempt recovery (Unix timestamp in seconds).
    pub next_attempt_time: Option<u64>,
}

/// Health status for a subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Health state: "healthy", "degraded", "unhealthy".
    pub status: String,
    /// Last health check timestamp (Unix timestamp in seconds).
    pub last_check_time: u64,
    /// Error message if unhealthy.
    pub error_message: Option<String>,
}

/// Event deduplication metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationMetrics {
    /// Total events processed.
    pub events_processed: u64,
    /// Number of duplicate events skipped.
    pub duplicates_skipped: u64,
    /// Number of events currently in flight.
    pub in_flight_count: usize,
    /// Deduplication rate (0.0 to 1.0).
    pub deduplication_rate: f64,
}

/// Backpressure monitoring metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureMetrics {
    /// Number of active requests.
    pub active_requests: usize,
    /// Current load percentage (0.0 to 100.0).
    pub load_percent: f64,
    /// Number of requests rejected due to backpressure.
    pub requests_rejected: u64,
    /// Peak load percentage since daemon start.
    pub peak_load_percent: f64,
}

/// Parameters for resetting circuit breakers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetCircuitBreakerParams {
    /// Subsystem name to reset ("embedder", "reranker", "index", "vector", or "all").
    pub subsystem: String,
}

// ---------------------------------------------------------------------------
// Historical context types
// ---------------------------------------------------------------------------

/// Parameters for commit context request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitContextParams {
    /// File path to get commit history for.
    pub file_path: String,
    /// Maximum number of commits to return.
    #[serde(default = "default_commit_limit")]
    pub limit: usize,
}

fn default_commit_limit() -> usize {
    10
}

/// Commit context response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitContextResponse {
    /// File path.
    pub file_path: String,
    /// Total commits indexed.
    pub commits_indexed: usize,
    /// Recent commits for this file.
    pub recent_commits: Vec<CommitSummary>,
}

/// Commit summary information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    /// Commit hash.
    pub hash: String,
    /// Commit message.
    pub message: String,
    /// Author name.
    pub author: String,
    /// Commit timestamp (Unix timestamp in seconds).
    pub timestamp: i64,
    /// Files changed in this commit.
    pub files_changed: usize,
    /// Lines added.
    pub lines_added: usize,
    /// Lines deleted.
    pub lines_deleted: usize,
}

/// Parameters for co-change analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoChangeParams {
    /// File path to analyze.
    pub file_path: String,
    /// Minimum co-change frequency (0.0 to 1.0).
    #[serde(default = "default_min_frequency")]
    pub min_frequency: f64,
    /// Maximum results to return.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_min_frequency() -> f64 {
    0.1
}

/// Co-change analysis response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoChangeResponse {
    /// Focal file path.
    pub file_path: String,
    /// Files that frequently change together.
    pub co_changed_files: Vec<CoChangeFile>,
}

/// Co-change file information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoChangeFile {
    /// File path.
    pub path: String,
    /// Co-change frequency (0.0 to 1.0).
    pub frequency: f64,
    /// Number of times changed together.
    pub change_count: usize,
}

/// Parameters for bug-prone files request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugProneFilesParams {
    /// Maximum results to return.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

/// Bug-prone files response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugProneFilesResponse {
    /// Files with high bug frequency.
    pub files: Vec<BugProneFile>,
}

/// Bug-prone file information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugProneFile {
    /// File path.
    pub path: String,
    /// Number of bug-related commits.
    pub bug_count: usize,
    /// Last bug commit timestamp (Unix timestamp in seconds).
    pub last_bug_date: Option<i64>,
    /// Bug frequency (bugs per total commits).
    pub bug_frequency: f64,
}

/// Parameters for plan auditing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPlanParams {
    /// The plan text to audit.
    pub plan: String,
    /// Maximum depth for blast radius analysis.
    pub max_depth: Option<usize>,
}

// ---------------------------------------------------------------------------
// Graph visualization types
// ---------------------------------------------------------------------------

/// Parameters for architectural context request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalContextParams {
    /// File path to get context for.
    pub file_path: String,
    /// Maximum hops from focal file.
    #[serde(default = "default_max_hops")]
    pub max_hops: usize,
}

fn default_max_hops() -> usize {
    2
}

/// Architectural context response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalContextResponse {
    /// Focal file path.
    pub focal_file: String,
    /// Neighbor files with distances and edge types.
    pub neighbors: Vec<NeighborFileInfo>,
    /// Total files in graph.
    pub total_files: usize,
    /// Maximum hops used.
    pub max_hops: usize,
}

/// Neighbor file information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborFileInfo {
    /// File path.
    pub path: String,
    /// Distance in hops from focal file.
    pub distance: usize,
    /// Edge types connecting to focal file.
    pub edge_types: Vec<String>,
    /// Importance score.
    pub importance: f32,
}

/// Cycle detection response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CyclesResponse {
    /// Number of cycles found.
    pub cycle_count: usize,
    /// List of cycles (each cycle is a list of symbol IDs).
    pub cycles: Vec<Vec<i64>>,
}

// ---------------------------------------------------------------------------
// Multi-repo workspace types
// ---------------------------------------------------------------------------

/// Repository information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryInfo {
    /// Repository path.
    pub path: String,
    /// Repository name (derived from path).
    pub name: String,
    /// Search priority (0.0 to 1.0).
    pub priority: f32,
    /// Number of files indexed.
    pub files_indexed: usize,
    /// Whether auto-indexing is enabled.
    pub auto_index: bool,
}

/// Parameters for adding a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddRepoParams {
    /// Repository path.
    pub path: String,
    /// Search priority (0.0 to 1.0).
    #[serde(default = "default_priority")]
    pub priority: f32,
    /// Whether to auto-index on changes.
    #[serde(default = "default_true")]
    pub auto_index: bool,
}

fn default_priority() -> f32 {
    0.5
}

/// Parameters for setting repository priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPriorityParams {
    /// Repository path.
    pub path: String,
    /// New priority (0.0 to 1.0).
    pub priority: f32,
}

/// Parameters for removing a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveRepoParams {
    /// Repository path.
    pub path: String,
}

// ---------------------------------------------------------------------------
// Performance control types
// ---------------------------------------------------------------------------

/// Embedder metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedderMetrics {
    /// Quantization mode (fp32, fp16, int8).
    pub quantization_mode: String,
    /// Memory usage in MB.
    pub memory_usage_mb: f64,
    /// Memory savings percentage compared to fp32.
    pub memory_savings_percent: f64,
    /// Throughput in chunks per second.
    pub throughput_chunks_per_sec: f64,
    /// Batch fill rate (0.0 to 1.0).
    pub batch_fill_rate: f64,
    /// Current batch size.
    pub batch_size: usize,
    /// Batch timeout in milliseconds.
    pub batch_timeout_ms: u64,
}

/// Parameters for configuring embedder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigureEmbedderParams {
    /// Quantization mode (fp32, fp16, int8).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization_mode: Option<String>,
    /// Batch size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<usize>,
    /// Batch timeout in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_timeout_ms: Option<u64>,
}

/// Index pool metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexPoolMetrics {
    /// Number of active connections.
    pub active_connections: usize,
    /// Maximum pool size.
    pub max_pool_size: usize,
    /// Pool utilization percentage.
    pub utilization_percent: f64,
    /// Total queries executed.
    pub total_queries: u64,
    /// Average query time in milliseconds.
    pub avg_query_time_ms: f64,
}

/// Compression statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    /// Total bytes before compression.
    pub bytes_before: u64,
    /// Total bytes after compression.
    pub bytes_after: u64,
    /// Compression ratio.
    pub compression_ratio: f64,
    /// Compression savings percentage.
    pub savings_percent: f64,
}
