//! HTTP Server-Sent Events transport for the `OmniContext` MCP server.
//!
//! ## Architecture
//!
//! Each SSE connection follows a session-per-connection model:
//!
//! ```text
//! AI Agent
//!   │
//!   ├─ GET /sse                            ← opens persistent event stream
//!   │       │
//!   │       └─ event: endpoint             ← server immediately sends endpoint URL
//!   │          data: /message?session_id=<uuid>
//!   │
//!   └─ POST /message?session_id=<uuid>     ← sends JSON-RPC requests
//!           │
//!           └─ dispatcher processes request, response streamed back via SSE
//! ```
//!
//! The MCP JSON-RPC dispatcher handles all standard MCP protocol methods
//! (`initialize`, `tools/list`, `tools/call`) by routing through a shared
//! [`crate::tools::OmniContextServer`] instance, bypassing the rmcp stdio
//! transport layer.
//!
//! ## Authentication
//!
//! When `OMNI_SERVER_TOKEN` is set, both `/sse` and `/message` require an
//! `Authorization: Bearer <token>` header. Without it, requests are rejected
//! with `401 Unauthorized`. When the env var is absent, no auth is enforced
//! (localhost-only assumption).
//!
//! ## CORS
//!
//! All origins are allowed, enabling browser-based clients and cross-origin
//! enterprise integrations.

use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Router,
};
use futures_util::{stream, StreamExt as _};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::tools::OmniContextServer;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the SSE transport server.
#[derive(Debug, Clone)]
pub struct SseConfig {
    /// Host to bind to. Defaults to `"127.0.0.1"` (loopback-only).
    pub host: String,
    /// Port to listen on. Defaults to `8080`.
    pub port: u16,
    /// Optional bearer token. `None` means no authentication is required.
    ///
    /// When set, both `/sse` and `/message` validate the
    /// `Authorization: Bearer <token>` header.
    pub token: Option<String>,
}

impl Default for SseConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            token: None,
        }
    }
}

impl SseConfig {
    /// Load configuration from environment variables.
    ///
    /// | Variable            | Field  | Default     |
    /// |---------------------|--------|-------------|
    /// | `OMNI_SERVER_HOST`  | `host` | `127.0.0.1` |
    /// | `OMNI_SERVER_PORT`  | `port` | `8080`      |
    /// | `OMNI_SERVER_TOKEN` | `token`| _(none)_    |
    pub fn from_env() -> Self {
        let host = std::env::var("OMNI_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

        let port = std::env::var("OMNI_SERVER_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8080);

        let token = std::env::var("OMNI_SERVER_TOKEN").ok();

        Self { host, port, token }
    }

    /// Format the bind address as `host:port`.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

// ---------------------------------------------------------------------------
// Session registry
// ---------------------------------------------------------------------------

/// A single active SSE session.
///
/// The sender half pushes SSE-formatted strings into the stream held open by
/// the `/sse` handler. The `/message` endpoint writes into this sender after
/// dispatching the JSON-RPC request.
struct Session {
    /// Channel sender — push SSE payload strings here.
    tx: mpsc::Sender<String>,
}

/// Shared application state injected into every axum handler.
struct AppState {
    /// Active SSE sessions, keyed by UUID.
    sessions: Mutex<HashMap<String, Session>>,
    /// Server configuration (auth token, bind address).
    config: SseConfig,
    /// Shared MCP server, used to dispatch tool calls across all sessions.
    server: OmniContextServer,
}

// ---------------------------------------------------------------------------
// JSON-RPC primitive types
// ---------------------------------------------------------------------------

/// A minimal JSON-RPC 2.0 request envelope.
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

/// A JSON-RPC 2.0 success response.
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: serde_json::Value,
    result: serde_json::Value,
}

/// A JSON-RPC 2.0 error response.
#[derive(Debug, Serialize)]
struct JsonRpcError {
    jsonrpc: &'static str,
    id: serde_json::Value,
    error: JsonRpcErrorObject,
}

/// The error object nested inside a JSON-RPC error response.
#[derive(Debug, Serialize)]
struct JsonRpcErrorObject {
    code: i32,
    message: String,
}

// JSON-RPC error codes (spec §5.1)
const ERR_PARSE_ERROR: i32 = -32700;
const ERR_METHOD_NOT_FOUND: i32 = -32601;
const ERR_INTERNAL: i32 = -32603;

/// Build a JSON-RPC error response string.
fn jsonrpc_error(id: serde_json::Value, code: i32, message: &str) -> String {
    serde_json::to_string(&JsonRpcError {
        jsonrpc: "2.0",
        id,
        error: JsonRpcErrorObject {
            code,
            message: message.to_string(),
        },
    })
    .unwrap_or_else(|_| {
        r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"serialization failed"}}"#
            .to_string()
    })
}

