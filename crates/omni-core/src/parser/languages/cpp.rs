//! C++ language analyzer.
//!
//! Extracts structural elements from C++ source files using tree-sitter.
//! Handles classes, namespaces, templates, and methods on top of C constructs.

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, DependencyKind, ImportStatement, Visibility};

/// Analyzer for C++ source files.
pub struct CppAnalyzer;

impl LanguageAnalyzer for CppAnalyzer {
    fn language_id(&self) -> &str {
        "cpp"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_cpp::LANGUAGE.into()
    }

    fn extract_structure(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> Vec<StructuralElement> {
        let mut elements = Vec::new();
        let module_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");

        let root = tree.root_node();
        self.walk_node(root, source, module_name, &[], &mut elements);
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

impl CppAnalyzer {
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
                    if let Some(declarator) = child.child_by_field_name("declarator") {
                        if let Some(name) = extract_cpp_name(declarator, source) {
                            let symbol_path = build_path(module_name, scope_path, &name);
                            let vis = detect_cpp_access(child, source);
                            let doc = extract_cpp_doc(child, source);

                            elements.push(StructuralElement {
                                symbol_path,
                                name,
                                kind: ChunkKind::Function,
                                visibility: vis,
                                line_start: child.start_position().row as u32 + 1,
                                line_end: child.end_position().row as u32 + 1,
                                content: node_text(child, source).to_string(),
                                doc_comment: doc,
                                references: Vec::new(),
                            });
                        }
                    }
                }
                "class_specifier" | "struct_specifier" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = node_text(name_node, source).to_string();
                        let symbol_path = build_path(module_name, scope_path, &name);
                        let doc = extract_cpp_doc(child, source);

                        elements.push(StructuralElement {
                            symbol_path: symbol_path.clone(),
                            name: name.clone(),
                            kind: ChunkKind::Class,
                            visibility: Visibility::Public,
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            content: node_text(child, source).to_string(),
                            doc_comment: doc,
                            references: Vec::new(),
                        });

                        // Recurse into body
                        if let Some(body) = child.child_by_field_name("body") {
                            let mut inner = scope_path.to_vec();
                            inner.push(name);
                            self.walk_node(body, source, module_name, &inner, elements);
                        }
                    }
                }
                "namespace_definition" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| node_text(n, source).to_string())
                        .unwrap_or_default();

                    if !name.is_empty() {
                        elements.push(StructuralElement {
                            symbol_path: build_path(module_name, scope_path, &name),
                            name: name.clone(),
                            kind: ChunkKind::Module,
                            visibility: Visibility::Public,
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            content: node_text(child, source).to_string(),
                            doc_comment: None,
                            references: Vec::new(),
                        });

                        if let Some(body) = child.child_by_field_name("body") {
                            let mut inner = scope_path.to_vec();
                            inner.push(name);
                            self.walk_node(body, source, module_name, &inner, elements);
                        }
                    }
                }
                "template_declaration" => {
                    // Recurse into template body to find the class/function
                    self.walk_node(child, source, module_name, scope_path, elements);
                }
                "enum_specifier" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = node_text(name_node, source).to_string();
                        elements.push(StructuralElement {
                            symbol_path: build_path(module_name, scope_path, &name),
                            name,
                            kind: ChunkKind::TypeDef,
                            visibility: Visibility::Public,
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            content: node_text(child, source).to_string(),
                            doc_comment: None,
                            references: Vec::new(),
                        });
                    }
                }
                "preproc_def" | "preproc_function_def" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = node_text(name_node, source).to_string();
                        elements.push(StructuralElement {
                            symbol_path: build_path(module_name, scope_path, &name),
                            name,
                            kind: ChunkKind::Const,
                            visibility: Visibility::Public,
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            content: node_text(child, source).to_string(),
                            doc_comment: None,
                            references: Vec::new(),
                        });
                    }
                }
                _ => {
                    if child.child_count() > 0 && !child.kind().starts_with("preproc_") {
                        self.walk_node(child, source, module_name, scope_path, elements);
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

fn build_path(module: &str, scope: &[String], name: &str) -> String {
    let mut parts: Vec<&str> = vec![module];
    for s in scope {
        parts.push(s);
    }
    parts.push(name);
    parts.join("::")
}

fn extract_cpp_name(declarator: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    match declarator.kind() {
        "function_declarator" => {
            declarator
                .child_by_field_name("declarator")
                .and_then(|n| extract_cpp_name(n, source))
        }
        "qualified_identifier" | "scoped_identifier" => {
            Some(node_text(declarator, source).to_string())
        }
        "identifier" | "destructor_name" | "operator_name" => {
            Some(node_text(declarator, source).to_string())
        }
        "pointer_declarator" | "reference_declarator" => {
            let mut cursor = declarator.walk();
            for child in declarator.children(&mut cursor) {
                if let Some(n) = extract_cpp_name(child, source) {
                    return Some(n);
                }
            }
            None
        }
        _ => None,
    }
}

fn detect_cpp_access(node: tree_sitter::Node<'_>, source: &[u8]) -> Visibility {
    // Check if parent is an access_specifier section
    if let Some(parent) = node.parent() {
        if parent.kind() == "access_specifier" {
            let text = node_text(parent, source);
            if text.contains("private") {
                return Visibility::Private;
            } else if text.contains("protected") {
                return Visibility::Protected;
            }
        }
    }
    Visibility::Public
}

fn extract_cpp_doc(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
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

    fn parse_cpp(source: &str) -> Vec<StructuralElement> {
        let analyzer = CppAnalyzer;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(source.as_bytes(), None).expect("parse");
        analyzer.extract_structure(&tree, source.as_bytes(), Path::new("test.cpp"))
    }

    #[test]
    fn test_cpp_class() {
        let src = r#"
class Vector {
public:
    int x, y;
    void normalize() {}
};
"#;
        let elements = parse_cpp(src);
        assert!(elements.iter().any(|e| e.name == "Vector" && e.kind == ChunkKind::Class));
    }

    #[test]
    fn test_cpp_namespace() {
        let src = r#"
namespace engine {
    void init() {}
}
"#;
        let elements = parse_cpp(src);
        assert!(elements.iter().any(|e| e.name == "engine" && e.kind == ChunkKind::Module));
        assert!(elements.iter().any(|e| e.name == "init" && e.kind == ChunkKind::Function));
    }
}
