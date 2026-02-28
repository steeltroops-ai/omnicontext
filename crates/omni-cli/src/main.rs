//! OmniContext CLI.
//!
//! Command-line interface for indexing, searching, and managing
//! OmniContext indexes.

use std::time::Instant;

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
        Commands::Search { query, limit, language, kind } => {
            cmd_search(&query, limit, language.as_deref(), kind.as_deref(), cli.json)?;
        }
        Commands::Status { path } => {
            cmd_status(&path, cli.json)?;
        }
        Commands::Mcp { repo, transport, port } => {
            cmd_mcp(&repo, &transport, port).await?;
        }
        Commands::Config { show, init } => {
            cmd_config(show, init)?;
        }
    }

    Ok(())
}

/// Index a repository.
async fn cmd_index(path: &str, _force: bool, json: bool) -> Result<()> {
    let repo_path = std::path::PathBuf::from(path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(path));

    if !json {
        println!("OmniContext - Indexing: {}", repo_path.display());
        println!("---");
    }

    let start = Instant::now();

    let mut engine = omni_core::Engine::new(&repo_path)?;
    let result = engine.run_index().await?;

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
    }

    // Persist on shutdown
    engine.shutdown()?;

    Ok(())
}

/// Search the indexed codebase.
fn cmd_search(query: &str, limit: usize, _language: Option<&str>, _kind: Option<&str>, json: bool) -> Result<()> {
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
        println!("No results found for: \"{}\"", query);
        println!();
        println!("Tip: Make sure you've run `omnicontext index .` first.");
        return Ok(());
    }

    println!("Results for \"{}\" ({} found, {:.1}ms):", query, results.len(), elapsed.as_secs_f64() * 1000.0);
    println!();

    for (i, result) in results.iter().enumerate() {
        let path = &result.file_path;
        let score = result.score;
        let kind = format!("{:?}", result.chunk.kind);
        let symbol = &result.chunk.symbol_path;
        let lines = format!("L{}-L{}", result.chunk.line_start, result.chunk.line_end);

        println!("  {}. {} (score: {:.4})", i + 1, path.display(), score);
        println!("     {} {} [{}]", kind, symbol, lines);

        // Print a preview of the content (first 2 lines)
        let preview: String = result.chunk.content
            .lines()
            .take(2)
            .map(|l| format!("     | {}", l))
            .collect::<Vec<_>>()
            .join("\n");
        if !preview.is_empty() {
            println!("{}", preview);
        }
        println!();
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
    let mcp_binary = current_exe
        .parent()
        .map(|p| p.join("omnicontext-mcp"))
        .unwrap_or_else(|| std::path::PathBuf::from("omnicontext-mcp"));

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
        Ok(s) => anyhow::bail!("MCP server exited with code: {}", s),
        Err(e) => {
            eprintln!("Failed to launch MCP server binary: {}", e);
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
        println!("  exclude_patterns = {:?}", config.indexing.exclude_patterns);
        println!("  max_file_size = {}", config.indexing.max_file_size);
        println!("  max_chunk_tokens = {}", config.indexing.max_chunk_tokens);
        println!("  parse_concurrency = {}", config.indexing.parse_concurrency);
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
