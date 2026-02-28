//! Document and config file analyzer.
//!
//! Handles non-code files: Markdown, TOML, YAML, JSON, HTML, Shell.
//! These use section-based text chunking rather than AST parsing since
//! their structural elements are simpler (sections, keys, blocks).

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, Language, Visibility};

/// Text-based analyzer for documentation and configuration files.
///
/// Unlike code analyzers, this doesn't need a tree-sitter grammar.
/// It splits on structural boundaries (headings, keys, blank lines).
pub struct DocumentAnalyzer {
    lang: Language,
}

impl DocumentAnalyzer {
    /// Create a document analyzer for the given language/format.
    pub fn new(lang: Language) -> Self {
        Self { lang }
    }

    /// Parse Markdown into heading-delimited sections.
    fn parse_markdown(source: &str, module_name: &str) -> Vec<StructuralElement> {
        let mut elements = Vec::new();
        let mut current_heading: Option<String> = None;
        let mut current_content = String::new();
        let mut section_start: u32 = 1;

        for (i, line) in source.lines().enumerate() {
            let line_num = (i + 1) as u32;

            if line.starts_with('#') {
                // Flush previous section
                if current_heading.is_some() && !current_content.trim().is_empty() {
                    let name = current_heading.take().unwrap_or_default();
                    elements.push(StructuralElement {
                        symbol_path: format!("{module_name}.{name}"),
                        name,
                        kind: ChunkKind::Module,
                        visibility: Visibility::Public,
                        line_start: section_start,
                        line_end: line_num - 1,
                        content: current_content.clone(),
                        doc_comment: None,
                        references: Vec::new(),
                    });
                }

                // Start new section
                let heading = line.trim_start_matches('#').trim().to_string();
                current_heading = Some(heading);
                current_content.clear();
                current_content.push_str(line);
                current_content.push('\n');
                section_start = line_num;
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        // Flush last section
        let total_lines = source.lines().count() as u32;
        if let Some(name) = current_heading {
            if !current_content.trim().is_empty() {
                elements.push(StructuralElement {
                    symbol_path: format!("{module_name}.{name}"),
                    name,
                    kind: ChunkKind::Module,
                    visibility: Visibility::Public,
                    line_start: section_start,
                    line_end: total_lines,
                    content: current_content,
                    doc_comment: None,
                    references: Vec::new(),
                });
            }
        } else if !source.trim().is_empty() {
            // No headings -- treat entire file as one section
            elements.push(StructuralElement {
                symbol_path: module_name.to_string(),
                name: module_name.to_string(),
                kind: ChunkKind::TopLevel,
                visibility: Visibility::Public,
                line_start: 1,
                line_end: total_lines,
                content: source.to_string(),
                doc_comment: None,
                references: Vec::new(),
            });
        }

        elements
    }

    /// Parse TOML into top-level table sections.
    fn parse_toml(source: &str, module_name: &str) -> Vec<StructuralElement> {
        let mut elements = Vec::new();
        let mut current_table: Option<String> = None;
        let mut current_content = String::new();
        let mut section_start: u32 = 1;

        for (i, line) in source.lines().enumerate() {
            let line_num = (i + 1) as u32;
            let trimmed = line.trim();

            if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
                // [table] header
                if current_table.is_some() && !current_content.trim().is_empty() {
                    let name = current_table.take().unwrap_or_default();
                    elements.push(StructuralElement {
                        symbol_path: format!("{module_name}.{name}"),
                        name,
                        kind: ChunkKind::Module,
                        visibility: Visibility::Public,
                        line_start: section_start,
                        line_end: line_num - 1,
                        content: current_content.clone(),
                        doc_comment: None,
                        references: Vec::new(),
                    });
                }

                let name = trimmed
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .trim()
                    .to_string();
                current_table = Some(name);
                current_content.clear();
                current_content.push_str(line);
                current_content.push('\n');
                section_start = line_num;
            } else if trimmed.starts_with("[[") {
                // [[array-of-tables]] header
                if current_table.is_some() && !current_content.trim().is_empty() {
                    let name = current_table.take().unwrap_or_default();
                    elements.push(StructuralElement {
                        symbol_path: format!("{module_name}.{name}"),
                        name,
                        kind: ChunkKind::Module,
                        visibility: Visibility::Public,
                        line_start: section_start,
                        line_end: line_num - 1,
                        content: current_content.clone(),
                        doc_comment: None,
                        references: Vec::new(),
                    });
                }

                let name = trimmed
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .trim()
                    .to_string();
                current_table = Some(name);
                current_content.clear();
                current_content.push_str(line);
                current_content.push('\n');
                section_start = line_num;
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        // Flush last section
        let total_lines = source.lines().count() as u32;
        if let Some(name) = current_table {
            if !current_content.trim().is_empty() {
                elements.push(StructuralElement {
                    symbol_path: format!("{module_name}.{name}"),
                    name,
                    kind: ChunkKind::Module,
                    visibility: Visibility::Public,
                    line_start: section_start,
                    line_end: total_lines,
                    content: current_content,
                    doc_comment: None,
                    references: Vec::new(),
                });
            }
        } else if !source.trim().is_empty() {
            elements.push(StructuralElement {
                symbol_path: module_name.to_string(),
                name: module_name.to_string(),
                kind: ChunkKind::TopLevel,
                visibility: Visibility::Public,
                line_start: 1,
                line_end: total_lines,
                content: source.to_string(),
                doc_comment: None,
                references: Vec::new(),
            });
        }

        elements
    }

    /// Parse YAML/JSON/HTML/Shell as top-level blocks split by blank lines.
    fn parse_generic(source: &str, module_name: &str) -> Vec<StructuralElement> {
        // Split into blocks separated by blank lines
        let mut elements = Vec::new();
        let mut block = String::new();
        let mut block_start: u32 = 1;
        let mut block_idx = 0;

        for (i, line) in source.lines().enumerate() {
            let line_num = (i + 1) as u32;

            if line.trim().is_empty() && !block.trim().is_empty() {
                block_idx += 1;
                let name = format!("block_{block_idx}");
                elements.push(StructuralElement {
                    symbol_path: format!("{module_name}.{name}"),
                    name,
                    kind: ChunkKind::TopLevel,
                    visibility: Visibility::Public,
                    line_start: block_start,
                    line_end: line_num - 1,
                    content: block.clone(),
                    doc_comment: None,
                    references: Vec::new(),
                });
                block.clear();
                block_start = line_num + 1;
            } else {
                if block.is_empty() {
                    block_start = line_num;
                }
                block.push_str(line);
                block.push('\n');
            }
        }

        // Flush last block
        if !block.trim().is_empty() {
            block_idx += 1;
            let name = if block_idx == 1 {
                module_name.to_string()
            } else {
                format!("block_{block_idx}")
            };
            let total_lines = source.lines().count() as u32;
            elements.push(StructuralElement {
                symbol_path: format!("{module_name}.{name}"),
                name,
                kind: ChunkKind::TopLevel,
                visibility: Visibility::Public,
                line_start: block_start,
                line_end: total_lines,
                content: block,
                doc_comment: None,
                references: Vec::new(),
            });
        }

        elements
    }
}

impl LanguageAnalyzer for DocumentAnalyzer {
    fn language_id(&self) -> &str {
        self.lang.as_str()
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        // Document analyzer doesn't use tree-sitter for parsing,
        // but the trait requires this method.
        // Return a dummy -- the actual parsing bypasses tree-sitter entirely.
        // We use Markdown grammar as a placeholder since it's always available.
        tree_sitter_md::LANGUAGE.into()
    }

    fn extract_structure(
        &self,
        _tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> Vec<StructuralElement> {
        let source_str = std::str::from_utf8(source).unwrap_or("");
        let module_name_str = crate::parser::build_module_name_from_path(file_path);
        let module_name = &module_name_str;

        match self.lang {
            Language::Markdown => Self::parse_markdown(source_str, module_name),
            Language::Toml => Self::parse_toml(source_str, module_name),
            _ => Self::parse_generic(source_str, module_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::LanguageAnalyzer;

    fn parse_md(source: &str) -> Vec<StructuralElement> {
        let analyzer = DocumentAnalyzer::new(Language::Markdown);
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(source.as_bytes(), None).expect("parse");
        analyzer.extract_structure(&tree, source.as_bytes(), Path::new("README.md"))
    }

    #[test]
    fn test_markdown_sections() {
        let src = "# Introduction\n\nSome text.\n\n## Getting Started\n\nMore text.\n";
        let elements = parse_md(src);
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].name, "Introduction");
        assert_eq!(elements[1].name, "Getting Started");
    }

    #[test]
    fn test_markdown_no_headings() {
        let src = "Just some plain text without headings.\n";
        let elements = parse_md(src);
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].kind, ChunkKind::TopLevel);
    }

    fn parse_toml_str(source: &str) -> Vec<StructuralElement> {
        let analyzer = DocumentAnalyzer::new(Language::Toml);
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(source.as_bytes(), None).expect("parse");
        analyzer.extract_structure(&tree, source.as_bytes(), Path::new("Cargo.toml"))
    }

    #[test]
    fn test_toml_sections() {
        let src = "[package]\nname = \"foo\"\nversion = \"1.0\"\n\n[dependencies]\nserde = \"1\"\n";
        let elements = parse_toml_str(src);
        assert_eq!(elements.len(), 2);
        assert!(elements.iter().any(|e| e.name == "package"));
        assert!(elements.iter().any(|e| e.name == "dependencies"));
    }
}
