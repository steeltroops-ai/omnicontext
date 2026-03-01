//! `OmniContext` Daemon -- Persistent background engine with IPC.
//!
//! Provides a long-running process that keeps the `OmniContext` engine
//! hot in memory and exposes a JSON-RPC interface over named pipes
//! (Windows) or Unix domain sockets (Linux/macOS).
//!
//! ## Architecture
//!
//! The daemon owns a single `Engine` instance and multiplexes client
//! requests through a `tokio::sync::Mutex`. Each connected client
//! (typically the VS Code extension) sends JSON-RPC requests and
//! receives JSON-RPC responses over the pipe.
//!
//! ## Usage
//!
//! ```text
//! # Start the daemon (auto-indexes if needed)
//! omnicontext-daemon --repo /path/to/repo
//!
//! # The VS Code extension connects automatically via named pipe
//! ```

mod ipc;
mod protocol;

use anyhow::Result;
use clap::Parser;

/// `OmniContext` Daemon -- persistent background engine
#[derive(Parser, Debug)]
#[command(
    name = "omnicontext-daemon",
    version,
    about = "Persistent background engine with IPC interface"
)]
struct Args {
    /// Path to the repository to serve.
    #[arg(long, default_value = ".")]
    repo: String,

    /// Log level (trace, debug, info, warn, error).
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Skip automatic indexing on startup.
    #[arg(long)]
    no_auto_index: bool,

    /// Named pipe/socket name override.
    #[arg(long)]
    pipe_name: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

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

    tracing::info!(repo = %repo_path.display(), "initializing daemon engine");

    // Initialize the core engine
    let mut engine = omni_core::Engine::new(&repo_path)?;

    // Auto-index if needed
    if !args.no_auto_index {
        let status = engine.status()?;
        if status.files_indexed == 0 {
            tracing::info!("no existing index, running auto-index...");
            let start = std::time::Instant::now();
            match engine.run_index().await {
                Ok(result) => {
                    tracing::info!(
                        files = result.files_processed,
                        chunks = result.chunks_created,
                        symbols = result.symbols_extracted,
                        elapsed_ms = {
                            #[allow(clippy::cast_possible_truncation)]
                            let ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                            ms
                        },
                        "auto-index complete"
                    );
                }
                Err(e) => {
                    tracing::warn!(error = %e, "auto-index failed");
                }
            }
        } else {
            tracing::info!(files = status.files_indexed, "using existing index");
        }
    }

    // Derive pipe name from repo path hash
    let pipe_name = args
        .pipe_name
        .unwrap_or_else(|| ipc::default_pipe_name(&repo_path));

    tracing::info!(pipe = %pipe_name, "starting IPC server");

    // Run the IPC server
    ipc::serve(engine, &pipe_name).await?;

    tracing::info!("daemon shut down");
    Ok(())
}