/// Build a JSON-RPC success response string.
fn jsonrpc_ok(id: serde_json::Value, result: serde_json::Value) -> String {
    serde_json::to_string(&JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result,
    })
    .unwrap_or_else(|_| r#"{"jsonrpc":"2.0","id":null,"result":{}}"#.to_string())
}

// ---------------------------------------------------------------------------
// MCP JSON-RPC dispatcher
// ---------------------------------------------------------------------------

/// Dispatch a single MCP JSON-RPC request string and return the response string.
///
/// Handles the MCP lifecycle methods:
///
/// | Method                        | Behaviour                                               |
/// |-------------------------------|---------------------------------------------------------|
/// | `initialize`                  | Returns server capabilities and version.               |
/// | `tools/list`                  | Returns the full tool catalogue via `OmniContextServer`.|
/// | `tools/call`                  | Invokes the named tool via `OmniContextServer`.         |
/// | `ping`                        | Returns an empty success response.                      |
/// | `notifications/initialized`   | Notification — produces no response (empty string).    |
/// | _(anything else)_             | Returns a JSON-RPC method-not-found error.              |
///
/// An empty return string signals a notification — the caller must not send it.
async fn dispatch(raw: &str, server: &OmniContextServer) -> String {
    // Parse the JSON-RPC envelope.
    let req: JsonRpcRequest = match serde_json::from_str(raw) {
        Ok(r) => r,
        Err(e) => {
            return jsonrpc_error(
                serde_json::Value::Null,
                ERR_PARSE_ERROR,
                &format!("parse error: {e}"),
            );
        }
    };

    let id = req.id.clone().unwrap_or(serde_json::Value::Null);

    match req.method.as_str() {
        // ------------------------------------------------------------------
        // initialize — return server capabilities
        // ------------------------------------------------------------------
        "initialize" => {
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "omnicontext",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "instructions": "OmniContext provides deep code intelligence for AI coding agents. \
                                 Use search_code for general queries, context_window for token-budget-aware \
                                 context, get_symbol for specific lookups, and tools/list for the full catalogue."
            });
            jsonrpc_ok(id, result)
        }

        // ------------------------------------------------------------------
        // ping — liveness probe
        // ------------------------------------------------------------------
        "ping" => jsonrpc_ok(id, serde_json::json!({})),

        // ------------------------------------------------------------------
        // notifications/initialized — no response required
        // ------------------------------------------------------------------
        "notifications/initialized" => {
            // Client notifies that initialisation is complete.
            // Return empty to signal the caller to skip sending.
            String::new()
        }

        // ------------------------------------------------------------------
        // tools/list — enumerate available tools via the rmcp ToolRouter
        // ------------------------------------------------------------------
        "tools/list" => {
            let tools = server.list_tools_json();
            jsonrpc_ok(id, serde_json::json!({ "tools": tools }))
        }

        // ------------------------------------------------------------------
        // tools/call — invoke a tool by name
        // ------------------------------------------------------------------
        "tools/call" => {
            let params = req.params.unwrap_or(serde_json::Value::Null);
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let tool_args = params
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            if tool_name.is_empty() {
                return jsonrpc_error(id, ERR_INTERNAL, "tools/call requires 'name' parameter");
            }

            match server.call_tool_json(tool_name, tool_args).await {
                Ok(content) => jsonrpc_ok(
                    id,
                    serde_json::json!({
                        "content": content,
                        "isError": false
                    }),
                ),
                Err(e) => jsonrpc_ok(
                    id,
                    serde_json::json!({
                        "content": [{ "type": "text", "text": e }],
                        "isError": true
                    }),
                ),
            }
        }

        // ------------------------------------------------------------------
        // Unknown method
        // ------------------------------------------------------------------
        other => jsonrpc_error(
            id,
            ERR_METHOD_NOT_FOUND,
            &format!("method not found: {other}"),
        ),
    }
}

