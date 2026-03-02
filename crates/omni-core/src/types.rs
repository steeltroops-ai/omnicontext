//! Core domain types shared across all omni-core subsystems.
//!
//! These types form the API contract between modules. Changing them
//! requires updating all consumers, so they should be stable and minimal.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// File-level types
// ---------------------------------------------------------------------------

/// Metadata about an indexed file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Database ID.
    pub id: i64,
    /// Path relative to the repository root.
    pub path: PathBuf,
    /// Detected programming language.
    pub language: Language,
    /// SHA-256 hash of file content at time of indexing.
    pub content_hash: String,
    /// File size in bytes.
    pub size_bytes: u64,
}

// ---------------------------------------------------------------------------
// Language
// ---------------------------------------------------------------------------

/// Supported programming languages and document formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Python (.py)
    Python,
    /// TypeScript (.ts, .tsx)
    TypeScript,
    /// JavaScript (.js, .jsx)
    JavaScript,
    /// Rust (.rs)
    Rust,
    /// Go (.go)
    Go,
    /// Java (.java)
    Java,
    /// C (.c, .h)
    C,
    /// C++ (.cpp, .cc, .cxx, .hpp, .hxx)
    Cpp,
    /// C# (.cs)
    CSharp,
    /// CSS / SCSS (.css, .scss)
    Css,
    /// Ruby (.rb)
    Ruby,
    /// PHP (.php)
    Php,
    /// Swift (.swift)
    Swift,
    /// Kotlin (.kt, .kts)
    Kotlin,
    /// HTML (.html, .htm)
    Html,
    /// Shell / Bash (.sh, .bash, .zsh)
    Shell,
    /// Markdown (.md, .mdx)
    Markdown,
    /// TOML configuration (.toml)
    Toml,
    /// YAML configuration (.yml, .yaml)
    Yaml,
    /// JSON data (.json, .jsonc)
    Json,
    /// Unknown / unsupported
    Unknown,
}

impl Language {
    /// Detect language from file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "py" => Self::Python,
            "ts" | "tsx" => Self::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "rs" => Self::Rust,
            "go" => Self::Go,
            "java" => Self::Java,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Self::Cpp,
            "cs" => Self::CSharp,
            "css" | "scss" => Self::Css,
            "rb" => Self::Ruby,
            "php" => Self::Php,
            "swift" => Self::Swift,
            "kt" | "kts" => Self::Kotlin,
            "html" | "htm" => Self::Html,
            "sh" | "bash" | "zsh" => Self::Shell,
            "md" | "mdx" => Self::Markdown,
            "toml" => Self::Toml,
            "yml" | "yaml" => Self::Yaml,
            "json" | "jsonc" => Self::Json,
            _ => Self::Unknown,
        }
    }

    /// Returns the language identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Rust => "rust",
            Self::Go => "go",
            Self::Java => "java",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::CSharp => "csharp",
            Self::Css => "css",
            Self::Ruby => "ruby",
            Self::Php => "php",
            Self::Swift => "swift",
            Self::Kotlin => "kotlin",
            Self::Html => "html",
            Self::Shell => "shell",
            Self::Markdown => "markdown",
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Unknown => "unknown",
        }
    }

    /// Returns true if this is an AST-parseable programming language.
    pub fn is_code(&self) -> bool {
        matches!(
            self,
            Self::Python
                | Self::TypeScript
                | Self::JavaScript
                | Self::Rust
                | Self::Go
                | Self::Java
                | Self::C
                | Self::Cpp
                | Self::CSharp
                | Self::Css
                | Self::Ruby
                | Self::Php
                | Self::Swift
                | Self::Kotlin
        )
    }

    /// Returns true if this is a documentation or config format.
    pub fn is_document(&self) -> bool {
        matches!(
            self,
            Self::Markdown | Self::Toml | Self::Yaml | Self::Json | Self::Html | Self::Shell
        )
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Chunk types
// ---------------------------------------------------------------------------

/// The kind of code construct a chunk represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkKind {
    /// Function or method definition.
    Function,
    /// Class, struct, or record definition.
    Class,
    /// Trait, interface, or protocol definition.
    Trait,
    /// Implementation block (Rust `impl`, Java anonymous class, etc.).
    Impl,
    /// Constant or static variable.
    Const,
    /// Type alias or definition.
    TypeDef,
    /// Module or namespace declaration.
    Module,
    /// Test function or test block.
    Test,
    /// Top-level statements that don't fit other categories.
    TopLevel,
}

impl ChunkKind {
    /// Returns the default structural importance weight for this kind.
    pub fn default_weight(&self) -> f64 {
        match self {
            Self::Function => 0.85,
            Self::Class => 0.95,
            Self::Trait => 0.95,
            Self::Impl => 0.85,
            Self::Const => 0.70,
            Self::TypeDef => 0.90,
            Self::Module => 0.60,
            Self::Test => 0.60,
            Self::TopLevel => 0.50,
        }
    }

