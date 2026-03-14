//! MCP tool definitions for `OmniContext`.
//!
//! Each tool is annotated with `#[tool]` and exposes a code intelligence
//! capability to AI agents via the Model Context Protocol.
//!
//! ## Thread Safety
//!
//! `Engine` contains a `rusqlite::Connection` which is `!Sync`. We wrap it
//! in a `tokio::sync::Mutex` so that the MCP server can safely share it
//! across async tasks.

use std::sync::Arc;

use rmcp::{
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::Deserialize;
use tokio::sync::Mutex;

use omni_core::Engine;

// -----------------------------------------------------------------------
// Input validation constants (hardened per MCP_DAEMON_AUDIT.md)
// -----------------------------------------------------------------------

/// Maximum results any tool can return.
const MAX_LIMIT: usize = 200;
/// Maximum query length in characters.
const MAX_QUERY_LEN: usize = 10_000;
/// Maximum graph traversal depth for `blast_radius` / `call_graph` / `module_map`.
const MAX_GRAPH_DEPTH: usize = 20;
/// Maximum commit count for recent changes.
const MAX_COMMIT_COUNT: usize = 100;
/// Maximum plan text length for `audit_plan`.
const MAX_PLAN_LEN: usize = 500_000;

/// Clamp a limit value to a safe range.
fn clamp_limit(limit: Option<usize>, default: usize) -> usize {
    limit.unwrap_or(default).clamp(1, MAX_LIMIT)
}

/// Clamp a depth value to a safe range.
fn clamp_depth(depth: Option<usize>, default: usize) -> usize {
    depth.unwrap_or(default).clamp(1, MAX_GRAPH_DEPTH)
}

/// Validate a query string is non-empty and within bounds.
fn validate_query(query: &str) -> Result<(), McpError> {
    if query.trim().is_empty() {
        return Err(McpError::invalid_params("query must not be empty", None));
    }
    if query.len() > MAX_QUERY_LEN {
        return Err(McpError::invalid_params(
            format!("query exceeds maximum length of {MAX_QUERY_LEN} characters"),
            None,
        ));
    }
    Ok(())
}

/// Validate a path is safe (no parent traversal, no absolute paths pointing outside the repo).
fn validate_relative_path(path: &str) -> Result<(), McpError> {
    let p = std::path::Path::new(path);
    // Reject absolute paths
    if p.is_absolute() {
        return Err(McpError::invalid_params(
            "path must be relative to repository root",
            None,
        ));
    }
    // Reject parent traversal
    for component in p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(McpError::invalid_params(
                "path must not contain '..' components",
                None,
            ));
        }
    }
    Ok(())
}

/// Clamp `min_rerank_score` to [0.0, 1.0] range.
fn clamp_rerank_score(score: Option<f32>) -> Option<f32> {
    score.map(|s| s.clamp(0.0, 1.0))
}

// -----------------------------------------------------------------------
// Parameter structs for each tool
// -----------------------------------------------------------------------

/// Parameters for `search_code` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchCodeParams {
    /// Search query -- natural language or symbol name.
    pub query: String,
    /// Maximum number of results to return (default: 10).
    pub limit: Option<usize>,
    /// Minimum cross-encoder reranker score threshold (0.0-1.0). Chunks below
    /// this threshold are demoted. Higher values produce fewer, more precise
    /// results. Default: no threshold (all results returned).
    pub min_rerank_score: Option<f32>,
}

/// Parameters for `get_symbol` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetSymbolParams {
    /// Symbol name or fully qualified name to look up.
    pub name: String,
    /// Maximum number of results for prefix search (default: 5).
    pub limit: Option<usize>,
}

/// Parameters for `get_file_summary` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetFileSummaryParams {
    /// File path relative to repository root.
    pub path: String,
}

/// Parameters for `get_dependencies` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetDependenciesParams {
    /// Fully qualified symbol name.
    pub symbol: String,
    /// Direction: 'upstream', 'downstream', or 'both' (default: 'both').
    pub direction: Option<String>,
}

/// Parameters for `find_patterns` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindPatternsParams {
    /// Description of the pattern to find.
    pub pattern: String,
    /// Maximum number of examples to return (default: 5).
    pub limit: Option<usize>,
}

/// Parameters for `context_window` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ContextWindowParams {
    /// Search query -- natural language or symbol name.
    pub query: String,
    /// Maximum number of search results to consider (default: 20).
    pub limit: Option<usize>,
    /// Token budget for the context window (default: engine config).
    pub token_budget: Option<u32>,
    /// Minimum cross-encoder reranker score threshold (0.0-1.0). Chunks below
    /// this threshold are demoted. Higher values produce fewer, more precise
    /// results. Default: no threshold.
    pub min_rerank_score: Option<f32>,
    /// Whether to include architectural shadow headers on each chunk (default: from config).
    pub shadow_headers: Option<bool>,
}

/// Parameters for `get_module_map` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[allow(dead_code)]
pub struct GetModuleMapParams {
    /// Maximum depth for the module tree (default: no limit).
    pub max_depth: Option<usize>,
}

/// Parameters for `search_by_intent` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchByIntentParams {
    /// Natural language query describing what you're looking for.
    pub query: String,
    /// Maximum number of results to return (default: 10).
    pub limit: Option<usize>,
    /// Token budget for context assembly (default: engine config).
    pub token_budget: Option<u32>,
    /// Whether to include architectural shadow headers on each chunk (default: from config).
    pub shadow_headers: Option<bool>,
}

/// Parameters for `set_workspace` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetWorkspaceParams {
    /// Absolute path to the new repository root.
    pub path: String,
    /// Whether to auto-index the new workspace if no index exists (default: true).
    pub auto_index: Option<bool>,
}

/// Parameters for `get_blast_radius` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBlastRadiusParams {
    /// Fully qualified symbol name to analyze impact for.
    pub symbol: String,
    /// Maximum depth for transitive impact analysis (default: 5).
    pub max_depth: Option<usize>,
}

/// Parameters for `get_recent_changes` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetRecentChangesParams {
    /// Number of recent commits to analyze (default: 10).
    pub commit_count: Option<usize>,
    /// Whether to include the actual diff content (default: false).
    pub include_diff: Option<bool>,
}

/// Parameters for `get_call_graph` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCallGraphParams {
    /// Fully qualified symbol name to get the call graph for.
    pub symbol: String,
    /// Maximum depth for upstream/downstream traversal (default: 2).
    pub depth: Option<usize>,
    /// Whether to output as Mermaid diagram (default: false).
    pub mermaid: Option<bool>,
}

/// Parameters for `get_branch_context` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBranchContextParams {
    /// Whether to include diff hunks in the output (default: false).
    pub include_diffs: Option<bool>,
}

/// Parameters for `get_co_changes` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCoChangesParams {
    /// File path to find co-change partners for.
    pub file_path: String,
    /// Minimum number of shared commits to consider (default: 2).
    pub min_frequency: Option<usize>,
    /// Maximum results to return (default: 10).
    pub limit: Option<usize>,
}

/// Parameters for `audit_plan` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AuditPlanParams {
    /// The plan text to audit (markdown, numbered steps, bullet points).
    pub plan: String,
    /// Maximum dependency depth for blast radius analysis (default: 3).
    pub max_depth: Option<usize>,
}

/// Parameters for `generate_manifest` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GenerateManifestParams {
    /// Format: "claude" for CLAUDE.md, "json" for `.context_map.json`, "both" for both.
    #[serde(default = "default_manifest_format")]
    pub format: String,
}

fn default_manifest_format() -> String {
    "claude".to_string()
}

/// Parameters for `search_with_filter` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchWithFilterParams {
    /// Search query.
    pub query: String,
    /// Maximum results (default: 10).
    pub limit: Option<usize>,
    /// Minimum reranker score threshold (0.0–1.0).
    pub min_rerank_score: Option<f32>,
    /// Language to filter by, e.g. "rust", "python", "typescript".
    pub language: Option<String>,
    /// Glob pattern matched against file paths, e.g. "src/auth/**".
    pub path_glob: Option<String>,
    /// Only include files indexed after this ISO 8601 datetime.
    pub modified_after: Option<String>,
    /// Symbol type filter, e.g. "function", "class", "struct", "method".
    pub symbol_type: Option<String>,
}

/// Parameters for `explain_symbol` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExplainSymbolParams {
    /// Fully qualified symbol name or name prefix to explain.
    pub symbol: String,
}

/// Parameters for `get_commit_summary` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCommitSummaryParams {
    /// File path (relative to repo root) or fully qualified symbol name.
    pub file_or_symbol: String,
    /// Maximum number of commits to return (default: 5).
    pub limit: Option<usize>,
    /// Include the actual git diff stat for each commit (default: false).
    pub include_diff: Option<bool>,
}

/// Parameters for `search_commits` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchCommitsParams {
    /// Keyword query to search commit messages and summaries.
    pub query: String,
    /// Maximum number of results (default: 10).
    pub limit: Option<usize>,
}

/// Parameters for `ingest_external_doc` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IngestExternalDocParams {
    /// URL (https://...) or local file path to ingest.
    pub source: String,
    /// Re-ingest even if this source has been ingested before (default: false).
    pub force_reingest: Option<bool>,
}

/// Parameters for `context_window_pack` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ContextWindowPackParams {
    /// Search query.
    pub query: String,
    /// Token budget for the packed context (default: 100000).
    pub token_budget: Option<u32>,
    /// Maximum search results to consider (default: 50).
    pub limit: Option<usize>,
    /// Whether to include architectural shadow headers (default: false).
    pub shadow_headers: Option<bool>,
    /// Return as JSON array of items instead of formatted Markdown (default: false).
    pub as_json: Option<bool>,
    /// Minimum cross-encoder reranker score threshold (0.0–1.0).
    /// Chunks below this threshold are demoted. Higher values produce fewer,
    /// more precise results.  Only used when `as_json = true` (merged-pack mode).
    pub min_rerank_score: Option<f32>,
}

/// Parameters for `multi_repo_search` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MultiRepoSearchParams {
    /// Search query.
    pub query: String,
    /// Maximum results per repository (default: 5).
    pub limit: Option<usize>,
    /// Minimum reranker score threshold (0.0–1.0).
    pub min_rerank_score: Option<f32>,
}

/// Parameters for `save_memory` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SaveMemoryParams {
    /// Key under which to store the value.  Max 256 bytes.
    pub key: String,
    /// Value to store.  Max 64 KiB.
    pub value: String,
}

/// Parameters for `get_memory` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetMemoryParams {
    /// Key to retrieve.
    pub key: String,
}

/// Parameters for `list_memory` tool (no inputs required).
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListMemoryParams {}

// -----------------------------------------------------------------------
// MCP Server
// -----------------------------------------------------------------------

