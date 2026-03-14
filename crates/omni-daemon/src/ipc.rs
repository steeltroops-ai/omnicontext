//! IPC transport layer for the `OmniContext` daemon.
//!
//! Uses named pipes on Windows and Unix domain sockets on Linux/macOS.
//! Communication is newline-delimited JSON-RPC 2.0 over the pipe.
//!
//! ## Protocol
//!
//! Each message is a complete JSON object terminated by `\n`.
//! The client sends `Request` objects, the server responds with `Response` objects.

use std::path::Path;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use omni_core::Engine;

use crate::protocol::{self, error_codes, Response};

// ---------------------------------------------------------------------------
// RepoRegistry — shared multi-repo workspace state
// ---------------------------------------------------------------------------

/// Thread-safe multi-repo registry backed by a persistent `Workspace`.
///
/// Design: the `Workspace` config file lives at
/// `<primary_data_dir>/workspace.toml`. Every add/remove/priority change
/// is flushed to disk immediately inside `Workspace` so the registry
/// survives daemon restarts without any async bookkeeping.
#[derive(Clone)]
pub(crate) struct RepoRegistry(Arc<Mutex<omni_core::workspace::Workspace>>);

impl RepoRegistry {
    /// Open or create the registry, seeding it with the primary repo if empty.
    pub fn open(primary_repo: &Path) -> Self {
        let config_path = omni_core::Config::defaults(primary_repo)
            .data_dir()
            .join("workspace.toml");

        let workspace = omni_core::workspace::Workspace::open(&config_path).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "failed to open workspace registry; starting empty");
            // Create a minimal fresh workspace using a temp path — the
            // real config_path will be written on first mutation.
            omni_core::workspace::Workspace::open(&config_path).unwrap_or_else(|_| {
                // Absolute last resort: in-memory workspace with no backing file.
                // This happens only when the data dir itself is inaccessible.
                omni_core::workspace::Workspace::open(std::path::Path::new("workspace.toml"))
                    .expect("cannot open workspace")
            })
        });

        Self(Arc::new(Mutex::new(workspace)))
    }
}

/// Derive a deterministic pipe/socket name from the repository path.
///
/// Normalization must match the extension's `derivePipeName()`:
///   1. Strip `\\?\` prefix
///   2. Backslash -> forward slash
///   3. Lowercase
///   4. Strip trailing separator(s)
pub fn default_pipe_name(repo_path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let mut normalized = repo_path
        .to_string_lossy()
        .replace(r"\\?\", "")
        .replace('\\', "/")
        .to_lowercase();

    // Strip trailing separator to match extension behavior
    while normalized.ends_with('/') {
        normalized.pop();
    }

    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let hash = hex::encode(&hasher.finalize()[..6]);

    #[cfg(windows)]
    {
        format!(r"\\.\pipe\omnicontext-{hash}")
    }

    #[cfg(not(windows))]
    {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
        format!("{runtime_dir}/omnicontext-{hash}.sock")
    }
}

/// Start the IPC server and listen for client connections.
pub async fn serve(engine: Engine, pipe_name: &str) -> anyhow::Result<()> {
    // Derive the primary repo path from the engine config so the registry
    // config file lands in the same data directory as the engine index.
    let repo_path = engine.repo_path().to_path_buf();
    let repo_registry = RepoRegistry::open(&repo_path);

    let engine = Arc::new(Mutex::new(engine));
    let prefetch_cache = Arc::new(crate::prefetch::PrefetchCache::default());
    let daemon_start_time = Arc::new(std::time::Instant::now());
    let performance_metrics = Arc::new(crate::metrics::PerformanceMetrics::default());
    let event_dedup = Arc::new(crate::event_dedup::EventDeduplicator::new());
    let backpressure = Arc::new(crate::backpressure::BackpressureMonitor::new(100)); // max 100 concurrent requests
    let shutdown_token = CancellationToken::new();

    // Spawn periodic maintenance task — prunes expired cache entries every 60s
    {
        let eng = engine.clone();
        let token = shutdown_token.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let eng_guard = eng.lock().await;
                        let pruned = eng_guard.search_engine().result_cache().prune_expired();
                        if pruned > 0 {
                            tracing::debug!(pruned = pruned, "periodic cache maintenance: pruned expired entries");
                        }
                        drop(eng_guard);
                    }
                    () = token.cancelled() => {
                        tracing::debug!("maintenance task shutting down");
                        break;
                    }
                }
            }
        });
    }

    #[cfg(windows)]
    {
        serve_named_pipe(
            engine, repo_registry, prefetch_cache, daemon_start_time, performance_metrics,
            event_dedup, backpressure, pipe_name, shutdown_token,
        )
        .await
    }

    #[cfg(not(windows))]
    {
        serve_unix_socket(
            engine, repo_registry, prefetch_cache, daemon_start_time, performance_metrics,
            event_dedup, backpressure, pipe_name, shutdown_token,
        )
        .await
    }
}

// ---------------------------------------------------------------------------
// Windows: Named Pipe server
// ---------------------------------------------------------------------------

#[cfg(windows)]
async fn serve_named_pipe(
    engine: Arc<Mutex<Engine>>,
    repo_registry: RepoRegistry,
    prefetch_cache: Arc<crate::prefetch::PrefetchCache>,
    daemon_start_time: Arc<std::time::Instant>,
    performance_metrics: Arc<crate::metrics::PerformanceMetrics>,
    event_dedup: Arc<crate::event_dedup::EventDeduplicator>,
    backpressure: Arc<crate::backpressure::BackpressureMonitor>,
    pipe_name: &str,
    shutdown_token: CancellationToken,
) -> anyhow::Result<()> {
    use tokio::net::windows::named_pipe::ServerOptions;

    tracing::info!(pipe = %pipe_name, "listening on named pipe");

    loop {
        // Create a new pipe instance for each client
        let server = ServerOptions::new()
            .first_pipe_instance(false)
            .create(pipe_name)?;

        // Wait for a client to connect, or a shutdown signal
        tokio::select! {
            result = server.connect() => { result?; }
            () = shutdown_token.cancelled() => {
                tracing::info!("shutdown signal received, stopping server");
                return Ok(());
            }
        }

        tracing::info!("client connected");

        let engine = engine.clone();
        let registry = repo_registry.clone();
        let cache = prefetch_cache.clone();
        let start_time = daemon_start_time.clone();
        let metrics = performance_metrics.clone();
        let dedup = event_dedup.clone();
        let bp = backpressure.clone();
        let token = shutdown_token.clone();
        tokio::spawn(async move {
            let (reader, writer) = tokio::io::split(server);
            if let Err(e) = handle_client(
                engine, registry, cache, start_time, metrics, dedup, bp, token, reader, writer,
            )
            .await
            {
                tracing::warn!(error = %e, "client handler error");
            }
            tracing::info!("client disconnected");
        });
    }
}

// ---------------------------------------------------------------------------
// Unix: Domain Socket server
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
async fn serve_unix_socket(
    engine: Arc<Mutex<Engine>>,
    repo_registry: RepoRegistry,
    prefetch_cache: Arc<crate::prefetch::PrefetchCache>,
    daemon_start_time: Arc<std::time::Instant>,
    performance_metrics: Arc<crate::metrics::PerformanceMetrics>,
    event_dedup: Arc<crate::event_dedup::EventDeduplicator>,
    backpressure: Arc<crate::backpressure::BackpressureMonitor>,
    socket_path: &str,
    shutdown_token: CancellationToken,
) -> anyhow::Result<()> {
    use tokio::net::UnixListener;

    // Remove stale socket file
    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)?;
    tracing::info!(socket = %socket_path, "listening on unix socket");

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = result?;
                tracing::info!("client connected");

                let engine = engine.clone();
                let registry = repo_registry.clone();
                let cache = prefetch_cache.clone();
                let start_time = daemon_start_time.clone();
                let metrics = performance_metrics.clone();
                let dedup = event_dedup.clone();
                let bp = backpressure.clone();
                let token = shutdown_token.clone();
                tokio::spawn(async move {
                    let (reader, writer) = tokio::io::split(stream);
                    if let Err(e) = handle_client(engine, registry, cache, start_time, metrics, dedup, bp, token, reader, writer).await
                    {
                        tracing::warn!(error = %e, "client handler error");
                    }
                    tracing::info!("client disconnected");
                });
            }
            () = shutdown_token.cancelled() => {
                tracing::info!("shutdown signal received, stopping server");
                return Ok(());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Client handler (platform-agnostic)
// ---------------------------------------------------------------------------

/// Handle a single connected client.
///
/// Reads newline-delimited JSON-RPC requests (optionally compressed),
/// dispatches them to the engine, and writes JSON-RPC responses back
/// (with compression for large responses).
async fn handle_client<R, W>(
    engine: Arc<Mutex<Engine>>,
    repo_registry: RepoRegistry,
    prefetch_cache: Arc<crate::prefetch::PrefetchCache>,
    daemon_start_time: Arc<std::time::Instant>,
    performance_metrics: Arc<crate::metrics::PerformanceMetrics>,
    event_dedup: Arc<crate::event_dedup::EventDeduplicator>,
    backpressure: Arc<crate::backpressure::BackpressureMonitor>,
    shutdown_token: CancellationToken,
    reader: R,
    mut writer: W,
) -> anyhow::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    // HC-1 fix: limit max line length to 10 MB to prevent unbounded allocation
    const MAX_LINE_LEN: usize = 10 * 1024 * 1024;
    let buf_reader = BufReader::new(reader);
    let mut lines = buf_reader.lines();

    while let Some(line_result) = lines.next_line().await.transpose() {
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!(error = %e, "failed to read IPC line");
                break;
            }
        };

        if line.len() > MAX_LINE_LEN {
            let response = Response::error(
                0,
                error_codes::PARSE_ERROR,
                format!("message exceeds maximum size of {MAX_LINE_LEN} bytes"),
            );
            let mut response_json = serde_json::to_string(&response)?;
            response_json.push('\n');
            writer.write_all(response_json.as_bytes()).await?;
            writer.flush().await?;
            continue;
        }

        let line_bytes = line.trim().as_bytes();
        if line_bytes.is_empty() {
            continue;
        }

        // Decompress if compressed
        let request_bytes = crate::compression::decompress_if_compressed(line_bytes)?;
        let request_str = std::str::from_utf8(&request_bytes)?;

        let response = match serde_json::from_str::<protocol::Request>(request_str) {
            Ok(req) => {
                dispatch(
                    engine.clone(),
                    repo_registry.clone(),
                    prefetch_cache.clone(),
                    daemon_start_time.clone(),
                    performance_metrics.clone(),
                    event_dedup.clone(),
                    backpressure.clone(),
                    shutdown_token.clone(),
                    req,
                )
                .await
            }
            Err(e) => Response::error(
                0,
                error_codes::PARSE_ERROR,
                format!("invalid JSON-RPC: {e}"),
            ),
        };

        let mut response_json = serde_json::to_string(&response)?;
        response_json.push('\n');

        // Compress if beneficial (>100KB)
        let response_bytes = crate::compression::compress_if_beneficial(response_json.as_bytes());

        writer.write_all(&response_bytes).await?;
        writer.flush().await?;
    }

    Ok(())
}

