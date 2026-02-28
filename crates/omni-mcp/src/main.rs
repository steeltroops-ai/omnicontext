//! OmniContext MCP Server.
//!
//! Exposes code intelligence tools to AI coding agents via the
//! Model Context Protocol (MCP). Supports stdio transport.
//!
//! ## Usage
//!
//! ```text
//! # Start the MCP server (AI agents connect via stdio)
//! omnicontext-mcp --repo /path/to/repo
//!
//! # Or from the CLI
//! omnicontext mcp --repo .
//! ```

mod tools;

use anyhow::Result;
use clap::Parser;
use rmcp::ServiceExt;

/// OmniContext MCP Server
#[derive(Parser, Debug)]
#[command(name = "omnicontext-mcp", version, about = "MCP server for AI code intelligence")]
struct Args {
    /// Path to the repository to serve.
    #[arg(long, default_value = ".")]
    repo: String,

    /// Log level (trace, debug, info, warn, error).
    #[arg(long, default_value = "info")]
    log_level: String,
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
    let engine = omni_core::Engine::new(&repo_path)?;

    tracing::info!("engine initialized, starting MCP server on stdio");

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