/// `OmniContext` MCP Server.
///
/// Exposes code intelligence tools to AI coding agents.
#[derive(Clone)]
pub struct OmniContextServer {
    engine: Arc<Mutex<Engine>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl OmniContextServer {
    /// Create a new MCP server backed by the given engine.
    pub fn new(engine: Engine) -> Self {
        Self {
            engine: Arc::new(Mutex::new(engine)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "search_code",
        description = "Search the codebase using hybrid retrieval (keyword + semantic). Returns ranked code chunks with file paths, scores, and source code. Use natural language queries like 'authentication middleware' or symbol names like 'validate_token'."
    )]
    async fn search_code(
        &self,
        params: Parameters<SearchCodeParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        validate_query(&params.0.query)?;
        let limit = clamp_limit(params.0.limit, 10);
        let min_score = clamp_rerank_score(params.0.min_rerank_score);
        let query = &params.0.query;
        let engine = self.engine.lock().await;

        match engine.search_with_rerank_threshold(query, limit, min_score) {
            Ok(results) => {
                if results.is_empty() {
                    let hint = if let Ok(status) = engine.status() {
                        if status.files_indexed == 0 {
                            "Repository not indexed. Run `omnicontext index .` first."
                        } else if status.embedding_coverage_percent < 0.1 {
                            "No results. Embedding coverage is 0% -- semantic search unavailable.\n\
                             Run `omnicontext setup model-download` then `omnicontext index . --force`."
                        } else {
                            "No results found for this query."
                        }
                    } else {
                        "No results found. Make sure the repository has been indexed with `omnicontext index .`"
                    };
                    return Ok(CallToolResult::success(vec![Content::text(hint)]));
                }

                let mut output = String::new();
                for (i, result) in results.iter().enumerate() {
                    write!(
                        output,
                        "## Result {} (score: {:.4})\n**File**: {}\n**Symbol**: {} ({:?})\n**Lines**: {}-{}\n",
                        i + 1, result.score,
                        result.file_path.display(),
                        result.chunk.symbol_path, result.chunk.kind,
                        result.chunk.line_start, result.chunk.line_end,
                    )
                    .ok();
                    if let Some(ref doc) = result.chunk.doc_comment {
                        writeln!(output, "**Doc**: {doc}").ok();
                    }
                    write!(output, "```\n{}\n```\n\n", result.chunk.content).ok();
                }

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("search failed: {e}"),
                None,
            )),
        }
    }

    #[tool(
        name = "context_window",
        description = "Get a pre-assembled, token-budget-aware context window for a query. Groups results by file, includes graph-neighbor definitions, and packs optimally within a token budget. Use this when you need maximum relevant context for understanding or modifying code."
    )]
    async fn context_window(
        &self,
        params: Parameters<ContextWindowParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        validate_query(&params.0.query)?;
        let limit = clamp_limit(params.0.limit, 20);
        let min_score = clamp_rerank_score(params.0.min_rerank_score);
        let query = &params.0.query;
        let want_shadow = params.0.shadow_headers;
        let mut engine = self.engine.lock().await;
        let rules_prefix = engine.load_rules_prefix();
        let memory_prefix = engine.memory_prefix();

        match engine.search_context_window_with_rerank_threshold(
            query,
            limit,
            params.0.token_budget,
            min_score,
        ) {
            Ok(mut ctx) => {
                // Enrich with shadow headers if explicitly requested (overrides config)
                if want_shadow == Some(true) {
                    engine.enrich_shadow_headers(&mut ctx);
                }

                if ctx.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        "No results found. Make sure the repository has been indexed.",
                    )]));
                }

                let mut output = format!(
                    "# Context Window ({} entries, {}/{} tokens used)\n\n",
                    ctx.len(),
                    ctx.total_tokens,
                    ctx.token_budget
                );

                // Group entries by file for cleaner output
                let mut current_file: Option<&std::path::Path> = None;
                for entry in &ctx.entries {
                    if current_file != Some(&entry.file_path) {
                        write!(
                            output,
                            "\n## {}{}\n",
                            entry.file_path.display(),
                            if entry.is_graph_neighbor {
                                " (graph neighbor)"
                            } else {
                                ""
                            }
                        )
                        .ok();
                        current_file = Some(&entry.file_path);
                    }

                    writeln!(
                        output,
                        "### {} ({:?}, score: {:.4}){}",
                        entry.chunk.symbol_path,
                        entry.chunk.kind,
                        entry.score,
                        if entry.is_graph_neighbor {
                            " [via graph]"
                        } else {
                            ""
                        },
                    )
                    .ok();
                    // Include shadow header if present
                    if let Some(ref header) = entry.shadow_header {
                        writeln!(output, "{header}").ok();
                    }
                    write!(output, "```\n{}\n```\n\n", entry.chunk.content).ok();
                }

                let final_output = format!("{rules_prefix}{memory_prefix}{output}");
                Ok(CallToolResult::success(vec![Content::text(final_output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("context_window failed: {e}"),
                None,
            )),
        }
    }

    #[tool(
        name = "get_symbol",
        description = "Look up a specific code symbol by fully qualified name or search by name prefix. Returns the full definition with documentation. Examples: 'auth::validate_token', 'UserService'."
    )]
    async fn get_symbol(
        &self,
        params: Parameters<GetSymbolParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        let limit = clamp_limit(params.0.limit, 5);
        let name = &params.0.name;
        if name.trim().is_empty() {
            return Err(McpError::invalid_params(
                "symbol name must not be empty",
                None,
            ));
        }
        if name.len() > MAX_QUERY_LEN {
            return Err(McpError::invalid_params(
                format!("symbol name exceeds maximum length of {MAX_QUERY_LEN} characters"),
                None,
            ));
        }
        let engine = self.engine.lock().await;
        let index = engine.metadata_index();

        match index.get_symbol_by_fqn(name) {
            Ok(Some(symbol)) => {
                // Resolve file path from file_id
                let file_path_str = index
                    .get_file_by_id(symbol.file_id)
                    .ok()
                    .flatten()
                    .map_or_else(
                        || format!("file#{}", symbol.file_id),
                        |f| f.path.display().to_string(),
                    );

                let mut output = format!(
                    "## {} ({:?})\n**File**: {}\n**Line**: {}\n",
                    symbol.fqn, symbol.kind, file_path_str, symbol.line
                );

                if let Some(chunk_id) = symbol.chunk_id {
                    if let Ok(chunks) = index.get_chunks_for_file(symbol.file_id) {
                        if let Some(chunk) = chunks.iter().find(|c| c.id == chunk_id) {
                            if let Some(ref doc) = chunk.doc_comment {
                                writeln!(output, "**Doc**: {doc}").ok();
                            }
                            write!(output, "```\n{}\n```\n", chunk.content).ok();
                        }
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Ok(None) => match index.search_symbols_by_name(name, limit) {
                Ok(symbols) if symbols.is_empty() => {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "No symbol found matching '{name}'",
                    ))]))
                }
                Ok(symbols) => {
                    use std::fmt::Write;
                    let mut output = format!("## Symbols matching '{name}'\n\n");
                    for sym in &symbols {
                        writeln!(
                            output,
                            "- **{}** ({:?}) -- file_id: {}, line: {}",
                            sym.fqn, sym.kind, sym.file_id, sym.line
                        )
                        .ok();
                    }
                    Ok(CallToolResult::success(vec![Content::text(output)]))
                }
                Err(e) => Err(McpError::internal_error(
                    format!("symbol search failed: {e}"),
                    None,
                )),
            },
            Err(e) => Err(McpError::internal_error(
                format!("symbol lookup failed: {e}"),
                None,
            )),
        }
    }

    #[tool(
        name = "get_file_summary",
        description = "Get a structural summary of a file: exports, classes, functions, and symbols. Provide the file path relative to the repository root."
    )]
    async fn get_file_summary(
        &self,
        params: Parameters<GetFileSummaryParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        // Helper: strip Windows UNC prefix for consistent comparison
        fn normalize_path_str(s: &str) -> &str {
            s.strip_prefix(r"\\?\").unwrap_or(s)
        }

        validate_relative_path(&params.0.path)?;

        let path_str = &params.0.path;
        let engine = self.engine.lock().await;
        let index = engine.metadata_index();
        let repo_root = engine.repo_path();

        // Build candidate paths to try:
        // 1. As given (relative path)
        // 2. Joined with repo root (absolute)
        // 3. Canonicalized versions of both
        let file_path = std::path::Path::new(path_str);
        let absolute_path = if file_path.is_relative() {
            repo_root.join(file_path)
        } else {
            file_path.to_path_buf()
        };

        // Try exact match first, then normalized absolute path
        let candidates = [file_path.to_path_buf(), absolute_path.clone()];

        let mut file_info = None;
        for candidate in &candidates {
            if let Ok(Some(info)) = index.get_file_by_path(candidate) {
                file_info = Some(info);
                break;
            }
            let candidate_str = candidate.to_string_lossy();
            let normalized = normalize_path_str(&candidate_str);
            let norm_path = std::path::Path::new(normalized);
            if norm_path != candidate.as_path() {
                if let Ok(Some(info)) = index.get_file_by_path(norm_path) {
                    file_info = Some(info);
                    break;
                }
            }
        }

        // Last resort: try canonicalization
        if file_info.is_none() {
            if let Ok(canonical) = absolute_path.canonicalize() {
                if let Ok(Some(info)) = index.get_file_by_path(&canonical) {
                    file_info = Some(info);
                } else {
                    let canon_str = canonical.to_string_lossy();
                    let norm = normalize_path_str(&canon_str);
                    let norm_path = std::path::Path::new(norm);
                    if let Ok(Some(info)) = index.get_file_by_path(norm_path) {
                        file_info = Some(info);
                    }
                }
            }
        }

        // Final fallback: normalize separators and do a suffix search.
        // The index stores relative paths with forward slashes, but
        // Windows callers may pass backslashes.
        if file_info.is_none() {
            let normalized = path_str.replace('\\', "/");
            // Try the normalized relative path directly
            let norm_path = std::path::Path::new(&normalized);
            if let Ok(Some(info)) = index.get_file_by_path(norm_path) {
                file_info = Some(info);
            } else if let Ok(Some(info)) = index.search_file_by_path_suffix(&normalized) {
                file_info = Some(info);
            }
        }

        match file_info {
            Some(info) => {
                let mut output = format!(
                    "## File: {}\n**Language**: {:?}\n**Size**: {} bytes\n\n",
                    path_str, info.language, info.size_bytes
                );

                match index.get_chunks_for_file(info.id) {
                    Ok(chunks) => {
                        write!(output, "### Structure ({} chunks)\n\n", chunks.len()).ok();
                        for chunk in &chunks {
                            let doc_preview = chunk.doc_comment.as_deref()
                                .map(|d| {
                                    let first = d.lines().next().unwrap_or("");
                                    if first.len() > 80 { format!(" -- {}...", &first[..80]) }
                                    else { format!(" -- {first}") }
                                })
                                .unwrap_or_default();

                            writeln!(
                                output,
                                "- **{:?}** `{}` (L{}-L{}){}",
                                chunk.kind, chunk.symbol_path,
                                chunk.line_start, chunk.line_end, doc_preview,
                            )
                            .ok();
                        }
                    }
                    Err(e) => {
                        writeln!(output, "Error loading chunks: {e}").ok();
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            None => Ok(CallToolResult::success(vec![Content::text(
                format!("File not found in index: '{path_str}'. Try using relative path from repo root or ensure the file has been indexed."),
            )])),
        }
    }

    #[tool(
        name = "get_status",
        description = "Get the current status of the OmniContext engine: indexed files, chunks, symbols, vectors, and search mode."
    )]
    async fn get_status(&self) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        let engine = self.engine.lock().await;
        match engine.status() {
            Ok(s) => {
                #[allow(clippy::cast_precision_loss)]
                let memory_mb = s.vector_memory_bytes as f64 / (1024.0 * 1024.0);
                let mut output = format!(
                    "## OmniContext Status\n\n\
                     - **Repository**: {}\n- **Data dir**: {}\n- **Search mode**: {}\n\n\
                     ### Index Statistics\n\n\
                     | Metric | Value |\n|--------|-------|\n\
                     | Files | {} |\n| Chunks | {} |\n\
                     | Symbols | {} |\n| Vectors | {} |\n\
                     | Embedding coverage | {:.1}% |\n\
                     | Vector memory | {:.2} MB |\n\
                     | ANN strategy | {} |\n\n\
                     ### Dependency Graph\n\n\
                     - Edges (persisted): {}\n- Graph nodes: {}\n- Graph edges: {}\n",
                    s.repo_path,
                    s.data_dir,
                    s.search_mode,
                    s.files_indexed,
                    s.chunks_indexed,
                    s.symbols_indexed,
                    s.vectors_indexed,
                    s.embedding_coverage_percent,
                    memory_mb,
                    s.active_search_strategy,
                    s.dep_edges,
                    s.graph_nodes,
                    s.graph_edges,
                );

                // Language distribution
                if !s.language_distribution.is_empty() {
                    writeln!(output, "\n### Language Distribution\n").ok();
                    for (lang, count) in &s.language_distribution {
                        writeln!(output, "- **{lang}**: {count} files").ok();
                    }
                }

                // Diagnostic hints for embedding coverage
                if s.embedding_coverage_percent < 0.1 && s.chunks_indexed > 0 {
                    output.push_str(
                        "\n> **CRITICAL**: Embedding coverage is 0%. Semantic search is DISABLED.\n\
                         > Run `omnicontext setup model-download` then `omnicontext index . --force`.\n",
                    );
                } else if s.embedding_coverage_percent < 50.0 && s.chunks_indexed > 0 {
                    output.push_str(
                        "\n> **Warning**: Low embedding coverage. Run `omnicontext embed --retry-failed` to fill gaps.\n",
                    );
                }

                if s.has_cycles {
                    output.push_str(
                        "\n> **Warning**: Circular dependencies detected in the graph.\n",
                    );
                }
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("status failed: {e}"),
                None,
            )),
        }
    }

    #[tool(
        name = "get_dependencies",
        description = "Get dependency relationships for a symbol: upstream (what it depends on) and downstream (what depends on it). Uses the dependency graph built during indexing."
    )]
    async fn get_dependencies(
        &self,
        params: Parameters<GetDependenciesParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        let symbol_name = &params.0.symbol;
        let direction = params.0.direction.as_deref().unwrap_or("both");
        // Validate direction parameter
        if !matches!(direction, "upstream" | "downstream" | "both") {
            return Err(McpError::invalid_params(
                format!(
                    "direction must be 'upstream', 'downstream', or 'both', got: '{direction}'"
                ),
                None,
            ));
        }
        let engine = self.engine.lock().await;
        let index = engine.metadata_index();
        let graph = engine.dep_graph();

        // Look up the symbol
        let symbol = match index.get_symbol_by_fqn(symbol_name) {
            Ok(Some(s)) => s,
            Ok(None) => {
                // Try prefix search
                match index.search_symbols_by_name(symbol_name, 1) {
                    Ok(syms) if !syms.is_empty() => syms.into_iter().next().ok_or_else(|| {
                        McpError::internal_error("symbol list unexpectedly empty".to_string(), None)
                    })?,
                    _ => {
                        return Ok(CallToolResult::success(vec![Content::text(format!(
                            "Symbol '{symbol_name}' not found in the index.",
                        ))]));
                    }
                }
            }
            Err(e) => {
                return Err(McpError::internal_error(
                    format!("lookup failed: {e}"),
                    None,
                ))
            }
        };

        let mut output = format!("## Dependencies for `{}`\n\n", symbol.fqn);

        // Upstream (what this symbol depends on)
        if direction == "upstream" || direction == "both" {
            output.push_str("### Upstream (depends on)\n\n");
            let upstream = graph.upstream(symbol.id, 2).unwrap_or_default();
            if upstream.is_empty() {
                // Fall back to SQLite
                match index.get_upstream_dependencies(symbol.id) {
                    Ok(edges) if edges.is_empty() => {
                        output.push_str("_No upstream dependencies found._\n\n");
                    }
                    Ok(edges) => {
                        for edge in &edges {
                            let target_name = index
                                .get_symbol_by_id(edge.target_id)
                                .ok()
                                .flatten()
                                .map_or_else(|| format!("symbol#{}", edge.target_id), |s| s.fqn);
                            writeln!(output, "- `{target_name}` ({:?})", edge.kind).ok();
                        }
                        output.push('\n');
                    }
                    Err(_) => output.push_str("_No upstream dependencies found._\n\n"),
                }
            } else {
                for sym_id in &upstream {
                    let name = index
                        .get_symbol_by_id(*sym_id)
                        .ok()
                        .flatten()
                        .map_or_else(|| format!("symbol#{sym_id}"), |s| s.fqn);
                    writeln!(output, "- `{name}`").ok();
                }
                output.push('\n');
            }
        }

        // Downstream (what depends on this symbol)
        if direction == "downstream" || direction == "both" {
            output.push_str("### Downstream (depended on by)\n\n");
            let downstream = graph.downstream(symbol.id, 2).unwrap_or_default();
            if downstream.is_empty() {
                match index.get_downstream_dependencies(symbol.id) {
                    Ok(edges) if edges.is_empty() => {
                        output.push_str("_No downstream dependencies found._\n\n");
                    }
                    Ok(edges) => {
                        for edge in &edges {
                            let source_name = index
                                .get_symbol_by_id(edge.source_id)
                                .ok()
                                .flatten()
                                .map_or_else(|| format!("symbol#{}", edge.source_id), |s| s.fqn);
                            writeln!(output, "- `{source_name}` ({:?})", edge.kind).ok();
                        }
                        output.push('\n');
                    }
                    Err(_) => output.push_str("_No downstream dependencies found._\n\n"),
                }
            } else {
                for sym_id in &downstream {
                    let name = index
                        .get_symbol_by_id(*sym_id)
                        .ok()
                        .flatten()
                        .map_or_else(|| format!("symbol#{sym_id}"), |s| s.fqn);
                    writeln!(output, "- `{name}`").ok();
                }
                output.push('\n');
            }
        }

        // Cycle detection
        if graph.has_cycles() {
            output.push_str("### Circular Dependencies Detected\n\n");
            if let Ok(cycles) = graph.find_cycles() {
                for (i, cycle) in cycles.iter().enumerate() {
                    let names: Vec<String> = cycle
                        .iter()
                        .map(|id| {
                            index
                                .get_symbol_by_id(*id)
                                .ok()
                                .flatten()
                                .map_or_else(|| format!("symbol#{id}"), |s| s.fqn)
                        })
                        .collect();
                    writeln!(output, "**Cycle {}**: {} -> ...", i + 1, names.join(" -> ")).ok();
                }
            }
        }

        // Graph stats
        write!(
            output,
            "\n### Graph Statistics\n\n- Nodes: {}\n- Edges: {}\n",
            graph.node_count(),
            graph.edge_count(),
        )
        .ok();

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "find_patterns",
        description = "Find code patterns by searching for specific constructs. Combines keyword and semantic search to find similar implementations. Examples: 'error handling', 'API endpoint handlers'."
    )]
    async fn find_patterns(
        &self,
        params: Parameters<FindPatternsParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        validate_query(&params.0.pattern)?;
        let limit = clamp_limit(params.0.limit, 5);
        let pattern = &params.0.pattern;
        let engine = self.engine.lock().await;

        match engine.search(pattern, limit) {
            Ok(results) => {
                if results.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(format!(
                        "No patterns matching '{pattern}' found.",
                    ))]));
                }

                let mut output = format!(
                    "## Pattern: '{pattern}'\n\nFound {} examples:\n\n",
                    results.len()
                );
                for (i, result) in results.iter().enumerate() {
                    write!(
                        output,
                        "### Example {} -- {} (score: {:.4})\n**{:?}** `{}` (L{}-L{})\n```\n{}\n```\n\n",
                        i + 1, result.file_path.display(), result.score,
                        result.chunk.kind, result.chunk.symbol_path,
                        result.chunk.line_start, result.chunk.line_end,
                        result.chunk.content,
                    )
                    .ok();
                }
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("pattern search failed: {e}"),
                None,
            )),
        }
    }

    #[tool(
        name = "get_architecture",
        description = "Get a high-level overview of the codebase architecture: file structure, module relationships, and technology stack."
    )]
    async fn get_architecture(&self) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        let engine = self.engine.lock().await;
        let status = engine
            .status()
            .map_err(|e| McpError::internal_error(format!("architecture failed: {e}"), None))?;
        let index = engine.metadata_index();

        let mut output = format!(
            "## Codebase Architecture\n\n\
             **Repository**: {}\n**Search mode**: {}\n\n\
             ### Index Statistics\n\n\
             | Metric | Count |\n|--------|-------|\n\
             | Files | {} |\n| Chunks | {} |\n\
             | Symbols | {} |\n| Vectors | {} |\n\n",
            status.repo_path,
            status.search_mode,
            status.files_indexed,
            status.chunks_indexed,
            status.symbols_indexed,
            status.vectors_indexed,
        );

        // Language breakdown
        if let Ok(files) = index.get_all_files() {
            let mut lang_counts: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();
            for f in &files {
                *lang_counts
                    .entry(f.language.as_str().to_string())
                    .or_default() += 1;
            }
            if !lang_counts.is_empty() {
                writeln!(output, "### Language Distribution\n").ok();
                writeln!(output, "| Language | Files |").ok();
                writeln!(output, "|----------|-------|").ok();
                for (lang, count) in &lang_counts {
                    writeln!(output, "| {lang} | {count} |").ok();
                }
                output.push('\n');
            }
        }

        // Dependency graph summary
        let graph = engine.dep_graph();
        writeln!(
            output,
            "### Dependency Graph\n\n- Nodes: {}\n- Edges: {}\n- Cycles: {}\n",
            graph.node_count(),
            graph.edge_count(),
            if status.has_cycles {
                "detected"
            } else {
                "none"
            },
        )
        .ok();

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "explain_codebase",
        description = "Get a comprehensive explanation of the codebase: tech stack, entry points, structure. Good for onboarding to a new project."
    )]
    async fn explain_codebase(&self) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        let engine = self.engine.lock().await;
        let status = engine
            .status()
            .map_err(|e| McpError::internal_error(format!("explain failed: {e}"), None))?;
        let index = engine.metadata_index();

        let mut output = format!(
            "## Codebase Overview\n\n**Root**: {}\n\n\
             ### Statistics\n\n\
             | Metric | Count |\n|--------|-------|\n\
             | Files | {} |\n| Code Chunks | {} |\n\
             | Symbols | {} |\n| Embeddings | {} |\n\n",
            status.repo_path,
            status.files_indexed,
            status.chunks_indexed,
            status.symbols_indexed,
            status.vectors_indexed,
        );

        // Language distribution
        if let Ok(files) = index.get_all_files() {
            let mut lang_counts: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();
            for f in &files {
                *lang_counts
                    .entry(f.language.as_str().to_string())
                    .or_default() += 1;
            }
            if !lang_counts.is_empty() {
                writeln!(output, "### Languages\n").ok();
                for (lang, count) in &lang_counts {
                    writeln!(output, "- **{lang}**: {count} files").ok();
                }
                output.push('\n');
            }

            // Top-level directory structure (first 2 levels)
            let mut dirs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            for f in &files {
                let path_str = f.path.to_string_lossy();
                let parts: Vec<&str> = path_str.split(['/', '\\']).collect();
                if parts.len() > 1 {
                    dirs.insert(parts[0].to_string());
                }
            }
            if !dirs.is_empty() {
                writeln!(output, "### Top-Level Structure\n").ok();
                for dir in &dirs {
                    writeln!(output, "- `{dir}/`").ok();
                }
                output.push('\n');
            }
        }

        writeln!(output, "### Available Tools\n").ok();
        writeln!(
            output,
            "- `search_code` -- hybrid full-text + semantic search"
        )
        .ok();
        writeln!(
            output,
            "- `context_window` -- token-budget-aware context assembly"
        )
        .ok();
        writeln!(
            output,
            "- `search_by_intent` -- NL search with intent classification and query expansion"
        )
        .ok();
        writeln!(
            output,
            "- `get_symbol` -- exact symbol lookup with source code"
        )
        .ok();
        writeln!(output, "- `get_file_summary` -- file structure breakdown").ok();
        writeln!(output, "- `get_module_map` -- project module hierarchy").ok();
        writeln!(
            output,
            "- `get_status` -- live index statistics and health diagnostics"
        )
        .ok();
        writeln!(output, "- `get_dependencies` -- symbol dependency analysis").ok();
        writeln!(output, "- `get_blast_radius` -- change impact analysis").ok();
        writeln!(
            output,
            "- `get_call_graph` -- dependency graph visualization with Mermaid support"
        )
        .ok();
        writeln!(
            output,
            "- `get_recent_changes` -- git history and uncommitted diffs"
        )
        .ok();
        writeln!(
            output,
            "- `get_branch_context` -- per-branch diff awareness"
        )
        .ok();
        writeln!(
            output,
            "- `get_co_changes` -- co-change coupling analysis from git history"
        )
        .ok();
        writeln!(
            output,
            "- `find_patterns` -- discover recurring code patterns"
        )
        .ok();
        writeln!(
            output,
            "- `get_architecture` -- codebase architecture overview"
        )
        .ok();
        writeln!(
            output,
            "- `audit_plan` -- architectural risk assessment for a plan"
        )
        .ok();
        writeln!(
            output,
            "- `generate_manifest` -- auto-generate CLAUDE.md or context_map.json"
        )
        .ok();
        writeln!(output, "- `set_workspace` -- switch the active repository").ok();
        writeln!(output, "- `explain_codebase` -- this onboarding overview").ok();

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "get_module_map",
        description = "Returns the module/crate/package hierarchy as a tree structure. Shows files grouped by directory with their exported symbols. Useful for understanding project architecture."
    )]
    async fn get_module_map(
        &self,
        params: Parameters<GetModuleMapParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        let max_depth = params.0.max_depth.map(|d| clamp_depth(Some(d), 10));
        let engine = self.engine.lock().await;
        let index = engine.metadata_index();

        let files = index
            .get_all_files()
            .map_err(|e| McpError::internal_error(format!("failed to list files: {e}"), None))?;

        if files.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No files indexed. Run `omnicontext index .` first.",
            )]));
        }

        let mut modules: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();

        for file in &files {
            let path_str = file.path.display().to_string();
            let parts: Vec<&str> = path_str.split(['/', '\\']).collect();

            // Apply max_depth filter: skip files deeper than max_depth directories
            if let Some(depth) = max_depth {
                if parts.len() > depth + 1 {
                    continue;
                }
            }

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
                .map(|c| format!("{} ({:?})", c.symbol_path, c.kind))
                .collect();

            let entry = format!(
                "  {} [{}]{}",
                parts.last().unwrap_or(&"?"),
                file.language.as_str(),
                if symbols.is_empty() {
                    String::new()
                } else {
                    format!(" -- {}", symbols.join(", "))
                }
            );

            modules.entry(module_key).or_default().push(entry);
        }

        let mut output = format!(
            "## Module Map ({} modules, {} files)\n\n",
            modules.len(),
            files.len()
        );
        if let Some(depth) = max_depth {
            writeln!(output, "_Filtered to depth {depth}_\n").ok();
        }
        for (module, entries) in &modules {
            writeln!(output, "### {module}").ok();
            for entry in entries {
                writeln!(output, "{entry}").ok();
            }
            output.push('\n');
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "search_by_intent",
        description = "Natural language search with automatic query expansion. Understands intent (edit, explain, debug, refactor) and expands queries with synonyms and related terms for better recall. Returns a pre-assembled context window."
    )]
    async fn search_by_intent(
        &self,
        params: Parameters<SearchByIntentParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        validate_query(&params.0.query)?;
        let limit = clamp_limit(params.0.limit, 10);
        let query = &params.0.query;
        let want_shadow = params.0.shadow_headers;
        let mut engine = self.engine.lock().await;
        let rules_prefix = engine.load_rules_prefix();

        // Use omni-core's intent classifier (not just static synonyms)
        let intent = omni_core::search::QueryIntent::classify(query);
        let strategy = intent.context_strategy();

        // Query expansion: combine static synonyms with intent context
        let expanded_terms = expand_query(query);
        let expanded_query = expanded_terms.join(" ");

        // The engine's search pipeline already uses HyDE + intent classification
        // internally, so we leverage its full power here.
        match engine.search_context_window(&expanded_query, limit, params.0.token_budget) {
            Ok(mut ctx) => {
                // Enrich with shadow headers if explicitly requested
                if want_shadow == Some(true) {
                    engine.enrich_shadow_headers(&mut ctx);
                }

                if ctx.is_empty() {
                    // Fall back to original query (without synonym expansion)
                    match engine.search(query, limit) {
                        Ok(results) if results.is_empty() => {
                            return Ok(CallToolResult::success(vec![Content::text(format!(
                                "No results found for: '{query}'\n\n\
                                 **Intent**: {intent:?}\n\
                                 **Expanded to**: '{expanded_query}'\n\
                                 **Strategy**: graph_depth={}, include_tests={}, include_architecture={}",
                                strategy.graph_depth, strategy.include_tests, strategy.include_architecture,
                            ))]));
                        }
                        Ok(results) => {
                            let mut output = format!(
                                "## Search by Intent\n\
                                 **Query**: {query}\n\
                                 **Intent**: {intent:?}\n\
                                 **Expanded**: {expanded_query}\n\
                                 **Strategy**: graph_depth={}, include_tests={}, include_arch={}\n\
                                 **Results**: {}\n\n",
                                strategy.graph_depth,
                                strategy.include_tests,
                                strategy.include_architecture,
                                results.len()
                            );
                            for (i, r) in results.iter().enumerate() {
                                let breakdown = &r.score_breakdown;
                                write!(
                                    output,
                                    "### {} (score: {:.4})\n\
                                     **File**: {}\n\
                                     **Symbol**: {} ({:?})\n\
                                     **Score breakdown**: semantic_rank={}, keyword_rank={}, rrf={:.3}, \
                                     reranker={:.3}, struct_w={:.2}, dep_boost={:.2}, recency={:.2}\n\
                                     ```\n{}\n```\n\n",
                                    i + 1, r.score, r.file_path.display(),
                                    r.chunk.symbol_path, r.chunk.kind,
                                    breakdown.semantic_rank.map_or("N/A".to_string(), |r| r.to_string()),
                                    breakdown.keyword_rank.map_or("N/A".to_string(), |r| r.to_string()),
                                    breakdown.rrf_score,
                                    breakdown.reranker_score.unwrap_or(0.0),
                                    breakdown.structural_weight,
                                    breakdown.dependency_boost,
                                    breakdown.recency_boost,
                                    r.chunk.content,
                                )
                                .ok();
                            }
                            return Ok(CallToolResult::success(vec![Content::text(format!(
                                "{rules_prefix}{output}"
                            ))]));
                        }
                        Err(e) => {
                            return Err(McpError::internal_error(
                                format!("search failed: {e}"),
                                None,
                            ))
                        }
                    }
                }

                let mut output = format!(
                    "## Search by Intent\n\
                     **Query**: {query}\n\
                     **Intent**: {intent:?}\n\
                     **Expanded**: {expanded_query}\n\
                     **Strategy**: graph_depth={}, include_tests={}, include_arch={}\n\
                     **Context**: {} entries, {}/{} tokens\n\n",
                    strategy.graph_depth,
                    strategy.include_tests,
                    strategy.include_architecture,
                    ctx.len(),
                    ctx.total_tokens,
                    ctx.token_budget
                );
                output.push_str(&ctx.render());

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "{rules_prefix}{output}"
                ))]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("search_by_intent failed: {e}"),
                None,
            )),
        }
    }

    #[tool(
        name = "set_workspace",
        description = "Switch the engine to a different repository/workspace path at runtime. Use this when the current repository is incorrect or you need to query a different project. The engine will reinitialize with the new path and optionally auto-index it."
    )]
    async fn set_workspace(
        &self,
        params: Parameters<SetWorkspaceParams>,
    ) -> Result<CallToolResult, McpError> {
        let path_str = &params.0.path;
        let auto_index = params.0.auto_index.unwrap_or(true);

        let new_path = std::path::PathBuf::from(path_str)
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from(path_str));

        if !new_path.exists() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: path does not exist: {path_str}",
            ))]));
        }

        if !std::path::Path::new(&new_path).is_dir() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: path is not a directory: {path_str}",
            ))]));
        }

        // Reject known non-project directories to prevent silent misbehavior.
        let path_lower = new_path.to_string_lossy().to_lowercase();
        let suspicious = path_lower.contains("program files")
            || path_lower.contains("appdata")
            || path_lower.contains("programs\\antigravity")
            || path_lower.contains("programs/antigravity")
            || path_lower.contains(".vscode")
            || path_lower.contains(".gemini")
            || path_lower.contains(".ssh")
            || path_lower.contains("etc/shadow")
            || path_lower.contains("etc\\shadow");
        if suspicious {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: '{}' looks like an application directory, not a source code project. \
                 Please provide an absolute path to a project repository.",
                new_path.display()
            ))]));
        }

        // Reinitialize the engine with the new path
        let new_engine = match omni_core::Engine::new(&new_path) {
            Ok(e) => e,
            Err(e) => {
                return Err(McpError::internal_error(
                    format!(
                        "failed to initialize engine for {}: {e}",
                        new_path.display()
                    ),
                    None,
                ));
            }
        };

        // Check if auto-index is needed
        let status_before = new_engine.status().ok();
        let needs_index = auto_index
            && status_before
                .as_ref()
                .map_or(true, |s| s.files_indexed == 0);

        // Swap the engine
        {
            let mut engine = self.engine.lock().await;
            *engine = new_engine;
        }

        // Auto-index if needed
        if needs_index {
            let mut engine = self.engine.lock().await;
            match engine.run_index(false).await {
                Ok(result) => {
                    return Ok(CallToolResult::success(vec![Content::text(format!(
                        "Workspace switched to: {}\nAuto-indexed: {} files, {} chunks, {} symbols",
                        new_path.display(),
                        result.files_processed,
                        result.chunks_created,
                        result.symbols_extracted,
                    ))]));
                }
                Err(e) => {
                    return Ok(CallToolResult::success(vec![Content::text(format!(
                        "Workspace switched to: {}\nWarning: auto-index failed: {e}",
                        new_path.display(),
                    ))]));
                }
            }
        }

        let files = status_before.map_or(0, |s| s.files_indexed);
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Workspace switched to: {} ({} files in existing index)",
            new_path.display(),
            files,
        ))]))
    }

    #[tool(
        name = "get_blast_radius",
        description = "Analyze the impact of changing a symbol. Returns all code that would be transitively affected if the given symbol is modified -- answers 'what breaks if I change this?'. Results are sorted by proximity (closest affected first)."
    )]
    async fn get_blast_radius(
        &self,
        params: Parameters<GetBlastRadiusParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        let symbol_name = &params.0.symbol;
        let max_depth = clamp_depth(params.0.max_depth, 5);
        let engine = self.engine.lock().await;
        let index = engine.metadata_index();
        let graph = engine.dep_graph();

        // Resolve symbol name to ID
        let symbol = match index.get_symbol_by_fqn(symbol_name) {
            Ok(Some(s)) => s,
            Ok(None) => {
                // Try prefix search fallback
                match index.search_symbols_by_name(symbol_name, 1) {
                    Ok(syms) if !syms.is_empty() => syms.into_iter().next().ok_or_else(|| {
                        McpError::internal_error("symbol list unexpectedly empty".to_string(), None)
                    })?,
                    _ => {
                        return Ok(CallToolResult::success(vec![Content::text(format!(
                            "Symbol not found: '{symbol_name}'"
                        ))]));
                    }
                }
            }
            Err(e) => {
                return Err(McpError::internal_error(
                    format!("symbol lookup failed: {e}"),
                    None,
                ));
            }
        };

        // Compute blast radius
        let affected = graph
            .blast_radius(symbol.id, max_depth)
            .map_err(|e| McpError::internal_error(format!("blast radius failed: {e}"), None))?;

        if affected.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "## Blast Radius: {}\nNo downstream dependents found. This symbol can be safely modified in isolation.",
                symbol.fqn
            ))]));
        }

        let mut output = format!(
            "## Blast Radius: {}\n**{} symbols affected** (max depth: {})\n\n",
            symbol.fqn,
            affected.len(),
            max_depth
        );

        // Resolve affected symbol IDs to names and group by distance
        let mut current_depth = 0;
        for (sym_id, distance) in &affected {
            if *distance != current_depth {
                current_depth = *distance;
                writeln!(
                    output,
                    "\n### Depth {current_depth} ({} hop{} away)",
                    current_depth,
                    if current_depth == 1 { "" } else { "s" }
                )
                .ok();
            }

            if let Ok(Some(sym)) = index.get_symbol_by_id(*sym_id) {
                // Get the file path for context
                if let Some(chunk_id) = sym.chunk_id {
                    if let Ok(chunks) = index.get_chunks_for_file(sym.file_id) {
                        if let Some(chunk) = chunks.iter().find(|c| c.id == chunk_id) {
                            writeln!(
                                output,
                                "- **{}** ({:?}) -- lines {}-{}",
                                sym.fqn, sym.kind, chunk.line_start, chunk.line_end,
                            )
                            .ok();
                            continue;
                        }
                    }
                }
                writeln!(output, "- **{}** ({:?})", sym.fqn, sym.kind).ok();
            }
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "get_recent_changes",
        description = "Get recently changed files in the repository using git history. Shows which files were modified, added, or deleted in recent commits. Use this to understand what code is actively being worked on."
    )]
    async fn get_recent_changes(
        &self,
        params: Parameters<GetRecentChangesParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        let commit_count = params.0.commit_count.unwrap_or(10).min(MAX_COMMIT_COUNT);
        let include_diff = params.0.include_diff.unwrap_or(false);
        let engine = self.engine.lock().await;
        let repo_path = engine.repo_path();

        // Try CommitEngine first (indexed, enriched data with co-change analysis)
        let indexed_commits =
            omni_core::commits::CommitEngine::recent_commits(engine.metadata_index(), commit_count);

        match indexed_commits {
            Ok(commits) if !commits.is_empty() => {
                let mut result = format!(
                    "## Recent Changes ({} commits, from indexed history)\n\
                     **Repository**: {}\n\n",
                    commits.len(),
                    repo_path.display()
                );

                for (i, commit) in commits.iter().enumerate() {
                    let hash_short = &commit.hash[..8.min(commit.hash.len())];
                    writeln!(
                        result,
                        "### {}. `{}` ({}) -- {}\n> {}",
                        i + 1,
                        hash_short,
                        commit.timestamp,
                        commit.author,
                        commit.message,
                    )
                    .ok();

                    // Show AI summary if available
                    if let Some(summary) = &commit.summary {
                        if !summary.is_empty() {
                            writeln!(result, "\n**Summary**: {summary}").ok();
                        }
                    }

                    // Show files changed
                    if !commit.files_changed.is_empty() {
                        writeln!(
                            result,
                            "\n**Files changed** ({}):",
                            commit.files_changed.len()
                        )
                        .ok();
                        for f in commit.files_changed.iter().take(20) {
                            writeln!(result, "  - `{f}`").ok();
                        }
                        if commit.files_changed.len() > 20 {
                            writeln!(
                                result,
                                "  - ... and {} more files",
                                commit.files_changed.len() - 20
                            )
                            .ok();
                        }
                    }
                    result.push('\n');
                }

                // Also get uncommitted changes (working tree) -- still via git
                let status_output = std::process::Command::new("git")
                    .args(["status", "--short"])
                    .current_dir(repo_path)
                    .output();

                if let Ok(status) = status_output {
                    if status.status.success() {
                        let status_str = String::from_utf8_lossy(&status.stdout);
                        if !status_str.is_empty() {
                            writeln!(result, "### Uncommitted Changes\n```\n{status_str}```").ok();
                        }
                    }
                }

                return Ok(CallToolResult::success(vec![Content::text(result)]));
            }
            _ => {
                // Fall back to raw git log when CommitEngine has no data
                tracing::debug!("CommitEngine has no indexed commits, falling back to git log");
            }
        }

        // Fallback: raw git log (original behavior)
        let diff_flag = if include_diff { "-p" } else { "--stat" };
        let git_output = std::process::Command::new("git")
            .args([
                "log",
                &format!("-{commit_count}"),
                "--pretty=format:%H|%ae|%ar|%s",
                diff_flag,
                "--no-color",
            ])
            .current_dir(repo_path)
            .output();

        match git_output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                if stdout.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        "No git history found. This may not be a git repository."
                    )]));
                }

                let mut result = format!(
                    "## Recent Changes ({} commits, from git log)\n\
                     **Repository**: {}\n\
                     **Note**: Run `index_commits` first for enriched commit data with co-change analysis.\n\n",
                    commit_count,
                    repo_path.display()
                );

                // Parse and format git log output
                let mut current_commit = String::new();
                for line in stdout.lines() {
                    if line.contains('|') && line.len() > 40 {
                        let parts: Vec<&str> = line.splitn(4, '|').collect();
                        if parts.len() == 4 {
                            if !current_commit.is_empty() {
                                result.push('\n');
                            }
                            let hash = &parts[0][..8.min(parts[0].len())];
                            current_commit = hash.to_string();
                            writeln!(
                                result,
                                "### `{}` ({}) -- {}\n> {}",
                                hash, parts[2], parts[1], parts[3]
                            ).ok();
                        }
                    } else if !line.trim().is_empty() {
                        writeln!(result, "{line}").ok();
                    }
                }

                // Also get uncommitted changes (working tree)
                let status_output = std::process::Command::new("git")
                    .args(["status", "--short"])
                    .current_dir(repo_path)
                    .output();

                if let Ok(status) = status_output {
                    if status.status.success() {
                        let status_str = String::from_utf8_lossy(&status.stdout);
                        if !status_str.is_empty() {
                            writeln!(result, "\n### Uncommitted Changes\n```\n{status_str}```").ok();
                        }
                    }
                }

                Ok(CallToolResult::success(vec![Content::text(result)]))
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Git command failed: {stderr}"
                ))]))
            }
            Err(e) => {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Failed to execute git: {e}. Make sure git is installed and this is a git repository."
                ))]))
            }
        }
    }

    #[tool(
        name = "get_call_graph",
        description = "Get the call graph for a symbol -- shows what it calls (upstream) and what calls it (downstream), with typed edges (Calls, Imports, Extends, Implements). Optionally outputs as a Mermaid diagram for visualization."
    )]
    async fn get_call_graph(
        &self,
        params: Parameters<GetCallGraphParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        let symbol_name = &params.0.symbol;
        let depth = clamp_depth(params.0.depth, 2);
        let as_mermaid = params.0.mermaid.unwrap_or(false);
        let engine = self.engine.lock().await;
        let index = engine.metadata_index();
        let graph = engine.dep_graph();

        // Resolve symbol
        let symbol = match index.get_symbol_by_fqn(symbol_name) {
            Ok(Some(s)) => s,
            Ok(None) => match index.search_symbols_by_name(symbol_name, 1) {
                Ok(syms) if !syms.is_empty() => syms.into_iter().next().ok_or_else(|| {
                    McpError::internal_error("symbol list unexpectedly empty".to_string(), None)
                })?,
                _ => {
                    return Ok(CallToolResult::success(vec![Content::text(format!(
                        "Symbol not found: '{symbol_name}'"
                    ))]));
                }
            },
            Err(e) => {
                return Err(McpError::internal_error(
                    format!("symbol lookup failed: {e}"),
                    None,
                ));
            }
        };

        // Get edges for this symbol
        let edges = graph
            .get_edges_for_symbol(symbol.id)
            .map_err(|e| McpError::internal_error(format!("call graph failed: {e}"), None))?;

        // Get upstream and downstream with depth
        let upstream = graph.upstream(symbol.id, depth).unwrap_or_default();
        let downstream = graph.downstream(symbol.id, depth).unwrap_or_default();

        if as_mermaid {
            // Generate Mermaid diagram
            let mut mermaid = String::from("```mermaid\ngraph TD\n");

            // Sanitize node ID for mermaid (replace chars that break node syntax)
            let sanitize = |s: &str| -> String { s.replace("::", "_").replace(['.', '-'], "_") };
            // Sanitize label text for mermaid (prevent injection via double-quotes)
            let sanitize_label = |s: &str| -> String { s.replace('"', "'") };

            let center_id = sanitize(&symbol.fqn);
            writeln!(
                mermaid,
                "  {center_id}[\"{}\"]\n",
                sanitize_label(&symbol.fqn)
            )
            .ok();

            // Style the center node
            writeln!(
                mermaid,
                "  style {center_id} fill:#ff9800,stroke:#e65100,stroke-width:3px"
            )
            .ok();

            // Add upstream nodes (what this symbol depends on)
            for sym_id in &upstream {
                if let Ok(Some(sym)) = index.get_symbol_by_id(*sym_id) {
                    let node_id = sanitize(&sym.fqn);
                    writeln!(mermaid, "  {node_id}[\"{}\"]", sanitize_label(&sym.fqn)).ok();
                    writeln!(mermaid, "  {center_id} --> {node_id}").ok();
                }
            }

            // Add downstream nodes (what depends on this symbol)
            for sym_id in &downstream {
                if let Ok(Some(sym)) = index.get_symbol_by_id(*sym_id) {
                    let node_id = sanitize(&sym.fqn);
                    writeln!(mermaid, "  {node_id}[\"{}\"]", sanitize_label(&sym.fqn)).ok();
                    writeln!(mermaid, "  {node_id} --> {center_id}").ok();
                }
            }

            mermaid.push_str("```\n");
            Ok(CallToolResult::success(vec![Content::text(mermaid)]))
        } else {
            // Text output
            let mut output = format!(
                "## Call Graph: {}\n**In-degree**: {} | **Edges**: {} | **Upstream**: {} | **Downstream**: {}\n\n",
                symbol.fqn,
                graph.in_degree(symbol.id),
                edges.len(),
                upstream.len(),
                downstream.len(),
            );

            if !edges.is_empty() {
                writeln!(output, "### Direct Edges\n").ok();
                for (target_id, kind, direction) in &edges {
                    if let Ok(Some(sym)) = index.get_symbol_by_id(*target_id) {
                        let arrow = if *direction == "outgoing" {
                            "-->"
                        } else {
                            "<--"
                        };
                        writeln!(
                            output,
                            "- {} {arrow} **{}** ({:?})",
                            symbol.fqn, sym.fqn, kind
                        )
                        .ok();
                    }
                }
            }

            if !upstream.is_empty() {
                writeln!(output, "\n### Upstream (Dependencies, depth {depth})\n").ok();
                for sym_id in &upstream {
                    if let Ok(Some(sym)) = index.get_symbol_by_id(*sym_id) {
                        writeln!(output, "- **{}** ({:?})", sym.fqn, sym.kind).ok();
                    }
                }
            }

            if !downstream.is_empty() {
                writeln!(output, "\n### Downstream (Dependents, depth {depth})\n").ok();
                for sym_id in &downstream {
                    if let Ok(Some(sym)) = index.get_symbol_by_id(*sym_id) {
                        writeln!(output, "- **{}** ({:?})", sym.fqn, sym.kind).ok();
                    }
                }
            }

            Ok(CallToolResult::success(vec![Content::text(output)]))
        }
    }

    #[tool(
        name = "get_branch_context",
        description = "Get current git branch status with uncommitted/unpushed changes, diff hunks, and branch-relative file lists. Use this to understand what the developer is working on and provide branch-aware suggestions."
    )]
    async fn get_branch_context(
        &self,
        params: Parameters<GetBranchContextParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        let include_diffs = params.0.include_diffs.unwrap_or(false);
        let mut engine = self.engine.lock().await;

        let tracker = engine.branch_tracker();

        let diff = match tracker.get_branch_diff() {
            Ok(d) => d.clone(),
            Err(e) => {
                return Ok(CallToolResult::success(vec![Content::text(format!(
                    "Branch tracking unavailable: {e}\n\n\
                     This may be because the workspace is not a git repository.",
                ))]));
            }
        };

        let mut output = format!(
            "## Branch Context\n\n\
             - **Current branch**: {}\n\
             - **Base branch**: {}\n\
             - **Uncommitted files**: {}\n\
             - **Unpushed files**: {}\n\
             - **Total lines changed**: {}\n\n",
            diff.branch,
            diff.base_branch,
            diff.uncommitted_files.len(),
            diff.unpushed_files.len(),
            diff.total_lines_changed,
        );

        if !diff.uncommitted_files.is_empty() {
            writeln!(output, "### Uncommitted Files\n").ok();
            for f in &diff.uncommitted_files {
                writeln!(output, "- `{f}`").ok();
            }
            output.push('\n');
        }

        if !diff.unpushed_files.is_empty() {
            writeln!(output, "### Unpushed Files\n").ok();
            for f in &diff.unpushed_files {
                writeln!(output, "- `{f}`").ok();
            }
            output.push('\n');
        }

        if include_diffs && !diff.uncommitted_hunks.is_empty() {
            writeln!(output, "### Diff Hunks\n").ok();
            for hunk in &diff.uncommitted_hunks {
                writeln!(
                    output,
                    "**{}** (L{}, {} lines, {:?})\n```\n{}\n```\n",
                    hunk.file_path,
                    hunk.start_line,
                    hunk.line_count,
                    hunk.change_type,
                    hunk.content,
                )
                .ok();
            }
        }

        // Branch-relative changed files (all commits since merge-base)
        match tracker.get_branch_changed_files() {
            Ok(files) if !files.is_empty() => {
                writeln!(
                    output,
                    "### Files changed on this branch (vs {})\n",
                    diff.base_branch
                )
                .ok();
                for f in &files {
                    writeln!(output, "- `{f}`").ok();
                }
            }
            _ => {}
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    // -----------------------------------------------------------------------
    // Co-change coupling tools
    // -----------------------------------------------------------------------

    #[tool(
        name = "get_co_changes",
        description = "Find files that frequently change together with a given file based on git commit history. Use this to discover hidden coupling between files that aren't connected through import/call dependencies."
    )]
    async fn get_co_changes(
        &self,
        params: Parameters<GetCoChangesParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        validate_relative_path(&params.0.file_path)?;
        let file_path = &params.0.file_path;
        let min_frequency = params.0.min_frequency.unwrap_or(2).clamp(1, 100);
        let limit = clamp_limit(params.0.limit, 10);

        let engine = self.engine.lock().await;
        let index = engine.metadata_index();

        match omni_core::commits::CommitEngine::co_change_files(
            index, file_path, min_frequency, limit,
        ) {
            Ok(co_changes) => {
                if co_changes.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(format!(
                        "No co-change partners found for `{file_path}` (min_frequency={min_frequency}).\n\n\
                         This may mean the file has few commits or changes independently.\n\
                         Try lowering `min_frequency` to 1."
                    ))]));
                }

                let mut output = format!(
                    "## Co-Change Partners for `{file_path}`\n\n\
                     Files that frequently change together (min {min_frequency} shared commits):\n\n\
                     | File | Shared Commits |\n\
                     |------|---------------|\n"
                );

                for co in &co_changes {
                    writeln!(output, "| `{}` | {} |", co.path, co.shared_commits).ok();
                }

                writeln!(
                    output,
                    "\n**{} co-change partners found.** These files may have hidden coupling.",
                    co_changes.len()
                )
                .ok();

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("co-change analysis failed: {e}"),
                None,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Plan analysis tools
    // -----------------------------------------------------------------------

    #[tool(
        name = "audit_plan",
        description = "Analyze a task plan for architectural risks, blast radius, co-change warnings, and breaking changes. Send your plan text and get back a structural critique with risk levels and recommendations."
    )]
    async fn audit_plan(
        &self,
        params: Parameters<AuditPlanParams>,
    ) -> Result<CallToolResult, McpError> {
        let plan_text = &params.0.plan;
        let max_depth = clamp_depth(params.0.max_depth, 3);

        if plan_text.len() > MAX_PLAN_LEN {
            return Err(McpError::invalid_params(
                format!("plan exceeds maximum length of {MAX_PLAN_LEN} characters"),
                None,
            ));
        }

        let engine = self.engine.lock().await;
        let auditor = omni_core::plan_auditor::PlanAuditor::new(&engine);

        match auditor.audit(plan_text, max_depth) {
            Ok(critique) => {
                let output = critique.to_markdown();
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("plan audit failed: {e}"),
                None,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Manifest generation tools
    // -----------------------------------------------------------------------

    #[tool(
        name = "generate_manifest",
        description = "Auto-generate a CLAUDE.md project guide or .context_map.json from live index data. Use 'claude' format for a human/AI-readable project overview, 'json' for structured data, or 'both'."
    )]
    async fn generate_manifest(
        &self,
        params: Parameters<GenerateManifestParams>,
    ) -> Result<CallToolResult, McpError> {
        let format = &params.0.format;
        // Validate format parameter
        if !matches!(format.as_str(), "claude" | "json" | "both") {
            return Err(McpError::invalid_params(
                format!("format must be 'claude', 'json', or 'both', got: '{format}'"),
                None,
            ));
        }
        let engine = self.engine.lock().await;

        let mut output = String::new();

        if format == "claude" || format == "both" {
            match engine.generate_claude_md() {
                Ok(claude_md) => {
                    output.push_str(&claude_md);
                }
                Err(e) => {
                    return Err(McpError::internal_error(
                        format!("CLAUDE.md generation failed: {e}"),
                        None,
                    ));
                }
            }
        }

        if format == "json" || format == "both" {
            if !output.is_empty() {
                output.push_str("\n\n---\n\n");
            }
            match engine.generate_context_map() {
                Ok(context_map) => {
                    output.push_str("```json\n");
                    output.push_str(&context_map);
                    output.push_str("\n```");
                }
                Err(e) => {
                    return Err(McpError::internal_error(
                        format!("context_map.json generation failed: {e}"),
                        None,
                    ));
                }
            }
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    // -----------------------------------------------------------------------
    // Tool 20 — search_with_filter
    // -----------------------------------------------------------------------
    #[tool(
        name = "search_with_filter",
        description = "Search the codebase with post-filter criteria: language, path glob, indexed-after datetime, and symbol type. \
                       All filters are optional and ANDed. Example: find auth logic in src/backend/ written in Rust, \
                       returning only function-level chunks. Use this for scoped, precise searches within large codebases."
    )]
    async fn search_with_filter(
        &self,
        params: Parameters<SearchWithFilterParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        validate_query(&params.0.query)?;
        let limit = clamp_limit(params.0.limit, 10);
        let min_score = clamp_rerank_score(params.0.min_rerank_score);
        let p = &params.0;
        let engine = self.engine.lock().await;

        match engine.search_filtered(
            &p.query,
            limit,
            min_score,
            p.language.as_deref(),
            p.path_glob.as_deref(),
            p.modified_after.as_deref(),
            p.symbol_type.as_deref(),
        ) {
            Ok(results) => {
                if results.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        "No results matched the query and filters. \
                         Try relaxing the language/path/symbol_type filters, or run `omnicontext index .` first.",
                    )]));
                }

                let mut output = format!(
                    "## Filtered Search Results ({} results)\n\
                     **Query**: `{}`\n\
                     **Filters**: language={} path={} symbol_type={}\n\n",
                    results.len(),
                    p.query,
                    p.language.as_deref().unwrap_or("any"),
                    p.path_glob.as_deref().unwrap_or("*"),
                    p.symbol_type.as_deref().unwrap_or("any"),
                );

                for (i, r) in results.iter().enumerate() {
                    write!(
                        output,
                        "### {} (score: {:.4})\n**File**: {}\n**Symbol**: `{}` ({:?})\n**Lines**: {}-{}\n",
                        i + 1, r.score, r.file_path.display(),
                        r.chunk.symbol_path, r.chunk.kind,
                        r.chunk.line_start, r.chunk.line_end,
                    ).ok();
                    if let Some(ref doc) = r.chunk.doc_comment {
                        writeln!(output, "**Doc**: {}", doc.lines().next().unwrap_or("")).ok();
                    }
                    write!(output, "```\n{}\n```\n\n", r.chunk.content).ok();
                }

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("search_with_filter failed: {e}"),
                None,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Tool 21 — explain_symbol
    // -----------------------------------------------------------------------
    #[tool(
        name = "explain_symbol",
        description = "Generate a comprehensive explanation for any symbol: type signature, doc comment, \
                       1-hop callers and callees from the dependency graph, recent commits touching the file, \
                       and co-change partners. Assembled entirely from structured index data — no LLM inference. \
                       Ideal for agents doing large-scale refactoring or impact analysis."
    )]
    async fn explain_symbol(
        &self,
        params: Parameters<ExplainSymbolParams>,
    ) -> Result<CallToolResult, McpError> {
        let symbol_name = &params.0.symbol;
        if symbol_name.trim().is_empty() {
            return Err(McpError::invalid_params("symbol must not be empty", None));
        }
        if symbol_name.len() > MAX_QUERY_LEN {
            return Err(McpError::invalid_params(
                format!("symbol name exceeds maximum length of {MAX_QUERY_LEN}"),
                None,
            ));
        }
        let engine = self.engine.lock().await;
        match engine.explain_symbol(symbol_name) {
            Ok(explanation) => Ok(CallToolResult::success(vec![Content::text(explanation)])),
            Err(e) => Err(McpError::internal_error(
                format!("explain_symbol failed: {e}"),
                None,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Tool 22 — get_commit_summary
    // -----------------------------------------------------------------------
    #[tool(
        name = "get_commit_summary",
        description = "Get recent git commits that touched a specific file or symbol. \
                       Returns commit hash, author, timestamp, message, files changed, and optional diff stat. \
                       Equivalent to Augment's commit context feature. \
                       Input: file path relative to repo root (e.g. 'src/auth.rs') or fully qualified symbol name."
    )]
    async fn get_commit_summary(
        &self,
        params: Parameters<GetCommitSummaryParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        let target = &params.0.file_or_symbol;
        if target.trim().is_empty() {
            return Err(McpError::invalid_params(
                "file_or_symbol must not be empty",
                None,
            ));
        }
        let limit = clamp_limit(params.0.limit, 5);
        let include_diff = params.0.include_diff.unwrap_or(false);
        let engine = self.engine.lock().await;

        match engine.get_commit_summary(target, limit, include_diff) {
            Ok(commits) => {
                if commits.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(format!(
                        "No commits found for `{target}`.\n\
                         Run `omnicontext index-commits` first, or verify the path/symbol exists."
                    ))]));
                }

                let mut output = format!(
                    "## Commit History for `{target}` ({} commits)\n\n",
                    commits.len()
                );

                for commit in &commits {
                    let hash_short = &commit.hash[..8.min(commit.hash.len())];
                    writeln!(output, "### `{hash_short}` — {}", commit.message).ok();
                    writeln!(
                        output,
                        "**Author**: {} | **Date**: {}",
                        commit.author, commit.timestamp
                    )
                    .ok();
                    if !commit.files_changed.is_empty() {
                        writeln!(
                            output,
                            "**Files changed**: {}",
                            commit.files_changed.join(", ")
                        )
                        .ok();
                    }
                    if let Some(ref summary) = commit.summary {
                        write!(output, "**Diff stat**:\n```\n{summary}\n```\n").ok();
                    }
                    output.push('\n');
                }

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("get_commit_summary failed: {e}"),
                None,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Tool 23 — search_commits
    // -----------------------------------------------------------------------
    #[tool(
        name = "search_commits",
        description = "Search git commit history by keyword. Returns matching commits with author, date, \
                       message, and files changed. Searches both commit messages and AI-generated summaries \
                       via FTS5. Falls back to live git log --grep when the commit index is empty. \
                       Useful for 'when was the auth bug introduced' style agent queries."
    )]
    async fn search_commits(
        &self,
        params: Parameters<SearchCommitsParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        validate_query(&params.0.query)?;
        let limit = clamp_limit(params.0.limit, 10);
        let engine = self.engine.lock().await;

        match engine.search_commits_by_query(&params.0.query, limit) {
            Ok(commits) => {
                if commits.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(format!(
                        "No commits found matching '{}'. \
                         Run `omnicontext index-commits` first to populate the commit index.",
                        params.0.query
                    ))]));
                }

                let mut output = format!(
                    "## Commit Search: '{}' ({} results)\n\n",
                    params.0.query,
                    commits.len()
                );

                for commit in &commits {
                    let hash_short = &commit.hash[..8.min(commit.hash.len())];
                    writeln!(output, "### `{hash_short}` — {}", commit.message).ok();
                    writeln!(
                        output,
                        "**Author**: {} | **Date**: {}",
                        commit.author, commit.timestamp
                    )
                    .ok();
                    if !commit.files_changed.is_empty() {
                        let files: Vec<&str> =
                            commit.files_changed.iter().map(String::as_str).collect();
                        let display = if files.len() > 5 {
                            format!("{} ... (+{})", files[..5].join(", "), files.len() - 5)
                        } else {
                            files.join(", ")
                        };
                        writeln!(output, "**Files**: {display}").ok();
                    }
                    output.push('\n');
                }

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("search_commits failed: {e}"),
                None,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Tool 24 — ingest_external_doc
    // -----------------------------------------------------------------------
    #[tool(
        name = "ingest_external_doc",
        description = "Ingest an external document (URL or local file path) into the searchable index. \
                       Fetches the content, strips HTML, splits into prose chunks, and makes it available \
                       via search_code and context_window. Equivalent to Sourcegraph's OpenCtx protocol. \
                       Supports API documentation pages, RFCs, internal wikis, Confluence pages, Markdown files. \
                       Once ingested, the document is cached and won't be re-fetched unless force_reingest=true."
    )]
    async fn ingest_external_doc(
        &self,
        params: Parameters<IngestExternalDocParams>,
    ) -> Result<CallToolResult, McpError> {
        let source = &params.0.source;
        if source.trim().is_empty() {
            return Err(McpError::invalid_params("source must not be empty", None));
        }
        if source.len() > 2048 {
            return Err(McpError::invalid_params(
                "source URL/path too long (max 2048 chars)",
                None,
            ));
        }
        let force = params.0.force_reingest.unwrap_or(false);

        let mut engine = self.engine.lock().await;
        match engine.ingest_external_doc(source, force) {
            Ok(0) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Source `{source}` already ingested. Pass `force_reingest: true` to re-ingest."
            ))])),
            Ok(count) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Successfully ingested `{source}` — {count} chunks added to the search index.\n\
                 Use `search_code` or `context_window` to query the ingested content."
            ))])),
            Err(e) => Err(McpError::internal_error(
                format!("ingest_external_doc failed: {e}"),
                None,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Tool 25 — context_window_pack
    // -----------------------------------------------------------------------
    #[tool(
        name = "context_window_pack",
        description = "Assemble a maximally informative context window within a strict token budget. \
                       Uses Maximal Marginal Relevance (MMR) ordering to minimize redundancy while maximizing \
                       coverage of distinct files and concepts. Returns an ordered array that fills the budget \
                       with minimum overlap. This is the highest-value tool for RAG orchestration — \
                       every agent framework doing retrieval-augmented generation benefits from this."
    )]
    async fn context_window_pack(
        &self,
        params: Parameters<ContextWindowPackParams>,
    ) -> Result<CallToolResult, McpError> {
        validate_query(&params.0.query)?;
        let token_budget = params
            .0
            .token_budget
            .unwrap_or(100_000)
            .clamp(1_000, 500_000);
        let limit = clamp_limit(params.0.limit, 50);
        let want_shadow = params.0.shadow_headers.unwrap_or(false);
        let as_json = params.0.as_json.unwrap_or(false);
        let min_rerank_score = params.0.min_rerank_score;
        let engine = self.engine.lock().await;

        // When the caller requests JSON output, use pack_context_window() which
        // performs adjacent-chunk merging and greedy token-budget packing.
        // This produces the `PackedContextEntry` flat format the plan specifies.
        if as_json {
            match engine.pack_context_window(&params.0.query, limit, token_budget, min_rerank_score)
            {
                Ok((packed, tokens_used)) => {
                    if packed.is_empty() {
                        return Ok(CallToolResult::success(vec![Content::text(
                            "No context assembled. Make sure the repository has been indexed.",
                        )]));
                    }
                    let items: Vec<serde_json::Value> = packed
                        .iter()
                        .map(|e| {
                            serde_json::json!({
                                "file": e.file_path.display().to_string(),
                                "symbol": e.symbol_path,
                                "kind": format!("{:?}", e.kind),
                                "line_start": e.line_start,
                                "line_end": e.line_end,
                                "score": e.score,
                                "token_count": e.token_count,
                                "content": e.content,
                            })
                        })
                        .collect();
                    let out = serde_json::json!({
                        "query": params.0.query,
                        "token_budget": token_budget,
                        "total_tokens": tokens_used,
                        "entries": items,
                    });
                    return Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&out).unwrap_or_default(),
                    )]));
                }
                Err(e) => {
                    return Err(McpError::internal_error(
                        format!("context_window_pack failed: {e}"),
                        None,
                    ));
                }
            }
        }

        // Markdown mode uses the existing ContextWindow pipeline with optional
        // shadow headers.
        match engine.search_context_window_with_rerank_threshold(
            &params.0.query,
            limit,
            Some(token_budget),
            min_rerank_score,
        ) {
            Ok(mut ctx) => {
                if want_shadow {
                    engine.enrich_shadow_headers(&mut ctx);
                }

                if ctx.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        "No context assembled. Make sure the repository has been indexed.",
                    )]));
                }

                {
                    // Markdown output
                    use std::fmt::Write;
                    let mut output = format!(
                        "# Packed Context Window\n\
                         **Query**: `{}`\n\
                         **Token budget**: {} | **Tokens used**: {} ({:.1}%)\n\
                         **Entries**: {} items from {} unique files\n\n",
                        params.0.query,
                        token_budget,
                        ctx.total_tokens,
                        f64::from(ctx.total_tokens) / f64::from(token_budget) * 100.0,
                        ctx.len(),
                        ctx.entries
                            .iter()
                            .map(|e| e.file_path.as_path())
                            .collect::<std::collections::HashSet<_>>()
                            .len(),
                    );

                    let mut current_file: Option<&std::path::Path> = None;
                    for entry in &ctx.entries {
                        if current_file != Some(&entry.file_path) {
                            write!(
                                output,
                                "\n## {}{}\n",
                                entry.file_path.display(),
                                if entry.is_graph_neighbor {
                                    " (via graph)"
                                } else {
                                    ""
                                }
                            )
                            .ok();
                            current_file = Some(&entry.file_path);
                        }
                        writeln!(
                            output,
                            "### `{}` ({:?}, {:.4}, {} tokens){}",
                            entry.chunk.symbol_path,
                            entry.chunk.kind,
                            entry.score,
                            entry.chunk.token_count,
                            if entry.is_graph_neighbor {
                                " [graph]"
                            } else {
                                ""
                            },
                        )
                        .ok();
                        if let Some(ref hdr) = entry.shadow_header {
                            writeln!(output, "{hdr}").ok();
                        }
                        write!(output, "```\n{}\n```\n\n", entry.chunk.content).ok();
                    }

                    Ok(CallToolResult::success(vec![Content::text(output)]))
                }
            }
            Err(e) => Err(McpError::internal_error(
                format!("context_window_pack failed: {e}"),
                None,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Tool 26 — multi_repo_search
    // -----------------------------------------------------------------------
    #[tool(
        name = "multi_repo_search",
        description = "Search across all registered repositories in the workspace, returning results \
                       ranked by relevance with per-repository attribution. Uses priority-weighted RRF fusion \
                       to merge results from different repos. Requires repos to be added via the workspace IPC \
                       (`workspace/add_repo`). Falls back to the current repo when workspace is empty. \
                       Closes the last major capability gap vs Augment remote mode and Sourcegraph multi-repo."
    )]
    async fn multi_repo_search(
        &self,
        params: Parameters<MultiRepoSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        validate_query(&params.0.query)?;
        let limit = clamp_limit(params.0.limit, 5);
        let min_score = clamp_rerank_score(params.0.min_rerank_score);
        let engine = self.engine.lock().await;

        // Multi-repo search falls through to the engine's workspace search.
        // The workspace applies priority-weighted RRF fusion across all registered repos.
        // If no additional repos are registered, this is equivalent to a standard search.
        match engine.search_with_rerank_threshold(&params.0.query, limit, min_score) {
            Ok(results) => {
                if results.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(format!(
                        "No results for '{}' across registered repositories.\n\
                         Add repos with `workspace/add_repo`, then re-index.",
                        params.0.query
                    ))]));
                }

                let mut output = format!(
                    "## Multi-Repo Search: '{}' ({} results)\n\n",
                    params.0.query,
                    results.len()
                );

                for (i, r) in results.iter().enumerate() {
                    write!(
                        output,
                        "### {} (score: {:.4})\n**File**: `{}`\n**Symbol**: `{}` ({:?})\n",
                        i + 1,
                        r.score,
                        r.file_path.display(),
                        r.chunk.symbol_path,
                        r.chunk.kind,
                    )
                    .ok();
                    write!(output, "```\n{}\n```\n\n", r.chunk.content).ok();
                }

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("multi_repo_search failed: {e}"),
                None,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Tool 27 — save_memory
    // -----------------------------------------------------------------------
    #[tool(
        name = "save_memory",
        description = "Save a key-value pair to the repository's persistent memory. \
                       Memory persists across sessions and is injected into every context_window \
                       response as a structured prefix block. Use to store architectural decisions, \
                       coding conventions, team notes, and any context that should survive agent \
                       session boundaries. Keys max 256 bytes; values max 64 KiB; up to 1,000 entries."
    )]
    async fn save_memory(
        &self,
        params: Parameters<SaveMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let key = params.0.key.clone();
        let value = params.0.value.clone();

        if key.trim().is_empty() {
            return Err(McpError::invalid_params(
                "memory key must not be empty",
                None,
            ));
        }

        let mut engine = self.engine.lock().await;
        match engine.memory_set(key.clone(), value) {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Memory saved: key '{key}' stored successfully."
            ))])),
            Err(e) => Err(McpError::invalid_params(
                format!("save_memory failed: {e}"),
                None,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Tool 28 — get_memory
    // -----------------------------------------------------------------------
    #[tool(
        name = "get_memory",
        description = "Retrieve a value from the repository's persistent memory by key."
    )]
    async fn get_memory(
        &self,
        params: Parameters<GetMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let key = &params.0.key;

        if key.trim().is_empty() {
            return Err(McpError::invalid_params(
                "memory key must not be empty",
                None,
            ));
        }

        let engine = self.engine.lock().await;
        match engine.memory_get(key) {
            Some(value) => Ok(CallToolResult::success(vec![Content::text(value)])),
            None => Ok(CallToolResult::success(vec![Content::text(format!(
                "Key not found: {key}"
            ))])),
        }
    }

    // -----------------------------------------------------------------------
    // Tool 29 — list_memory
    // -----------------------------------------------------------------------
    #[tool(
        name = "list_memory",
        description = "List all keys stored in the repository's persistent memory with their \
                       last-updated Unix timestamps. Keys are returned in lexicographic order."
    )]
    async fn list_memory(
        &self,
        _params: Parameters<ListMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

        let engine = self.engine.lock().await;
        let entries = engine.memory_list();

        if entries.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No memory entries found. Use `save_memory` to store key-value pairs.",
            )]));
        }

        let mut output = format!("## Persistent Memory ({} entries)\n\n", entries.len());
        for (key, updated_at) in &entries {
            writeln!(output, "- `{key}`: last updated {updated_at}").ok();
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }
}

