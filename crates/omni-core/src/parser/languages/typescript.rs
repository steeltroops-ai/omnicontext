//! TypeScript structural extractor for OmniContext.
//!
//! Extracts functions, arrow functions, classes, interfaces, type aliases,
//! exports, JSDoc comments, and imports from TypeScript source files.

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, DependencyKind, ImportStatement, Visibility};

/// Analyzer for TypeScript source files.
pub struct TypeScriptAnalyzer;

impl LanguageAnalyzer for TypeScriptAnalyzer {
    fn language_id(&self) -> &str {
        "typescript"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
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
        walk_ts_node(root, source, module_name, &[], &mut elements);
        elements
    }

    fn extract_imports(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        _file_path: &Path,
    ) -> Vec<ImportStatement> {
        let mut imports = Vec::new();
        collect_ts_imports(tree.root_node(), source, &mut imports);
        imports
    }
}

// ---------------------------------------------------------------------------
// Shared TS/JS traversal logic
// ---------------------------------------------------------------------------

/// Walk a TypeScript/JavaScript AST node and extract structural elements.
pub(crate) fn walk_ts_node(
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
                    extract_function_decl(child, source, module_name, scope_path)
                {
                    elements.push(elem);
                }
            }
            "class_declaration" => {
                if let Some(elem) =
                    extract_class_decl(child, source, module_name, scope_path)
                {
                    let mut inner_scope = scope_path.to_vec();
                    inner_scope.push(elem.name.clone());
                    if let Some(body) = child.child_by_field_name("body") {
                        walk_ts_node(body, source, module_name, &inner_scope, elements);
                    }
                    elements.push(elem);
                }
            }
            "interface_declaration" => {
                if let Some(elem) =
                    extract_interface(child, source, module_name, scope_path)
                {
                    elements.push(elem);
                }
            }
            "type_alias_declaration" => {
                if let Some(elem) =
                    extract_type_alias(child, source, module_name, scope_path)
                {
                    elements.push(elem);
                }
            }
            "export_statement" => {
                // Unwrap the exported item and mark as public
                let mut inner_cursor = child.walk();
                for inner in child.children(&mut inner_cursor) {
                    match inner.kind() {
                        "function_declaration" => {
                            if let Some(mut elem) =
                                extract_function_decl(inner, source, module_name, scope_path)
                            {
                                elem.visibility = Visibility::Public;
                                elem.line_start = child.start_position().row as u32 + 1;
                                elem.content = node_text(child, source).to_string();
                                elements.push(elem);
                            }
                        }
                        "class_declaration" => {
                            if let Some(mut elem) =
                                extract_class_decl(inner, source, module_name, scope_path)
                            {
                                elem.visibility = Visibility::Public;
                                elem.line_start = child.start_position().row as u32 + 1;
                                elem.content = node_text(child, source).to_string();
                                let mut inner_scope = scope_path.to_vec();
                                inner_scope.push(elem.name.clone());
                                if let Some(body) = inner.child_by_field_name("body") {
                                    walk_ts_node(
                                        body,
                                        source,
                                        module_name,
                                        &inner_scope,
                                        elements,
                                    );
                                }
                                elements.push(elem);
                            }
                        }
                        "interface_declaration" => {
                            if let Some(mut elem) =
                                extract_interface(inner, source, module_name, scope_path)
                            {
                                elem.visibility = Visibility::Public;
                                elements.push(elem);
                            }
                        }
                        "type_alias_declaration" => {
                            if let Some(mut elem) =
                                extract_type_alias(inner, source, module_name, scope_path)
                            {
                                elem.visibility = Visibility::Public;
                                elements.push(elem);
                            }
                        }
                        "lexical_declaration" => {
                            // export const/let arrow functions
                            extract_variable_declarations(
                                inner,
                                source,
                                module_name,
                                scope_path,
                                Visibility::Public,
                                elements,
                            );
                        }
                        _ => {}
                    }
                }
            }
            "lexical_declaration" => {
                extract_variable_declarations(
                    child,
                    source,
                    module_name,
                    scope_path,
                    Visibility::Private,
                    elements,
                );
            }
            "method_definition" => {
                if let Some(elem) =
                    extract_method(child, source, module_name, scope_path)
                {
                    elements.push(elem);
                }
            }
            _ => {
                if child.child_count() > 0
                    && child.kind() != "string"
                    && child.kind() != "template_string"
                {
                    walk_ts_node(child, source, module_name, scope_path, elements);
                }
            }
        }
    }
}