// ---------------------------------------------------------------------------
// Authentication helper
// ---------------------------------------------------------------------------

/// Check the `Authorization: Bearer <token>` header against the configured token.
///
/// Returns `Ok(())` if auth passes (no token configured, or valid token provided).
/// Returns `Err(StatusCode::UNAUTHORIZED)` if a token is configured but the
/// header is missing or the token is incorrect.
fn check_auth(headers: &HeaderMap, config: &SseConfig) -> Result<(), StatusCode> {
    let Some(ref expected) = config.token else {
        return Ok(());
    };

    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match bearer {
        Some(token) if token == expected => Ok(()),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

// ---------------------------------------------------------------------------
// Query parameter extractors
// ---------------------------------------------------------------------------

/// Query parameters accepted by the `/sse` and `/message` endpoints.
#[derive(Debug, Deserialize)]
struct SessionQuery {
    session_id: Option<String>,
}

// ---------------------------------------------------------------------------
// axum handlers
// ---------------------------------------------------------------------------

/// `GET /sse` — establish a persistent SSE connection.
///
/// On connection:
/// 1. Authenticates the request (if a token is configured).
/// 2. Allocates a new session UUID.
/// 3. Registers the session sender in `AppState`.
/// 4. Immediately sends an `endpoint` event containing the `/message` URL
///    with the session ID embedded, so the client knows where to POST.
/// 5. Streams all subsequent MCP responses as `message` SSE events.
async fn sse_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    check_auth(&headers, &state.config)?;

    let session_id = Uuid::new_v4().to_string();

    // Per-connection channel: dispatcher pushes SSE payload strings, the
    // stream adapter pulls them out and wraps them as SSE events.
    let (tx, rx) = mpsc::channel::<String>(64);

    {
        let mut sessions = state.sessions.lock().await;
        sessions.insert(session_id.clone(), Session { tx: tx.clone() });
    }

    tracing::info!(session = %session_id, "SSE connection established");

    let cleanup_id = session_id.clone();
    let cleanup_state = Arc::clone(&state);

    // Build the SSE stream. The first event is always the endpoint URL.
    // Subsequent events arrive via the mpsc channel.
    let endpoint_event = Event::default()
        .event("endpoint")
        .data(format!("/message?session_id={session_id}"));

    // Convert the receiver into a stream of SSE events.
    let rx_stream = ReceiverStream::new(rx)
        .map(|payload| Ok::<Event, Infallible>(Event::default().event("message").data(payload)));

    // Prepend the endpoint event then chain the receiver stream.
    let first = stream::once(async move { Ok::<Event, Infallible>(endpoint_event) });
    let combined = first.chain(rx_stream);

    // Spawn a cleanup task: remove the session when the sender closes.
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            if tx.is_closed() {
                let mut sessions = cleanup_state.sessions.lock().await;
                sessions.remove(&cleanup_id);
                tracing::info!(session = %cleanup_id, "SSE session cleaned up");
                break;
            }
            // Safety check: if the session was already removed, stop polling.
            let still_present = cleanup_state
                .sessions
                .lock()
                .await
                .contains_key(&cleanup_id);
            if !still_present {
                break;
            }
        }
    });

    let sse = Sse::new(combined).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    );

    Ok(sse)
}