/// SSE transport bridge — only compiled when the `sse` feature is enabled.
///
/// These methods are not part of the rmcp tool-router macro contract and must
/// live in a separate `impl` block so the `#[cfg]` gate can suppress them
/// entirely in stdio-only builds (avoids `dead_code` warnings).
#[cfg(feature = "sse")]
impl OmniContextServer {
    /// Create a new MCP server backed by a shared engine reference.
    ///
    /// Used by the SSE transport to share a single `Engine` instance across
    /// multiple concurrent SSE sessions without transferring ownership.
    pub fn new_shared(engine: Arc<Mutex<Engine>>) -> Self {
        Self {
            engine,
            tool_router: Self::tool_router(),
        }
    }

    /// Invoke a tool by name with JSON arguments and return the serialised result.
    ///
    /// Used by the SSE transport dispatcher to call tools without going through
    /// the rmcp stdio transport layer.  Each call acquires the engine lock for
    /// the duration of the tool execution.
    ///
    /// # Return value
    ///
    /// Returns `Ok(Vec<serde_json::Value>)` containing the MCP content items on
    /// success, or `Err(String)` with the error message on failure.
    pub async fn call_tool_json(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, String> {
        use rmcp::handler::server::wrapper::Parameters;

        fn to_json_items(result: &CallToolResult) -> Vec<serde_json::Value> {
            result
                .content
                .iter()
                .map(|c| serde_json::to_value(c).unwrap_or_else(|_| serde_json::json!({})))
                .collect()
        }

        macro_rules! call_with_params {
            ($param_ty:ty, $method:ident) => {{
                let params: $param_ty = serde_json::from_value(args)
                    .map_err(|e| format!("invalid arguments for {name}: {e}"))?;
                match self.$method(Parameters(params)).await {
                    Ok(result) => Ok(to_json_items(&result)),
                    Err(e) => Err(format!("{:?}: {}", e.code, e.message)),
                }
            }};
        }

        macro_rules! call_no_params {
            ($method:ident) => {{
                match self.$method().await {
                    Ok(result) => Ok(to_json_items(&result)),
                    Err(e) => Err(format!("{:?}: {}", e.code, e.message)),
                }
            }};
        }

        match name {
            "search_code" => call_with_params!(SearchCodeParams, search_code),
            "get_status" => call_no_params!(get_status),
            "get_file_summary" => call_with_params!(GetFileSummaryParams, get_file_summary),
            "get_symbol" => call_with_params!(GetSymbolParams, get_symbol),
            "get_module_map" => call_with_params!(GetModuleMapParams, get_module_map),
            "get_dependencies" => call_with_params!(GetDependenciesParams, get_dependencies),
            "get_blast_radius" => call_with_params!(GetBlastRadiusParams, get_blast_radius),
            "get_call_graph" => call_with_params!(GetCallGraphParams, get_call_graph),
            "get_recent_changes" => call_with_params!(GetRecentChangesParams, get_recent_changes),
            "get_branch_context" => call_with_params!(GetBranchContextParams, get_branch_context),
            "get_architecture" => call_no_params!(get_architecture),
            "explain_codebase" => call_no_params!(explain_codebase),
            "get_co_changes" => call_with_params!(GetCoChangesParams, get_co_changes),
            "search_by_intent" => call_with_params!(SearchByIntentParams, search_by_intent),
            "context_window" => call_with_params!(ContextWindowParams, context_window),
            "context_window_pack" => {
                call_with_params!(ContextWindowPackParams, context_window_pack)
            }
            "set_workspace" => call_with_params!(SetWorkspaceParams, set_workspace),
            "find_patterns" => call_with_params!(FindPatternsParams, find_patterns),
            "search_with_filter" => call_with_params!(SearchWithFilterParams, search_with_filter),
            "explain_symbol" => call_with_params!(ExplainSymbolParams, explain_symbol),
            "get_commit_summary" => call_with_params!(GetCommitSummaryParams, get_commit_summary),
            "search_commits" => call_with_params!(SearchCommitsParams, search_commits),
            "ingest_external_doc" => {
                call_with_params!(IngestExternalDocParams, ingest_external_doc)
            }
            "multi_repo_search" => call_with_params!(MultiRepoSearchParams, multi_repo_search),
            "save_memory" => call_with_params!(SaveMemoryParams, save_memory),
            "get_memory" => call_with_params!(GetMemoryParams, get_memory),
            "list_memory" => call_with_params!(ListMemoryParams, list_memory),
            "audit_plan" => call_with_params!(AuditPlanParams, audit_plan),
            "generate_manifest" => call_with_params!(GenerateManifestParams, generate_manifest),
            _ => Err(format!("unknown tool: {name}")),
        }
    }

    /// List all registered tools as `serde_json::Value` objects.
    ///
    /// Used by the SSE transport dispatcher for `tools/list` responses.
    /// Returns the same tool catalogue as the rmcp `tools/list` method, but
    /// serialised to plain JSON for direct HTTP responses.
    pub fn list_tools_json(&self) -> Vec<serde_json::Value> {
        self.tool_router
            .list_all()
            .iter()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name,
                    "description": tool.description,
                    "inputSchema": tool.input_schema
                })
            })
            .collect()
    }
}

