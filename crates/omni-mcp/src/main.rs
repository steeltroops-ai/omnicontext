//! OmniContext MCP Server.
//!
//! Exposes code intelligence tools to AI coding agents via the
//! Model Context Protocol (MCP). Supports stdio and SSE transports.

use anyhow::Result;
use clap::Parser;

/// OmniContext MCP Server
#[derive(Parser, Debug)]
#[command(name = "omnicontext-mcp", version, about)]
struct Args {
    /// Path to the repository to index.
    #[arg(long, default_value = ".")]
    repo: String,

    /// Transport protocol to use.
    #[arg(long, default_value = "stdio", value_parser = ["stdio", "sse"])]
    transport: String,

    /// Port for SSE transport (ignored for stdio).
    #[arg(long, default_value_t = 3179)]
    port: u16,

    /// Log level.
    #[arg(long, default_value = "info")]
    log_level: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .init();

    tracing::info!(
        repo = %args.repo,
        transport = %args.transport,
        "OmniContext MCP server starting"
    );

    // TODO: Initialize engine, start MCP server
    // For now, just validate args and exit cleanly
    let repo_path = std::path::Path::new(&args.repo);
    if !repo_path.exists() {
        anyhow::bail!("repository path does not exist: {}", args.repo);
    }

    tracing::info!("MCP server ready");

    Ok(())
}