/// Extract a function declaration.
pub(crate) fn extract_function_decl(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    module_name: &str,
    scope_path: &[String],
) -> Option<StructuralElement> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source).to_string();

    let symbol_path = build_symbol_path(module_name, scope_path, &name);
    let doc_comment = extract_jsdoc(node, source);
    let kind = if name.starts_with("test") || name.contains("Test") {
        ChunkKind::Test
    } else {
        ChunkKind::Function
    };

    Some(StructuralElement {
        symbol_path,
        name,
        kind,
        visibility: Visibility::Private,
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        content: node_text(node, source).to_string(),
        doc_comment,
        references: Vec::new(),
    })
}

/// Extract a class declaration.
pub(crate) fn extract_class_decl(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    module_name: &str,
    scope_path: &[String],
) -> Option<StructuralElement> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source).to_string();

    let symbol_path = build_symbol_path(module_name, scope_path, &name);
    let doc_comment = extract_jsdoc(node, source);

    // Extract superclass references
    let mut references = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "class_heritage" {
            let heritage_text = node_text(child, source);
            // Parse "extends Foo implements Bar, Baz"
            for part in heritage_text.split_whitespace() {
                if part != "extends" && part != "implements" {
                    let clean = part.trim_end_matches(',');
                    if !clean.is_empty() {
                        references.push(clean.to_string());
                    }
                }
            }
        }
    }

    Some(StructuralElement {
        symbol_path,
        name,
        kind: ChunkKind::Class,
        visibility: Visibility::Private,
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        content: node_text(node, source).to_string(),
        doc_comment,
        references,
    })
}

/// Extract a TypeScript interface declaration.
fn extract_interface(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    module_name: &str,
    scope_path: &[String],
) -> Option<StructuralElement> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source).to_string();

    let symbol_path = build_symbol_path(module_name, scope_path, &name);
    let doc_comment = extract_jsdoc(node, source);

    Some(StructuralElement {
        symbol_path,
        name,
        kind: ChunkKind::Trait,
        visibility: Visibility::Private,
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        content: node_text(node, source).to_string(),
        doc_comment,
        references: Vec::new(),
    })
}

/// Extract a TypeScript type alias.
fn extract_type_alias(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    module_name: &str,
    scope_path: &[String],
) -> Option<StructuralElement> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source).to_string();

    let symbol_path = build_symbol_path(module_name, scope_path, &name);

    Some(StructuralElement {
        symbol_path,
        name,
        kind: ChunkKind::TypeDef,
        visibility: Visibility::Private,
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        content: node_text(node, source).to_string(),
        doc_comment: None,
        references: Vec::new(),
    })
}

/// Extract a class method definition.
pub(crate) fn extract_method(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    module_name: &str,
    scope_path: &[String],
) -> Option<StructuralElement> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source).to_string();

    let symbol_path = build_symbol_path(module_name, scope_path, &name);
    let doc_comment = extract_jsdoc(node, source);

    Some(StructuralElement {
        symbol_path,
        name,
        kind: ChunkKind::Function,
        visibility: Visibility::Public,
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        content: node_text(node, source).to_string(),
        doc_comment,
        references: Vec::new(),
    })
}

