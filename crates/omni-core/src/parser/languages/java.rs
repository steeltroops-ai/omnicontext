//! Java language analyzer.
//!
//! Extracts structural elements from Java source files using tree-sitter.

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, DependencyKind, ImportStatement, Visibility};

/// Analyzer for Java source files.
pub struct JavaAnalyzer;

impl LanguageAnalyzer for JavaAnalyzer {
    fn language_id(&self) -> &str {
        "java"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_java::LANGUAGE.into()
    }

    fn extract_structure(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> Vec<StructuralElement> {
        let mut elements = Vec::new();
        let module_name_str = crate::parser::build_module_name_from_path(file_path).replace("/", ".");
        let module_name = &module_name_str;

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
            if child.kind() == "import_declaration" {
                let line = child.start_position().row as u32 + 1;
                let text = node_text(child, source).to_string();
                // Extract the import path (between "import " and ";")
                let path = text
                    .trim_start_matches("import ")
                    .trim_start_matches("static ")
                    .trim_end_matches(';')
                    .trim()
                    .to_string();

                if !path.is_empty() {
                    let name = path.rsplit('.').next().unwrap_or(&path).to_string();
                    imports.push(ImportStatement {
                        import_path: path,
                        imported_names: vec![name],
                        line,
                        kind: DependencyKind::Imports,
                    });
                }
            }
        }

        imports
    }
}

impl JavaAnalyzer {
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
                "class_declaration" | "interface_declaration" | "enum_declaration" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = node_text(name_node, source).to_string();
                        let kind = match child.kind() {
                            "interface_declaration" => ChunkKind::Trait,
                            "enum_declaration" => ChunkKind::TypeDef,
                            _ => ChunkKind::Class,
                        };
                        let symbol_path = build_symbol_path(module_name, scope_path, &name);
                        let visibility = extract_java_visibility(child, source);
                        let doc_comment = extract_javadoc(child, source);

                        elements.push(StructuralElement {
                            symbol_path: symbol_path.clone(),
                            name: name.clone(),
                            kind,
                            visibility,
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            content: node_text(child, source).to_string(),
                            doc_comment,
                            references: Vec::new(),
                        });

                        // Recurse into class body
                        if let Some(body) = child.child_by_field_name("body") {
                            let mut inner_scope = scope_path.to_vec();
                            inner_scope.push(name);
                            self.walk_node(body, source, module_name, &inner_scope, elements);
                        }
                    }
                }
                "method_declaration" | "constructor_declaration" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = node_text(name_node, source).to_string();
                        let symbol_path = build_symbol_path(module_name, scope_path, &name);
                        let visibility = extract_java_visibility(child, source);
                        let doc_comment = extract_javadoc(child, source);

                        elements.push(StructuralElement {
                            symbol_path,
                            name,
                            kind: ChunkKind::Function,
                            visibility,
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            content: node_text(child, source).to_string(),
                            doc_comment,
                            references: Vec::new(),
                        });
                    }
                }
                "field_declaration" => {
                    // Extract constant fields (static final)
                    let text = node_text(child, source);
                    if text.contains("static") && text.contains("final") {
                        if let Some(declarator) = child.child_by_field_name("declarator") {
                            if let Some(name_node) = declarator.child_by_field_name("name") {
                                let name = node_text(name_node, source).to_string();
                                let symbol_path =
                                    build_symbol_path(module_name, scope_path, &name);
                                let visibility = extract_java_visibility(child, source);

                                elements.push(StructuralElement {
                                    symbol_path,
                                    name,
                                    kind: ChunkKind::Const,
                                    visibility,
                                    line_start: child.start_position().row as u32 + 1,
                                    line_end: child.end_position().row as u32 + 1,
                                    content: node_text(child, source).to_string(),
                                    doc_comment: None,
                                    references: Vec::new(),
                                });
                            }
                        }
                    }
                }
                _ => {
                    if child.child_count() > 0 {
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

fn build_symbol_path(module: &str, scope: &[String], name: &str) -> String {
    let mut parts: Vec<&str> = vec![module];
    for s in scope {
        parts.push(s);
    }
    parts.push(name);
    parts.join(".")
}

fn extract_java_visibility(node: tree_sitter::Node<'_>, source: &[u8]) -> Visibility {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let text = node_text(child, source);
            if text.contains("public") {
                return Visibility::Public;
            } else if text.contains("protected") {
                return Visibility::Protected;
            } else if text.contains("private") {
                return Visibility::Private;
            }
        }
    }
    // Java default: package-private
    Visibility::Crate
}

fn extract_javadoc(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    // Look for block_comment (/** ... */) immediately before this node
    if let Some(prev) = node.prev_named_sibling() {
        if prev.kind() == "block_comment" {
            let text = node_text(prev, source);
            if text.starts_with("/**") {
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

    fn parse_java(source: &str) -> Vec<StructuralElement> {
        let analyzer = JavaAnalyzer;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(source.as_bytes(), None).expect("parse");
        analyzer.extract_structure(&tree, source.as_bytes(), Path::new("Test.java"))
    }

    #[test]
    fn test_java_class() {
        let src = r#"
public class UserService {
    public void getUser() {}
}
"#;
        let elements = parse_java(src);
        assert!(elements.iter().any(|e| e.name == "UserService" && e.kind == ChunkKind::Class));
        assert!(elements.iter().any(|e| e.name == "getUser" && e.kind == ChunkKind::Function));
    }

    #[test]
    fn test_java_interface() {
        let src = r#"
public interface Repository {
    void save(Object entity);
}
"#;
        let elements = parse_java(src);
        assert!(elements.iter().any(|e| e.name == "Repository" && e.kind == ChunkKind::Trait));
    }

    #[test]
    fn test_java_visibility() {
        let src = r#"
public class Foo {
    private int secret;
    protected void helper() {}
    public static final String NAME = "foo";
}
"#;
        let elements = parse_java(src);
        let helper = elements.iter().find(|e| e.name == "helper");
        assert!(helper.is_some());
        assert_eq!(helper.expect("helper").visibility, Visibility::Protected);
    }
}
