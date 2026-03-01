//! `OmniContext` MCP Server.
//!
//! Exposes code intelligence tools to AI coding agents via the
//! Model Context Protocol (MCP). Supports stdio transport.
//!
//! ## Auto-Index on Startup
//!
//! By default, the MCP server automatically indexes the repository
//! on startup if no existing index is found. This ensures AI agents
//! always connect to a ready-to-use engine without manual steps.
//!
//! ## Usage
//!
//! ```text
//! # Start the MCP server (AI agents connect via stdio)
//! omnicontext-mcp --repo /path/to/repo
//!
//! # Skip auto-index (use existing index only)
//! omnicontext-mcp --repo /path/to/repo --no-auto-index
//!
//! # Or from the CLI
//! omnicontext mcp --repo .
//! ```

mod tools;

use anyhow::Result;
use clap::Parser;
use rmcp::ServiceExt;

/// `OmniContext` MCP Server
#[derive(Parser, Debug)]
#[command(
    name = "omnicontext-mcp",
    version,
    about = "MCP server for AI code intelligence"
)]
struct Args {
    /// Path to the repository to serve.
    #[arg(long, default_value = ".")]
    repo: String,

    /// Log level (trace, debug, info, warn, error).
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Skip automatic indexing on startup.
    /// By default, the server indexes the repo if no index exists.
    #[arg(long)]
    no_auto_index: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing -- write to stderr so stdout stays clean for JSON-RPC
    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .with_writer(std::io::stderr)
        .init();

    let repo_path = std::path::PathBuf::from(&args.repo)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&args.repo));

    if !repo_path.exists() {
        anyhow::bail!("repository path does not exist: {}", args.repo);
    }

    tracing::info!(
        repo = %repo_path.display(),
        "initializing OmniContext engine"
    );

    // Initialize the core engine
    let mut engine = omni_core::Engine::new(&repo_path)?;

    // Auto-index: if the index is empty and auto-index is not disabled,
    // run a full index before starting the MCP server.
    // This ensures AI agents always connect to a ready engine.
    if !args.no_auto_index {
        let status = engine.status()?;
        if status.files_indexed == 0 {
            tracing::info!("no existing index found, running auto-index...");
            let start = std::time::Instant::now();
            match engine.run_index().await {
                Ok(result) => {
                    tracing::info!(
                        files = result.files_processed,
                        chunks = result.chunks_created,
                        symbols = result.symbols_extracted,
                        embeddings = result.embeddings_generated,
                        elapsed_ms = {
                            #[allow(clippy::cast_possible_truncation)]
                            let ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                            ms
                        },
                        "auto-index complete"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "auto-index failed, MCP tools may return empty results"
                    );
                }
            }
        } else {
            tracing::info!(
                files = status.files_indexed,
                chunks = status.chunks_indexed,
                "using existing index"
            );
        }
    }

    tracing::info!("engine ready, starting MCP server on stdio");

    // Create and start the MCP server
    let server = tools::OmniContextServer::new(engine);
    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .inspect_err(|e| {
            tracing::error!(error = %e, "failed to start MCP server");
        })?;

    // Wait for the service to complete (client disconnects)
    service.waiting().await?;

    tracing::info!("MCP server shut down");
    Ok(())
}
