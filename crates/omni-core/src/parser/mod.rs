//! AST parsing subsystem using tree-sitter.
//!
//! This module provides language-agnostic AST parsing with per-language
//! structural extractors. Each supported language registers an analyzer
//! that maps tree-sitter AST nodes to OmniContext structural elements.
//!
//! ## Architecture
//!
//! ```text
//! Source File -> Language Detection -> tree-sitter Grammar
//!            -> Incremental Parse -> CST
//!            -> Structural Extraction -> Vec<StructuralElement>
//! ```
//!
//! The parser is stateless and can be invoked from multiple threads
//! via `spawn_blocking`.

pub mod registry;
pub mod languages;

use std::path::Path;

use crate::error::OmniResult;
use crate::types::{ChunkKind, ImportStatement, Language, Visibility};

/// A structural element extracted from an AST.
#[derive(Debug, Clone)]
pub struct StructuralElement {
    /// Fully qualified name of this element.
    pub symbol_path: String,
    /// Short name (last component of symbol_path).
    pub name: String,
    /// What kind of construct this is.
    pub kind: ChunkKind,
    /// Visibility specifier.
    pub visibility: Visibility,
    /// Starting line (1-indexed).
    pub line_start: u32,
    /// Ending line (1-indexed, inclusive).
    pub line_end: u32,
    /// Raw source code of this element.
    pub content: String,
    /// Extracted doc comment, if present.
    pub doc_comment: Option<String>,
    /// Symbols referenced within this element (for dependency extraction).
    pub references: Vec<String>,
}

/// Trait that each language analyzer must implement.
pub trait LanguageAnalyzer: Send + Sync {
    /// Returns the language identifier (e.g., "python", "rust").
    fn language_id(&self) -> &str;

    /// Returns the tree-sitter `Language` for this analyzer.
    fn tree_sitter_language(&self) -> tree_sitter::Language;

    /// Extract structural elements from a parsed tree.
    fn extract_structure(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> Vec<StructuralElement>;

    /// Extract import statements from a parsed tree for dependency graph construction.
    ///
    /// Default implementation returns empty (languages can override).
    fn extract_imports(
        &self,
        _tree: &tree_sitter::Tree,
        _source: &[u8],
        _file_path: &Path,
    ) -> Vec<ImportStatement> {
        Vec::new()
    }
}

/// Parse a source file and extract its structural elements.
///
/// This is the primary entry point for the parser. It:
/// 1. Detects the language from the file extension
/// 2. Loads the appropriate tree-sitter grammar
/// 3. Parses the source code
/// 4. Extracts structural elements via the language analyzer
pub fn parse_file(
    file_path: &Path,
    source: &[u8],
    language: Language,
) -> OmniResult<Vec<StructuralElement>> {
    let registry = registry::global_registry();

    let analyzer = registry.get(language).ok_or_else(|| {
        crate::error::OmniError::Parse {
            path: file_path.to_path_buf(),
            message: format!("no analyzer registered for language: {language}"),
        }
    })?;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&analyzer.tree_sitter_language())
        .map_err(|e| crate::error::OmniError::Parse {
            path: file_path.to_path_buf(),
            message: format!("failed to set tree-sitter language: {e}"),
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        crate::error::OmniError::Parse {
            path: file_path.to_path_buf(),
            message: "tree-sitter returned None (parse timeout or cancellation)".into(),
        }
    })?;

    Ok(analyzer.extract_structure(&tree, source, file_path))
}

/// Extract import statements from a source file.
///
/// Uses the same tree-sitter parse infrastructure as `parse_file`.
pub fn parse_imports(
    file_path: &Path,
    source: &[u8],
    language: Language,
) -> OmniResult<Vec<ImportStatement>> {
    let registry = registry::global_registry();

    let analyzer = registry.get(language).ok_or_else(|| {
        crate::error::OmniError::Parse {
            path: file_path.to_path_buf(),
            message: format!("no analyzer registered for language: {language}"),
        }
    })?;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&analyzer.tree_sitter_language())
        .map_err(|e| crate::error::OmniError::Parse {
            path: file_path.to_path_buf(),
            message: format!("failed to set tree-sitter language: {e}"),
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        crate::error::OmniError::Parse {
            path: file_path.to_path_buf(),
            message: "tree-sitter returned None".into(),
        }
    })?;

    Ok(analyzer.extract_imports(&tree, source, file_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_unknown_language_returns_error() {
        let result = parse_file(
            Path::new("test.xyz"),
            b"hello world",
            Language::Unknown,
        );
        assert!(result.is_err());
    }
}
