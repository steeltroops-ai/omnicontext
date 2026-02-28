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

/// Supported programming languages.
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
            Self::Unknown => "unknown",
        }
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
    /// RRF fusion score.
    pub rrf_score: f64,
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