/// `POST /message?session_id=<uuid>` — receive an MCP JSON-RPC request.
///
/// The body must be a valid JSON-RPC 2.0 request string. The handler:
/// 1. Authenticates the request.
/// 2. Locates the session by UUID.
/// 3. Dispatches the request through the MCP dispatcher asynchronously.
/// 4. Pushes the JSON-RPC response string into the session's SSE channel.
/// 5. Returns `202 Accepted` immediately (non-blocking).
async fn message_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SessionQuery>,
    body: String,
) -> StatusCode {
    if let Err(code) = check_auth(&headers, &state.config) {
        return code;
    }

    let session_id = if let Some(ref id) = params.session_id {
        id.clone()
    } else {
        tracing::warn!("POST /message called without session_id");
        return StatusCode::BAD_REQUEST;
    };

    let sender = {
        let sessions = state.sessions.lock().await;
        sessions.get(&session_id).map(|s| s.tx.clone())
    };

    let Some(tx) = sender else {
        tracing::warn!(session = %session_id, "POST /message: session not found");
        return StatusCode::NOT_FOUND;
    };

    // Dispatch asynchronously so we return 202 immediately.
    let server = state.server.clone();
    tokio::spawn(async move {
        let response = dispatch(&body, &server).await;

        // Empty response signals a notification (no reply expected).
        if response.is_empty() {
            return;
        }

        if let Err(e) = tx.send(response).await {
            tracing::warn!(
                session = %session_id,
                error = %e,
                "failed to send MCP response to SSE stream"
            );
        }
    });

    StatusCode::ACCEPTED
}

/// `GET /health` — liveness probe.
///
/// Returns `200 OK` with a JSON body. Authentication is not required;
/// intended for load balancers and container orchestrators.
async fn health_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "transport": "sse",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

// ---------------------------------------------------------------------------
// CORS middleware
// ---------------------------------------------------------------------------

/// Inject CORS headers on all responses.
///
/// Allows all origins, enabling browser-based clients and cross-origin
/// enterprise integrations without a reverse proxy.
async fn add_cors_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "access-control-allow-origin",
        axum::http::HeaderValue::from_static("*"),
    );
    headers.insert(
        "access-control-allow-methods",
        axum::http::HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    headers.insert(
        "access-control-allow-headers",
        axum::http::HeaderValue::from_static("content-type, authorization"),
    );
    response
}

// ---------------------------------------------------------------------------
// Server entry point
// ---------------------------------------------------------------------------

