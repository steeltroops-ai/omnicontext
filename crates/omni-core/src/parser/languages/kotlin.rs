//! Kotlin structural extractor for OmniContext.
//!
//! Extracts classes, functions, interfaces, and objects from Kotlin source files.

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, DependencyKind, ImportStatement, Visibility};

/// Analyzer for Kotlin source files.
pub struct KotlinAnalyzer;

impl LanguageAnalyzer for KotlinAnalyzer {
    fn language_id(&self) -> &str {
        "kotlin"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_kotlin_ng::LANGUAGE.into()
    }

    fn extract_structure(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> Vec<StructuralElement> {
        let mut elements = Vec::new();
        let module_name = crate::parser::build_module_name_from_path(file_path);

        let root = tree.root_node();
        self.walk_node(root, source, &module_name, &[], &mut elements);
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
            if child.kind() == "import_header" {
                let line = child.start_position().row as u32 + 1;
                if let Some(import_node) = child.child_by_field_name("identifier") {
                    let import_path = node_text(import_node, source).to_string();
                    if !import_path.is_empty() {
                        imports.push(ImportStatement {
                            import_path,
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

impl KotlinAnalyzer {
    fn walk_node(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
        elements: &mut Vec<StructuralElement>,
    ) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_declaration" => {
                    if let Some(elem) =
                        self.extract_function(child, source, module_name, scope_path)
                    {
                        elements.push(elem);
                    }
                }
                "class_declaration" => {
                    if let Some(elem) = self.extract_class(child, source, module_name, scope_path) {
                        let class_name = elem.name.clone();
                        elements.push(elem);
                        let mut new_scope = scope_path.to_vec();
                        new_scope.push(class_name);
                        if let Some(body) = child.child_by_field_name("body") {
                            self.walk_node(body, source, module_name, &new_scope, elements);
                        }
                    }
                }
                "object_declaration" => {
                    if let Some(elem) = self.extract_object(child, source, module_name, scope_path)
                    {
                        elements.push(elem);
                    }
                }
                _ => {
                    self.walk_node(child, source, module_name, scope_path, elements);
                }
            }
        }
    }

    fn extract_function(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
    ) -> Option<StructuralElement> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source).to_string();

        let symbol_path = if scope_path.is_empty() {
            format!("{module_name}.{name}")
        } else {
            format!("{}.{}.{}", module_name, scope_path.join("."), name)
        };

        let visibility = self.extract_visibility(node, source);
        let line_start = node.start_position().row as u32 + 1;
        let line_end = node.end_position().row as u32 + 1;
        let content = node_text(node, source).to_string();

        Some(StructuralElement {
            kind: ChunkKind::Function,
            symbol_path,
            name,
            visibility,
            line_start,
            line_end,
            content,
            doc_comment: None,
            references: vec![],
            extends: vec![],
            implements: vec![],
        })
    }

    fn extract_class(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
    ) -> Option<StructuralElement> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source).to_string();

        let symbol_path = if scope_path.is_empty() {
            format!("{module_name}.{name}")
        } else {
            format!("{}.{}.{}", module_name, scope_path.join("."), name)
        };

        let line_start = node.start_position().row as u32 + 1;
        let line_end = node.end_position().row as u32 + 1;
        let content = node_text(node, source).to_string();

        Some(StructuralElement {
            kind: ChunkKind::Class,
            symbol_path: symbol_path.clone(),
            name,
            visibility: Visibility::Public,
            line_start,
            line_end,
            content,
            doc_comment: None,
            references: vec![],
            extends: vec![],
            implements: vec![],
        })
    }

    fn extract_object(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
    ) -> Option<StructuralElement> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source).to_string();

        let symbol_path = if scope_path.is_empty() {
            format!("{module_name}.{name}")
        } else {
            format!("{}.{}.{}", module_name, scope_path.join("."), name)
        };

        let line_start = node.start_position().row as u32 + 1;
        let line_end = node.end_position().row as u32 + 1;
        let content = node_text(node, source).to_string();

        Some(StructuralElement {
            kind: ChunkKind::Class,
            symbol_path,
            name,
            visibility: Visibility::Public,
            line_start,
            line_end,
            content,
            doc_comment: None,
            references: vec![],
            extends: vec![],
            implements: vec![],
        })
    }

    fn extract_visibility(&self, node: tree_sitter::Node<'_>, source: &[u8]) -> Visibility {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "modifiers" {
                let text = node_text(child, source);
                if text.contains("private") {
                    return Visibility::Private;
                } else if text.contains("internal") || text.contains("protected") {
                    return Visibility::Protected;
                }
            }
        }
        Visibility::Public
    }
}

fn node_text<'a>(node: tree_sitter::Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kotlin_class() {
        let code = r#"
class User(val name: String) {
    fun greet() {
        println("Hello, $name!")
    }
}
"#;
        let analyzer = KotlinAnalyzer;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(code, None).expect("parse");
        let elements = analyzer.extract_structure(&tree, code.as_bytes(), Path::new("User.kt"));

        assert!(!elements.is_empty());
        assert!(elements.iter().any(|e| e.kind == ChunkKind::Class));
    }

    #[test]
    fn test_kotlin_function() {
        let code = r#"
fun greet(name: String): String {
    return "Hello, $name"
}
"#;
        let analyzer = KotlinAnalyzer;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(code, None).expect("parse");
        let elements = analyzer.extract_structure(&tree, code.as_bytes(), Path::new("greet.kt"));

        assert!(!elements.is_empty());
        assert!(elements.iter().any(|e| e.kind == ChunkKind::Function));
    }
}