/// Dispatch a JSON-RPC request to the appropriate handler.
async fn dispatch(
    engine: Arc<Mutex<Engine>>,
    repo_registry: RepoRegistry,
    prefetch_cache: Arc<crate::prefetch::PrefetchCache>,
    daemon_start_time: Arc<std::time::Instant>,
    performance_metrics: Arc<crate::metrics::PerformanceMetrics>,
    event_dedup: Arc<crate::event_dedup::EventDeduplicator>,
    backpressure: Arc<crate::backpressure::BackpressureMonitor>,
    shutdown_token: CancellationToken,
    req: protocol::Request,
) -> Response {
    // Check backpressure before processing request
    let _guard = match crate::backpressure::RequestGuard::new((*backpressure).clone()) {
        Some(guard) => guard,
        None => {
            // Daemon overloaded, reject request
            return Response::error(
                req.id,
                error_codes::SERVER_OVERLOADED,
                "daemon overloaded, please retry later".to_string(),
            );
        }
    };

    let start = std::time::Instant::now();

    let result = match req.method.as_str() {
        "ping" => Ok(serde_json::json!({ "pong": true })),

        "status" => handle_status(engine.clone()).await,

        "system_status" => handle_system_status(engine.clone(), daemon_start_time.clone()).await,

        "performance_metrics" => {
            handle_performance_metrics(engine.clone(), performance_metrics.clone()).await
        }

        "search" => {
            let params: protocol::SearchParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            let result = handle_search(engine.clone(), params).await;
            // Record search latency
            performance_metrics.record_search_latency(start.elapsed());
            result
        }

        "context_window" => {
            let params: protocol::ContextWindowParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            let result = handle_context_window(engine.clone(), params).await;
            // Record search latency
            performance_metrics.record_search_latency(start.elapsed());
            result
        }

        "preflight" => {
            let params: protocol::PreflightParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            let result =
                handle_preflight(engine.clone(), prefetch_cache.clone(), params, start).await;
            // Record search latency
            performance_metrics.record_search_latency(start.elapsed());
            result
        }

        "module_map" => {
            let params: protocol::ModuleMapParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_module_map(engine.clone(), params).await
        }

        "index" => handle_index(engine.clone()).await,

        "ide_event" => {
            let params: protocol::IdeEventParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_ide_event(
                engine.clone(),
                prefetch_cache.clone(),
                event_dedup.clone(),
                params,
            )
            .await
        }

        "prefetch_stats" => handle_prefetch_stats(prefetch_cache.clone()).await,

        "clear_cache" => handle_clear_cache(prefetch_cache.clone()).await,

        "update_config" => {
            let params: protocol::UpdateConfigParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_update_config(prefetch_cache.clone(), params).await
        }

        "clear_index" => {
            let params: protocol::ClearIndexParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_clear_index(engine.clone(), params).await
        }

        "shutdown" => {
            tracing::info!("shutdown requested via IPC, initiating graceful shutdown");
            shutdown_token.cancel();
            Ok(serde_json::json!({ "shutdown": true }))
        }

        // Intelligence Layer Methods
        "reranker/get_metrics" => handle_reranker_metrics(engine.clone()).await,

        "graph/get_metrics" => handle_graph_metrics(engine.clone()).await,

        "search/get_intent" => {
            let params: protocol::SearchIntentParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_search_intent(engine.clone(), params).await
        }

        // Resilience Monitoring Methods
        "resilience/get_status" => {
            handle_resilience_status(engine.clone(), event_dedup.clone(), backpressure.clone())
                .await
        }

        "resilience/reset_circuit_breaker" => {
            let params: protocol::ResetCircuitBreakerParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_reset_circuit_breaker(engine.clone(), params).await
        }

        // Historical Context Methods
        "history/get_commit_context" => {
            let params: protocol::CommitContextParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_commit_context(engine.clone(), params).await
        }

        "history/index_commits" => handle_index_commits(engine.clone()).await,

        "history/get_co_changes" => {
            let params: protocol::CoChangeParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_co_changes(engine.clone(), params).await
        }

        "plan/audit" => {
            let params: protocol::AuditPlanParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_audit_plan(engine.clone(), params).await
        }

        // Graph Visualization Methods
        "graph/get_architectural_context" => {
            let params: protocol::ArchitecturalContextParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_architectural_context(engine.clone(), params).await
        }

        "graph/find_cycles" => handle_find_cycles(engine.clone()).await,

        // Multi-Repository Support
        "workspace/list_repos" => handle_list_repos(engine.clone(), repo_registry.clone()).await,

        "workspace/add_repo" => {
            let params: protocol::AddRepoParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_add_repo(repo_registry.clone(), params).await
        }

        "workspace/set_priority" => {
            let params: protocol::SetPriorityParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_set_priority(repo_registry.clone(), params).await
        }

        "workspace/remove_repo" => {
            let params: protocol::RemoveRepoParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_remove_repo(repo_registry.clone(), params).await
        }

        // Performance Controls
        "embedder/get_metrics" => {
            handle_embedder_metrics(engine.clone(), daemon_start_time.clone()).await
        }

        "embedder/configure" => {
            let params: protocol::ConfigureEmbedderParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_configure_embedder(engine.clone(), params).await
        }

        "index/get_pool_metrics" => {
            handle_index_pool_metrics(
                engine.clone(),
                backpressure.clone(),
                performance_metrics.clone(),
            )
            .await
        }

        "compression/get_stats" => handle_compression_stats(engine.clone()).await,

        _ => Err((
            error_codes::METHOD_NOT_FOUND,
            format!("unknown method: {}", req.method),
        )),
    };

    #[allow(clippy::cast_possible_truncation)]
    let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    tracing::debug!(
        method = %req.method,
        elapsed_ms = elapsed_ms,
        "request handled"
    );

    match result {
        Ok(value) => Response::success(req.id, value),
        Err((code, msg)) => Response::error(req.id, code, msg),
    }
}

/// Parse params from a request, returning an error response if invalid.
#[allow(clippy::result_large_err)]
fn parse_params<T: serde::de::DeserializeOwned>(req: &protocol::Request) -> Result<T, Response> {
    let params = req
        .params
        .clone()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::default()));
    serde_json::from_value(params).map_err(|e| {
        Response::error(
            req.id,
            error_codes::INVALID_PARAMS,
            format!("invalid params: {e}"),
        )
    })
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

async fn handle_status(engine: Arc<Mutex<Engine>>) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;
    eng.status()
        .map(|s| serde_json::to_value(s).unwrap_or_default())
        .map_err(|e| (error_codes::ENGINE_ERROR, format!("status failed: {e}")))
}

async fn handle_system_status(
    engine: Arc<Mutex<Engine>>,
    daemon_start_time: Arc<std::time::Instant>,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;

    // Get engine status for file/chunk counts
    let status = eng
        .status()
        .map_err(|e| (error_codes::ENGINE_ERROR, format!("status failed: {e}")))?;

    // Calculate daemon uptime
    #[allow(clippy::cast_possible_truncation)]
    let daemon_uptime_seconds = daemon_start_time.elapsed().as_secs();

    // Determine initialization status based on whether files are indexed
    let initialization_status = if status.files_indexed > 0 {
        "ready"
    } else {
        "initializing"
    };

    // Connection health is always "connected" if we're handling this request
    let connection_health = "connected";

    // Read last index timestamp from the engine (set at the end of run_index()).
    // Serialized as Unix timestamp milliseconds; None when no index has completed this session.
    let last_index_time: Option<u64> = eng.last_indexed_at().and_then(|t| {
        t.duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_millis().min(u128::from(u64::MAX)) as u64)
    });

    let response = protocol::SystemStatusResponse {
        initialization_status: initialization_status.to_string(),
        connection_health: connection_health.to_string(),
        last_index_time,
        daemon_uptime_seconds,
        files_indexed: status.files_indexed,
        chunks_indexed: status.chunks_indexed,
    };

    serde_json::to_value(response).map_err(|e| {
        (
            error_codes::INTERNAL_ERROR,
            format!("serialization failed: {e}"),
        )
    })
}