    /// Convert to database string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Class => "class",
            Self::Trait => "trait",
            Self::Impl => "impl",
            Self::Const => "const",
            Self::TypeDef => "typedef",
            Self::Module => "module",
            Self::Test => "test",
            Self::TopLevel => "top_level",
        }
    }

    /// Parse from database string.
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "function" => Self::Function,
            "class" => Self::Class,
            "trait" => Self::Trait,
            "impl" => Self::Impl,
            "const" => Self::Const,
            "typedef" => Self::TypeDef,
            "module" => Self::Module,
            "test" => Self::Test,
            _ => Self::TopLevel,
        }
    }
}

/// Visibility of a code symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Accessible from outside the module/crate.
    Public,
    /// Accessible only within the current crate/package.
    Crate,
    /// Accessible from parent class or subclasses.
    Protected,
    /// Accessible only within the defining scope.
    Private,
}

impl Visibility {
    /// Convert to database string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Crate => "crate",
            Self::Protected => "protected",
            Self::Private => "private",
        }
    }

    /// Parse from database string.
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "public" => Self::Public,
            "crate" => Self::Crate,
            "protected" => Self::Protected,
            "private" => Self::Private,
            _ => Self::Private,
        }
    }

    /// Returns a weight multiplier for public vs private apis.
    pub fn weight_multiplier(&self) -> f64 {
        match self {
            Self::Public => 1.0,
            Self::Crate => 0.9,
            Self::Protected => 0.85,
            Self::Private => 0.70,
        }
    }
}

/// A semantically meaningful chunk of code extracted from a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Database ID (0 if not yet persisted).
    pub id: i64,
    /// ID of the parent file in the index.
    pub file_id: i64,
    /// Fully qualified symbol path (e.g., `crate::auth::middleware::validate_token`).
    pub symbol_path: String,
    /// What kind of code construct this is.
    pub kind: ChunkKind,
    /// Visibility of the symbol.
    pub visibility: Visibility,
    /// Starting line number (1-indexed).
    pub line_start: u32,
    /// Ending line number (1-indexed, inclusive).
    pub line_end: u32,
    /// The source code content of this chunk.
    pub content: String,
    /// Extracted doc comment, if any.
    pub doc_comment: Option<String>,
    /// Estimated token count for this chunk.
    pub token_count: u32,
    /// Structural importance weight (0.0 - 1.0).
    pub weight: f64,
    /// ID of the corresponding vector in usearch (None if not yet embedded).
    pub vector_id: Option<u64>,
}

// ---------------------------------------------------------------------------
// Symbol types
// ---------------------------------------------------------------------------

/// A resolved symbol in the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Database ID.
    pub id: i64,
    /// Short name (e.g., `validate_token`).
    pub name: String,
    /// Fully qualified name (e.g., `crate::auth::middleware::validate_token`).
    pub fqn: String,
    /// What kind of symbol this is.
    pub kind: ChunkKind,
    /// File this symbol is defined in.
    pub file_id: i64,
    /// Line number of definition.
    pub line: u32,
    /// Associated chunk ID, if the full definition was chunked.
    pub chunk_id: Option<i64>,
}

// ---------------------------------------------------------------------------
// Dependency edge types
// ---------------------------------------------------------------------------

/// The kind of dependency relationship between two symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    /// File/module A imports module B.
    Imports,
    /// Function A calls function B.
    Calls,
    /// Class A extends/inherits from class B.
    Extends,
    /// Struct/class A implements trait/interface B.
    Implements,
    /// Function A uses type B as parameter or return type.
    UsesType,
    /// Function A creates an instance of struct/class B.
    Instantiates,
    /// Function A accesses a field of struct B.
    FieldAccess,
}

impl DependencyKind {
    /// Convert to database string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Imports => "imports",
            Self::Calls => "calls",
            Self::Extends => "extends",
            Self::Implements => "implements",
            Self::UsesType => "uses_type",
            Self::Instantiates => "instantiates",
            Self::FieldAccess => "field_access",
        }
    }

    /// Parse from database string.
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "imports" => Self::Imports,
            "calls" => Self::Calls,
            "extends" => Self::Extends,
            "implements" => Self::Implements,
            "uses_type" => Self::UsesType,
            "instantiates" => Self::Instantiates,
            "field_access" => Self::FieldAccess,
            _ => Self::Calls, // fallback
        }
    }
}

/// A directed edge in the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Source symbol ID.
    pub source_id: i64,
    /// Target symbol ID.
    pub target_id: i64,
    /// Kind of dependency.
    pub kind: DependencyKind,
}

/// An import statement extracted from source code.
///
/// Used for dependency graph construction. Each import is later resolved
/// to a target symbol in the index.
#[derive(Debug, Clone)]
pub struct ImportStatement {
    /// The raw import path (e.g., "os.path", "crate::config", "./utils").
    pub import_path: String,
    /// Optional specific names imported (e.g., ["Config", "load"]).
    pub imported_names: Vec<String>,
    /// Line number where the import appears.
    pub line: u32,
    /// Kind of dependency this import represents.
    pub kind: DependencyKind,
}

