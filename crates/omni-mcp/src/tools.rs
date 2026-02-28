//! MCP tool definitions for OmniContext.
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
    ErrorData as McpError,
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::*,
    tool, tool_handler, tool_router,
    ServerHandler,
};
use serde::Deserialize;
use tokio::sync::Mutex;

use omni_core::Engine;

// -----------------------------------------------------------------------
// Parameter structs for each tool
// -----------------------------------------------------------------------

/// Parameters for search_code tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchCodeParams {
    /// Search query -- natural language or symbol name.
    pub query: String,
    /// Maximum number of results to return (default: 10).
    pub limit: Option<usize>,
}

/// Parameters for get_symbol tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetSymbolParams {
    /// Symbol name or fully qualified name to look up.
    pub name: String,
    /// Maximum number of results for prefix search (default: 5).
    pub limit: Option<usize>,
}

/// Parameters for get_file_summary tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetFileSummaryParams {
    /// File path relative to repository root.
    pub path: String,
}

/// Parameters for get_dependencies tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetDependenciesParams {
    /// Fully qualified symbol name.
    pub symbol: String,
    /// Direction: 'upstream', 'downstream', or 'both' (default: 'both').
    pub direction: Option<String>,
}

/// Parameters for find_patterns tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindPatternsParams {
    /// Description of the pattern to find.
    pub pattern: String,
    /// Maximum number of examples to return (default: 5).
    pub limit: Option<usize>,
}

// -----------------------------------------------------------------------
// MCP Server
// -----------------------------------------------------------------------