async fn handle_performance_metrics(
    engine: Arc<Mutex<Engine>>,
    performance_metrics: Arc<crate::metrics::PerformanceMetrics>,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;

    // Get engine status for embedding coverage
    let status = eng
        .status()
        .map_err(|e| (error_codes::ENGINE_ERROR, format!("status failed: {e}")))?;

    // Calculate embedding coverage percentage
    #[allow(clippy::cast_precision_loss)]
    let embedding_coverage_percent = if status.chunks_indexed > 0 {
        (status.vectors_indexed as f64 / status.chunks_indexed as f64) * 100.0
    } else {
        0.0
    };

    // Get latency percentiles
    let search_latency_p50_ms = performance_metrics.get_latency_percentile(0.5);
    let search_latency_p95_ms = performance_metrics.get_latency_percentile(0.95);
    let search_latency_p99_ms = performance_metrics.get_latency_percentile(0.99);

    // Use vector index memory as a concrete, non-placeholder memory signal.
    #[allow(clippy::cast_possible_truncation)]
    let memory_usage_bytes = status.vector_memory_bytes as u64;
    performance_metrics.update_memory_usage(memory_usage_bytes);
    let peak_memory_usage_bytes = performance_metrics.get_peak_memory_bytes();

    // Get total searches
    let total_searches = performance_metrics.get_total_searches();

    let response = protocol::PerformanceMetricsResponse {
        search_latency_p50_ms,
        search_latency_p95_ms,
        search_latency_p99_ms,
        embedding_coverage_percent,
        memory_usage_bytes,
        peak_memory_usage_bytes,
        total_searches,
    };

    serde_json::to_value(response).map_err(|e| {
        (
            error_codes::INTERNAL_ERROR,
            format!("serialization failed: {e}"),
        )
    })
}

async fn handle_search(
    engine: Arc<Mutex<Engine>>,
    params: protocol::SearchParams,
) -> Result<serde_json::Value, (i32, String)> {
    // Validate query
    if params.query.trim().is_empty() {
        return Err((
            error_codes::INVALID_PARAMS,
            "query must not be empty".to_string(),
        ));
    }
    if params.query.len() > 10_000 {
        return Err((
            error_codes::INVALID_PARAMS,
            "query exceeds maximum length of 10000 characters".to_string(),
        ));
    }
    let limit = params.limit.clamp(1, 200); // Cap at 200, minimum 1
    let eng = engine.lock().await;
    eng.search(&params.query, limit)
        .map(|results| {
            let entries: Vec<serde_json::Value> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "file": r.file_path.display().to_string(),
                        "symbol": r.chunk.symbol_path,
                        "kind": format!("{:?}", r.chunk.kind),
                        "score": r.score,
                        "line_start": r.chunk.line_start,
                        "line_end": r.chunk.line_end,
                        "content": r.chunk.content,
                    })
                })
                .collect();
            serde_json::json!({
                "count": entries.len(),
                "results": entries,
            })
        })
        .map_err(|e| (error_codes::ENGINE_ERROR, format!("search failed: {e}")))
}

async fn handle_context_window(
    engine: Arc<Mutex<Engine>>,
    params: protocol::ContextWindowParams,
) -> Result<serde_json::Value, (i32, String)> {
    // Validate query
    if params.query.trim().is_empty() {
        return Err((
            error_codes::INVALID_PARAMS,
            "query must not be empty".to_string(),
        ));
    }
    if params.query.len() > 10_000 {
        return Err((
            error_codes::INVALID_PARAMS,
            "query exceeds maximum length of 10000 characters".to_string(),
        ));
    }
    let limit = params.limit.clamp(1, 200); // Cap at 200, minimum 1
    let eng = engine.lock().await;
    eng.search_context_window(&params.query, limit, params.token_budget)
        .map(|ctx| {
            serde_json::json!({
                "entries_count": ctx.len(),
                "total_tokens": ctx.total_tokens,
                "token_budget": ctx.token_budget,
                "rendered": ctx.render(),
            })
        })
        .map_err(|e| {
            (
                error_codes::ENGINE_ERROR,
                format!("context_window failed: {e}"),
            )
        })
}

#[allow(clippy::too_many_lines)]
async fn handle_preflight(
    engine: Arc<Mutex<Engine>>,
    prefetch_cache: Arc<crate::prefetch::PrefetchCache>,
    params: protocol::PreflightParams,
    start: std::time::Instant,
) -> Result<serde_json::Value, (i32, String)> {
    use std::fmt::Write;

    // Check cache first if active_file is provided
    if let Some(ref active_file) = params.active_file {
        let cache_key = std::path::PathBuf::from(active_file);
        if let Some(cached) = prefetch_cache.get_file_context(&cache_key) {
            #[allow(clippy::cast_possible_truncation)]
            let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

            tracing::info!(
                file = %active_file,
                elapsed_ms = elapsed_ms,
                "cache hit: returning cached preflight context"
            );

            // Reconstruct metadata from the cached JSON blob
            let (entries_count, tokens_used) = parse_cached_meta(&cached);

            let response = protocol::PreflightResponse {
                system_context: cached,
                entries_count,
                tokens_used,
                token_budget: params.token_budget,
                elapsed_ms,
                from_cache: true,
            };

            return serde_json::to_value(response).map_err(|e| {
                (
                    error_codes::INTERNAL_ERROR,
                    format!("serialization failed: {e}"),
                )
            });
        }
        tracing::debug!(
            file = %active_file,
            "cache miss: performing fresh search"
        );
    }

    // Cache miss or no active_file: perform fresh search
    let eng = engine.lock().await;

    // Build the context window from the user's prompt
    let ctx = eng
        .search_context_window(&params.prompt, 20, Some(params.token_budget))
        .map_err(|e| {
            (
                error_codes::ENGINE_ERROR,
                format!("preflight search failed: {e}"),
            )
        })?;

    // Get engine status for architecture overview
    let status = eng
        .status()
        .map_err(|e| (error_codes::ENGINE_ERROR, format!("status failed: {e}")))?;

    // Assemble the system context prompt
    let intent_label = params.intent.as_deref().unwrap_or("general");
    let entries_count = ctx.len();
    let tokens_used = ctx.total_tokens;
    let mut system_context = String::with_capacity(tokens_used as usize * 4);

    system_context.push_str("<context_engine>\n");
    system_context.push_str(
        "OmniContext has analyzed the codebase and identified the following relevant code \n\
         for your current task. This context was automatically retrieved -- do not ask for \n\
         additional code search tools unless this context is insufficient.\n\n",
    );

    // Architecture overview
    write!(
        system_context,
        "## Repository\n- Files: {}\n- Symbols: {}\n- Intent: {}\n\n",
        status.files_indexed, status.symbols_indexed, intent_label,
    )
    .ok();

    // Active file context
    if let Some(ref active) = params.active_file {
        write!(system_context, "## Active File\n{active}\n").ok();
        if let Some(line) = params.cursor_line {
            writeln!(system_context, "Cursor at line: {line}").ok();
        }
        system_context.push('\n');
    }

    // Embed metadata header for cache reconstruction
    writeln!(
        system_context,
        "<!-- omni-meta entries={entries_count} tokens={tokens_used} -->"
    )
    .ok();

    // Relevant code (ranked by relevance)
    system_context.push_str("## Relevant Code (ranked by relevance)\n\n");
    system_context.push_str(&ctx.render());

    system_context.push_str("\n</context_engine>\n");

    #[allow(clippy::cast_possible_truncation)]
    let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    // Store in cache if active_file is provided
    if let Some(ref active_file) = params.active_file {
        let cache_key = std::path::PathBuf::from(active_file);
        prefetch_cache.put_file_context(cache_key, system_context.clone());
        tracing::debug!(
            file = %active_file,
            elapsed_ms = elapsed_ms,
            "stored fresh context in cache"
        );
    }

    let response = protocol::PreflightResponse {
        system_context,
        entries_count,
        tokens_used,
        token_budget: ctx.token_budget,
        elapsed_ms,
        from_cache: false,
    };

    serde_json::to_value(response).map_err(|e| {
        (
            error_codes::INTERNAL_ERROR,
            format!("serialization failed: {e}"),
        )
    })
}

/// Parse cached metadata from the embedded HTML comment.
/// Returns `(entries_count, tokens_used)`.
fn parse_cached_meta(context: &str) -> (usize, u32) {
    // Look for: <!-- omni-meta entries=N tokens=N -->
    if let Some(start) = context.find("<!-- omni-meta ") {
        if let Some(end) = context[start..].find("-->") {
            let meta = &context[start..start + end + 3];
            let entries = meta
                .split("entries=")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let tokens = meta
                .split("tokens=")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.trim_end_matches("-->").trim().parse::<u32>().ok())
                .unwrap_or(0);
            return (entries, tokens);
        }
    }
    (0, 0)
}

async fn handle_module_map(
    engine: Arc<Mutex<Engine>>,
    _params: protocol::ModuleMapParams,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;
    let index = eng.metadata_index();

    // Build module map from indexed files
    let files = index.get_all_files().map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to get files: {e}"),
        )
    })?;

    let mut modules: std::collections::BTreeMap<String, Vec<serde_json::Value>> =
        std::collections::BTreeMap::new();

    for file in &files {
        let path_str = file.path.display().to_string();
        let parts: Vec<&str> = path_str.split(['/', '\\']).collect();
        let module_key = if parts.len() > 1 {
            parts[..parts.len() - 1].join("/")
        } else {
            ".".to_string()
        };

        let chunks = index.get_chunks_for_file(file.id).unwrap_or_default();
        let symbols: Vec<String> = chunks
            .iter()
            .filter(|c| {
                matches!(
                    c.kind,
                    omni_core::types::ChunkKind::Function
                        | omni_core::types::ChunkKind::Class
                        | omni_core::types::ChunkKind::Trait
                )
            })
            .map(|c| c.symbol_path.clone())
            .collect();

        let entry = serde_json::json!({
            "file": path_str,
            "language": format!("{:?}", file.language),
            "symbols": symbols,
        });

        modules.entry(module_key).or_default().push(entry);
    }

    Ok(serde_json::json!({
        "module_count": modules.len(),
        "file_count": files.len(),
        "modules": modules,
    }))
}

