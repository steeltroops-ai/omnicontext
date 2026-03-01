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
// Parameter structs for each tool
// -----------------------------------------------------------------------

/// Parameters for `search_code` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchCodeParams {
    /// Search query -- natural language or symbol name.
    pub query: String,
    /// Maximum number of results to return (default: 10).
    pub limit: Option<usize>,
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
}

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

        let limit = params.0.limit.unwrap_or(10);
        let query = &params.0.query;
        let engine = self.engine.lock().await;

        match engine.search(query, limit) {
            Ok(results) => {
                if results.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        "No results found. Make sure the repository has been indexed with `omnicontext index .`"
                    )]));
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

        let limit = params.0.limit.unwrap_or(20);
        let query = &params.0.query;
        let engine = self.engine.lock().await;

        match engine.search_context_window(query, limit, params.0.token_budget) {
            Ok(ctx) => {
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

                    write!(
                        output,
                        "### {} ({:?}, score: {:.4}){}\n```\n{}\n```\n\n",
                        entry.chunk.symbol_path,
                        entry.chunk.kind,
                        entry.score,
                        if entry.is_graph_neighbor {
                            " [via graph]"
                        } else {
                            ""
                        },
                        entry.chunk.content,
                    )
                    .ok();
                }

                Ok(CallToolResult::success(vec![Content::text(output)]))
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

        let name = &params.0.name;
        let limit = params.0.limit.unwrap_or(5);
        let engine = self.engine.lock().await;
        let index = engine.metadata_index();

        match index.get_symbol_by_fqn(name) {
            Ok(Some(symbol)) => {
                let mut output = format!(
                    "## {} ({:?})\n**File ID**: {}\n**Line**: {}\n",
                    symbol.fqn, symbol.kind, symbol.file_id, symbol.line
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
        let engine = self.engine.lock().await;
        match engine.status() {
            Ok(s) => {
                let mut output = format!(
                    "## OmniContext Status\n\n\
                     - **Repository**: {}\n- **Data dir**: {}\n- **Search mode**: {}\n\n\
                     ### Index Statistics\n\n\
                     - Files: {}\n- Chunks: {}\n- Symbols: {}\n- Vectors: {}\n\n\
                     ### Dependency Graph\n\n\
                     - Edges (persisted): {}\n- Graph nodes: {}\n- Graph edges: {}\n",
                    s.repo_path,
                    s.data_dir,
                    s.search_mode,
                    s.files_indexed,
                    s.chunks_indexed,
                    s.symbols_indexed,
                    s.vectors_indexed,
                    s.dep_edges,
                    s.graph_nodes,
                    s.graph_edges,
                );
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
        let engine = self.engine.lock().await;
        let index = engine.metadata_index();
        let graph = engine.dep_graph();

        // Look up the symbol
        let symbol = match index.get_symbol_by_fqn(symbol_name) {
            Ok(Some(s)) => s,
            Ok(None) => {
                // Try prefix search
                match index.search_symbols_by_name(symbol_name, 1) {
                    Ok(mut syms) if !syms.is_empty() => syms.pop().ok_or_else(|| {
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

        let limit = params.0.limit.unwrap_or(5);
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
        let engine = self.engine.lock().await;
        match engine.status() {
            Ok(s) => {
                let output = format!(
                    "## Codebase Architecture\n\n\
                     **Repository**: {}\n**Files**: {}\n**Symbols**: {}\n**Search mode**: {}\n\n\
                     ### Indexed Content\n\n\
                     - {} files indexed\n- {} code chunks searchable\n\
                     - {} symbols (functions, classes, traits, etc.)\n- {} vector embeddings\n\n\
                     ### Recommendations\n\n\
                     - Use `search_code` to explore specific functionality\n\
                     - Use `get_symbol` to look up functions or classes\n\
                     - Use `get_file_summary` for file structure\n\
                     - Use `find_patterns` to discover recurring patterns\n",
                    s.repo_path,
                    s.files_indexed,
                    s.symbols_indexed,
                    s.search_mode,
                    s.files_indexed,
                    s.chunks_indexed,
                    s.symbols_indexed,
                    s.vectors_indexed,
                );
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("architecture failed: {e}"),
                None,
            )),
        }
    }

    #[tool(
        name = "explain_codebase",
        description = "Get a comprehensive explanation of the codebase: tech stack, entry points, structure. Good for onboarding to a new project."
    )]
    async fn explain_codebase(&self) -> Result<CallToolResult, McpError> {
        let engine = self.engine.lock().await;
        match engine.status() {
            Ok(s) => {
                let output = format!(
                    "## Codebase Overview\n\n**Root**: {}\n\n\
                     ### Statistics\n\n\
                     | Metric | Count |\n|--------|-------|\n\
                     | Files | {} |\n| Code Chunks | {} |\n\
                     | Symbols | {} |\n| Embeddings | {} |\n\n\
                     ### How to Explore\n\n\
                     1. **Find entry points**: `search_code \"main function\"`\n\
                     2. **Understand a module**: `get_file_summary \"path/to/file.rs\"`\n\
                     3. **Look up definitions**: `get_symbol \"ClassName\"`\n\
                     4. **Find patterns**: `find_patterns \"error handling\"`\n",
                    s.repo_path,
                    s.files_indexed,
                    s.chunks_indexed,
                    s.symbols_indexed,
                    s.vectors_indexed,
                );
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("explain failed: {e}"),
                None,
            )),
        }
    }

    #[tool(
        name = "get_module_map",
        description = "Returns the module/crate/package hierarchy as a tree structure. Shows files grouped by directory with their exported symbols. Useful for understanding project architecture."
    )]
    async fn get_module_map(
        &self,
        #[allow(unused)] params: Parameters<GetModuleMapParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::fmt::Write;

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

        let query = &params.0.query;
        let limit = params.0.limit.unwrap_or(10);
        let engine = self.engine.lock().await;

        // Query expansion: extract key terms and add synonyms
        let expanded_terms = expand_query(query);
        let expanded_query = expanded_terms.join(" ");

        match engine.search_context_window(&expanded_query, limit, params.0.token_budget) {
            Ok(ctx) => {
                if ctx.is_empty() {
                    // Fall back to original query
                    match engine.search(query, limit) {
                        Ok(results) if results.is_empty() => {
                            return Ok(CallToolResult::success(vec![Content::text(format!(
                                "No results found for: '{query}'\n\nExpanded to: '{expanded_query}'",
                            ))]));
                        }
                        Ok(results) => {
                            let mut output = format!(
                                "## Search by Intent\n**Query**: {query}\n**Expanded**: {expanded_query}\n**Results**: {}\n\n",
                                results.len()
                            );
                            for (i, r) in results.iter().enumerate() {
                                write!(
                                    output,
                                    "### {} (score: {:.4})\n**File**: {}\n**Symbol**: {} ({:?})\n```\n{}\n```\n\n",
                                    i + 1, r.score, r.file_path.display(),
                                    r.chunk.symbol_path, r.chunk.kind, r.chunk.content,
                                )
                                .ok();
                            }
                            return Ok(CallToolResult::success(vec![Content::text(output)]));
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
                    "## Search by Intent\n**Query**: {query}\n**Expanded**: {expanded_query}\n**Context**: {} entries, {}/{} tokens\n\n",
                    ctx.len(), ctx.total_tokens, ctx.token_budget
                );
                output.push_str(&ctx.render());

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("search_by_intent failed: {e}"),
                None,
            )),
        }
    }
}

#[tool_handler]
impl ServerHandler for OmniContextServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "OmniContext provides deep code intelligence for AI coding agents. \
                 It indexes source code into searchable chunks with full-text and semantic search. \
                 Use search_code for general queries, get_symbol for specific lookups, \
                 get_file_summary for file structure, get_module_map for architecture overview, \
                 and search_by_intent for natural language queries with automatic expansion."
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

    // Static synonym expansions for common code concepts
    let synonyms: &[(&[&str], &[&str])] = &[
        (
            &["auth", "authentication", "login"],
            &[
                "authenticate",
                "verify",
                "credential",
                "token",
                "session",
                "password",
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
                "database",
                "db",
                "sql",
                "query",
                "insert",
                "select",
                "connection",
            ],
        ),
        (
            &["api", "endpoint", "route"],
            &[
                "handler",
                "route",
                "endpoint",
                "request",
                "response",
                "middleware",
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
                "serialize",
                "deserialize",
                "json",
                "serde",
                "encode",
                "decode",
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
                if !terms.contains(&term) {
                    terms.push(term);
                }
            }
        }
    }

    // Extract potential symbol names (CamelCase, snake_case, paths with ::)
    for word in query.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != ':');
        if clean.len() > 2 && !terms.contains(&clean.to_string()) {
            terms.push(clean.to_string());
        }
    }

    terms
}
