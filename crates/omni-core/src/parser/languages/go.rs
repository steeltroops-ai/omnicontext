//! Go structural extractor for OmniContext.
//!
//! Extracts functions, methods, structs, interfaces, constants,
//! and type aliases from Go source files using tree-sitter.
//!
//! Go uses capitalization for visibility:
//! - Capitalized names are exported (Public)
//! - Lowercase names are unexported (Private)

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, DependencyKind, ImportStatement, Visibility};

/// Analyzer for Go source files.
pub struct GoAnalyzer;

impl LanguageAnalyzer for GoAnalyzer {
    fn language_id(&self) -> &str {
        "go"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_go::LANGUAGE.into()
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
            let line = child.start_position().row as u32 + 1;

            match child.kind() {
                // `import "fmt"` or `import ( "fmt" ; "os" )`
                "import_declaration" => {
                    let mut inner = child.walk();
                    for spec in child.children(&mut inner) {
                        if spec.kind() == "import_spec" || spec.kind() == "import_spec_list" {
                            self.collect_import_specs(spec, source, &mut imports);
                        } else if spec.kind() == "interpreted_string_literal" {
                            // single import: `import "fmt"`
                            let path = node_text(spec, source)
                                .trim_matches('"')
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
                _ => {}
            }
        }

        imports
    }
}

impl GoAnalyzer {
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
                "method_declaration" => {
                    if let Some(elem) =
                        self.extract_method(child, source, module_name, scope_path)
                    {
                        elements.push(elem);
                    }
                }
                "type_declaration" => {
                    self.extract_type_declarations(
                        child,
                        source,
                        module_name,
                        scope_path,
                        elements,
                    );
                }
                "const_declaration" | "var_declaration" => {
                    self.extract_const_declarations(
                        child,
                        source,
                        module_name,
                        scope_path,
                        elements,
                    );
                }
                _ => {
                    if child.child_count() > 0 {
                        self.walk_node(child, source, module_name, scope_path, elements);
                    }
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

        let symbol_path = build_symbol_path(module_name, scope_path, &name);
        let visibility = go_visibility(&name);
        let doc_comment = extract_go_doc(node, source);

        let kind = if name.starts_with("Test") || name.starts_with("Benchmark") {
            ChunkKind::Test
        } else {
            ChunkKind::Function
        };

        Some(StructuralElement {
            symbol_path,
            name,
            kind,
            visibility,
            line_start: node.start_position().row as u32 + 1,
            line_end: node.end_position().row as u32 + 1,
            content: node_text(node, source).to_string(),
            doc_comment,
            references: Vec::new(),
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

        // Get receiver type for the symbol path
        let receiver = node
            .child_by_field_name("receiver")
            .map(|r| {
                // Extract type name from parameter list
                let text = node_text(r, source);
                text.trim_matches(|c: char| c == '(' || c == ')' || c == '*' || c.is_whitespace())
                    .split_whitespace()
                    .last()
                    .unwrap_or("")
                    .trim_start_matches('*')
                    .to_string()
            })
            .unwrap_or_default();

        let mut full_scope = scope_path.to_vec();
        if !receiver.is_empty() {
            full_scope.push(receiver);
        }

        let symbol_path = build_symbol_path(module_name, &full_scope, &name);
        let visibility = go_visibility(&name);
        let doc_comment = extract_go_doc(node, source);

        Some(StructuralElement {
            symbol_path,
            name,
            kind: ChunkKind::Function,
            visibility,
            line_start: node.start_position().row as u32 + 1,
            line_end: node.end_position().row as u32 + 1,
            content: node_text(node, source).to_string(),
            doc_comment,
            references: Vec::new(),
        })
    }

    fn extract_type_declarations(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
        elements: &mut Vec<StructuralElement>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_spec" {
                let name_node = match child.child_by_field_name("name") {
                    Some(n) => n,
                    None => continue,
                };
                let name = node_text(name_node, source).to_string();
                let symbol_path = build_symbol_path(module_name, scope_path, &name);
                let visibility = go_visibility(&name);
                let doc_comment = extract_go_doc(node, source);

                // Determine kind from the type body
                let kind = match child.child_by_field_name("type") {
                    Some(type_node) => match type_node.kind() {
                        "struct_type" => ChunkKind::Class,
                        "interface_type" => ChunkKind::Trait,
                        _ => ChunkKind::TypeDef,
                    },
                    None => ChunkKind::TypeDef,
                };

                elements.push(StructuralElement {
                    symbol_path,
                    name,
                    kind,
                    visibility,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    content: node_text(node, source).to_string(),
                    doc_comment,
                    references: Vec::new(),
                });
            }
        }
    }

    fn extract_const_declarations(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
        elements: &mut Vec<StructuralElement>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "const_spec" || child.kind() == "var_spec" {
                let name_node = match child.child_by_field_name("name") {
                    Some(n) => n,
                    None => continue,
                };
                let name = node_text(name_node, source).to_string();
                let symbol_path = build_symbol_path(module_name, scope_path, &name);
                let visibility = go_visibility(&name);

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

    fn collect_import_specs(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        imports: &mut Vec<ImportStatement>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "import_spec" {
                let line = child.start_position().row as u32 + 1;
                // Get the path (string literal)
                if let Some(path_node) = child.child_by_field_name("path") {
                    let path = node_text(path_node, source)
                        .trim_matches('"')
                        .to_string();
                    // Get optional alias
                    let alias = child.child_by_field_name("name")
                        .map(|n| node_text(n, source).to_string());
                    let names = alias.into_iter().collect();
                    if !path.is_empty() {
                        imports.push(ImportStatement {
                            import_path: path,
                            imported_names: names,
                            line,
                            kind: DependencyKind::Imports,
                        });
                    }
                }
            } else if child.kind() == "interpreted_string_literal" {
                let line = child.start_position().row as u32 + 1;
                let path = node_text(child, source)
                    .trim_matches('"')
                    .to_string();
                if !path.is_empty() {
                    imports.push(ImportStatement {
                        import_path: path,
                        imported_names: vec![],
                        line,
                        kind: DependencyKind::Imports,
                    });
                }
            } else if child.child_count() > 0 {
                self.collect_import_specs(child, source, imports);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn node_text<'a>(node: tree_sitter::Node<'_>, source: &'a [u8]) -> &'a str {
    let start = node.start_byte();
    let end = node.end_byte();
    std::str::from_utf8(&source[start..end]).unwrap_or("")
}

fn build_symbol_path(module_name: &str, scope_path: &[String], name: &str) -> String {
    let mut parts = vec![module_name.to_string()];
    parts.extend_from_slice(scope_path);
    parts.push(name.to_string());
    parts.join(".")
}

/// Go visibility: capitalized = exported (public), lowercase = unexported (private).
fn go_visibility(name: &str) -> Visibility {
    if name.starts_with(|c: char| c.is_uppercase()) {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

/// Extract Go doc comments (consecutive `//` lines preceding a declaration).
fn extract_go_doc(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    let mut doc_lines = Vec::new();
    let mut current = node.prev_sibling();

    while let Some(sibling) = current {
        if sibling.kind() == "comment" {
            let text = node_text(sibling, source).trim();
            if let Some(line) = text.strip_prefix("//") {
                doc_lines.push(line.trim().to_string());
                current = sibling.prev_sibling();
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if doc_lines.is_empty() {
        return None;
    }

    doc_lines.reverse();
    Some(doc_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_file;
    use crate::types::Language;

    fn parse_go(source: &str) -> Vec<StructuralElement> {
        parse_file(Path::new("main.go"), source.as_bytes(), Language::Go)
            .expect("parse should succeed")
    }

    #[test]
    fn test_go_function() {
        let src = "package main\n\nfunc hello(name string) string {\n\treturn \"Hello, \" + name\n}\n";
        let elements = parse_go(src);
        let func = elements.iter().find(|e| e.name == "hello");
        assert!(func.is_some());
        assert_eq!(func.expect("hello").kind, ChunkKind::Function);
        assert_eq!(func.expect("hello").visibility, Visibility::Private);
    }

    #[test]
    fn test_go_exported_function() {
        let src =
            "package main\n\nfunc Hello(name string) string {\n\treturn \"Hello, \" + name\n}\n";
        let elements = parse_go(src);
        let func = elements.iter().find(|e| e.name == "Hello");
        assert!(func.is_some());
        assert_eq!(func.expect("Hello").visibility, Visibility::Public);
    }

    #[test]
    fn test_go_struct() {
        let src = "package main\n\ntype Config struct {\n\tName string\n\tPort int\n}\n";
        let elements = parse_go(src);
        let s = elements.iter().find(|e| e.name == "Config");
        assert!(s.is_some());
        assert_eq!(s.expect("Config").kind, ChunkKind::Class);
    }

    #[test]
    fn test_go_interface() {
        let src = "package main\n\ntype Reader interface {\n\tRead(p []byte) (n int, err error)\n}\n";
        let elements = parse_go(src);
        let i = elements.iter().find(|e| e.name == "Reader");
        assert!(i.is_some());
        assert_eq!(i.expect("Reader").kind, ChunkKind::Trait);
    }

    #[test]
    fn test_go_method() {
        let src = "package main\n\nfunc (c *Config) Validate() bool {\n\treturn true\n}\n";
        let elements = parse_go(src);
        let m = elements.iter().find(|e| e.name == "Validate");
        assert!(m.is_some());
        assert!(
            m.expect("Validate")
                .symbol_path
                .contains("Config.Validate")
        );
    }

    #[test]
    fn test_go_test_function() {
        let src =
            "package main\n\nimport \"testing\"\n\nfunc TestAdd(t *testing.T) {\n\t// test\n}\n";
        let elements = parse_go(src);
        let t = elements.iter().find(|e| e.name == "TestAdd");
        assert!(t.is_some());
        assert_eq!(t.expect("TestAdd").kind, ChunkKind::Test);
    }

    #[test]
    fn test_go_const() {
        let src = "package main\n\nconst MaxRetries = 3\n";
        let elements = parse_go(src);
        let c = elements.iter().find(|e| e.name == "MaxRetries");
        assert!(c.is_some());
        assert_eq!(c.expect("MaxRetries").kind, ChunkKind::Const);
    }

    #[test]
    fn test_go_visibility() {
        assert_eq!(go_visibility("Hello"), Visibility::Public);
        assert_eq!(go_visibility("hello"), Visibility::Private);
        assert_eq!(go_visibility("Config"), Visibility::Public);
        assert_eq!(go_visibility("config"), Visibility::Private);
    }
}