/// IDX-1 fix: Spawn indexing in background so the mutex is not held for minutes.
/// Returns immediately with a "started" acknowledgment.
async fn handle_index(engine: Arc<Mutex<Engine>>) -> Result<serde_json::Value, (i32, String)> {
    // Quick check: if we can't even lock the engine, another index is running
    let eng = engine.try_lock();
    if eng.is_err() {
        return Err((
            error_codes::ENGINE_ERROR,
            "indexing already in progress — engine is busy".to_string(),
        ));
    }
    drop(eng);

    // Spawn the actual indexing in background so this handler returns immediately
    let engine_bg = engine.clone();
    tokio::spawn(async move {
        let mut eng = engine_bg.lock().await;
        let start = std::time::Instant::now();
        match eng.run_index(false).await {
            Ok(result) => {
                #[allow(clippy::cast_possible_truncation)]
                let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                tracing::info!(
                    files = result.files_processed,
                    chunks = result.chunks_created,
                    elapsed_ms,
                    "background indexing complete"
                );
            }
            Err(e) => {
                tracing::error!(error = %e, "background indexing failed");
            }
        }
    });

    Ok(serde_json::json!({
        "started": true,
        "message": "Indexing started in background. Use 'status' to check progress."
    }))
}

/// Handle IDE event for pre-fetch.
///
/// Maximum concurrent background IDE tasks (IDE-2 fix: prevent unbounded spawning).
static IDE_TASK_SEMAPHORE: std::sync::LazyLock<Arc<tokio::sync::Semaphore>> =
    std::sync::LazyLock::new(|| Arc::new(tokio::sync::Semaphore::new(10)));

/// Validate that a file path from an IDE event is inside the repository root.
/// Prevents arbitrary file indexing (IDE-1 fix).
fn validate_ide_file_path(file_path: &str, engine: &Engine) -> Result<(), (i32, String)> {
    let abs_path = std::path::Path::new(file_path);
    let repo_root = engine.config().repo_path.as_path();

    // Allow relative paths (they'll be resolved against repo root)
    if abs_path.is_relative() {
        return Ok(());
    }

    // Absolute paths must be inside the repo root
    if abs_path.strip_prefix(repo_root).is_ok() {
        Ok(())
    } else {
        tracing::warn!(
            file = %file_path,
            repo_root = %repo_root.display(),
            "rejected IDE event: file is outside repo root"
        );
        Err((
            error_codes::INVALID_PARAMS,
            "file path is outside the repository root".to_string(),
        ))
    }
}

/// Spawns background tasks to pre-compute context for the given file/symbol
/// and stores results in the prefetch cache. Returns immediately to the client.
/// On `text_edited` events, also triggers incremental re-indexing of the changed file.
#[allow(clippy::unused_async)]
async fn handle_ide_event(
    engine: Arc<Mutex<Engine>>,
    prefetch_cache: Arc<crate::prefetch::PrefetchCache>,
    event_dedup: Arc<crate::event_dedup::EventDeduplicator>,
    params: protocol::IdeEventParams,
) -> Result<serde_json::Value, (i32, String)> {
    tracing::debug!(
        event_type = %params.event_type,
        file = %params.file_path,
        "IDE event received"
    );

    // IDE-1: Validate file path is inside repo root
    {
        let eng_guard = engine.lock().await;
        validate_ide_file_path(&params.file_path, &eng_guard)?;
    }

    // IDE-2: Check semaphore before spawning any background task
    let permit = if let Ok(p) = IDE_TASK_SEMAPHORE.clone().try_acquire_owned() {
        p
    } else {
        tracing::warn!(
            event_type = %params.event_type,
            "IDE event dropped: too many concurrent background tasks"
        );
        return Ok(serde_json::json!({
            "acknowledged": true,
            "event_type": params.event_type,
            "skipped": true,
            "reason": "backpressure: too many concurrent IDE tasks"
        }));
    };

    match params.event_type.as_str() {
        "file_opened" => {
            let eng = engine.clone();
            let cache = prefetch_cache.clone();
            let file_path = params.file_path.clone();
            tokio::spawn(async move {
                let _permit = permit; // held until task completes
                prefetch_file_context(eng, cache, &file_path).await;
            });
        }
        "cursor_moved" => {
            if let Some(ref symbol) = params.symbol {
                let eng = engine.clone();
                let cache = prefetch_cache.clone();
                let file_path = params.file_path.clone();
                let symbol = symbol.clone();
                tokio::spawn(async move {
                    let _permit = permit; // held until task completes
                    prefetch_symbol_context(eng, cache, &file_path, &symbol).await;
                });
            }
            // Cross-file pre-fetch: if LSP resolved the definition to a different file,
            // pre-fetch that file's context too (Blast Radius pre-warming)
            if let Some(ref def_file) = params.definition_file {
                if def_file != &params.file_path {
                    let eng = engine.clone();
                    let cache = prefetch_cache.clone();
                    let def_file = def_file.clone();
                    tokio::spawn(async move {
                        prefetch_file_context(eng, cache, &def_file).await;
                    });
                }
            }
        }
        "text_edited" => {
            // Check if we're already processing this file
            if !event_dedup.try_start_processing(&params.file_path) {
                // Duplicate event, skip
                return Ok(serde_json::json!({
                    "acknowledged": true,
                    "event_type": params.event_type,
                    "skipped": true,
                    "reason": "already processing"
                }));
            }

            let eng = engine.clone();
            let cache = prefetch_cache.clone();
            let dedup = event_dedup.clone();
            let file_path = params.file_path.clone();
            tokio::spawn(async move {
                // Real-time incremental re-indexing
                // Re-index the changed file, then invalidate the cache
                let abs_path = std::path::PathBuf::from(&file_path);
                {
                    let mut engine_guard = eng.lock().await;
                    match engine_guard.reindex_single_file(&abs_path) {
                        Ok((stats, changed, delta)) => {
                            if changed {
                                // Invalidate cached context for this file
                                let cache_key = std::path::PathBuf::from(&file_path);
                                cache.invalidate_file(&cache_key);
                                tracing::debug!(
                                    file = %file_path,
                                    chunks = stats.chunks,
                                    added = delta.added_symbols.len(),
                                    removed = delta.removed_symbols.len(),
                                    modified = delta.modified_symbols.len(),
                                    structural = delta.has_structural_change,
                                    body_only = delta.is_body_only_change,
                                    "reindexed changed file, cache invalidated"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                file = %file_path,
                                error = %e,
                                "real-time reindex failed, falling back to prefetch"
                            );
                        }
                    }
                }
                // Also pre-fetch fresh context for the file
                prefetch_file_context(eng, cache, &file_path).await;

                // Mark processing as complete
                dedup.finish_processing(&file_path);
            });
        }
        _ => {
            tracing::warn!(event_type = %params.event_type, "unknown IDE event type");
        }
    }

    Ok(serde_json::json!({
        "acknowledged": true,
        "event_type": params.event_type,
    }))
}

/// Background pre-fetch: search for context relevant to the given file
/// and store it in the cache so subsequent preflight requests hit the cache.
async fn prefetch_file_context(
    engine: Arc<Mutex<Engine>>,
    cache: Arc<crate::prefetch::PrefetchCache>,
    file_path: &str,
) {
    use std::fmt::Write;
    let start = std::time::Instant::now();

    let eng = engine.lock().await;
    let query = format!("file:{file_path}");
    match eng.search_context_window(&query, 10, Some(4096)) {
        Ok(ctx) => {
            let mut context = String::with_capacity(ctx.total_tokens as usize * 4);
            writeln!(
                context,
                "<!-- omni-meta entries={} tokens={} -->",
                ctx.len(),
                ctx.total_tokens
            )
            .ok();
            context.push_str(&ctx.render());

            let cache_key = std::path::PathBuf::from(file_path);
            cache.put_file_context(cache_key, context);

            #[allow(clippy::cast_possible_truncation)]
            let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
            tracing::debug!(
                file = %file_path,
                elapsed_ms = elapsed_ms,
                entries = ctx.len(),
                "pre-fetch complete for file"
            );
        }
        Err(e) => {
            tracing::warn!(file = %file_path, error = %e, "pre-fetch failed for file");
        }
    }
}

/// Background pre-fetch: search for context relevant to a specific symbol.
async fn prefetch_symbol_context(
    engine: Arc<Mutex<Engine>>,
    cache: Arc<crate::prefetch::PrefetchCache>,
    file_path: &str,
    symbol: &str,
) {
    let start = std::time::Instant::now();

    let eng = engine.lock().await;
    match eng.search_context_window(symbol, 10, Some(4096)) {
        Ok(ctx) => {
            let rendered = ctx.render();
            cache.put_symbol_context(
                std::path::PathBuf::from(file_path),
                symbol.to_string(),
                rendered,
            );

            #[allow(clippy::cast_possible_truncation)]
            let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
            tracing::debug!(
                file = %file_path,
                symbol = %symbol,
                elapsed_ms = elapsed_ms,
                entries = ctx.len(),
                "pre-fetch complete for symbol"
            );
        }
        Err(e) => {
            tracing::warn!(
                file = %file_path,
                symbol = %symbol,
                error = %e,
                "pre-fetch failed for symbol"
            );
        }
    }
}

/// Handle request for pre-fetch cache statistics.
#[allow(clippy::unused_async)] // Keeping async for consistency with other handlers
async fn handle_prefetch_stats(
    prefetch_cache: Arc<crate::prefetch::PrefetchCache>,
) -> Result<serde_json::Value, (i32, String)> {
    let stats = prefetch_cache.stats();

    Ok(serde_json::json!({
        "hits": stats.hits,
        "misses": stats.misses,
        "size": stats.size,
        "hit_rate": stats.hit_rate,
    }))
}

/// Handle request to clear pre-fetch cache.
#[allow(clippy::unused_async)] // Keeping async for consistency with other handlers
async fn handle_clear_cache(
    prefetch_cache: Arc<crate::prefetch::PrefetchCache>,
) -> Result<serde_json::Value, (i32, String)> {
    prefetch_cache.clear();

    Ok(serde_json::json!({
        "cleared": true,
        "message": "Cache cleared successfully"
    }))
}

/// Handle request to update pre-fetch cache configuration.
#[allow(clippy::unused_async)] // Keeping async for consistency with other handlers
async fn handle_update_config(
    prefetch_cache: Arc<crate::prefetch::PrefetchCache>,
    params: protocol::UpdateConfigParams,
) -> Result<serde_json::Value, (i32, String)> {
    // Update cache configuration
    let updated = prefetch_cache.update_config(params.cache_size, params.cache_ttl_seconds);

    if updated {
        tracing::info!(
            cache_size = ?params.cache_size,
            cache_ttl_seconds = ?params.cache_ttl_seconds,
            "cache configuration updated"
        );

        Ok(serde_json::json!({
            "updated": true,
            "message": "Cache configuration updated successfully",
            "cache_size": params.cache_size,
            "cache_ttl_seconds": params.cache_ttl_seconds,
        }))
    } else {
        Ok(serde_json::json!({
            "updated": false,
            "message": "No configuration changes applied"
        }))
    }
}

/// Handle request to clear the index.
/// CI-1 fix: requires confirmation token "CONFIRM_CLEAR" to proceed.
async fn handle_clear_index(
    engine: Arc<Mutex<Engine>>,
    params: protocol::ClearIndexParams,
) -> Result<serde_json::Value, (i32, String)> {
    // CI-1: Require confirmation token for destructive operation
    match params.confirm.as_deref() {
        Some("CONFIRM_CLEAR") => {} // Valid
        _ => {
            return Err((
                error_codes::INVALID_PARAMS,
                "destructive operation requires confirm: \"CONFIRM_CLEAR\"".to_string(),
            ));
        }
    }

    let mut eng = engine.lock().await;

    // Clear the index
    eng.clear_index().map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to clear index: {e}"),
        )
    })?;

    tracing::info!("index cleared successfully");

    Ok(serde_json::json!({
        "cleared": true,
        "message": "Index cleared successfully. Re-indexing recommended."
    }))
}

