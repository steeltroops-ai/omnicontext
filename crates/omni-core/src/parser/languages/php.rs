//! PHP structural extractor for OmniContext.
//!
//! Extracts classes, functions, methods, and interfaces from PHP source files.

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, DependencyKind, ImportStatement, Visibility};

/// Analyzer for PHP source files.
pub struct PhpAnalyzer;

impl LanguageAnalyzer for PhpAnalyzer {
    fn language_id(&self) -> &str {
        "php"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_php::LANGUAGE_PHP.into()
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
            let line = child.start_position().row as u32 + 1;

            match child.kind() {
                "namespace_use_declaration" => {
                    let mut use_cursor = child.walk();
                    for use_child in child.children(&mut use_cursor) {
                        if use_child.kind() == "namespace_use_clause" {
                            if let Some(name_node) = use_child.child_by_field_name("name") {
                                let import_path = node_text(name_node, source).to_string();
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
                }
                _ => {}
            }
        }

        imports
    }
}

impl PhpAnalyzer {
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
                "function_definition" => {
                    if let Some(elem) =
                        self.extract_function(child, source, module_name, scope_path)
                    {
                        elements.push(elem);
                    }
                }
                "method_declaration" => {
                    if let Some(elem) = self.extract_method(child, source, module_name, scope_path)
                    {
                        elements.push(elem);
                    }
                }
                "class_declaration" => {
                    if let Some(elem) = self.extract_class(child, source, module_name, scope_path) {
                        let class_name = elem.name.clone();
                        elements.push(elem);
                        // Recurse into class body
                        let mut new_scope = scope_path.to_vec();
                        new_scope.push(class_name);
                        if let Some(body) = child.child_by_field_name("body") {
                            self.walk_node(body, source, module_name, &new_scope, elements);
                        }
                    }
                }
                "interface_declaration" => {
                    if let Some(elem) =
                        self.extract_interface(child, source, module_name, scope_path)
                    {
                        elements.push(elem);
                    }
                }
                _ => {
                    // Recurse into other nodes
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

        let line_start = node.start_position().row as u32 + 1;
        let line_end = node.end_position().row as u32 + 1;
        let content = node_text(node, source).to_string();

        Some(StructuralElement {
            kind: ChunkKind::Function,
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

    fn extract_method(
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

        // Check visibility modifiers
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

    fn extract_interface(
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
            kind: ChunkKind::Trait,
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
            if child.kind() == "visibility_modifier" {
                let modifier = node_text(child, source);
                return match modifier {
                    "private" => Visibility::Private,
                    "protected" => Visibility::Protected,
                    "public" => Visibility::Public,
                    _ => Visibility::Public,
                };
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
    fn test_php_class() {
        let code = r#"
<?php
class User {
    private $name;

    public function __construct($name) {
        $this->name = $name;
    }

    public function getName() {
        return $this->name;
    }
}
"#;
        let analyzer = PhpAnalyzer;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(code, None).expect("parse");
        let elements = analyzer.extract_structure(&tree, code.as_bytes(), Path::new("User.php"));

        assert!(!elements.is_empty());
        assert!(elements.iter().any(|e| e.kind == ChunkKind::Class));
        assert!(elements.iter().any(|e| e.kind == ChunkKind::Function));
    }

    #[test]
    fn test_php_function() {
        let code = r#"
<?php
function greet($name) {
    return "Hello, " . $name;
}
"#;
        let analyzer = PhpAnalyzer;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(code, None).expect("parse");
        let elements = analyzer.extract_structure(&tree, code.as_bytes(), Path::new("greet.php"));

        assert!(!elements.is_empty());
        assert!(elements.iter().any(|e| e.kind == ChunkKind::Function));
    }
}
