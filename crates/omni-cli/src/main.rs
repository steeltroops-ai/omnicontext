//! OmniContext CLI.
//!
//! Command-line interface for indexing, searching, and managing
//! OmniContext indexes.

use anyhow::Result;
use clap::{Parser, Subcommand};

/// OmniContext - Universal Code Context Engine
#[derive(Parser, Debug)]
#[command(name = "omnicontext", version, about = "Universal code context engine for AI coding agents")]
struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    command: Commands,

    /// Log level.
    #[arg(long, global = true, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Index a repository.
    Index {
        /// Path to the repository root.
        #[arg(default_value = ".")]
        path: String,

        /// Force full reindex, ignoring cached state.
        #[arg(long)]
        force: bool,
    },

    /// Search the indexed codebase.
    Search {
        /// Search query (natural language or keywords).
        query: String,

        /// Maximum number of results.
        #[arg(short, long, default_value_t = 10)]
        limit: usize,

        /// Filter by programming language.
        #[arg(long)]
        language: Option<String>,

        /// Filter by code kind (function, class, trait, etc.).
        #[arg(long)]
        kind: Option<String>,
    },

    /// Show engine status and index statistics.
    Status {
        /// Path to the repository root.
        #[arg(default_value = ".")]
        path: String,

        /// Show files that failed to parse.
        #[arg(long)]
        failed: bool,
    },

    /// Start the MCP server for AI agent integration.
    Mcp {
        /// Path to the repository root.
        #[arg(long, default_value = ".")]
        repo: String,

        /// Transport protocol.
        #[arg(long, default_value = "stdio", value_parser = ["stdio", "sse"])]
        transport: String,

        /// Port for SSE transport.
        #[arg(long, default_value_t = 3179)]
        port: u16,
    },

    /// Manage configuration.
    Config {
        /// Show current effective configuration.
        #[arg(long)]
        show: bool,

        /// Initialize a .omnicontext/config.toml in the current directory.
        #[arg(long)]
        init: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();

    match cli.command {
        Commands::Index { path, force } => {
            tracing::info!(path = %path, force, "indexing repository");
            // TODO: Initialize engine, run indexing
            println!("Indexing: {path} (force={force})");
        }
        Commands::Search { query, limit, language, kind } => {
            tracing::info!(query = %query, limit, "searching");
            // TODO: Initialize engine, run search, display results
            println!("Searching for: \"{query}\" (limit={limit})");
            let _ = (language, kind);
        }
        Commands::Status { path, failed } => {
            tracing::info!(path = %path, "showing status");
            // TODO: Load engine, display status
            println!("Status for: {path} (show_failed={failed})");
        }
        Commands::Mcp { repo, transport, port } => {
            tracing::info!(repo = %repo, transport = %transport, port, "starting MCP server");
            // TODO: Delegate to omni-mcp logic
            println!("MCP server: repo={repo} transport={transport} port={port}");
        }
        Commands::Config { show, init } => {
            if init {
                // TODO: Generate default config file
                println!("Initialized .omnicontext/config.toml");
            }
            if show {
                // TODO: Load and display effective config
                println!("Configuration: (not yet implemented)");
            }
        }
    }

    Ok(())
}
