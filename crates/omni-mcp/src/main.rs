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

// Suppress common test-helper lints that are intentional in unit tests
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::float_cmp,
        clippy::field_reassign_with_default,
        clippy::ignore_without_reason,
    )
)]

mod tools;

use anyhow::Result;
use clap::Parser;
use rmcp::ServiceExt;
use std::time::Duration;

/// `OmniContext` MCP Server
#[derive(Parser, Debug)]
#[command(
    name = "omnicontext-mcp",
    version,
    about = "MCP server for AI code intelligence"
)]
struct Args {
    /// Path to the repository to serve.
    ///
    /// Resolution priority (highest wins):
    ///   1. This flag if explicitly set to something other than "."
    ///   2. `OMNICONTEXT_REPO` environment variable
    ///   3. `--cwd` flag (if provided)
    ///   4. Process working directory (the "." default)
    #[arg(long, default_value = ".")]
    repo: String,

    /// Override the working directory used when `--repo .` is the default.
    ///
    /// External AI agent launchers (Antigravity, Claude Desktop, Cursor)
    /// can pass the active workspace root here so the "." default resolves
    /// correctly even when the agent's spawned process inherits a different cwd.
    #[arg(long)]
    cwd: Option<String>,

    /// Log level (trace, debug, info, warn, error).
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Skip automatic indexing on startup.
    /// By default, the server indexes the repo if no index exists.
    #[arg(long)]
    no_auto_index: bool,
}

#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing -- write to stderr so stdout stays clean for JSON-RPC
    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .with_writer(std::io::stderr)
        .init();

    // Resolve repository path with priority order:
    //   1. --repo if the caller explicitly passed something other than the default "."
    //   2. OMNICONTEXT_REPO environment variable (set by external agent launchers)
    //   3. --cwd flag (overrides the cwd used to resolve ".")
    //   4. The bare "." default, which resolves against the process cwd
    let repo_str: String = if args.repo != "." {
        // Explicit --repo wins unconditionally.
        args.repo.clone()
    } else if let Ok(env_repo) = std::env::var("OMNICONTEXT_REPO") {
        tracing::info!(
            env_repo = %env_repo,
            "using OMNICONTEXT_REPO environment variable for repository path"
        );
        env_repo
    } else if let Some(ref cwd_override) = args.cwd {
        tracing::info!(
            cwd = %cwd_override,
            "using --cwd override to resolve repository path"
        );
        cwd_override.clone()
    } else {
        args.repo.clone()
    };

    let repo_path = std::path::PathBuf::from(&repo_str)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&repo_str));

    // Fail fast on the installer placeholder -- the user hasn't configured a real path.
    if repo_str == "REPLACE_WITH_YOUR_REPO_PATH" {
        anyhow::bail!(
            "repository path is still the install placeholder. \
             Set --repo to your actual project path, or install the VS Code extension \
             which auto-configures this for you."
        );
    }

    if !repo_path.exists() {
        anyhow::bail!("repository path does not exist: {repo_str}");
    }

    // Defensive check: refuse to start if the resolved path is clearly NOT
    // a source code repository. This catches the case where an AI agent
    // launcher spawns the MCP process with its own install dir as the cwd,
    // causing --repo "." to resolve to a non-project directory.
    let repo_str_lower = repo_path.to_string_lossy().to_lowercase();
    let suspicious = repo_str_lower.contains("program files")
        || repo_str_lower.contains("appdata")
        || repo_str_lower.contains("programs\\antigravity")
        || repo_str_lower.contains("programs/antigravity")
        || repo_str_lower.contains(".vscode")
        || repo_str_lower.contains(".gemini");

    if suspicious {
        anyhow::bail!(
            "resolved repository path looks like an application directory, not a source \
             code project: {}. The MCP server was likely launched with --repo \".\" from \
             the wrong working directory. Pass --repo <path> explicitly or set the \
             OMNICONTEXT_REPO environment variable.",
            repo_path.display()
        );
    }

    // Heuristic: check if the path has any common project markers.
    // Only warn (don't bail) since the directory might be a valid but new project.
    let has_git = repo_path.join(".git").exists();
    let has_cargo = repo_path.join("Cargo.toml").exists();
    let has_package = repo_path.join("package.json").exists();
    let has_pyproject = repo_path.join("pyproject.toml").exists();
    let has_go_mod = repo_path.join("go.mod").exists();
    let has_makefile = repo_path.join("Makefile").exists();
    let has_readme = repo_path.join("README.md").exists();
    let has_src = repo_path.join("src").exists();
    let looks_like_project = has_git
        || has_cargo
        || has_package
        || has_pyproject
        || has_go_mod
        || has_makefile
        || has_readme
        || has_src;

    if !looks_like_project {
        tracing::warn!(
            resolved_path = %repo_path.display(),
            original_arg = %repo_str,
            "resolved repository path has no recognizable project markers (.git, Cargo.toml, \
             package.json, etc.). If you see unexpected results, pass --repo <path> explicitly."
        );
    }

    tracing::info!(
        repo = %repo_path.display(),
        "initializing OmniContext engine"
    );

    // Initialize the core engine with a timeout and degraded fallback.
    // This prevents MCP startup from hanging indefinitely during model init.
    let (mut engine, degraded_mode) = initialize_engine_with_fallback(repo_path.clone()).await?;
    if degraded_mode {
        tracing::warn!(
            "MCP started in degraded mode (OMNI_SKIP_MODEL_DOWNLOAD=1). Semantic embeddings and reranking are temporarily disabled."
        );
    }

    // Auto-index: if the index is empty and auto-index is not disabled,
    // run a full index before starting the MCP server.
    // This ensures AI agents always connect to a ready engine.
    if !args.no_auto_index {
        let status = engine.status()?;
        if status.files_indexed == 0 {
            tracing::info!("no existing index found, running auto-index...");
            let start = std::time::Instant::now();
            match engine.run_index(false).await {
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

async fn initialize_engine_with_fallback(
    repo_path: std::path::PathBuf,
) -> Result<(omni_core::Engine, bool)> {
    let normal_repo = repo_path.clone();
    let normal_init = tokio::task::spawn_blocking(move || omni_core::Engine::new(&normal_repo));

    match tokio::time::timeout(Duration::from_secs(90), normal_init).await {
        Ok(joined) => match joined {
            Ok(Ok(engine)) => {
                tracing::info!("engine initialized in normal mode");
                return Ok((engine, false));
            }
            Ok(Err(err)) => {
                let message = err.to_string().to_lowercase();
                let recoverable = message.contains("onnx")
                    || message.contains("model")
                    || message.contains("download")
                    || message.contains("tokenizer");
                if !recoverable {
                    return Err(err.into());
                }
                tracing::warn!(
                    error = %err,
                    "normal engine init failed with recoverable model error; retrying degraded mode"
                );
            }
            Err(join_err) => {
                tracing::warn!(
                    error = %join_err,
                    "engine init task join failed; retrying degraded mode"
                );
            }
        },
        Err(_) => {
            tracing::warn!("engine init timed out after 90s; retrying degraded mode");
        }
    }

    std::env::set_var("OMNI_SKIP_MODEL_DOWNLOAD", "1");
    let degraded_repo = repo_path.clone();
    let degraded_init = tokio::task::spawn_blocking(move || omni_core::Engine::new(&degraded_repo));
    let degraded = tokio::time::timeout(Duration::from_secs(30), degraded_init)
        .await
        .map_err(|_| anyhow::anyhow!("degraded engine init timed out after 30s"))?
        .map_err(|e| anyhow::anyhow!("degraded init task failed: {e}"))??;

    Ok((degraded, true))
}
