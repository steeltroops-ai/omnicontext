//! `OmniContext` CLI.
//!
//! Command-line interface for indexing, searching, and managing
//! `OmniContext` indexes.

mod orchestrator;

use std::time::Instant;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// `OmniContext` - Universal Code Context Engine
#[derive(Parser, Debug)]
#[command(
    name = "omnicontext",
    version,
    about = "Universal code context engine for AI coding agents"
)]
struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    command: Commands,

    /// Log level.
    #[arg(long, global = true, default_value = "info")]
    log_level: String,

    /// Output results as JSON (for scripting and CI/CD).
    #[arg(long, global = true)]
    json: bool,
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

    /// Retry embedding chunks that failed during indexing.
    Embed {
        /// Path to the repository root.
        #[arg(default_value = ".")]
        path: String,

        /// Only retry chunks without embeddings.
        #[arg(long)]
        retry_failed: bool,
    },

    /// Show engine status and index statistics.
    Status {
        /// Path to the repository root.
        #[arg(default_value = ".")]
        path: String,
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

    /// Initial setup and maintenance tasks.
    Setup {
        /// Type of setup task.
        #[command(subcommand)]
        action: SetupAction,
    },

    /// Auto-detect installed IDEs and inject MCP server configuration.
    Autopilot {
        /// Only configure a specific IDE (e.g., "cursor", "vscode", "claude").
        #[arg(long)]
        ide: Option<String>,

        /// Show what would be configured without making changes.
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate project manifest files (`CLAUDE.md` / `.context_map.json`).
    Manifest {
        /// Output format: "claude", "json", or "both".
        #[arg(long, default_value = "claude")]
        format: String,

        /// Write output to file(s) in repo root instead of stdout.
        #[arg(long)]
        write: bool,

        /// Path to the repository root.
        #[arg(default_value = ".")]
        path: String,
    },
}

#[derive(Subcommand, Debug, Clone, Copy)]
enum SetupAction {
    /// Download and cache the embedding model.
    ModelDownload {
        /// Force re-download even if already cached.
        #[arg(long)]
        force: bool,
    },
    /// Show the status of the embedding model.
    ModelStatus,
    /// Auto-wire `OmniContext` into every detected AI IDE and agent.
    ///
    /// Injects a single universal `omnicontext` MCP server entry using
    /// `--repo .` into all installed IDEs (Claude Desktop, Cursor, Windsurf,
    /// VS Code, Cline, Continue.dev, Zed, `Kiro`, `PearAI`, Claude Code CLI).
    /// Purges legacy project-specific entries automatically.
    All {
        /// Show what would be configured without writing any files.
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();

    match cli.command {
        Commands::Index { path, force } => {
            cmd_index(&path, force, cli.json).await?;
        }
        Commands::Search {
            query,
            limit,
            language,
            kind,
        } => {
            cmd_search(
                &query,
                limit,
                language.as_deref(),
                kind.as_deref(),
                cli.json,
            )?;
        }
        Commands::Embed { path, retry_failed } => {
            cmd_embed(&path, retry_failed, cli.json)?;
        }
        Commands::Status { path } => {
            cmd_status(&path, cli.json)?;
        }
        Commands::Mcp {
            repo,
            transport,
            port,
        } => {
            cmd_mcp(&repo, &transport, port).await?;
        }
        Commands::Config { show, init } => {
            cmd_config(show, init)?;
        }
        Commands::Setup { action } => {
            cmd_setup(action, cli.json)?;
        }
        Commands::Autopilot { ide, dry_run } => {
            cmd_autopilot(ide.as_deref(), dry_run)?;
        }
        Commands::Manifest {
            format,
            write,
            path,
        } => {
            cmd_manifest(&path, &format, write, cli.json)?;
        }
    }

    Ok(())
}

/// Index a repository.
async fn cmd_index(path: &str, force: bool, json: bool) -> Result<()> {
    let repo_path = std::path::PathBuf::from(path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(path));

    if !json {
        println!("OmniContext - Indexing: {}", repo_path.display());
        println!("---");
    }

    let start = Instant::now();

    let mut engine = omni_core::Engine::new(&repo_path)?;
    let result = engine.run_index(force).await?;

    let elapsed = start.elapsed();

    if json {
        let output = serde_json::json!({
            "status": "ok",
            "elapsed_ms": elapsed.as_millis(),
            "files_processed": result.files_processed,
            "files_failed": result.files_failed,
            "chunks_created": result.chunks_created,
            "symbols_extracted": result.symbols_extracted,
            "embeddings_generated": result.embeddings_generated,
            "embedding_failures": result.embedding_failures,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  Indexing complete in {:.2}s", elapsed.as_secs_f64());
        println!("  Files processed:  {}", result.files_processed);
        println!("  Files failed:     {}", result.files_failed);
        println!("  Chunks created:   {}", result.chunks_created);
        println!("  Symbols found:    {}", result.symbols_extracted);
        println!("  Embeddings:       {}", result.embeddings_generated);
        println!("  Embed flush errs: {}", result.embedding_failures);
    }

    // Persist on shutdown
    engine.shutdown()?;

    Ok(())
}

/// Search the indexed codebase.
fn cmd_search(
    query: &str,
    limit: usize,
    _language: Option<&str>,
    _kind: Option<&str>,
    json: bool,
) -> Result<()> {
    let repo_path = std::env::current_dir()?;
    let engine = omni_core::Engine::new(&repo_path)?;

    let start = Instant::now();
    let results = engine.search(query, limit)?;
    let elapsed = start.elapsed();

    if json {
        let output = serde_json::json!({
            "query": query,
            "elapsed_ms": elapsed.as_millis(),
            "count": results.len(),
            "results": results.iter().map(|r| serde_json::json!({
                "file": r.file_path.display().to_string(),
                "score": r.score,
                "kind": format!("{:?}", r.chunk.kind),
                "symbol": r.chunk.symbol_path,
                "line_start": r.chunk.line_start,
                "line_end": r.chunk.line_end,
                "content": r.chunk.content,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    if results.is_empty() {
        println!("No results found for: \"{query}\"");
        println!();
        println!("Tip: Make sure you've run `omnicontext index .` first.");
        return Ok(());
    }

    println!(
        "Results for \"{}\" ({} found, {:.1}ms):",
        query,
        results.len(),
        elapsed.as_secs_f64() * 1000.0
    );
    println!();

    for (i, result) in results.iter().enumerate() {
        let path = &result.file_path;
        let score = result.score;
        let kind = format!("{:?}", result.chunk.kind);
        let symbol = &result.chunk.symbol_path;
        let lines = format!("L{}-L{}", result.chunk.line_start, result.chunk.line_end);

        println!("  {}. {} (score: {:.4})", i + 1, path.display(), score);
        println!("     {kind} {symbol} [{lines}]");

        // Print a preview of the content (first 2 lines)
        let preview: String = result
            .chunk
            .content
            .lines()
            .take(2)
            .map(|l| format!("     | {l}"))
            .collect::<Vec<_>>()
            .join("\n");
        if !preview.is_empty() {
            println!("{preview}");
        }
        println!();
    }

    Ok(())
}

/// Retry embedding chunks that failed during indexing.
fn cmd_embed(path: &str, retry_failed: bool, json: bool) -> Result<()> {
    let repo_path = std::path::PathBuf::from(path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(path));

    if !json {
        println!("OmniContext - Retrying Failed Embeddings");
        println!("  Repository: {}", repo_path.display());
        println!("---");
    }

    let mut engine = omni_core::Engine::new(&repo_path)?;

    if retry_failed {
        let start = Instant::now();
        let result = engine.retry_failed_embeddings()?;
        let elapsed = start.elapsed();

        if json {
            let output = serde_json::json!({
                "status": "ok",
                "elapsed_ms": elapsed.as_millis(),
                "total_attempted": result.total_attempted,
                "successful": result.successful,
                "failed": result.failed,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!();
            println!("  Retry complete in {:.2}s", elapsed.as_secs_f64());
            println!("  Chunks attempted:  {}", result.total_attempted);
            println!("  Successful:        {}", result.successful);
            println!("  Failed:            {}", result.failed);

            if result.total_attempted == 0 {
                println!();
                println!("  ✓ All chunks already have embeddings!");
            } else if result.failed > 0 {
                println!();
                println!("  ⚠ Some chunks still failed to embed.");
                println!("    Check logs for details.");
            } else {
                println!();
                println!("  ✓ All failed chunks successfully embedded!");
            }
        }

        // Persist on shutdown
        engine.shutdown()?;
    } else if !json {
        println!();
        println!("Usage: omnicontext embed --retry-failed");
        println!();
        println!("This command retries embedding chunks that failed during indexing.");
    }

    Ok(())
}

/// Show engine status and index statistics.
fn cmd_status(path: &str, json: bool) -> Result<()> {
    let repo_path = std::path::PathBuf::from(path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(path));

    let engine = omni_core::Engine::new(&repo_path)?;
    let status = engine.status()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&status)?);
        return Ok(());
    }

    println!("OmniContext Status");
    println!("---");
    println!("  Repository:       {}", status.repo_path);
    println!("  Data directory:   {}", status.data_dir);
    println!("  Search mode:      {}", status.search_mode);
    println!();
    println!("  Files indexed:    {}", status.files_indexed);
    println!("  Chunks indexed:   {}", status.chunks_indexed);
    println!("  Symbols indexed:  {}", status.symbols_indexed);
    println!("  Vectors indexed:  {}", status.vectors_indexed);
    println!();
    println!("  Dep edges (db):   {}", status.dep_edges);
    println!("  Graph nodes:      {}", status.graph_nodes);
    println!("  Graph edges:      {}", status.graph_edges);
    if status.has_cycles {
        println!("  [!] Circular dependencies detected");
    }

    Ok(())
}

/// Start the MCP server by launching the dedicated omnicontext-mcp binary.
async fn cmd_mcp(repo: &str, _transport: &str, _port: u16) -> Result<()> {
    let repo_path = std::path::PathBuf::from(repo)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(repo));

    eprintln!("OmniContext MCP Server starting...");
    eprintln!("  Repository: {}", repo_path.display());

    // Try to find the MCP binary next to the current executable
    let current_exe = std::env::current_exe()?;
    let mcp_binary = current_exe.parent().map_or_else(
        || std::path::PathBuf::from("omnicontext-mcp"),
        |p| p.join("omnicontext-mcp"),
    );

    let status = tokio::process::Command::new(&mcp_binary)
        .arg("--repo")
        .arg(&repo_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await;

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => anyhow::bail!("MCP server exited with code: {s}"),
        Err(e) => {
            eprintln!("Failed to launch MCP server binary: {e}");
            eprintln!();
            eprintln!("The MCP server is shipped as a separate binary: omnicontext-mcp");
            eprintln!("Install it with: cargo install --path crates/omni-mcp");
            anyhow::bail!("MCP binary not found: {}", mcp_binary.display());
        }
    }
}

/// Manage configuration.
fn cmd_config(show: bool, init: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if init {
        let config_dir = cwd.join(".omnicontext");
        let config_file = config_dir.join("config.toml");

        if config_file.exists() {
            println!("Configuration already exists: {}", config_file.display());
        } else {
            std::fs::create_dir_all(&config_dir)?;
            let default_config = r#"# OmniContext Configuration
# See https://github.com/omnicontext/omnicontext for documentation.

[indexing]
# exclude_patterns = [".git", "node_modules", "target", "__pycache__"]
# max_file_size = 1048576  # 1 MB
# max_chunk_tokens = 512

[search]
# default_limit = 10
# rrf_k = 60
# token_budget = 8192

[embedding]
# dimensions = 384

[watcher]
# debounce_ms = 100
# poll_interval_secs = 300
"#;
            std::fs::write(&config_file, default_config)?;
            println!("Created: {}", config_file.display());
        }
    }

    if show {
        let config = omni_core::Config::load(&cwd)?;
        println!("Effective configuration for: {}", cwd.display());
        println!();

        println!("[indexing]");
        println!(
            "  exclude_patterns = {:?}",
            config.indexing.exclude_patterns
        );
        println!("  max_file_size = {}", config.indexing.max_file_size);
        println!("  max_chunk_tokens = {}", config.indexing.max_chunk_tokens);
        println!(
            "  parse_concurrency = {}",
            config.indexing.parse_concurrency
        );
        println!();

        println!("[search]");
        println!("  default_limit = {}", config.search.default_limit);
        println!("  rrf_k = {}", config.search.rrf_k);
        println!("  token_budget = {}", config.search.token_budget);
        println!();

        println!("[embedding]");
        println!("  dimensions = {}", config.embedding.dimensions);
        println!("  model_path = {}", config.embedding.model_path.display());
    }

    if !show && !init {
        println!("Usage: omnicontext config --init   Create default config");
        println!("       omnicontext config --show   Show effective config");
    }

    Ok(())
}

/// Handle setup and maintenance tasks.
fn cmd_setup(action: SetupAction, json: bool) -> Result<()> {
    match action {
        SetupAction::ModelDownload { force } => {
            let spec = omni_core::embedder::model_manager::resolve_model_spec();
            if !force && omni_core::embedder::model_manager::is_model_ready(spec) {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "status": "ok",
                            "model": spec.name,
                            "message": "model already cached"
                        })
                    );
                } else {
                    println!("Embedding model '{}' is already cached.", spec.name);
                }
                return Ok(());
            }

            if !json {
                println!("Preparing embedding model: {}", spec.name);
            }

            omni_core::embedder::model_manager::ensure_model(spec)?;

            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "ok",
                        "model": spec.name,
                        "message": "download complete"
                    })
                );
            } else {
                println!();
                println!("Model setup complete.");
            }
        }
        SetupAction::ModelStatus => {
            let spec = omni_core::embedder::model_manager::resolve_model_spec();
            let ready = omni_core::embedder::model_manager::is_model_ready(spec);
            let path = omni_core::embedder::model_manager::model_path(spec);
            let size = if ready {
                std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
            } else {
                0
            };

            if json {
                let output = serde_json::json!({
                    "model_name": spec.name,
                    "model_ready": ready,
                    "model_path": path.display().to_string(),
                    "model_size_bytes": size,
                    "dimensions": spec.dimensions,
                    "max_seq_length": spec.max_seq_length,
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("Embedding Model Status");
                println!("---");
                println!("  Name:             {}", spec.name);
                println!("  Ready:            {}", if ready { "Yes" } else { "No" });
                println!("  Path:             {}", path.display());
                if ready {
                    println!("  Size:             {} MB", size / 1024 / 1024);
                } else {
                    println!(
                        "  Expected Size:    ~{} MB",
                        spec.approx_size_bytes / 1024 / 1024
                    );
                }
                println!("  Dimensions:       {}", spec.dimensions);
                println!("  Max Seq Length:   {}", spec.max_seq_length);
            }
        }
        SetupAction::All { dry_run } => {
            cmd_setup_all(dry_run, json)?;
        }
    }

    Ok(())
}

/// Run the Universal IDE Orchestrator (`setup --all`).
fn cmd_setup_all(dry_run: bool, json: bool) -> Result<()> {
    let result = orchestrator::orchestrate(dry_run)?;

    if json {
        let output = serde_json::json!({
            "dry_run": dry_run,
            "mcp_binary": result.mcp_binary.display().to_string(),
            "detected": result.detected(),
            "configured": result.configured(),
            "total_purged": result.total_purged(),
            "results": result.results.iter().map(|r| {
                let (status_str, detail) = match &r.status {
                    orchestrator::IdeStatus::Configured       => ("configured", String::new()),
                    orchestrator::IdeStatus::AlreadyCurrent   => ("already_current", String::new()),
                    orchestrator::IdeStatus::NotInstalled     => ("not_installed", String::new()),
                    orchestrator::IdeStatus::PermissionDenied(e) => ("permission_denied", e.clone()),
                    orchestrator::IdeStatus::Error(e)         => ("error", e.clone()),
                };
                serde_json::json!({
                    "ide": r.name,
                    "status": status_str,
                    "detail": detail,
                    "purged": r.purged,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        orchestrator::print_orchestration_matrix(&result, dry_run);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Phase D: Autopilot IDE Config
// ---------------------------------------------------------------------------

/// Supported IDE configuration target.
struct IdeTarget {
    name: &'static str,
    /// Lowercase match key for --ide filter.
    key: &'static str,
    /// Config file path (with environment expansion).
    config_path: std::path::PathBuf,
    /// JSON key that holds the MCP server map.
    server_key: &'static str,
}

#[allow(clippy::too_many_lines, clippy::vec_init_then_push)]
fn detect_installed_ides(filter: Option<&str>) -> Vec<IdeTarget> {
    let home = dirs::home_dir().unwrap_or_default();
    #[cfg(windows)]
    let appdata = std::env::var("APPDATA").map_or_else(
        |_| home.join("AppData").join("Roaming"),
        std::path::PathBuf::from,
    );
    #[cfg(not(windows))]
    let appdata = home.join(".config"); // ~/.config on Linux/macOS

    let mut targets = Vec::new();

    // Claude Desktop
    #[cfg(windows)]
    targets.push(IdeTarget {
        name: "Claude Desktop",
        key: "claude",
        config_path: appdata.join("Claude").join("claude_desktop_config.json"),
        server_key: "mcpServers",
    });
    #[cfg(not(windows))]
    targets.push(IdeTarget {
        name: "Claude Desktop",
        key: "claude",
        config_path: appdata.join("Claude").join("claude_desktop_config.json"),
        server_key: "mcpServers",
    });

    // Cursor
    #[cfg(windows)]
    targets.push(IdeTarget {
        name: "Cursor",
        key: "cursor",
        config_path: appdata
            .join("Cursor")
            .join("User")
            .join("globalStorage")
            .join("cursor.mcp")
            .join("config.json"),
        server_key: "mcpServers",
    });
    #[cfg(not(windows))]
    targets.push(IdeTarget {
        name: "Cursor",
        key: "cursor",
        config_path: home.join(".cursor").join("mcp.json"),
        server_key: "mcpServers",
    });

    // VS Code
    #[cfg(windows)]
    targets.push(IdeTarget {
        name: "VS Code",
        key: "vscode",
        config_path: appdata.join("Code").join("User").join("mcp.json"),
        server_key: "servers",
    });
    #[cfg(not(windows))]
    targets.push(IdeTarget {
        name: "VS Code",
        key: "vscode",
        config_path: appdata.join("Code").join("User").join("mcp.json"),
        server_key: "servers",
    });

    // Windsurf
    #[cfg(windows)]
    targets.push(IdeTarget {
        name: "Windsurf",
        key: "windsurf",
        config_path: appdata
            .join("Windsurf")
            .join("User")
            .join("globalStorage")
            .join("codeium.windsurf")
            .join("mcp_config.json"),
        server_key: "mcpServers",
    });

    // Zed
    targets.push(IdeTarget {
        name: "Zed",
        key: "zed",
        config_path: home.join(".config").join("zed").join("settings.json"),
        server_key: "context_servers",
    });

    // Kiro
    targets.push(IdeTarget {
        name: "Kiro",
        key: "kiro",
        config_path: home.join(".kiro").join("settings").join("mcp.json"),
        server_key: "mcpServers",
    });

    // Continue.dev
    targets.push(IdeTarget {
        name: "Continue.dev",
        key: "continue",
        config_path: home.join(".continue").join("config.json"),
        server_key: "mcpServers",
    });

    // Cline
    targets.push(IdeTarget {
        name: "Cline",
        key: "cline",
        config_path: home.join(".cline").join("mcp_settings.json"),
        server_key: "mcpServers",
    });

    // PearAI
    #[cfg(windows)]
    targets.push(IdeTarget {
        name: "PearAI",
        key: "pearai",
        config_path: appdata.join("PearAI").join("User").join("mcp.json"),
        server_key: "mcpServers",
    });

    // Antigravity (VS Code fork)
    #[cfg(windows)]
    targets.push(IdeTarget {
        name: "Antigravity",
        key: "antigravity",
        config_path: appdata.join("Antigravity").join("User").join("mcp.json"),
        server_key: "servers",
    });
    #[cfg(not(windows))]
    targets.push(IdeTarget {
        name: "Antigravity",
        key: "antigravity",
        config_path: appdata.join("Antigravity").join("User").join("mcp.json"),
        server_key: "servers",
    });

    // Filter by --ide if specified
    if let Some(f) = filter {
        let f_lower = f.to_lowercase();
        targets.retain(|t| t.key == f_lower || t.name.to_lowercase().contains(&f_lower));
    }

    // Only keep IDEs whose config directory exists (or whose config file exists)
    targets.retain(|t| t.config_path.parent().is_some_and(std::path::Path::exists));

    targets
}

fn find_mcp_binary() -> Result<std::path::PathBuf> {
    let current_exe = std::env::current_exe()?;
    let dir = current_exe.parent().unwrap_or(std::path::Path::new("."));

    // Try omnicontext-mcp next to the current exe
    let mcp = dir.join("omnicontext-mcp");
    if mcp.exists() {
        return Ok(mcp);
    }

    // Try with .exe on Windows
    #[cfg(windows)]
    {
        let mcp_exe = dir.join("omnicontext-mcp.exe");
        if mcp_exe.exists() {
            return Ok(mcp_exe);
        }
    }

    // Fall back to the current exe itself (the CLI delegates to MCP)
    Ok(current_exe)
}

fn cmd_autopilot(ide_filter: Option<&str>, dry_run: bool) -> Result<()> {
    let repo_path = std::env::current_dir()?;
    let mcp_binary = find_mcp_binary()?;
    let repo_hash = omni_core::normalize_repo_hash(&repo_path.display().to_string());
    let entry_key = format!("omnicontext-{}", &repo_hash[..6.min(repo_hash.len())]);

    let server_entry = serde_json::json!({
        "command": mcp_binary.display().to_string(),
        "args": ["--repo", repo_path.display().to_string()],
        "env": { "OMNICONTEXT_REPO": repo_path.display().to_string() }
    });

    let ides = detect_installed_ides(ide_filter);

    if ides.is_empty() {
        println!("No supported IDEs detected.");
        if ide_filter.is_some() {
            println!("Try without --ide to see all available IDEs.");
        }
        return Ok(());
    }

    println!(
        "OmniContext Autopilot - Configuring MCP server for: {}",
        repo_path.display()
    );
    println!("---");

    for ide in &ides {
        if dry_run {
            println!(
                "[DRY RUN] Would configure {}: {}",
                ide.name,
                ide.config_path.display()
            );
        } else {
            match merge_mcp_config(&ide.config_path, ide.server_key, &entry_key, &server_entry) {
                Ok(()) => println!("  Configured {}", ide.name),
                Err(e) => println!("  Failed to configure {}: {e}", ide.name),
            }
        }
    }

    if dry_run {
        println!("\nRun without --dry-run to apply changes.");
    } else {
        println!("\nDone! Restart your IDE(s) to activate OmniContext.");
    }

    Ok(())
}

fn merge_mcp_config(
    config_path: &std::path::Path,
    server_key: &str,
    entry_key: &str,
    server_entry: &serde_json::Value,
) -> Result<()> {
    // Read existing or start fresh
    let mut config: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(config_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Navigate to server key, create if missing
    // Handle nested keys like "powers.mcpServers"
    let parts: Vec<&str> = server_key.split('.').collect();
    let mut current = &mut config;
    for part in &parts {
        if current.get(part).is_none() {
            current[part] = serde_json::json!({});
        }
        current = current.get_mut(part).unwrap_or_else(|| unreachable!());
    }

    // Insert/update the entry
    current[entry_key] = server_entry.clone();

    // Write back with pretty-print
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let output = serde_json::to_string_pretty(&config)?;
    std::fs::write(config_path, output)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Phase E: Proactive Manifests
// ---------------------------------------------------------------------------

fn cmd_manifest(path: &str, format: &str, write_files: bool, json: bool) -> Result<()> {
    let repo_path = std::path::PathBuf::from(path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(path));

    let engine = omni_core::Engine::new(&repo_path)?;

    if format == "claude" || format == "both" {
        match engine.generate_claude_md() {
            Ok(content) => {
                if write_files {
                    let out_path = repo_path.join("CLAUDE.md");
                    std::fs::write(&out_path, &content)?;
                    if !json {
                        println!("Wrote {}", out_path.display());
                    }
                } else if json {
                    println!(
                        "{}",
                        serde_json::json!({ "format": "claude", "content": content })
                    );
                } else {
                    println!("{content}");
                }
            }
            Err(e) => eprintln!("Failed to generate CLAUDE.md: {e}"),
        }
    }

    if format == "json" || format == "both" {
        match engine.generate_context_map() {
            Ok(content) => {
                if write_files {
                    let out_path = repo_path.join(".context_map.json");
                    std::fs::write(&out_path, &content)?;
                    if !json {
                        println!("Wrote {}", out_path.display());
                    }
                } else if json {
                    println!(
                        "{}",
                        serde_json::json!({ "format": "json", "content": content })
                    );
                } else {
                    println!("{content}");
                }
            }
            Err(e) => eprintln!("Failed to generate .context_map.json: {e}"),
        }
    }

    Ok(())
}
