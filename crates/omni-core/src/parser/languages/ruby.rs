//! Ruby structural extractor for OmniContext.
//!
//! Extracts classes, modules, methods, and constants from Ruby source files.

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, DependencyKind, ImportStatement, Visibility};

/// Analyzer for Ruby source files.
pub struct RubyAnalyzer;

impl LanguageAnalyzer for RubyAnalyzer {
    fn language_id(&self) -> &str {
        "ruby"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_ruby::LANGUAGE.into()
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

            // require 'module_name' or require_relative 'path'
            if child.kind() == "call" {
                if let Some(method) = child.child_by_field_name("method") {
                    let method_name = node_text(method, source);
                    if method_name == "require" || method_name == "require_relative" {
                        if let Some(args) = child.child_by_field_name("arguments") {
                            let mut arg_cursor = args.walk();
                            for arg in args.children(&mut arg_cursor) {
                                if arg.kind() == "string" {
                                    let import_path = extract_string_content(arg, source);
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
                }
            }
        }

        imports
    }
}

impl RubyAnalyzer {
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
                "method" => {
                    if let Some(elem) = self.extract_method(child, source, module_name, scope_path)
                    {
                        elements.push(elem);
                    }
                }
                "class" => {
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
                "module" => {
                    if let Some(elem) = self.extract_module(child, source, module_name, scope_path)
                    {
                        let mod_name = elem.name.clone();
                        elements.push(elem);
                        // Recurse into module body
                        let mut new_scope = scope_path.to_vec();
                        new_scope.push(mod_name);
                        if let Some(body) = child.child_by_field_name("body") {
                            self.walk_node(body, source, module_name, &new_scope, elements);
                        }
                    }
                }
                _ => {
                    // Recurse into other nodes
                    self.walk_node(child, source, module_name, scope_path, elements);
                }
            }
        }
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

        let visibility = if name.starts_with('_') {
            Visibility::Private
        } else {
            Visibility::Public
        };

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

    fn extract_module(
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
            kind: ChunkKind::Module,
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
}

fn node_text<'a>(node: tree_sitter::Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

fn extract_string_content(node: tree_sitter::Node<'_>, source: &[u8]) -> String {
    let text = node_text(node, source);
    // Remove quotes
    text.trim_matches(|c| c == '"' || c == '\'').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruby_class() {
        let code = r#"
class User
  def initialize(name)
    @name = name
  end

  def greet
    puts "Hello, #{@name}!"
  end
end
"#;
        let analyzer = RubyAnalyzer;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(code, None).expect("parse");
        let elements = analyzer.extract_structure(&tree, code.as_bytes(), Path::new("user.rb"));

        assert!(!elements.is_empty());
        assert!(elements.iter().any(|e| e.kind == ChunkKind::Class));
        assert!(elements.iter().any(|e| e.kind == ChunkKind::Function));
    }

    #[test]
    fn test_ruby_module() {
        let code = r#"
module Authentication
  def self.validate(token)
    token == "secret"
  end
end
"#;
        let analyzer = RubyAnalyzer;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(code, None).expect("parse");
        let elements = analyzer.extract_structure(&tree, code.as_bytes(), Path::new("auth.rb"));

        assert!(!elements.is_empty());
        assert!(elements.iter().any(|e| e.kind == ChunkKind::Module));
    }
}
