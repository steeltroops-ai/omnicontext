//! IPC transport layer for the OmniContext daemon.
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

use crate::protocol::{self, error_codes, Response};

/// Derive a deterministic pipe/socket name from the repository path.
pub fn default_pipe_name(repo_path: &Path) -> String {
    use sha2::{Sha256, Digest};
    let normalized = repo_path
        .to_string_lossy()
        .replace(r"\\?\", "")
        .to_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let hash = hex::encode(&hasher.finalize()[..6]);

    #[cfg(windows)]
    {
        format!(r"\\.\pipe\omnicontext-{}", hash)
    }

    #[cfg(not(windows))]
    {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| "/tmp".to_string());
        format!("{}/omnicontext-{}.sock", runtime_dir, hash)
    }
}

/// Start the IPC server and listen for client connections.
pub async fn serve(engine: Engine, pipe_name: &str) -> anyhow::Result<()> {
    let engine = Arc::new(Mutex::new(engine));

    #[cfg(windows)]
    {
        serve_named_pipe(engine, pipe_name).await
    }

    #[cfg(not(windows))]
    {
        serve_unix_socket(engine, pipe_name).await
    }
}

// ---------------------------------------------------------------------------
// Windows: Named Pipe server
// ---------------------------------------------------------------------------

#[cfg(windows)]
async fn serve_named_pipe(
    engine: Arc<Mutex<Engine>>,
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
        tokio::spawn(async move {
            let (reader, writer) = tokio::io::split(server);
            if let Err(e) = handle_client(engine, reader, writer).await {
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
        tokio::spawn(async move {
            let (reader, writer) = tokio::io::split(stream);
            if let Err(e) = handle_client(engine, reader, writer).await {
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
            Ok(req) => dispatch(engine.clone(), req).await,
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
    req: protocol::Request,
) -> Response {
    let start = std::time::Instant::now();

    let result = match req.method.as_str() {
        "ping" => Ok(serde_json::json!({ "pong": true })),

        "status" => handle_status(engine.clone()).await,

        "search" => {
            let params: protocol::SearchParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_search(engine.clone(), params).await
        }

        "context_window" => {
            let params: protocol::ContextWindowParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_context_window(engine.clone(), params).await
        }

        "preflight" => {
            let params: protocol::PreflightParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_preflight(engine.clone(), params, start).await
        }

        "module_map" => {
            let params: protocol::ModuleMapParams = match parse_params(&req) {
                Ok(p) => p,
                Err(r) => return r,
            };
            handle_module_map(engine.clone(), params).await
        }

        "index" => handle_index(engine.clone()).await,

        "shutdown" => {
            tracing::info!("shutdown requested via IPC");
            std::process::exit(0);
        }

        _ => Err((
            error_codes::METHOD_NOT_FOUND,
            format!("unknown method: {}", req.method),
        )),
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;
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
fn parse_params<T: serde::de::DeserializeOwned>(
    req: &protocol::Request,
) -> Result<T, Response> {
    let params = req.params.clone().unwrap_or(serde_json::Value::Object(Default::default()));
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
        .map_err(|e| (error_codes::ENGINE_ERROR, format!("context_window failed: {e}")))
}

async fn handle_preflight(
    engine: Arc<Mutex<Engine>>,
    params: protocol::PreflightParams,
    start: std::time::Instant,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;

    // Build the context window from the user's prompt
    let ctx = eng
        .search_context_window(&params.prompt, 20, Some(params.token_budget))
        .map_err(|e| (error_codes::ENGINE_ERROR, format!("preflight search failed: {e}")))?;

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
    system_context.push_str(&format!(
        "## Repository\n- Files: {}\n- Symbols: {}\n- Intent: {}\n\n",
        status.files_indexed, status.symbols_indexed, intent_label,
    ));

    // Active file context
    if let Some(ref active) = params.active_file {
        system_context.push_str(&format!("## Active File\n{}\n", active));
        if let Some(line) = params.cursor_line {
            system_context.push_str(&format!("Cursor at line: {}\n", line));
        }
        system_context.push('\n');
    }

    // Relevant code (ranked by relevance)
    system_context.push_str("## Relevant Code (ranked by relevance)\n\n");
    system_context.push_str(&ctx.render());

    system_context.push_str("\n</context_engine>\n");

    let elapsed_ms = start.elapsed().as_millis() as u64;

    let response = protocol::PreflightResponse {
        system_context,
        entries_count: ctx.len(),
        tokens_used: ctx.total_tokens,
        token_budget: ctx.token_budget,
        elapsed_ms,
    };

    serde_json::to_value(response)
        .map_err(|e| (error_codes::INTERNAL_ERROR, format!("serialization failed: {e}")))
}

async fn handle_module_map(
    engine: Arc<Mutex<Engine>>,
    _params: protocol::ModuleMapParams,
) -> Result<serde_json::Value, (i32, String)> {
    let eng = engine.lock().await;
    let index = eng.metadata_index();

    // Build module map from indexed files
    let files = index
        .get_all_files()
        .map_err(|e| (error_codes::ENGINE_ERROR, format!("failed to get files: {e}")))?;

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
            serde_json::json!({
                "files_processed": result.files_processed,
                "chunks_created": result.chunks_created,
                "symbols_extracted": result.symbols_extracted,
                "embeddings_generated": result.embeddings_generated,
                "elapsed_ms": start.elapsed().as_millis() as u64,
            })
        })
        .map_err(|e| (error_codes::ENGINE_ERROR, format!("indexing failed: {e}")))
}