/// Start the SSE MCP transport server and block until shutdown.
///
/// Binds an HTTP server to `config.bind_addr()` and serves:
///
/// - `GET  /sse`     — SSE event stream (MCP transport channel)
/// - `POST /message` — MCP JSON-RPC request intake
/// - `GET  /health`  — liveness probe (no auth)
///
/// The `engine` is shared (via `Arc<Mutex>`) across all active SSE sessions.
///
/// # Errors
///
/// Returns an error if the TCP listener cannot bind to the configured address,
/// or if the underlying axum server encounters a fatal error.
pub async fn serve(config: SseConfig, engine: Arc<Mutex<omni_core::Engine>>) -> anyhow::Result<()> {
    let server = OmniContextServer::new_shared(engine);

    let state = Arc::new(AppState {
        sessions: Mutex::new(HashMap::new()),
        config: config.clone(),
        server,
    });

    let app = Router::new()
        .route("/sse", get(sse_handler))
        .route("/message", post(message_handler))
        .route("/health", get(health_handler))
        .layer(axum::middleware::from_fn(add_cors_headers))
        .with_state(state);

    let bind_addr = config.bind_addr();
    tracing::info!(
        addr = %bind_addr,
        auth = config.token.is_some(),
        "SSE MCP transport listening"
    );

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| anyhow::anyhow!("failed to bind SSE listener to {bind_addr}: {e}"))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("SSE server error: {e}"))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt; // for `oneshot`

    // -----------------------------------------------------------------------
    // SseConfig unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sse_config_defaults() {
        let config = SseConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert!(config.token.is_none());
    }

    #[test]
    fn test_sse_config_bind_addr() {
        let config = SseConfig {
            host: "0.0.0.0".to_string(),
            port: 3179,
            token: None,
        };
        assert_eq!(config.bind_addr(), "0.0.0.0:3179");
    }

    #[test]
    fn test_sse_config_bind_addr_loopback() {
        let config = SseConfig::default();
        assert_eq!(config.bind_addr(), "127.0.0.1:8080");
    }

    #[test]
    fn test_sse_config_from_env_defaults() {
        std::env::remove_var("OMNI_SERVER_HOST");
        std::env::remove_var("OMNI_SERVER_PORT");
        std::env::remove_var("OMNI_SERVER_TOKEN");

        let config = SseConfig::from_env();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert!(config.token.is_none());
    }

    #[test]
    fn test_sse_config_from_env_override() {
        std::env::set_var("OMNI_SERVER_HOST", "0.0.0.0");
        std::env::set_var("OMNI_SERVER_PORT", "9000");
        std::env::set_var("OMNI_SERVER_TOKEN", "secret-abc");

        let config = SseConfig::from_env();

        std::env::remove_var("OMNI_SERVER_HOST");
        std::env::remove_var("OMNI_SERVER_PORT");
        std::env::remove_var("OMNI_SERVER_TOKEN");

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 9000);
        assert_eq!(config.token.as_deref(), Some("secret-abc"));
    }

    #[test]
    fn test_sse_config_from_env_invalid_port_falls_back_to_default() {
        std::env::set_var("OMNI_SERVER_PORT", "not-a-number");
        let config = SseConfig::from_env();
        std::env::remove_var("OMNI_SERVER_PORT");
        assert_eq!(config.port, 8080);
    }

    // -----------------------------------------------------------------------
    // Authentication helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_auth_passes_when_no_token_configured() {
        let config = SseConfig {
            token: None,
            ..Default::default()
        };
        let headers = HeaderMap::new();
        assert!(check_auth(&headers, &config).is_ok());
    }

    #[test]
    fn test_auth_rejects_missing_bearer() {
        let config = SseConfig {
            token: Some("supersecret".to_string()),
            ..Default::default()
        };
        let headers = HeaderMap::new();
        assert_eq!(check_auth(&headers, &config), Err(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn test_auth_rejects_wrong_token() {
        let config = SseConfig {
            token: Some("correct".to_string()),
            ..Default::default()
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            axum::http::HeaderValue::from_static("Bearer wrong"),
        );
        assert_eq!(check_auth(&headers, &config), Err(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn test_auth_accepts_correct_token() {
        let config = SseConfig {
            token: Some("mytoken123".to_string()),
            ..Default::default()
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            axum::http::HeaderValue::from_static("Bearer mytoken123"),
        );
        assert!(check_auth(&headers, &config).is_ok());
    }

    #[test]
    fn test_auth_rejects_non_bearer_scheme() {
        let config = SseConfig {
            token: Some("mytoken".to_string()),
            ..Default::default()
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            axum::http::HeaderValue::from_static("Basic mytoken"),
        );
        assert_eq!(check_auth(&headers, &config), Err(StatusCode::UNAUTHORIZED));
    }

    // -----------------------------------------------------------------------
    // JSON-RPC helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_jsonrpc_ok_serializes_correctly() {
        let resp = jsonrpc_ok(serde_json::json!(1), serde_json::json!({"key": "value"}));
        let parsed: serde_json::Value = serde_json::from_str(&resp).expect("valid json");
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["result"]["key"], "value");
        assert!(parsed.get("error").is_none());
    }

    #[test]
    fn test_jsonrpc_error_serializes_correctly() {
        let resp = jsonrpc_error(serde_json::json!(42), ERR_METHOD_NOT_FOUND, "not found");
        let parsed: serde_json::Value = serde_json::from_str(&resp).expect("valid json");
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 42);
        assert_eq!(parsed["error"]["code"], ERR_METHOD_NOT_FOUND);
        assert_eq!(parsed["error"]["message"], "not found");
        assert!(parsed.get("result").is_none());
    }

    // -----------------------------------------------------------------------
    // Test app factory
    // -----------------------------------------------------------------------

    /// Build a test axum app using a real (but empty) Engine from a tempdir.
    ///
    /// We test HTTP routing logic by targeting endpoints that don't need real
    /// index data: `/health`, `/sse` (connection setup), `/message` (session
    /// routing errors).
    fn make_test_app(token: Option<&'static str>) -> Router {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let engine = omni_core::Engine::new(tmpdir.path()).expect("engine");
        // Leak the tempdir handle so the path lives for the test duration.
        std::mem::forget(tmpdir);

        let engine = Arc::new(Mutex::new(engine));
        let server = OmniContextServer::new_shared(engine);

        let state = Arc::new(AppState {
            sessions: Mutex::new(HashMap::new()),
            config: SseConfig {
                token: token.map(str::to_string),
                ..Default::default()
            },
            server,
        });

        Router::new()
            .route("/sse", get(sse_handler))
            .route("/message", post(message_handler))
            .route("/health", get(health_handler))
            .layer(axum::middleware::from_fn(add_cors_headers))
            .with_state(state)
    }

    // -----------------------------------------------------------------------
    // /health endpoint tests
    // -----------------------------------------------------------------------

    #[tokio::test(flavor = "multi_thread")]
    async fn test_health_endpoint_returns_200() {
        let app = make_test_app(None);
        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(req).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_health_endpoint_has_cors_header() {
        let app = make_test_app(None);
        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(req).await.expect("response");
        assert_eq!(
            response.headers().get("access-control-allow-origin"),
            Some(&axum::http::HeaderValue::from_static("*"))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_health_endpoint_body_contains_status_ok() {
        use axum::body::to_bytes;

        let app = make_test_app(None);
        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(req).await.expect("response");
        let bytes = to_bytes(response.into_body(), 4096)
            .await
            .expect("body bytes");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(body["status"], "ok");
        assert_eq!(body["transport"], "sse");
    }

    // -----------------------------------------------------------------------
    // /sse endpoint tests
    // -----------------------------------------------------------------------

    #[tokio::test(flavor = "multi_thread")]
    async fn test_sse_endpoint_returns_200_no_auth() {
        let app = make_test_app(None);
        let req = Request::builder()
            .uri("/sse")
            .header("accept", "text/event-stream")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(req).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_sse_endpoint_content_type_is_event_stream() {
        let app = make_test_app(None);
        let req = Request::builder()
            .uri("/sse")
            .header("accept", "text/event-stream")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(req).await.expect("response");
        let ct = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(ct.contains("text/event-stream"), "content-type: {ct}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_sse_requires_auth_when_token_configured() {
        let app = make_test_app(Some("secure-token"));
        let req = Request::builder()
            .uri("/sse")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(req).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_sse_accepts_valid_token() {
        let app = make_test_app(Some("secure-token"));
        let req = Request::builder()
            .uri("/sse")
            .header("authorization", "Bearer secure-token")
            .header("accept", "text/event-stream")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(req).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    // -----------------------------------------------------------------------
    // /message endpoint tests
    // -----------------------------------------------------------------------

    #[tokio::test(flavor = "multi_thread")]
    async fn test_message_without_session_id_returns_bad_request() {
        let app = make_test_app(None);
        let req = Request::builder()
            .method("POST")
            .uri("/message")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#))
            .expect("request");
        let response = app.oneshot(req).await.expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_message_with_unknown_session_returns_not_found() {
        let app = make_test_app(None);
        let req = Request::builder()
            .method("POST")
            .uri("/message?session_id=00000000-0000-0000-0000-000000000000")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#))
            .expect("request");
        let response = app.oneshot(req).await.expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_message_requires_auth_when_token_configured() {
        let app = make_test_app(Some("tok"));
        let req = Request::builder()
            .method("POST")
            .uri("/message?session_id=some-id")
            .body(Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#))
            .expect("request");
        let response = app.oneshot(req).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cors_headers_on_message_endpoint() {
        let app = make_test_app(None);
        let req = Request::builder()
            .method("POST")
            .uri("/message")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(req).await.expect("response");
        assert_eq!(
            response.headers().get("access-control-allow-origin"),
            Some(&axum::http::HeaderValue::from_static("*"))
        );
    }

    // -----------------------------------------------------------------------
    // Dispatcher unit tests (no axum layer)
    // -----------------------------------------------------------------------

    fn make_server() -> OmniContextServer {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let engine = omni_core::Engine::new(tmpdir.path()).expect("engine");
        std::mem::forget(tmpdir);
        let engine = Arc::new(Mutex::new(engine));
        OmniContextServer::new_shared(engine)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dispatch_initialize() {
        let server = make_server();
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{}}}"#;
        let resp = dispatch(raw, &server).await;
        let parsed: serde_json::Value = serde_json::from_str(&resp).expect("json");

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert!(parsed["result"]["protocolVersion"].is_string());
        assert!(parsed["result"]["capabilities"].is_object());
        assert!(parsed["result"]["serverInfo"]["name"].is_string());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dispatch_ping() {
        let server = make_server();
        let raw = r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#;
        let resp = dispatch(raw, &server).await;
        let parsed: serde_json::Value = serde_json::from_str(&resp).expect("json");

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 2);
        assert_eq!(parsed["result"], serde_json::json!({}));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dispatch_tools_list() {
        let server = make_server();
        let raw = r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#;
        let resp = dispatch(raw, &server).await;
        let parsed: serde_json::Value = serde_json::from_str(&resp).expect("json");

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 3);
        let tools = parsed["result"]["tools"].as_array().expect("tools array");
        assert!(!tools.is_empty(), "tool list must not be empty");
        for tool in tools {
            assert!(tool["name"].is_string(), "tool has name");
            assert!(tool["description"].is_string(), "tool has description");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dispatch_notifications_initialized_returns_empty() {
        let server = make_server();
        let raw = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let resp = dispatch(raw, &server).await;
        assert!(resp.is_empty(), "notification must return empty string");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dispatch_unknown_method_returns_error() {
        let server = make_server();
        let raw = r#"{"jsonrpc":"2.0","id":99,"method":"nonexistent/method"}"#;
        let resp = dispatch(raw, &server).await;
        let parsed: serde_json::Value = serde_json::from_str(&resp).expect("json");

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 99);
        assert_eq!(parsed["error"]["code"], ERR_METHOD_NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dispatch_parse_error_on_invalid_json() {
        let server = make_server();
        let raw = "this is not json{{{{";
        let resp = dispatch(raw, &server).await;
        let parsed: serde_json::Value = serde_json::from_str(&resp).expect("json");
        assert_eq!(parsed["error"]["code"], ERR_PARSE_ERROR);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dispatch_tools_call_unknown_tool_returns_is_error_true() {
        let server = make_server();
        let raw = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"no_such_tool","arguments":{}}}"#;
        let resp = dispatch(raw, &server).await;
        let parsed: serde_json::Value = serde_json::from_str(&resp).expect("json");

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 5);
        assert_eq!(parsed["result"]["isError"], true);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dispatch_tools_call_search_code_empty_query_is_error() {
        let server = make_server();
        let raw = r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"search_code","arguments":{"query":""}}}"#;
        let resp = dispatch(raw, &server).await;
        let parsed: serde_json::Value = serde_json::from_str(&resp).expect("json");
        assert_eq!(parsed["result"]["isError"], true);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dispatch_tools_call_get_status_succeeds() {
        let server = make_server();
        let raw = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"get_status","arguments":{}}}"#;
        let resp = dispatch(raw, &server).await;
        let parsed: serde_json::Value = serde_json::from_str(&resp).expect("json");

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 7);
        // get_status returns isError: false
        assert_eq!(parsed["result"]["isError"], false);
        let content = parsed["result"]["content"]
            .as_array()
            .expect("content array");
        assert!(!content.is_empty());
    }
}