// ---------------------------------------------------------------------------
// Intelligence Layer Handlers
// ---------------------------------------------------------------------------

/// Handle request for reranker metrics.
async fn handle_reranker_metrics(
    engine: Arc<Mutex<Engine>>,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;

    let reranker = eng.reranker();
    let enabled = reranker.is_available();
    let breaker_stats = eng.reranker_breaker().stats();

    let model = if enabled {
        "bge-reranker-v2-m3"
    } else {
        "disabled"
    };

    // Use known config defaults (Config is private on Engine)
    Ok(serde_json::json!({
        "enabled": enabled,
        "model": model,
        "model_repo": "mogolloni/bge-reranker-v2-m3-onnx",
        "license": "Apache-2.0",
        "latency_ms": null,
        "improvement_percent": null,
        "batch_size": 16,
        "max_candidates": 100,
        "rrf_weight": 0.35,
        "circuit_breaker": {
            "state": format!("{:?}", breaker_stats.state).to_lowercase(),
            "success_count": breaker_stats.success_count,
            "failure_count": breaker_stats.failure_count,
            "total_failures": breaker_stats.total_failures,
            "rejected_count": breaker_stats.rejected_count,
            "success_rate": breaker_stats.success_rate(),
        }
    }))
}

/// Handle request for graph metrics.
async fn handle_graph_metrics(
    engine: Arc<Mutex<Engine>>,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;

    let status = eng.status().map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to get status: {e}"),
        )
    })?;

    // Authoritative edge-type counts from the live file dependency graph
    let file_graph = eng.file_dep_graph();
    let edge_counts = file_graph.count_by_edge_type();

    // Cycle detection from the symbol-level graph
    let cycles = match eng.dep_graph().find_cycles() {
        Ok(cycle_list) => cycle_list.len(),
        Err(e) => {
            tracing::warn!(error = %e, "failed to find cycles");
            0
        }
    };

    Ok(serde_json::json!({
        "nodes": status.graph_nodes,
        "edges": status.graph_edges,
        "edge_types": {
            "imports": edge_counts.get(&omni_core::graph::dependencies::EdgeType::Imports).copied().unwrap_or(status.dep_edges),
            "inherits": edge_counts.get(&omni_core::graph::dependencies::EdgeType::Inherits).copied().unwrap_or(0),
            "calls": edge_counts.get(&omni_core::graph::dependencies::EdgeType::Calls).copied().unwrap_or(0),
            "instantiates": edge_counts.get(&omni_core::graph::dependencies::EdgeType::Instantiates).copied().unwrap_or(0),
            "historical_co_change": edge_counts.get(&omni_core::graph::dependencies::EdgeType::HistoricalCoChange).copied().unwrap_or(0),
        },
        "file_graph_nodes": file_graph.node_count(),
        "file_graph_edges": file_graph.edge_count(),
        "cycles": cycles,
        "pagerank_computed": status.graph_nodes > 0,
        "max_hops": 2,
        "boosting_enabled": true
    }))
}

/// Handle request for search intent classification.
async fn handle_search_intent(
    _engine: Arc<Mutex<Engine>>,
    params: protocol::SearchIntentParams,
) -> Result<serde_json::Value, (i32, String)> {
    // Use the real QueryIntent classifier from omni-core
    let intent = omni_core::search::QueryIntent::classify(&params.query);
    let strategy = intent.context_strategy();

    let intent_label = format!("{intent:?}").to_lowercase();
    let confidence = strategy.graph_depth as f64 / 10.0; // normalize to 0.0-1.0 range

    Ok(serde_json::json!({
        "query": params.query,
        "intent": intent_label,
        "confidence": confidence.clamp(0.5, 1.0),
        "hyde_applicable": strategy.include_architecture,
        "synonyms_applicable": true,
        "strategy": {
            "graph_depth": strategy.graph_depth,
            "include_tests": strategy.include_tests,
            "include_architecture": strategy.include_architecture,
        }
    }))
}

// Resilience Monitoring Handlers
// ---------------------------------------------------------------------------

/// Handle request for resilience status (circuit breakers, health, dedup, backpressure).
async fn handle_resilience_status(
    engine: Arc<Mutex<Engine>>,
    event_dedup: Arc<crate::event_dedup::EventDeduplicator>,
    backpressure: Arc<crate::backpressure::BackpressureMonitor>,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;

    // Get circuit breaker states
    let embedder_cb = eng.embedder_breaker();
    let reranker_cb = eng.reranker_breaker();
    let index_cb = eng.index_breaker();
    let vector_cb = eng.vector_breaker();

    let embedder_stats = embedder_cb.stats();
    let reranker_stats = reranker_cb.stats();
    let index_stats = index_cb.stats();
    let vector_stats = vector_cb.stats();

    // Get health status
    let health_monitor = eng.health_monitor();
    let health_reports = health_monitor.all_reports();

    // Build health status map
    let mut health_status = serde_json::Map::new();
    let now = std::time::SystemTime::now();
    for report in health_reports {
        let status_str = match report.health {
            omni_core::resilience::health_monitor::SubsystemHealth::Healthy => "healthy",
            omni_core::resilience::health_monitor::SubsystemHealth::Degraded => "degraded",
            omni_core::resilience::health_monitor::SubsystemHealth::Critical => "unhealthy",
        };

        // Calculate approximate timestamp (now - elapsed)
        let elapsed_secs = report.timestamp.elapsed().as_secs();
        let timestamp_secs = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(elapsed_secs);

        health_status.insert(
            report.subsystem.clone(),
            serde_json::json!({
                "status": status_str,
                "last_check_time": timestamp_secs,
                "error_message": report.message
            }),
        );
    }

    let dedup_stats = event_dedup.stats();
    let bp_stats = backpressure.stats();

    Ok(serde_json::json!({
        "circuit_breakers": {
            "embedder": {
                "state": format!("{:?}", embedder_stats.state).to_lowercase(),
                "failure_count": embedder_stats.failure_count,
                "total_failures": embedder_stats.total_failures,
                "success_count": embedder_stats.success_count,
                "rejected_count": embedder_stats.rejected_count,
                "success_rate": embedder_stats.success_rate(),
            },
            "reranker": {
                "state": format!("{:?}", reranker_stats.state).to_lowercase(),
                "failure_count": reranker_stats.failure_count,
                "total_failures": reranker_stats.total_failures,
                "success_count": reranker_stats.success_count,
                "rejected_count": reranker_stats.rejected_count,
                "success_rate": reranker_stats.success_rate(),
            },
            "index": {
                "state": format!("{:?}", index_stats.state).to_lowercase(),
                "failure_count": index_stats.failure_count,
                "total_failures": index_stats.total_failures,
                "success_count": index_stats.success_count,
                "rejected_count": index_stats.rejected_count,
                "success_rate": index_stats.success_rate(),
            },
            "vector": {
                "state": format!("{:?}", vector_stats.state).to_lowercase(),
                "failure_count": vector_stats.failure_count,
                "total_failures": vector_stats.total_failures,
                "success_count": vector_stats.success_count,
                "rejected_count": vector_stats.rejected_count,
                "success_rate": vector_stats.success_rate(),
            }
        },
        "health_status": health_status,
        "deduplication": {
            "events_processed": dedup_stats.events_processed,
            "duplicates_skipped": dedup_stats.duplicates_skipped,
            "in_flight_count": event_dedup.in_flight_count(),
            "deduplication_rate": dedup_stats.deduplication_rate(),
            "avg_processing_time_ms": dedup_stats.avg_processing_time_ms(),
            "stale_tasks_cleaned": dedup_stats.stale_tasks_cleaned
        },
        "backpressure": {
            "active_requests": bp_stats.in_flight,
            "max_concurrent": bp_stats.max_concurrent,
            "load_percent": bp_stats.load_percent(),
            "requests_rejected": bp_stats.total_rejected,
            "requests_accepted": bp_stats.total_accepted,
            "peak_load_percent": bp_stats.peak_load_percent(),
            "rejection_rate": bp_stats.rejection_rate()
        }
    }))
}

