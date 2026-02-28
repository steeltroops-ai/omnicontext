//! C# language analyzer.
//!
//! Extracts structural elements from C# source files using tree-sitter.

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, DependencyKind, ImportStatement, Visibility};

/// Analyzer for C# source files.
pub struct CSharpAnalyzer;

impl LanguageAnalyzer for CSharpAnalyzer {
    fn language_id(&self) -> &str {
        "csharp"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_c_sharp::LANGUAGE.into()
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
            if child.kind() == "using_directive" {
                let line = child.start_position().row as u32 + 1;
                let text = node_text(child, source)
                    .trim_start_matches("using ")
                    .trim_end_matches(';')
                    .trim()
                    .to_string();
                // Skip using aliases like "using X = Y;"
                if !text.contains('=') && !text.is_empty() {
                    let name = text.rsplit('.').next().unwrap_or(&text).to_string();
                    imports.push(ImportStatement {
                        import_path: text,
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

impl CSharpAnalyzer {
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
                "class_declaration" | "record_declaration" => {
                    self.extract_type_decl(child, source, module_name, scope_path, ChunkKind::Class, elements);
                }
                "interface_declaration" => {
                    self.extract_type_decl(child, source, module_name, scope_path, ChunkKind::Trait, elements);
                }
                "struct_declaration" => {
                    self.extract_type_decl(child, source, module_name, scope_path, ChunkKind::Class, elements);
                }
                "enum_declaration" => {
                    self.extract_type_decl(child, source, module_name, scope_path, ChunkKind::TypeDef, elements);
                }
                "namespace_declaration" => {
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
                "method_declaration" | "constructor_declaration" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = node_text(name_node, source).to_string();
                        let vis = extract_cs_visibility(child, source);
                        let doc = extract_xml_doc(child, source);

                        elements.push(StructuralElement {
                            symbol_path: build_path(module_name, scope_path, &name),
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
                "property_declaration" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = node_text(name_node, source).to_string();
                        let vis = extract_cs_visibility(child, source);

                        elements.push(StructuralElement {
                            symbol_path: build_path(module_name, scope_path, &name),
                            name,
                            kind: ChunkKind::Const,
                            visibility: vis,
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            content: node_text(child, source).to_string(),
                            doc_comment: None,
                            references: Vec::new(),
                        });
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

    fn extract_type_decl(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
        kind: ChunkKind,
        elements: &mut Vec<StructuralElement>,
    ) {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source).to_string();
            let symbol_path = build_path(module_name, scope_path, &name);
            let vis = extract_cs_visibility(node, source);
            let doc = extract_xml_doc(node, source);

            elements.push(StructuralElement {
                symbol_path: symbol_path.clone(),
                name: name.clone(),
                kind,
                visibility: vis,
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                content: node_text(node, source).to_string(),
                doc_comment: doc,
                references: Vec::new(),
            });

            // Recurse into body
            if let Some(body) = node.child_by_field_name("body") {
                let mut inner = scope_path.to_vec();
                inner.push(name);
                self.walk_node(body, source, module_name, &inner, elements);
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
    parts.join(".")
}

fn extract_cs_visibility(node: tree_sitter::Node<'_>, source: &[u8]) -> Visibility {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifier" {
            let text = node_text(child, source);
            match text {
                "public" => return Visibility::Public,
                "protected" => return Visibility::Protected,
                "private" => return Visibility::Private,
                "internal" => return Visibility::Crate,
                _ => {}
            }
        }
    }
    Visibility::Private // C# default
}

fn extract_xml_doc(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    // C# uses /// XML doc comments
    if let Some(prev) = node.prev_named_sibling() {
        if prev.kind() == "comment" {
            let text = node_text(prev, source);
            if text.starts_with("///") {
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

    fn parse_cs(source: &str) -> Vec<StructuralElement> {
        let analyzer = CSharpAnalyzer;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&analyzer.tree_sitter_language())
            .expect("set language");
        let tree = parser.parse(source.as_bytes(), None).expect("parse");
        analyzer.extract_structure(&tree, source.as_bytes(), Path::new("Test.cs"))
    }

    #[test]
    fn test_cs_class() {
        let src = r#"
public class UserService {
    public void GetUser() {}
}
"#;
        let elements = parse_cs(src);
        assert!(elements.iter().any(|e| e.name == "UserService" && e.kind == ChunkKind::Class));
        assert!(elements.iter().any(|e| e.name == "GetUser" && e.kind == ChunkKind::Function));
    }

    #[test]
    fn test_cs_interface() {
        let src = r#"
public interface IRepository {
    void Save();
}
"#;
        let elements = parse_cs(src);
        assert!(elements.iter().any(|e| e.name == "IRepository" && e.kind == ChunkKind::Trait));
    }
}