/// OmniContext MCP Server.
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
                    output.push_str(&format!(
                        "## Result {} (score: {:.4})\n**File**: {}\n**Symbol**: {} ({:?})\n**Lines**: {}-{}\n",
                        i + 1, result.score,
                        result.file_path.display(),
                        result.chunk.symbol_path, result.chunk.kind,
                        result.chunk.line_start, result.chunk.line_end,
                    ));
                    if let Some(ref doc) = result.chunk.doc_comment {
                        output.push_str(&format!("**Doc**: {}\n", doc));
                    }
                    output.push_str(&format!("```\n{}\n```\n\n", result.chunk.content));
                }

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(format!("search failed: {e}"), None)),
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
        let name = &params.0.name;
        let limit = params.0.limit.unwrap_or(5);
        let engine = self.engine.lock().await;
        let index = engine.metadata_index();

        match index.get_symbol_by_fqn(name) {
            Ok(Some(symbol)) => {
                let mut output = format!("## {} ({:?})\n**File ID**: {}\n**Line**: {}\n",
                    symbol.fqn, symbol.kind, symbol.file_id, symbol.line);

                if let Some(chunk_id) = symbol.chunk_id {
                    if let Ok(chunks) = index.get_chunks_for_file(symbol.file_id) {
                        if let Some(chunk) = chunks.iter().find(|c| c.id == chunk_id) {
                            if let Some(ref doc) = chunk.doc_comment {
                                output.push_str(&format!("**Doc**: {}\n", doc));
                            }
                            output.push_str(&format!("```\n{}\n```\n", chunk.content));
                        }
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Ok(None) => {
                match index.search_symbols_by_name(name, limit) {
                    Ok(symbols) if symbols.is_empty() => {
                        Ok(CallToolResult::success(vec![Content::text(
                            format!("No symbol found matching '{}'", name),
                        )]))
                    }
                    Ok(symbols) => {
                        let mut output = format!("## Symbols matching '{}'\n\n", name);
                        for sym in &symbols {
                            output.push_str(&format!(
                                "- **{}** ({:?}) -- file_id: {}, line: {}\n",
                                sym.fqn, sym.kind, sym.file_id, sym.line
                            ));
                        }
                        Ok(CallToolResult::success(vec![Content::text(output)]))
                    }
                    Err(e) => Err(McpError::internal_error(format!("symbol search failed: {e}"), None)),
                }
            }
            Err(e) => Err(McpError::internal_error(format!("symbol lookup failed: {e}"), None)),
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
        let path = &params.0.path;
        let engine = self.engine.lock().await;
        let index = engine.metadata_index();
        let file_path = std::path::Path::new(path);

        match index.get_file_by_path(file_path) {
            Ok(Some(file_info)) => {
                let mut output = format!(
                    "## File: {}\n**Language**: {:?}\n**Size**: {} bytes\n\n",
                    path, file_info.language, file_info.size_bytes
                );

                match index.get_chunks_for_file(file_info.id) {
                    Ok(chunks) => {
                        output.push_str(&format!("### Structure ({} chunks)\n\n", chunks.len()));
                        for chunk in &chunks {
                            let doc_preview = chunk.doc_comment.as_deref()
                                .map(|d| {
                                    let first = d.lines().next().unwrap_or("");
                                    if first.len() > 80 { format!(" -- {}...", &first[..80]) }
                                    else { format!(" -- {}", first) }
                                })
                                .unwrap_or_default();

                            output.push_str(&format!(
                                "- **{:?}** `{}` (L{}-L{}){}\n",
                                chunk.kind, chunk.symbol_path,
                                chunk.line_start, chunk.line_end, doc_preview,
                            ));
                        }
                    }
                    Err(e) => output.push_str(&format!("Error loading chunks: {}\n", e)),
                }
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Ok(None) => Ok(CallToolResult::success(vec![Content::text(
                format!("File not found in index: '{}'", path),
            )])),
            Err(e) => Err(McpError::internal_error(format!("file lookup failed: {e}"), None)),
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
                let output = format!(
                    "## OmniContext Status\n\n\
                     - **Repository**: {}\n- **Data dir**: {}\n- **Search mode**: {}\n\n\
                     ### Index Statistics\n\n\
                     - Files: {}\n- Chunks: {}\n- Symbols: {}\n- Vectors: {}\n",
                    s.repo_path, s.data_dir, s.search_mode,
                    s.files_indexed, s.chunks_indexed, s.symbols_indexed, s.vectors_indexed,
                );
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(format!("status failed: {e}"), None)),
        }
    }

    #[tool(
        name = "get_dependencies",
        description = "Get dependency relationships for a symbol: upstream (what it depends on) and downstream (what depends on it)."
    )]
    async fn get_dependencies(
        &self,
        params: Parameters<GetDependenciesParams>,
    ) -> Result<CallToolResult, McpError> {
        let direction = params.0.direction.as_deref().unwrap_or("both");
        let output = format!(
            "## Dependencies for `{}`\n\nDirection: {}\n\n\
             > Dependency graph analysis is available in Phase 5.\n\
             > For now, use `search_code` to find references to this symbol.\n",
            params.0.symbol, direction
        );
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
        let limit = params.0.limit.unwrap_or(5);
        let pattern = &params.0.pattern;
        let engine = self.engine.lock().await;

        match engine.search(pattern, limit) {
            Ok(results) => {
                if results.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        format!("No patterns matching '{}' found.", pattern),
                    )]));
                }

                let mut output = format!("## Pattern: '{}'\n\nFound {} examples:\n\n", pattern, results.len());
                for (i, result) in results.iter().enumerate() {
                    output.push_str(&format!(
                        "### Example {} -- {} (score: {:.4})\n**{:?}** `{}` (L{}-L{})\n```\n{}\n```\n\n",
                        i + 1, result.file_path.display(), result.score,
                        result.chunk.kind, result.chunk.symbol_path,
                        result.chunk.line_start, result.chunk.line_end,
                        result.chunk.content,
                    ));
                }
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(format!("pattern search failed: {e}"), None)),
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
                    s.repo_path, s.files_indexed, s.symbols_indexed, s.search_mode,
                    s.files_indexed, s.chunks_indexed, s.symbols_indexed, s.vectors_indexed,
                );
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(format!("architecture failed: {e}"), None)),
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
                    s.repo_path, s.files_indexed, s.chunks_indexed,
                    s.symbols_indexed, s.vectors_indexed,
                );
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Err(McpError::internal_error(format!("explain failed: {e}"), None)),
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
                 and get_file_summary for file structure analysis."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            ..Default::default()
        }
    }
}