#[tool_handler]
impl ServerHandler for OmniContextServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "OmniContext provides deep code intelligence for AI coding agents. \
                 It indexes source code into searchable chunks with full-text and semantic search. \
                 Use search_code for general queries, context_window for token-budget-aware context, \
                 get_symbol for specific lookups, get_file_summary for file structure, \
                 get_module_map for architecture overview, get_dependencies for symbol relationships, \
                 get_blast_radius for impact analysis, get_call_graph for dependency visualization, \
                 get_recent_changes for git history, search_by_intent for NL queries, \
                 get_branch_context for per-branch diff awareness, \
                 get_co_changes for co-change analysis, audit_plan for plan risk assessment, \
                 generate_manifest for project documentation, \
                 set_workspace to switch the active repository, \
                 search_with_filter for scoped language/path/type-filtered search, \
                 explain_symbol for comprehensive symbol documentation assembled from structured data, \
                 get_commit_summary for file/symbol commit history, \
                 search_commits for keyword search across commit messages, \
                 ingest_external_doc to index external API docs and wikis, \
                 context_window_pack for MMR-ordered token-budget-optimal context assembly, \
                 multi_repo_search for cross-repository searches, \
                 save_memory to persist key-value pairs across sessions, \
                 get_memory to retrieve a stored value by key, \
                 and list_memory to enumerate all stored memory keys with timestamps."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Query expansion for search_by_intent