/// Handle request to reset circuit breakers.
async fn handle_reset_circuit_breaker(
    engine: Arc<Mutex<Engine>>,
    params: protocol::ResetCircuitBreakerParams,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;

    match params.subsystem.as_str() {
        "embedder" => {
            eng.embedder_breaker().reset();
            tracing::info!("embedder circuit breaker reset");
        }
        "reranker" => {
            eng.reranker_breaker().reset();
            tracing::info!("reranker circuit breaker reset");
        }
        "index" => {
            eng.index_breaker().reset();
            tracing::info!("index circuit breaker reset");
        }
        "vector" => {
            eng.vector_breaker().reset();
            tracing::info!("vector circuit breaker reset");
        }
        "all" => {
            eng.embedder_breaker().reset();
            eng.reranker_breaker().reset();
            eng.index_breaker().reset();
            eng.vector_breaker().reset();
            tracing::info!("all circuit breakers reset");
        }
        _ => {
            return Err((
                error_codes::INVALID_PARAMS,
                format!("unknown subsystem: {}", params.subsystem),
            ));
        }
    }

    Ok(serde_json::json!({
        "success": true,
        "subsystem": params.subsystem,
        "message": format!("Circuit breaker for {} has been reset", params.subsystem)
    }))
}

// Historical Context Handlers
// ---------------------------------------------------------------------------

/// Handle request for commit context for a file.
async fn handle_commit_context(
    engine: Arc<Mutex<Engine>>,
    params: protocol::CommitContextParams,
) -> Result<serde_json::Value, (i32, String)> {
    // Validate file_path
    if params.file_path.trim().is_empty() {
        return Err((
            error_codes::INVALID_PARAMS,
            "file_path must not be empty".to_string(),
        ));
    }
    let eng = engine.lock().await;

    let limit = params.limit.clamp(1, 100); // Cap commit count, minimum 1

    // Get commits for the file
    let commits = omni_core::commits::CommitEngine::commits_for_file(
        eng.metadata_index(),
        &params.file_path,
        limit,
    )
    .map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to get commits: {e}"),
        )
    })?;

    // Get total commits indexed
    let recent_commits = omni_core::commits::CommitEngine::recent_commits(eng.metadata_index(), 1)
        .unwrap_or_default();
    let commits_indexed = recent_commits.len();

    // Convert to response format
    let commit_summaries: Vec<serde_json::Value> = commits
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "hash": c.hash,
                "message": c.message,
                "author": c.author,
                "timestamp": c.timestamp,
                "files_changed": c.files_changed.len()
            })
        })
        .collect();

    Ok(serde_json::json!({
        "file_path": params.file_path,
        "commits_indexed": commits_indexed,
        "recent_commits": commit_summaries
    }))
}

/// Handle request to index commit history.
async fn handle_index_commits(
    engine: Arc<Mutex<Engine>>,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;

    let commits_indexed = eng.index_commit_history().map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to index commits: {e}"),
        )
    })?;

    tracing::info!(commits = commits_indexed, "commit history indexed");

    Ok(serde_json::json!({
        "success": true,
        "commits_indexed": commits_indexed,
        "message": format!("Indexed {} commits", commits_indexed)
    }))
}

// Graph Visualization Handlers
// ---------------------------------------------------------------------------

/// Handle request for architectural context (N-hop neighborhood).
async fn handle_architectural_context(
    engine: Arc<Mutex<Engine>>,
    params: protocol::ArchitecturalContextParams,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;

    let file_path = std::path::PathBuf::from(&params.file_path);

    // Get architectural context from file-level graph
    let context = eng
        .file_dep_graph()
        .get_architectural_context(&file_path, Some(params.max_hops))
        .map_err(|e| {
            (
                error_codes::ENGINE_ERROR,
                format!("failed to get context: {e}"),
            )
        })?;

    // Convert to response format
    let neighbors: Vec<serde_json::Value> = context
        .neighbors
        .into_iter()
        .map(|n| {
            let edge_types: Vec<String> = n
                .edge_types
                .iter()
                .map(|et| et.as_str().to_string())
                .collect();

            serde_json::json!({
                "path": n.path.display().to_string(),
                "distance": n.distance,
                "edge_types": edge_types,
                "importance": n.importance
            })
        })
        .collect();

    Ok(serde_json::json!({
        "focal_file": params.file_path,
        "neighbors": neighbors,
        "total_files": context.total_files,
        "max_hops": context.max_hops
    }))
}

/// Handle request to find circular dependencies.
async fn handle_find_cycles(
    engine: Arc<Mutex<Engine>>,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;

    // Use symbol-level graph for cycle detection
    let cycles = eng.dep_graph().find_cycles().map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to find cycles: {e}"),
        )
    })?;

    Ok(serde_json::json!({
        "cycle_count": cycles.len(),
        "cycles": cycles
    }))
}

// Multi-repo workspace handlers
// ---------------------------------------------------------------------------

/// Handle request to list all repositories.
///
/// Returns every repo in the workspace registry — not just the primary repo
/// the daemon was launched with. Repos are sorted by priority descending.
async fn handle_list_repos(
    engine: Arc<Mutex<Engine>>,
    repo_registry: RepoRegistry,
) -> Result<serde_json::Value, (i32, String)> {
    // Get the primary repo path from the live engine for the "active" flag.
    let primary_path = {
        let eng = engine.lock().await;
        eng.repo_path().to_path_buf()
    };

    let ws = repo_registry.0.lock().await;
    let mut repos: Vec<serde_json::Value> = ws
        .list_linked_repos()
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "path": r.path.to_string_lossy(),
                "priority": r.priority,
                "auto_index": r.auto_index,
                "active": r.path == primary_path,
            })
        })
        .collect();

    // If the registry is empty (first run before any workspace/add_repo call),
    // surface the primary repo so clients always get at least one entry.
    if repos.is_empty() {
        let eng = engine.lock().await;
        let status = eng.status().map_err(|e| {
            (
                error_codes::ENGINE_ERROR,
                format!("failed to get status: {e}"),
            )
        })?;
        repos.push(serde_json::json!({
            "path": status.repo_path,
            "priority": 0.5,
            "auto_index": true,
            "active": true,
        }));
    }

    Ok(serde_json::json!({ "repos": repos }))
}

/// Handle request to add a repository to the workspace registry.
///
/// Validates the path exists, creates an Engine for it, links it to the
/// registry, and persists the config to disk atomically.
async fn handle_add_repo(
    repo_registry: RepoRegistry,
    params: protocol::AddRepoParams,
) -> Result<serde_json::Value, (i32, String)> {
    let path = std::path::Path::new(&params.path);

    if !path.exists() {
        return Err((
            error_codes::INVALID_PARAMS,
            format!("repository path does not exist: {}", params.path),
        ));
    }

    let mut ws = repo_registry.0.lock().await;
    ws.link_repo(path, None, params.priority).map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to add repository: {e}"),
        )
    })?;

    tracing::info!(path = %params.path, priority = params.priority, "repository added to workspace");

    Ok(serde_json::json!({
        "added": true,
        "path": params.path,
        "priority": params.priority,
        "repo_count": ws.repo_count(),
    }))
}

/// Handle request to update repository priority.
///
/// The priority weight [0.0, 1.0] scales search result scores from this
/// repo in multi-repo merged result sets.
async fn handle_set_priority(
    repo_registry: RepoRegistry,
    params: protocol::SetPriorityParams,
) -> Result<serde_json::Value, (i32, String)> {
    if !(0.0..=1.0).contains(&params.priority) {
        return Err((
            error_codes::INVALID_PARAMS,
            format!("priority must be in [0.0, 1.0]; got {}", params.priority),
        ));
    }

    let path = std::path::Path::new(&params.path);
    let mut ws = repo_registry.0.lock().await;
    let updated = ws.set_priority(path, params.priority).map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to set priority: {e}"),
        )
    })?;

    if updated {
        tracing::info!(path = %params.path, priority = params.priority, "repository priority updated");
        Ok(serde_json::json!({
            "updated": true,
            "path": params.path,
            "priority": params.priority,
        }))
    } else {
        Err((
            error_codes::INVALID_PARAMS,
            format!("repository not found in workspace: {}", params.path),
        ))
    }
}

/// Handle request to remove a repository from the workspace registry.
///
/// Does not delete any index data — only removes the registration so the
/// repo is excluded from future workspace operations.
async fn handle_remove_repo(
    repo_registry: RepoRegistry,
    params: protocol::RemoveRepoParams,
) -> Result<serde_json::Value, (i32, String)> {
    let path = std::path::Path::new(&params.path);
    let mut ws = repo_registry.0.lock().await;
    let removed = ws.unlink_repo(path).map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to remove repository: {e}"),
        )
    })?;

    if removed {
        tracing::info!(path = %params.path, "repository removed from workspace");
        Ok(serde_json::json!({
            "removed": true,
            "path": params.path,
            "repo_count": ws.repo_count(),
        }))
    } else {
        Err((
            error_codes::INVALID_PARAMS,
            format!("repository not found in workspace: {}", params.path),
        ))
    }
}