// ---------------------------------------------------------------------------
// Search types
// ---------------------------------------------------------------------------

/// A search result with scoring details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The matched chunk.
    pub chunk: Chunk,
    /// File path of the matched chunk.
    pub file_path: PathBuf,
    /// Overall relevance score (higher is better).
    pub score: f64,
    /// Breakdown of how the score was computed (for debugging).
    pub score_breakdown: ScoreBreakdown,
}

/// Detailed scoring breakdown for a search result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Rank from semantic (vector) search (None if keyword-only match).
    pub semantic_rank: Option<u32>,
    /// Rank from keyword (FTS5) search (None if semantic-only match).
    pub keyword_rank: Option<u32>,
    /// RRF fusion score (before reranking).
    pub rrf_score: f64,
    /// Cross-encoder reranker score (None if not reranked).
    pub reranker_score: Option<f64>,
    /// Structural importance weight applied.
    pub structural_weight: f64,
    /// Dependency proximity boost applied.
    pub dependency_boost: f64,
    /// Recency boost applied.
    pub recency_boost: f64,
}

// ---------------------------------------------------------------------------
// Pipeline events
// ---------------------------------------------------------------------------

/// Events flowing through the indexing pipeline.
#[derive(Debug, Clone)]
pub enum PipelineEvent {
    /// A file was created or modified and needs (re-)indexing.
    FileChanged {
        /// Absolute path to the file.
        path: PathBuf,
    },
    /// A file was deleted and should be removed from the index.
    FileDeleted {
        /// Absolute path to the deleted file.
        path: PathBuf,
    },
    /// A full repository scan is requested.
    FullScan,
    /// Shutdown the pipeline gracefully.
    Shutdown,
}

// ---------------------------------------------------------------------------
// Context assembly types
// ---------------------------------------------------------------------------

/// Priority level for chunks in context assembly.
///
/// Used to pack maximum relevant context within token budget by
/// prioritizing critical chunks and compressing low-priority ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkPriority {
    /// Critical context: active file, cursor context, direct dependencies.
    /// Always included, never compressed.
    Critical = 4,
    /// High relevance: search results with score >0.8, test files.
    /// Included if space available, minimal compression.
    High = 3,
    /// Medium relevance: search results with score 0.5-0.8, related files.
    /// Included if space available, moderate compression.
    Medium = 2,
    /// Low relevance: architectural context, documentation, distant dependencies.
    /// Included only if space available, aggressive compression.
    Low = 1,
}

impl ChunkPriority {
    /// Determine priority from search score and context flags.
    pub fn from_score_and_context(
        score: f64,
        is_active_file: bool,
        is_test: bool,
        is_graph_neighbor: bool,
    ) -> Self {
        if is_active_file {
            return Self::Critical;
        }

        if is_test {
            return Self::High;
        }

        if is_graph_neighbor {
            return Self::Medium;
        }

        // Score-based priority
        if score >= 0.8 {
            Self::High
        } else if score >= 0.5 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    /// Compression factor for this priority (0.0 = no compression, 1.0 = maximum).
    pub fn compression_factor(&self) -> f64 {
        match self {
            Self::Critical => 0.0, // Never compress
            Self::High => 0.1,     // Minimal compression (10%)
            Self::Medium => 0.3,   // Moderate compression (30%)
            Self::Low => 0.6,      // Aggressive compression (60%)
        }
    }
}

/// A token-budget-aware context window assembled from search results.
///
/// Groups chunks by file and includes graph-neighbor chunks for
/// maximum relevant context within a fixed token budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    /// Ordered entries (highest score first).
    pub entries: Vec<ContextEntry>,
    /// Total tokens consumed.
    pub total_tokens: u32,
    /// Token budget this window was assembled for.
    pub token_budget: u32,
}

/// A single entry in a context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    /// File path of this chunk.
    pub file_path: PathBuf,
    /// The code chunk.
    pub chunk: Chunk,
    /// Relevance score.
    pub score: f64,
    /// Whether this chunk was included via graph traversal (not direct search match).
    pub is_graph_neighbor: bool,
    /// Priority level for this chunk.
    #[serde(default)]
    pub priority: Option<ChunkPriority>,
}

impl ContextWindow {
    /// Render the context window as a single string suitable for LLM consumption.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let mut current_file: Option<&std::path::Path> = None;

        for entry in &self.entries {
            if current_file != Some(&entry.file_path) {
                if current_file.is_some() {
                    out.push_str("\n\n");
                }
                out.push_str(&format!("// === {} ===\n", entry.file_path.display()));
                current_file = Some(&entry.file_path);
            }
            out.push_str(&entry.chunk.content);
            out.push('\n');
        }

        out
    }

    /// Number of entries in this window.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the window is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
