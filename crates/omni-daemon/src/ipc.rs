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

use crate::metrics::PerformanceMetrics;
use crate::protocol::{self, error_codes, Response};

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
    let engine = Arc::new(Mutex::new(engine));
    let prefetch_cache = Arc::new(crate::prefetch::PrefetchCache::default());
    let daemon_start_time = Arc::new(std::time::Instant::now());
    let performance_metrics = Arc::new(crate::metrics::PerformanceMetrics::default());
    let event_dedup = Arc::new(crate::event_dedup::EventDeduplicator::new());
    let backpressure = Arc::new(crate::backpressure::BackpressureMonitor::new(100)); // max 100 concurrent requests
    let shutdown_token = CancellationToken::new();

    #[cfg(windows)]
    {
        serve_named_pipe(
            engine, prefetch_cache, daemon_start_time, performance_metrics, event_dedup,
            backpressure, pipe_name, shutdown_token,
        )
        .await
    }

    #[cfg(not(windows))]
    {
        serve_unix_socket(
            engine, prefetch_cache, daemon_start_time, performance_metrics, event_dedup,
            backpressure, pipe_name, shutdown_token,
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
        let cache = prefetch_cache.clone();
        let start_time = daemon_start_time.clone();
        let metrics = performance_metrics.clone();
        let dedup = event_dedup.clone();
        let bp = backpressure.clone();
        let token = shutdown_token.clone();
        tokio::spawn(async move {
            let (reader, writer) = tokio::io::split(server);
            if let Err(e) = handle_client(
                engine, cache, start_time, metrics, dedup, bp, token, reader, writer,
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
                let cache = prefetch_cache.clone();
                let start_time = daemon_start_time.clone();
                let metrics = performance_metrics.clone();
                let dedup = event_dedup.clone();
                let bp = backpressure.clone();
                let token = shutdown_token.clone();
                tokio::spawn(async move {
                    let (reader, writer) = tokio::io::split(stream);
                    if let Err(e) = handle_client(engine, cache, start_time, metrics, dedup, bp, token, reader, writer).await
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
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
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

        // Phase 1: Intelligence Layer Methods
        "reranker/get_metrics" => handle_reranker_metrics(engine.clone()).await,

        "graph/get_metrics" => handle_graph_metrics(engine.clone()).await,

        "search/get_intent" => {
            let params: protocol::SearchIntentParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_search_intent(engine.clone(), params).await
        }

        // Phase 2: Resilience Monitoring Methods
        "resilience/get_status" => handle_resilience_status(engine.clone()).await,

        "resilience/reset_circuit_breaker" => {
            let params: protocol::ResetCircuitBreakerParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_reset_circuit_breaker(engine.clone(), params).await
        }

        // Phase 3: Historical Context Methods
        "history/get_commit_context" => {
            let params: protocol::CommitContextParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_commit_context(engine.clone(), params).await
        }

        "history/index_commits" => handle_index_commits(engine.clone()).await,

        // Phase 4: Graph Visualization Methods
        "graph/get_architectural_context" => {
            let params: protocol::ArchitecturalContextParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_architectural_context(engine.clone(), params).await
        }

        "graph/find_cycles" => handle_find_cycles(engine.clone()).await,

        // Phase 5: Multi-Repository Support
        "workspace/list_repos" => handle_list_repos(engine.clone()).await,

        "workspace/add_repo" => {
            let params: protocol::AddRepoParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_add_repo(engine.clone(), params).await
        }

        "workspace/set_priority" => {
            let params: protocol::SetPriorityParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_set_priority(engine.clone(), params).await
        }

        "workspace/remove_repo" => {
            let params: protocol::RemoveRepoParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_remove_repo(engine.clone(), params).await
        }

        // Phase 6: Performance Controls
        "embedder/get_metrics" => handle_embedder_metrics(engine.clone()).await,

        "embedder/configure" => {
            let params: protocol::ConfigureEmbedderParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_configure_embedder(engine.clone(), params).await
        }

        "index/get_pool_metrics" => handle_index_pool_metrics(engine.clone()).await,

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

    // For now, we don't track last index time, so return None
    // This can be enhanced later by storing index timestamps
    let last_index_time: Option<u64> = None;

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

    // Get memory usage
    let memory_usage_bytes = PerformanceMetrics::get_current_memory_bytes();
    let peak_memory_usage_bytes = performance_metrics.get_peak_memory_bytes();

    // Update peak memory if current is higher
    performance_metrics.update_memory_usage(memory_usage_bytes);

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
    let eng = engine.lock().await;
    eng.search(&params.query, params.limit)
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
    let eng = engine.lock().await;
    eng.search_context_window(&params.query, params.limit, params.token_budget)
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

async fn handle_index(engine: Arc<Mutex<Engine>>) -> Result<serde_json::Value, (i32, String)> {
    let mut eng = engine.lock().await;
    let start = std::time::Instant::now();

    eng.run_index()
        .await
        .map(|result| {
            #[allow(clippy::cast_possible_truncation)]
            let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

            serde_json::json!({
                "files_processed": result.files_processed,
                "files_failed": result.files_failed,
                "chunks_created": result.chunks_created,
                "symbols_extracted": result.symbols_extracted,
                "embeddings_generated": result.embeddings_generated,
                "embedding_failures": result.embedding_failures,
                "elapsed_ms": elapsed_ms,
            })
        })
        .map_err(|e| (error_codes::ENGINE_ERROR, format!("indexing failed: {e}")))
}

/// Handle IDE event for pre-fetch.
///
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

    match params.event_type.as_str() {
        "file_opened" => {
            let eng = engine.clone();
            let cache = prefetch_cache.clone();
            let file_path = params.file_path.clone();
            tokio::spawn(async move {
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
                        Ok((stats, changed)) => {
                            if changed {
                                // Invalidate cached context for this file
                                let cache_key = std::path::PathBuf::from(&file_path);
                                cache.invalidate_file(&cache_key);
                                tracing::debug!(
                                    file = %file_path,
                                    chunks = stats.chunks,
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
async fn handle_clear_index(
    engine: Arc<Mutex<Engine>>,
    _params: protocol::ClearIndexParams,
) -> Result<serde_json::Value, (i32, String)> {
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
// Phase 1: Intelligence Layer Handlers
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
        "jina-reranker-v2-base-multilingual"
    } else {
        "disabled"
    };

    // Use known config defaults (Config is private on Engine)
    Ok(serde_json::json!({
        "enabled": enabled,
        "model": model,
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

    // Get actual cycle count
    let dep_graph = eng.dep_graph();
    let cycles = match dep_graph.find_cycles() {
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
            "imports": status.dep_edges,
            "inherits": null,
            "calls": null,
            "instantiates": null
        },
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
    // Classify query intent (simplified - actual implementation in search module)
    let query_lower = params.query.to_lowercase();
    let (intent, confidence) = if query_lower.contains("architecture")
        || query_lower.contains("structure")
        || query_lower.contains("design")
        || query_lower.contains("how does")
    {
        ("architectural", 0.85)
    } else if query_lower.contains("bug")
        || query_lower.contains("error")
        || query_lower.contains("fix")
        || query_lower.contains("debug")
    {
        ("debugging", 0.80)
    } else {
        ("implementation", 0.75)
    };

    Ok(serde_json::json!({
        "query": params.query,
        "intent": intent,
        "confidence": confidence,
        "hyde_applicable": intent == "architectural",
        "synonyms_applicable": true
    }))
}

// Phase 2: Resilience Monitoring Handlers
// ---------------------------------------------------------------------------

/// Handle request for resilience status (circuit breakers, health, dedup, backpressure).
async fn handle_resilience_status(
    engine: Arc<Mutex<Engine>>,
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
            "events_processed": null,
            "duplicates_skipped": null,
            "in_flight_count": null,
            "deduplication_rate": null
        },
        "backpressure": {
            "active_requests": null,
            "load_percent": null,
            "requests_rejected": null,
            "peak_load_percent": null
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

// Phase 3: Historical Context Handlers
// ---------------------------------------------------------------------------

/// Handle request for commit context for a file.
async fn handle_commit_context(
    engine: Arc<Mutex<Engine>>,
    params: protocol::CommitContextParams,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;

    // Get commits for the file
    let commits = omni_core::commits::CommitEngine::commits_for_file(
        eng.metadata_index(),
        &params.file_path,
        params.limit,
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

// Phase 4: Graph Visualization Handlers
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

// Phase 5: Multi-Repository Support Handlers
// ---------------------------------------------------------------------------

/// Handle request to list all repositories.
async fn handle_list_repos(
    _engine: Arc<Mutex<Engine>>,
) -> Result<serde_json::Value, (i32, String)> {
    // TODO: Implement multi-repository support in Engine
    // For now, return empty list as placeholder
    Ok(serde_json::json!({
        "repos": []
    }))
}

/// Handle request to add a repository.
async fn handle_add_repo(
    _engine: Arc<Mutex<Engine>>,
    _params: protocol::AddRepoParams,
) -> Result<serde_json::Value, (i32, String)> {
    // TODO: Implement repository addition
    Ok(serde_json::json!({
        "success": true,
        "message": "Repository addition not yet implemented"
    }))
}

/// Handle request to set repository priority.
async fn handle_set_priority(
    _engine: Arc<Mutex<Engine>>,
    _params: protocol::SetPriorityParams,
) -> Result<serde_json::Value, (i32, String)> {
    // TODO: Implement priority setting
    Ok(serde_json::json!({
        "success": true,
        "message": "Priority setting not yet implemented"
    }))
}

/// Handle request to remove a repository.
async fn handle_remove_repo(
    _engine: Arc<Mutex<Engine>>,
    _params: protocol::RemoveRepoParams,
) -> Result<serde_json::Value, (i32, String)> {
    // TODO: Implement repository removal
    Ok(serde_json::json!({
        "success": true,
        "message": "Repository removal not yet implemented"
    }))
}

// Phase 6: Performance Controls Handlers
// ---------------------------------------------------------------------------

/// Handle request for embedder metrics.
async fn handle_embedder_metrics(
    engine: Arc<Mutex<Engine>>,
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

    Ok(serde_json::json!({
        "available": embedder.is_available(),
        "model_fingerprint": embedder.model_fingerprint(),
        "dimensions": embedder.dimensions(),
        "pool_size": embedder.pool_size(),
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
async fn handle_configure_embedder(
    _engine: Arc<Mutex<Engine>>,
    _params: protocol::ConfigureEmbedderParams,
) -> Result<serde_json::Value, (i32, String)> {
    // TODO: Implement embedder configuration
    Ok(serde_json::json!({
        "success": true,
        "message": "Embedder configuration not yet implemented"
    }))
}

/// Handle request for index pool metrics.
async fn handle_index_pool_metrics(
    engine: Arc<Mutex<Engine>>,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;
    let breaker_stats = eng.index_breaker().stats();
    let status = eng.status().map_err(|e| {
        (
            error_codes::ENGINE_ERROR,
            format!("failed to get status: {e}"),
        )
    })?;

    // Engine uses a single Connection (ConnectionPool not wired in yet)
    Ok(serde_json::json!({
        "active_connections": 1,
        "max_pool_size": 1,
        "utilization_percent": 100.0,
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