/// Extract arrow functions and const declarations from lexical declarations.
pub(crate) fn extract_variable_declarations(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    module_name: &str,
    scope_path: &[String],
    default_vis: Visibility,
    elements: &mut Vec<StructuralElement>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            let name_node = match child.child_by_field_name("name") {
                Some(n) => n,
                None => continue,
            };
            let name = node_text(name_node, source).to_string();
            let value = child.child_by_field_name("value");

            let kind = match value.map(|v| v.kind()) {
                Some("arrow_function") | Some("function") => ChunkKind::Function,
                _ => ChunkKind::Const,
            };

            let symbol_path = build_symbol_path(module_name, scope_path, &name);

            elements.push(StructuralElement {
                symbol_path,
                name,
                kind,
                visibility: default_vis,
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                content: node_text(node, source).to_string(),
                doc_comment: extract_jsdoc(node, source),
                references: Vec::new(),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Import extraction (shared TS/JS)
// ---------------------------------------------------------------------------

/// Collect ES6 import/export statements and CommonJS require() calls.
pub(crate) fn collect_ts_imports(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    imports: &mut Vec<ImportStatement>,
) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let line = child.start_position().row as u32 + 1;

        match child.kind() {
            // `import { Foo, Bar } from './module'`
            // `import Foo from './module'`
            // `import * as Foo from './module'`
            "import_statement" => {
                let source_node = child.child_by_field_name("source");
                let module_path = source_node
                    .map(|n| {
                        let t = node_text(n, source);
                        t.trim_matches(|c: char| c == '\'' || c == '"').to_string()
                    })
                    .unwrap_or_default();

                if module_path.is_empty() {
                    continue;
                }

                let mut names = Vec::new();
                let mut inner = child.walk();
                for import_child in child.children(&mut inner) {
                    match import_child.kind() {
                        "import_clause" => {
                            collect_import_names(import_child, source, &mut names);
                        }
                        "identifier" => {
                            // default import
                            let name = node_text(import_child, source).to_string();
                            if name != "import" && name != "from" {
                                names.push(name);
                            }
                        }
                        _ => {}
                    }
                }

                imports.push(ImportStatement {
                    import_path: module_path,
                    imported_names: names,
                    line,
                    kind: DependencyKind::Imports,
                });
            }
            // `export { Foo } from './module'` (re-exports)
            "export_statement" => {
                let source_node = child.child_by_field_name("source");
                if let Some(src_node) = source_node {
                    let module_path = {
                        let t = node_text(src_node, source);
                        t.trim_matches(|c: char| c == '\'' || c == '"').to_string()
                    };
                    if !module_path.is_empty() {
                        imports.push(ImportStatement {
                            import_path: module_path,
                            imported_names: vec![],
                            line,
                            kind: DependencyKind::Imports,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

/// Collect named imports from an import clause node.
fn collect_import_names(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    names: &mut Vec<String>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                names.push(node_text(child, source).to_string());
            }
            "named_imports" => {
                let mut inner = child.walk();
                for spec in child.children(&mut inner) {
                    if spec.kind() == "import_specifier" {
                        if let Some(name_node) = spec.child_by_field_name("name") {
                            names.push(node_text(name_node, source).to_string());
                        }
                    }
                }
            }
            "namespace_import" => {
                // `import * as X` -- push "*"
                names.push("*".to_string());
            }
            _ => {
                if child.child_count() > 0 {
                    collect_import_names(child, source, names);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get the text of a tree-sitter node.
pub(crate) fn node_text<'a>(node: tree_sitter::Node<'_>, source: &'a [u8]) -> &'a str {
    let start = node.start_byte();
    let end = node.end_byte();
    std::str::from_utf8(&source[start..end]).unwrap_or("")
}

/// Build a symbol path with `.` separator (JS/TS convention).
pub(crate) fn build_symbol_path(
    module_name: &str,
    scope_path: &[String],
    name: &str,
) -> String {
    let mut parts = vec![module_name.to_string()];
    parts.extend_from_slice(scope_path);
    parts.push(name.to_string());
    parts.join(".")
}

/// Extract JSDoc comment preceding a node.
///
/// JSDoc comments are `comment` nodes with `/** ... */` syntax.
fn extract_jsdoc(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    let prev = node.prev_sibling()?;
    if prev.kind() != "comment" {
        return None;
    }

    let text = node_text(prev, source).trim();
    if !text.starts_with("/**") {
        return None;
    }

    // Strip /** and */
    let stripped = text
        .strip_prefix("/**")
        .unwrap_or(text)
        .strip_suffix("*/")
        .unwrap_or(text)
        .trim();

    // Clean up leading * on each line
    let cleaned: Vec<&str> = stripped
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            trimmed.strip_prefix("* ").or_else(|| trimmed.strip_prefix('*')).unwrap_or(trimmed)
        })
        .collect();

    let result = cleaned.join("\n").trim().to_string();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_file;
    use crate::types::Language;

    fn parse_ts(source: &str) -> Vec<StructuralElement> {
        parse_file(Path::new("test.ts"), source.as_bytes(), Language::TypeScript)
            .expect("parse should succeed")
    }

    #[test]
    fn test_ts_function() {
        let src = "function greet(name: string): string {\n  return `Hello, ${name}`;\n}\n";
        let elements = parse_ts(src);
        let func = elements.iter().find(|e| e.name == "greet");
        assert!(func.is_some());
        assert_eq!(func.expect("greet").kind, ChunkKind::Function);
    }

    #[test]
    fn test_ts_class() {
        let src = r#"
class UserService {
    constructor(private db: Database) {}
    getUser(id: string): User {
        return this.db.find(id);
    }
}
"#;
        let elements = parse_ts(src);
        let class = elements.iter().find(|e| e.name == "UserService");
        assert!(class.is_some());
        assert_eq!(class.expect("UserService").kind, ChunkKind::Class);
    }

    #[test]
    fn test_ts_interface() {
        let src = r#"
interface User {
    id: string;
    name: string;
    email: string;
}
"#;
        let elements = parse_ts(src);
        let iface = elements.iter().find(|e| e.name == "User");
        assert!(iface.is_some());
        assert_eq!(iface.expect("User").kind, ChunkKind::Trait);
    }

    #[test]
    fn test_ts_type_alias() {
        let src = "type Result<T> = Success<T> | Failure;\n";
        let elements = parse_ts(src);
        let t = elements.iter().find(|e| e.name == "Result");
        assert!(t.is_some());
        assert_eq!(t.expect("Result").kind, ChunkKind::TypeDef);
    }

    #[test]
    fn test_ts_exported_function() {
        let src = "export function api(): void { }\n";
        let elements = parse_ts(src);
        let func = elements.iter().find(|e| e.name == "api");
        assert!(func.is_some());
        assert_eq!(func.expect("api").visibility, Visibility::Public);
    }

    #[test]
    fn test_ts_arrow_function() {
        let src = "const add = (a: number, b: number): number => a + b;\n";
        let elements = parse_ts(src);
        let func = elements.iter().find(|e| e.name == "add");
        assert!(func.is_some());
        assert_eq!(func.expect("add").kind, ChunkKind::Function);
    }

    #[test]
    fn test_ts_exported_const() {
        let src = "export const MAX_RETRIES = 3;\n";
        let elements = parse_ts(src);
        let c = elements.iter().find(|e| e.name == "MAX_RETRIES");
        assert!(c.is_some());
        assert_eq!(c.expect("MAX_RETRIES").kind, ChunkKind::Const);
        assert_eq!(c.expect("MAX_RETRIES").visibility, Visibility::Public);
    }

    #[test]
    fn test_ts_empty_file() {
        let elements = parse_ts("");
        assert!(elements.is_empty());
    }
}
