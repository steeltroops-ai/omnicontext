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

use omni_core::Engine;

use crate::metrics::PerformanceMetrics;
use crate::protocol::{self, error_codes, Response};

/// Derive a deterministic pipe/socket name from the repository path.
pub fn default_pipe_name(repo_path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let normalized = repo_path
        .to_string_lossy()
        .replace(r"\\?\", "")
        .to_lowercase();
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

    #[cfg(windows)]
    {
        serve_named_pipe(
            engine, prefetch_cache, daemon_start_time, performance_metrics, pipe_name,
        )
        .await
    }

    #[cfg(not(windows))]
    {
        serve_unix_socket(
            engine, prefetch_cache, daemon_start_time, performance_metrics, pipe_name,
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
    pipe_name: &str,
) -> anyhow::Result<()> {
    use tokio::net::windows::named_pipe::ServerOptions;

    tracing::info!(pipe = %pipe_name, "listening on named pipe");

    loop {
        // Create a new pipe instance for each client
        let server = ServerOptions::new()
            .first_pipe_instance(false)
            .create(pipe_name)?;

        // Wait for a client to connect
        server.connect().await?;

        tracing::info!("client connected");

        let engine = engine.clone();
        let cache = prefetch_cache.clone();
        let start_time = daemon_start_time.clone();
        let metrics = performance_metrics.clone();
        tokio::spawn(async move {
            let (reader, writer) = tokio::io::split(server);
            if let Err(e) = handle_client(engine, cache, start_time, metrics, reader, writer).await
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
    socket_path: &str,
) -> anyhow::Result<()> {
    use tokio::net::UnixListener;

    // Remove stale socket file
    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)?;
    tracing::info!(socket = %socket_path, "listening on unix socket");

    loop {
        let (stream, _) = listener.accept().await?;
        tracing::info!("client connected");

        let engine = engine.clone();
        let cache = prefetch_cache.clone();
        let start_time = daemon_start_time.clone();
        let metrics = performance_metrics.clone();
        tokio::spawn(async move {
            let (reader, writer) = tokio::io::split(stream);
            if let Err(e) = handle_client(engine, cache, start_time, metrics, reader, writer).await
            {
                tracing::warn!(error = %e, "client handler error");
            }
            tracing::info!("client disconnected");
        });
    }
}

// ---------------------------------------------------------------------------
// Client handler (platform-agnostic)
// ---------------------------------------------------------------------------

/// Handle a single connected client.
///
/// Reads newline-delimited JSON-RPC requests, dispatches them to the engine,
/// and writes JSON-RPC responses back.
async fn handle_client<R, W>(
    engine: Arc<Mutex<Engine>>,
    prefetch_cache: Arc<crate::prefetch::PrefetchCache>,
    daemon_start_time: Arc<std::time::Instant>,
    performance_metrics: Arc<crate::metrics::PerformanceMetrics>,
    reader: R,
    mut writer: W,
) -> anyhow::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<protocol::Request>(&line) {
            Ok(req) => {
                dispatch(
                    engine.clone(),
                    prefetch_cache.clone(),
                    daemon_start_time.clone(),
                    performance_metrics.clone(),
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
        writer.write_all(response_json.as_bytes()).await?;
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
    req: protocol::Request,
) -> Response {
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
            handle_ide_event(prefetch_cache.clone(), params).await
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
            tracing::info!("shutdown requested via IPC");
            std::process::exit(0);
        }

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
        if let Some(cached_context) = prefetch_cache.get_file_context(&cache_key) {
            #[allow(clippy::cast_possible_truncation)]
            let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

            tracing::info!(
                file = %active_file,
                elapsed_ms = elapsed_ms,
                "cache hit: returning cached preflight context"
            );

            // Parse the cached context to extract metadata
            // For now, we'll return reasonable defaults since we're caching the full context
            let response = protocol::PreflightResponse {
                system_context: cached_context,
                entries_count: 0, // Unknown from cache
                tokens_used: 0,   // Unknown from cache
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
    let mut system_context = String::with_capacity(ctx.total_tokens as usize * 4);

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
        entries_count: ctx.len(),
        tokens_used: ctx.total_tokens,
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
                "chunks_created": result.chunks_created,
                "symbols_extracted": result.symbols_extracted,
                "embeddings_generated": result.embeddings_generated,
                "elapsed_ms": elapsed_ms,
            })
        })
        .map_err(|e| (error_codes::ENGINE_ERROR, format!("indexing failed: {e}")))
}

/// Handle IDE event for pre-fetch.
#[allow(clippy::unused_async)] // Will be async when fully implemented
async fn handle_ide_event(
    _prefetch_cache: Arc<crate::prefetch::PrefetchCache>,
    params: protocol::IdeEventParams,
) -> Result<serde_json::Value, (i32, String)> {
    tracing::debug!(
        event_type = %params.event_type,
        file = %params.file_path,
        "IDE event received"
    );

    // For now, just acknowledge the event
    // In a full implementation, we would:
    // 1. Parse the event type
    // 2. Trigger appropriate pre-fetch based on heuristics
    // 3. Store results in the cache

    match params.event_type.as_str() {
        "file_opened" => {
            // Pre-fetch file-level context
            tracing::debug!(file = %params.file_path, "pre-fetching file context");
            // TODO: Implement actual pre-fetch logic
        }
        "cursor_moved" => {
            // Pre-fetch symbol dependencies if symbol is known
            if let Some(symbol) = &params.symbol {
                tracing::debug!(
                    file = %params.file_path,
                    symbol = %symbol,
                    "pre-fetching symbol context"
                );
                // TODO: Implement actual pre-fetch logic
            }
        }
        "text_edited" => {
            // Pre-fetch related tests
            tracing::debug!(file = %params.file_path, "pre-fetching related tests");
            // TODO: Implement actual pre-fetch logic
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

#[cfg(test)]
mod tests {
    use super::*;
    use omni_core::Engine;
    use std::path::PathBuf;
    use std::time::Duration;

    /// Helper to create a test engine with minimal setup
    fn create_test_engine() -> Engine {
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
        assert_eq!(value.get("from_cache").unwrap().as_bool().unwrap(), false);

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
        assert_eq!(value1.get("from_cache").unwrap().as_bool().unwrap(), false);

        // Second request: cache hit
        let start2 = std::time::Instant::now();
        let result2 = handle_preflight(engine.clone(), cache.clone(), params, start2).await;
        assert!(result2.is_ok());

        let value2 = result2.unwrap();
        assert_eq!(value2.get("from_cache").unwrap().as_bool().unwrap(), true);

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
        assert_eq!(value.get("from_cache").unwrap().as_bool().unwrap(), false);

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
        assert_eq!(value2.get("from_cache").unwrap().as_bool().unwrap(), false);

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
        assert_eq!(value2.get("from_cache").unwrap().as_bool().unwrap(), false);

        // Request for file1 again (should be cache hit)
        let start3 = std::time::Instant::now();
        let result3 = handle_preflight(engine.clone(), cache.clone(), params1, start3).await;
        assert!(result3.is_ok());

        let value3 = result3.unwrap();
        assert_eq!(value3.get("from_cache").unwrap().as_bool().unwrap(), true);

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
        assert_eq!(value.get("cleared").unwrap().as_bool().unwrap(), true);

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
        assert_eq!(value.get("updated").unwrap().as_bool().unwrap(), true);
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
        assert_eq!(value1.get("updated").unwrap().as_bool().unwrap(), true);

        // Update only TTL
        let params2 = protocol::UpdateConfigParams {
            cache_size: None,
            cache_ttl_seconds: Some(900),
        };

        let result2 = handle_update_config(cache.clone(), params2).await;
        assert!(result2.is_ok());

        let value2 = result2.unwrap();
        assert_eq!(value2.get("updated").unwrap().as_bool().unwrap(), true);
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
        assert_eq!(value.get("updated").unwrap().as_bool().unwrap(), false);
    }

    #[tokio::test]
    async fn test_update_config_capacity_reduction() {
        let cache = Arc::new(crate::prefetch::PrefetchCache::new(
            100,
            Duration::from_secs(300),
        ));

        // Add 5 entries
        for i in 1..=5 {
            cache.put_file_context(
                PathBuf::from(format!("test{}.rs", i)),
                format!("context{}", i),
            );
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