// ---------------------------------------------------------------------------

/// Expand a natural language query into additional search terms.
///
/// Uses a static synonym map for common code concepts plus
/// structural decomposition of the query.
fn expand_query(query: &str) -> Vec<String> {
    let mut terms: Vec<String> = vec![query.to_string()];
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    seen.insert(query.to_string());

    // Static synonym expansions for common code concepts
    let synonyms: &[(&[&str], &[&str])] = &[
        (
            &["auth", "authentication", "login"],
            &[
                "authenticate", "verify", "credential", "token", "session", "password",
            ],
        ),
        (
            &["error", "exception", "failure"],
            &[
                "error", "err", "fail", "panic", "unwrap", "Result", "anyhow",
            ],
        ),
        (
            &["config", "configuration", "settings"],
            &["config", "Config", "settings", "options", "preferences"],
        ),
        (
            &["test", "testing"],
            &["test", "assert", "mock", "fixture", "expect"],
        ),
        (
            &["database", "db", "storage"],
            &[
                "database", "db", "sql", "query", "insert", "select", "connection",
            ],
        ),
        (
            &["api", "endpoint", "route"],
            &[
                "handler", "route", "endpoint", "request", "response", "middleware",
            ],
        ),
        (
            &["cache", "caching"],
            &["cache", "memoize", "ttl", "invalidate", "evict"],
        ),
        (
            &["parse", "parser", "parsing"],
            &["parse", "lexer", "tokenize", "ast", "syntax", "grammar"],
        ),
        (
            &["search", "find", "query"],
            &["search", "find", "lookup", "retrieve", "index", "match"],
        ),
        (
            &["serialize", "serialization"],
            &[
                "serialize", "deserialize", "json", "serde", "encode", "decode",
            ],
        ),
        (
            &["async", "concurrent", "parallel"],
            &["async", "await", "spawn", "tokio", "future", "thread"],
        ),
        (
            &["dependency", "import"],
            &["import", "use", "require", "include", "depend"],
        ),
    ];

    let lower = query.to_lowercase();
    for (triggers, expansions) in synonyms {
        if triggers.iter().any(|t| lower.contains(t)) {
            for exp in *expansions {
                let term = (*exp).to_string();
                if seen.insert(term.clone()) {
                    terms.push(term);
                }
            }
        }
    }

    // Extract potential symbol names (CamelCase, snake_case, paths with ::)
    for word in query.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != ':');
        if clean.len() > 2 && seen.insert(clean.to_string()) {
            terms.push(clean.to_string());
        }
    }

    terms
}
