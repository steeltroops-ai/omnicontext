//! CSS language analyzer.
//!
//! Extracts structural elements from CSS/SCSS source files using tree-sitter.

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, Visibility};

/// Analyzer for CSS/SCSS source files.
pub struct CssAnalyzer;

impl LanguageAnalyzer for CssAnalyzer {
    fn language_id(&self) -> &str {
        "css"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_css::LANGUAGE.into()
    }

    fn extract_structure(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> Vec<StructuralElement> {
        let mut elements = Vec::new();
        let module_name_str = crate::parser::build_module_name_from_path(file_path);
        let module_name = &module_name_str;

        let root = tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            match child.kind() {
                "rule_set" => {
                    // Extract selector -- try field name first, fall back to first child
                    let selector_text = child
                        .child_by_field_name("selectors")
                        .map(|n| node_text(n, source).to_string())
                        .or_else(|| {
                            // Fallback: first named child that isn't a block
                            let mut inner = child.walk();
                            let result = child
                                .named_children(&mut inner)
                                .find(|c| c.kind() != "block")
                                .map(|c| node_text(c, source).to_string());
                            result
                        })
                        .or_else(|| {
                            // Last resort: text before first '{'
                            let full = node_text(child, source);
                            full.split('{').next().map(|s| s.trim().to_string())
                        });

                    if let Some(sel_text) = selector_text {
                        let name = sel_text
                            .lines()
                            .next()
                            .unwrap_or(&sel_text)
                            .trim()
                            .to_string();
                        if !name.is_empty() {
                            elements.push(StructuralElement {
                                symbol_path: format!("{module_name}.{name}"),
                                name,
                                kind: ChunkKind::Class,
                                visibility: Visibility::Public,
                                line_start: child.start_position().row as u32 + 1,
                                line_end: child.end_position().row as u32 + 1,
                                content: node_text(child, source).to_string(),
                                doc_comment: None,
                                references: Vec::new(),
                                extends: Vec::new(),
                                implements: Vec::new(),
                            });
                        }
                    }
                }
                "media_statement" | "keyframes_statement" | "supports_statement" => {
                    let name = node_text(child, source)
                        .lines()
                        .next()
                        .unwrap_or("@rule")
                        .trim()
                        .to_string();

                    elements.push(StructuralElement {
                        symbol_path: format!("{module_name}.{name}"),
                        name,
                        kind: ChunkKind::Module,
                        visibility: Visibility::Public,
                        line_start: child.start_position().row as u32 + 1,
                        line_end: child.end_position().row as u32 + 1,
                        content: node_text(child, source).to_string(),
                        doc_comment: None,
                        references: Vec::new(),
                        extends: Vec::new(),
                        implements: Vec::new(),
                    });
                }
                _ => {}
            }
        }

        elements
    }
}

fn node_text<'a>(node: tree_sitter::Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::LanguageAnalyzer;

    fn parse_css(source: &str) -> Vec<StructuralElement> {
        let analyzer = CssAnalyzer;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(source.as_bytes(), None).expect("parse");
        analyzer.extract_structure(&tree, source.as_bytes(), Path::new("styles.css"))
    }

    #[test]
    fn test_css_rule() {
        let src = ".container { display: flex; }";
        let elements = parse_css(src);
        assert!(!elements.is_empty());
        assert!(elements.iter().any(|e| e.name.contains("container")));
    }
}