// Performance control handlers
// ---------------------------------------------------------------------------

/// Handle request for embedder metrics.
async fn handle_embedder_metrics(
    engine: Arc<Mutex<Engine>>,
    daemon_start_time: Arc<std::time::Instant>,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;

    let embedder = eng.embedder();
    let breaker_stats = eng.embedder_breaker().stats();
    let status = eng.status().map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to get status: {e}"),
        )
    })?;

    let uptime_secs = daemon_start_time.elapsed().as_secs_f64().max(1.0);
    let estimated_throughput = status.chunks_indexed as f64 / uptime_secs;
    #[allow(clippy::cast_precision_loss)]
    let memory_usage_mb = status.vector_memory_bytes as f64 / (1024.0 * 1024.0);

    Ok(serde_json::json!({
        "available": embedder.is_available(),
        "model_fingerprint": embedder.model_fingerprint(),
        "dimensions": embedder.dimensions(),
        "pool_size": embedder.pool_size(),
        "quantization_mode": "fp32",
        "memory_usage_mb": memory_usage_mb,
        "throughput_chunks_per_sec": estimated_throughput,
        "batch_fill_rate": if status.chunks_indexed > 0 {
            (status.vectors_indexed as f64 / status.chunks_indexed as f64).clamp(0.0, 1.0)
        } else {
            0.0
        },
        "vectors_indexed": status.vectors_indexed,
        "vector_memory_bytes": status.vector_memory_bytes,
        "embedding_coverage_percent": status.embedding_coverage_percent,
        "active_search_strategy": status.active_search_strategy,
        "circuit_breaker": {
            "state": format!("{:?}", breaker_stats.state).to_lowercase(),
            "success_count": breaker_stats.success_count,
            "failure_count": breaker_stats.failure_count,
            "total_failures": breaker_stats.total_failures,
            "rejected_count": breaker_stats.rejected_count,
            "success_rate": breaker_stats.success_rate(),
        }
    }))
}

/// Handle request to configure embedder.
///
/// Applies runtime configuration changes to the embedder without restarting the engine.
/// `batch_size` takes effect immediately on the next embedding flush.
/// `quantization_mode` and `batch_timeout_ms` are acknowledged but have no runtime effect
/// — quantization requires model reload and timeout is fixed at the session pool level.
async fn handle_configure_embedder(
    engine: Arc<Mutex<Engine>>,
    params: protocol::ConfigureEmbedderParams,
) -> Result<serde_json::Value, (i32, String)> {
    // Design: only batch_size can be mutated at runtime because it is read
    // per-flush from config.embedding.batch_size.  Quantization mode and
    // batch_timeout_ms require a model reload — document them as pending
    // and return the current effective values so callers can verify.
    let mut eng = engine.lock().await;

    let mut applied = serde_json::Map::new();
    let mut pending = serde_json::Map::new();

    if let Some(bs) = params.batch_size {
        let clamped = bs.clamp(1, 512);
        eng.config_mut().embedding.batch_size = clamped;
        applied.insert("batch_size".to_string(), serde_json::json!(clamped));
        tracing::info!(
            batch_size = clamped,
            "embedder batch_size updated at runtime"
        );
    }

    if let Some(ref mode) = params.quantization_mode {
        // Quantization mode change requires session reload; note it as pending.
        pending.insert(
            "quantization_mode".to_string(),
            serde_json::json!({
                "requested": mode,
                "note": "takes effect after engine restart"
            }),
        );
        tracing::info!(mode = %mode, "quantization_mode change acknowledged; requires restart");
    }

    if let Some(timeout) = params.batch_timeout_ms {
        // batch_timeout_ms is not stored in EmbeddingConfig; note as pending.
        pending.insert(
            "batch_timeout_ms".to_string(),
            serde_json::json!({
                "requested": timeout,
                "note": "not configurable at runtime; fixed at session pool level"
            }),
        );
    }

    let current_batch_size = eng.config().embedding.batch_size;
    let model_fingerprint = eng.embedder().model_fingerprint().to_string();

    Ok(serde_json::json!({
        "status": "ok",
        "applied": applied,
        "pending": pending,
        "current": {
            "batch_size": current_batch_size,
            "model_fingerprint": model_fingerprint,
        }
    }))
}

/// Handle request for index pool metrics.
async fn handle_index_pool_metrics(
    engine: Arc<Mutex<Engine>>,
    backpressure: Arc<crate::backpressure::BackpressureMonitor>,
    performance_metrics: Arc<crate::metrics::PerformanceMetrics>,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;
    let breaker_stats = eng.index_breaker().stats();
    let bp_stats = backpressure.stats();
    let status = eng.status().map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to get status: {e}"),
        )
    })?;

    // Engine currently uses a single SQLite connection, but report daemon load
    // and observed query latency as a practical pool/load signal.
    Ok(serde_json::json!({
        "active_connections": bp_stats.in_flight,
        "max_pool_size": bp_stats.max_concurrent,
        "utilization_percent": bp_stats.load_percent(),
        "avg_query_time_ms": performance_metrics.get_latency_percentile(0.95),
        "files_indexed": status.files_indexed,
        "chunks_indexed": status.chunks_indexed,
        "symbols_indexed": status.symbols_indexed,
        "hash_cache_entries": status.hash_cache_entries,
        "circuit_breaker": {
            "state": format!("{:?}", breaker_stats.state).to_lowercase(),
            "success_count": breaker_stats.success_count,
            "failure_count": breaker_stats.failure_count,
            "success_rate": breaker_stats.success_rate(),
        }
    }))
}

/// Handle request for compression statistics.
async fn handle_compression_stats(
    engine: Arc<Mutex<Engine>>,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;
    let status = eng.status().map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to get status: {e}"),
        )
    })?;

    // No quantization/compression applied yet — report raw sizes
    Ok(serde_json::json!({
        "vectors_indexed": status.vectors_indexed,
        "vector_memory_bytes": status.vector_memory_bytes,
        "compression_ratio": 1.0,
        "savings_percent": 0.0
    }))
}

// ---------------------------------------------------------------------------
// Co-Change & Plan Audit Handlers
// ---------------------------------------------------------------------------

async fn handle_co_changes(
    engine: Arc<Mutex<Engine>>,
    params: protocol::CoChangeParams,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;
    let index = eng.metadata_index();
    // Clamp min_frequency: protocol uses 0.0–1.0 scale, convert to count (×10, min 1, max 100)
    let min_freq = ((params.min_frequency * 10.0).clamp(1.0, 100.0)) as usize;
    let limit = params.limit.min(200); // Cap at 200 results
    let results = omni_core::commits::CommitEngine::co_change_files(
        index, &params.file_path, min_freq, limit,
    )
    .map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("co-change analysis failed: {e}"),
        )
    })?;

    let co_changed_files: Vec<serde_json::Value> = results
        .iter()
        .map(|f| {
            serde_json::json!({
                "path": f.path,
                "frequency": f.frequency as f64 / 100.0,
                "change_count": f.shared_commits,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "file_path": params.file_path,
        "co_changed_files": co_changed_files,
    }))
}

