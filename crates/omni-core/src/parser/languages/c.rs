//! C language analyzer.
//!
//! Extracts structural elements from C source files using tree-sitter.

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, DependencyKind, ImportStatement, Visibility};

/// Analyzer for C source files.
pub struct CAnalyzer;

impl LanguageAnalyzer for CAnalyzer {
    fn language_id(&self) -> &str {
        "c"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_c::LANGUAGE.into()
    }

    fn extract_structure(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> Vec<StructuralElement> {
        let mut elements = Vec::new();
        let module_name_str = crate::parser::build_module_name_from_path(file_path).replace("/", "::");
        let module_name = &module_name_str;

        let root = tree.root_node();
        self.walk_node(root, source, module_name, &mut elements);
        elements
    }

    fn extract_imports(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        _file_path: &Path,
    ) -> Vec<ImportStatement> {
        let mut imports = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if child.kind() == "preproc_include" {
                let line = child.start_position().row as u32 + 1;
                if let Some(path_node) = child.child_by_field_name("path") {
                    let path = node_text(path_node, source)
                        .trim_matches(|c: char| c == '"' || c == '<' || c == '>')
                        .to_string();
                    if !path.is_empty() {
                        imports.push(ImportStatement {
                            import_path: path,
                            imported_names: vec![],
                            line,
                            kind: DependencyKind::Imports,
                        });
                    }
                }
            }
        }

        imports
    }
}

impl CAnalyzer {
    fn walk_node(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        elements: &mut Vec<StructuralElement>,
    ) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_definition" => {
                    if let Some(declarator) = child.child_by_field_name("declarator") {
                        if let Some(name) = extract_function_name(declarator, source) {
                            let is_static = is_static_declaration(child, source);
                            let doc = extract_c_doc(child, source);

                            elements.push(StructuralElement {
                                symbol_path: format!("{module_name}.{name}"),
                                name,
                                kind: ChunkKind::Function,
                                visibility: if is_static {
                                    Visibility::Private
                                } else {
                                    Visibility::Public
                                },
                                line_start: child.start_position().row as u32 + 1,
                                line_end: child.end_position().row as u32 + 1,
                                content: node_text(child, source).to_string(),
                                doc_comment: doc,
                                references: Vec::new(),
                extends: Vec::new(),
                implements: Vec::new(),
                            });
                        }
                    }
                }
                "struct_specifier" | "union_specifier" | "enum_specifier" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = node_text(name_node, source).to_string();
                        let kind = match child.kind() {
                            "enum_specifier" => ChunkKind::TypeDef,
                            _ => ChunkKind::Class,
                        };
                        let doc = extract_c_doc(child, source);

                        elements.push(StructuralElement {
                            symbol_path: format!("{module_name}.{name}"),
                            name,
                            kind,
                            visibility: Visibility::Public,
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            content: node_text(child, source).to_string(),
                            doc_comment: doc,
                            references: Vec::new(),
                extends: Vec::new(),
                implements: Vec::new(),
                        });
                    }
                }
                "type_definition" => {
                    // typedef struct { ... } Name;
                    let text = node_text(child, source).to_string();
                    // Get the last identifier before the semicolon
                    if let Some(name_node) = child.child_by_field_name("declarator") {
                        let name = node_text(name_node, source).to_string();
                        if !name.is_empty() {
                            elements.push(StructuralElement {
                                symbol_path: format!("{module_name}.{name}"),
                                name,
                                kind: ChunkKind::TypeDef,
                                visibility: Visibility::Public,
                                line_start: child.start_position().row as u32 + 1,
                                line_end: child.end_position().row as u32 + 1,
                                content: text,
                                doc_comment: None,
                                references: Vec::new(),
                extends: Vec::new(),
                implements: Vec::new(),
                            });
                        }
                    }
                }
                "preproc_def" | "preproc_function_def" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = node_text(name_node, source).to_string();
                        elements.push(StructuralElement {
                            symbol_path: format!("{module_name}.{name}"),
                            name,
                            kind: ChunkKind::Const,
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
                _ => {
                    if child.child_count() > 0 && !child.kind().starts_with("preproc_") {
                        self.walk_node(child, source, module_name, elements);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn node_text<'a>(node: tree_sitter::Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

fn extract_function_name(declarator: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    // function_declarator -> declarator (identifier)
    match declarator.kind() {
        "function_declarator" => {
            declarator
                .child_by_field_name("declarator")
                .map(|n| node_text(n, source).to_string())
        }
        "pointer_declarator" => {
            // *func_name(...)
            let mut cursor = declarator.walk();
            for child in declarator.children(&mut cursor) {
                if let Some(name) = extract_function_name(child, source) {
                    return Some(name);
                }
            }
            None
        }
        "identifier" => Some(node_text(declarator, source).to_string()),
        _ => None,
    }
}

fn is_static_declaration(node: tree_sitter::Node<'_>, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "storage_class_specifier" && node_text(child, source) == "static" {
            return true;
        }
    }
    false
}

fn extract_c_doc(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    if let Some(prev) = node.prev_named_sibling() {
        if prev.kind() == "comment" {
            let text = node_text(prev, source);
            if text.starts_with("/**") || text.starts_with("///") {
                return Some(text.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::LanguageAnalyzer;

    fn parse_c(source: &str) -> Vec<StructuralElement> {
        let analyzer = CAnalyzer;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(source.as_bytes(), None).expect("parse");
        analyzer.extract_structure(&tree, source.as_bytes(), Path::new("test.c"))
    }

    #[test]
    fn test_c_function() {
        let src = "int main(int argc, char **argv) { return 0; }";
        let elements = parse_c(src);
        assert!(elements.iter().any(|e| e.name == "main" && e.kind == ChunkKind::Function));
    }

    #[test]
    fn test_c_struct() {
        let src = "struct Point { int x; int y; };";
        let elements = parse_c(src);
        assert!(elements.iter().any(|e| e.name == "Point" && e.kind == ChunkKind::Class));
    }

    #[test]
    fn test_c_macro() {
        let src = "#define MAX_SIZE 1024";
        let elements = parse_c(src);
        assert!(elements.iter().any(|e| e.name == "MAX_SIZE" && e.kind == ChunkKind::Const));
    }
}