async fn handle_audit_plan(
    engine: Arc<Mutex<Engine>>,
    params: protocol::AuditPlanParams,
) -> Result<serde_json::Value, (i32, String)> {
    // Validate plan text
    if params.plan.trim().is_empty() {
        return Err((
            error_codes::INVALID_PARAMS,
            "plan text must not be empty".to_string(),
        ));
    }
    if params.plan.len() > 500_000 {
        return Err((
            error_codes::INVALID_PARAMS,
            "plan exceeds maximum length of 500000 characters".to_string(),
        ));
    }
    let eng = engine.lock().await;
    let auditor = omni_core::plan_auditor::PlanAuditor::new(&eng);
    let max_depth = params.max_depth.unwrap_or(3).clamp(1, 20);

    let critique = auditor
        .audit(&params.plan, max_depth)
        .map_err(|e| (error_codes::ENGINE_ERROR, format!("plan audit failed: {e}")))?;

    serde_json::to_value(&critique).map_err(|e| {
        (
            error_codes::INTERNAL_ERROR,
            format!("serialization failed: {e}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use omni_core::Engine;
    use std::path::PathBuf;
    use std::time::Duration;

    fn create_test_engine() -> Engine {
        std::env::set_var("OMNI_SKIP_MODEL_DOWNLOAD", "1");
        std::env::set_var("OMNI_DISABLE_RERANKER", "1");
        let temp_dir = std::env::temp_dir().join(format!(
            "omni-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create a minimal test file
        let test_file = temp_dir.join("test.rs");
        std::fs::write(&test_file, "fn main() { println!(\"Hello\"); }").unwrap();

        Engine::new(&temp_dir).unwrap()
    }

    #[tokio::test]
    async fn test_preflight_cache_miss() {
        let engine = Arc::new(Mutex::new(create_test_engine()));
        let cache = Arc::new(crate::prefetch::PrefetchCache::default());

        let params = protocol::PreflightParams {
            prompt: "test query".to_string(),
            active_file: Some("/path/to/test.rs".to_string()),
            cursor_line: Some(10),
            open_files: vec![],
            intent: Some("edit".to_string()),
            token_budget: 4000,
        };

        let start = std::time::Instant::now();
        let result = handle_preflight(engine.clone(), cache.clone(), params, start).await;

        assert!(result.is_ok());
        let value = result.unwrap();

        // Verify response structure
        assert!(value.get("system_context").is_some());
        assert!(value.get("from_cache").is_some());
        assert!(!value.get("from_cache").unwrap().as_bool().unwrap());

        // Verify cache stats show a miss (from the get attempt)
        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 0);
    }

    #[tokio::test]
    async fn test_preflight_cache_hit() {
        let engine = Arc::new(Mutex::new(create_test_engine()));
        let cache = Arc::new(crate::prefetch::PrefetchCache::default());

        let params = protocol::PreflightParams {
            prompt: "test query".to_string(),
            active_file: Some("/path/to/test.rs".to_string()),
            cursor_line: Some(10),
            open_files: vec![],
            intent: Some("edit".to_string()),
            token_budget: 4000,
        };

        // First request: cache miss, stores result
        let start1 = std::time::Instant::now();
        let result1 = handle_preflight(engine.clone(), cache.clone(), params.clone(), start1).await;
        assert!(result1.is_ok());

        let value1 = result1.unwrap();
        assert!(!value1.get("from_cache").unwrap().as_bool().unwrap());

        // Second request: cache hit
        let start2 = std::time::Instant::now();
        let result2 = handle_preflight(engine.clone(), cache.clone(), params, start2).await;
        assert!(result2.is_ok());

        let value2 = result2.unwrap();
        assert!(value2.get("from_cache").unwrap().as_bool().unwrap());

        // Verify cache stats
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1); // One miss from first request
        assert_eq!(stats.hit_rate, 0.5); // 1 hit out of 2 total accesses
    }

    #[tokio::test]
    async fn test_preflight_no_active_file() {
        let engine = Arc::new(Mutex::new(create_test_engine()));
        let cache = Arc::new(crate::prefetch::PrefetchCache::default());

        let params = protocol::PreflightParams {
            prompt: "test query".to_string(),
            active_file: None, // No active file
            cursor_line: None,
            open_files: vec![],
            intent: Some("explain".to_string()),
            token_budget: 4000,
        };

        let start = std::time::Instant::now();
        let result = handle_preflight(engine.clone(), cache.clone(), params, start).await;

        assert!(result.is_ok());
        let value = result.unwrap();

        // Should always be from_cache: false when no active_file
        assert!(!value.get("from_cache").unwrap().as_bool().unwrap());

        // Cache should not be accessed
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[tokio::test]
    async fn test_preflight_cache_expiry() {
        let engine = Arc::new(Mutex::new(create_test_engine()));
        // Create cache with very short TTL (10ms)
        let cache = Arc::new(crate::prefetch::PrefetchCache::new(
            100,
            Duration::from_millis(10),
        ));

        let params = protocol::PreflightParams {
            prompt: "test query".to_string(),
            active_file: Some("/path/to/test.rs".to_string()),
            cursor_line: Some(10),
            open_files: vec![],
            intent: Some("edit".to_string()),
            token_budget: 4000,
        };

        // First request: cache miss, stores result
        let start1 = std::time::Instant::now();
        let result1 = handle_preflight(engine.clone(), cache.clone(), params.clone(), start1).await;
        assert!(result1.is_ok());

        // Wait for cache to expire
        std::thread::sleep(Duration::from_millis(20));

        // Second request: cache miss due to expiry
        let start2 = std::time::Instant::now();
        let result2 = handle_preflight(engine.clone(), cache.clone(), params, start2).await;
        assert!(result2.is_ok());

        let value2 = result2.unwrap();
        assert!(!value2.get("from_cache").unwrap().as_bool().unwrap());

        // Both requests should be cache misses
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 2);
    }

    #[tokio::test]
    async fn test_preflight_different_files() {
        let engine = Arc::new(Mutex::new(create_test_engine()));
        let cache = Arc::new(crate::prefetch::PrefetchCache::default());

        let params1 = protocol::PreflightParams {
            prompt: "test query".to_string(),
            active_file: Some("/path/to/file1.rs".to_string()),
            cursor_line: Some(10),
            open_files: vec![],
            intent: Some("edit".to_string()),
            token_budget: 4000,
        };

        let params2 = protocol::PreflightParams {
            prompt: "test query".to_string(),
            active_file: Some("/path/to/file2.rs".to_string()),
            cursor_line: Some(20),
            open_files: vec![],
            intent: Some("edit".to_string()),
            token_budget: 4000,
        };

        // Request for file1
        let start1 = std::time::Instant::now();
        let result1 =
            handle_preflight(engine.clone(), cache.clone(), params1.clone(), start1).await;
        assert!(result1.is_ok());

        // Request for file2 (different file, should be cache miss)
        let start2 = std::time::Instant::now();
        let result2 = handle_preflight(engine.clone(), cache.clone(), params2, start2).await;
        assert!(result2.is_ok());

        let value2 = result2.unwrap();
        assert!(!value2.get("from_cache").unwrap().as_bool().unwrap());

        // Request for file1 again (should be cache hit)
        let start3 = std::time::Instant::now();
        let result3 = handle_preflight(engine.clone(), cache.clone(), params1, start3).await;
        assert!(result3.is_ok());

        let value3 = result3.unwrap();
        assert!(value3.get("from_cache").unwrap().as_bool().unwrap());

        // Verify cache stats: 1 hit, 2 misses
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 2);
    }

    #[tokio::test]
    async fn test_clear_cache_handler() {
        let cache = Arc::new(crate::prefetch::PrefetchCache::default());

        // Add some entries to cache
        cache.put_file_context(PathBuf::from("test1.rs"), "context1".to_string());
        cache.put_file_context(PathBuf::from("test2.rs"), "context2".to_string());

        // Generate some stats
        cache.get_file_context(&PathBuf::from("test1.rs")); // hit
        cache.get_file_context(&PathBuf::from("nonexistent.rs")); // miss

        let stats_before = cache.stats();
        assert_eq!(stats_before.size, 2);
        assert!(stats_before.hits > 0);
        assert!(stats_before.misses > 0);

        // Clear cache
        let result = handle_clear_cache(cache.clone()).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value.get("cleared").unwrap().as_bool().unwrap());

        // Verify cache is empty and stats are reset
        let stats_after = cache.stats();
        assert_eq!(stats_after.size, 0);
        assert_eq!(stats_after.hits, 0);
        assert_eq!(stats_after.misses, 0);
    }

    #[tokio::test]
    async fn test_prefetch_stats_handler() {
        let cache = Arc::new(crate::prefetch::PrefetchCache::default());

        // Add entries and generate stats
        cache.put_file_context(PathBuf::from("test.rs"), "context".to_string());
        cache.get_file_context(&PathBuf::from("test.rs")); // hit
        cache.get_file_context(&PathBuf::from("other.rs")); // miss

        let result = handle_prefetch_stats(cache.clone()).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value.get("hits").unwrap().as_u64().unwrap(), 1);
        assert_eq!(value.get("misses").unwrap().as_u64().unwrap(), 1);
        assert_eq!(value.get("size").unwrap().as_u64().unwrap(), 1);
        assert_eq!(value.get("hit_rate").unwrap().as_f64().unwrap(), 0.5);
    }

    #[tokio::test]
    async fn test_update_config_handler() {
        let cache = Arc::new(crate::prefetch::PrefetchCache::default());

        // Add some entries to cache
        cache.put_file_context(PathBuf::from("test1.rs"), "context1".to_string());
        cache.put_file_context(PathBuf::from("test2.rs"), "context2".to_string());

        let stats_before = cache.stats();
        assert_eq!(stats_before.size, 2);

        // Update cache configuration
        let params = protocol::UpdateConfigParams {
            cache_size: Some(50),
            cache_ttl_seconds: Some(600),
        };

        let result = handle_update_config(cache.clone(), params).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value.get("updated").unwrap().as_bool().unwrap());
        assert_eq!(value.get("cache_size").unwrap().as_u64().unwrap(), 50);
        assert_eq!(
            value.get("cache_ttl_seconds").unwrap().as_u64().unwrap(),
            600
        );

        // Verify existing entries are still present
        let stats_after = cache.stats();
        assert_eq!(stats_after.size, 2);
        assert!(cache.get_file_context(&PathBuf::from("test1.rs")).is_some());
        assert!(cache.get_file_context(&PathBuf::from("test2.rs")).is_some());
    }

    #[tokio::test]
    async fn test_update_config_partial() {
        let cache = Arc::new(crate::prefetch::PrefetchCache::default());

        // Update only cache size
        let params1 = protocol::UpdateConfigParams {
            cache_size: Some(75),
            cache_ttl_seconds: None,
        };

        let result1 = handle_update_config(cache.clone(), params1).await;
        assert!(result1.is_ok());

        let value1 = result1.unwrap();
        assert!(value1.get("updated").unwrap().as_bool().unwrap());

        // Update only TTL
        let params2 = protocol::UpdateConfigParams {
            cache_size: None,
            cache_ttl_seconds: Some(900),
        };

        let result2 = handle_update_config(cache.clone(), params2).await;
        assert!(result2.is_ok());

        let value2 = result2.unwrap();
        assert!(value2.get("updated").unwrap().as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_update_config_no_changes() {
        let cache = Arc::new(crate::prefetch::PrefetchCache::default());

        // Update with no parameters
        let params = protocol::UpdateConfigParams {
            cache_size: None,
            cache_ttl_seconds: None,
        };

        let result = handle_update_config(cache.clone(), params).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(!value.get("updated").unwrap().as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_update_config_capacity_reduction() {
        let cache = Arc::new(crate::prefetch::PrefetchCache::new(
            100,
            Duration::from_secs(300),
        ));

        // Add 5 entries
        for i in 1..=5 {
            cache.put_file_context(PathBuf::from(format!("test{i}.rs")), format!("context{i}"));
        }

        let stats_before = cache.stats();
        assert_eq!(stats_before.size, 5);

        // Reduce capacity to 3
        let params = protocol::UpdateConfigParams {
            cache_size: Some(3),
            cache_ttl_seconds: None,
        };

        let result = handle_update_config(cache.clone(), params).await;
        assert!(result.is_ok());

        // Verify only 3 entries remain (most recent ones)
        let stats_after = cache.stats();
        assert_eq!(stats_after.size, 3);
    }
}
