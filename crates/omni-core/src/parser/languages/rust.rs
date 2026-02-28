//! Rust structural extractor for OmniContext.
//!
//! Extracts functions, structs, enums, traits, impls, constants, type aliases,
//! modules, and test functions from Rust source files using tree-sitter.
//!
//! ## Rust AST Node Types (tree-sitter-rust)
//!
//! - `function_item` -> Function
//! - `struct_item` -> Class
//! - `enum_item` -> Class
//! - `trait_item` -> Trait
//! - `impl_item` -> Impl
//! - `const_item` / `static_item` -> Const
//! - `type_item` -> TypeDef
//! - `mod_item` -> Module
//! - `attribute_item` with `#[test]` or `#[cfg(test)]` -> Test detection

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, Visibility};

/// Analyzer for Rust source files.
pub struct RustAnalyzer;

impl LanguageAnalyzer for RustAnalyzer {
    fn language_id(&self) -> &str {
        "rust"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_rust::LANGUAGE.into()
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
            .unwrap_or("mod");

        let root = tree.root_node();
        self.walk_node(root, source, module_name, &[], &mut elements, false);
        elements
    }
}

impl RustAnalyzer {
    /// Recursively walk the AST and extract structural elements.
    ///
    /// `in_test_mod` tracks whether we're inside a `#[cfg(test)]` module.
    fn walk_node(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
        elements: &mut Vec<StructuralElement>,
        in_test_mod: bool,
    ) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_item" => {
                    if let Some(elem) = self.extract_function(
                        child,
                        source,
                        module_name,
                        scope_path,
                        in_test_mod,
                    ) {
                        elements.push(elem);
                    }
                }
                "struct_item" | "enum_item" => {
                    if let Some(elem) =
                        self.extract_type_def(child, source, module_name, scope_path, ChunkKind::Class)
                    {
                        elements.push(elem);
                    }
                }
                "trait_item" => {
                    if let Some(elem) =
                        self.extract_type_def(child, source, module_name, scope_path, ChunkKind::Trait)
                    {
                        // Recurse into trait body for method signatures
                        let mut inner_scope = scope_path.to_vec();
                        inner_scope.push(elem.name.clone());
                        if let Some(body) = child.child_by_field_name("body") {
                            self.walk_node(
                                body,
                                source,
                                module_name,
                                &inner_scope,
                                elements,
                                in_test_mod,
                            );
                        }
                        elements.push(elem);
                    }
                }
                "impl_item" => {
                    if let Some(elem) =
                        self.extract_impl(child, source, module_name, scope_path)
                    {
                        // Recurse into impl body for methods
                        let mut inner_scope = scope_path.to_vec();
                        inner_scope.push(elem.name.clone());
                        if let Some(body) = child.child_by_field_name("body") {
                            self.walk_node(
                                body,
                                source,
                                module_name,
                                &inner_scope,
                                elements,
                                in_test_mod,
                            );
                        }
                        elements.push(elem);
                    }
                }
                "const_item" | "static_item" => {
                    if let Some(elem) =
                        self.extract_const(child, source, module_name, scope_path)
                    {
                        elements.push(elem);
                    }
                }
                "type_item" => {
                    if let Some(elem) = self.extract_type_def(
                        child,
                        source,
                        module_name,
                        scope_path,
                        ChunkKind::TypeDef,
                    ) {
                        elements.push(elem);
                    }
                }
                "mod_item" => {
                    self.handle_mod_item(child, source, module_name, scope_path, elements);
                }
                "attribute_item" => {
                    // Skip standalone attributes, they're handled contextually
                }
                _ => {
                    // Recurse into other compound nodes
                    if child.child_count() > 0 && child.kind() != "string_literal" {
                        self.walk_node(
                            child,
                            source,
                            module_name,
                            scope_path,
                            elements,
                            in_test_mod,
                        );
                    }
                }
            }
        }
    }

    /// Extract a function or method definition.
    fn extract_function(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
        in_test_mod: bool,
    ) -> Option<StructuralElement> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source).to_string();

        let symbol_path = build_symbol_path(module_name, scope_path, &name);
        let visibility = extract_rust_visibility(node, source);
        let doc_comment = extract_rust_doc_comment(node, source);
        let references = extract_use_references(node, source);

        // Determine if this is a test function
        let has_test_attr = has_attribute(node, source, "test");
        let kind = if has_test_attr || in_test_mod {
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
            references,
        })
    }

    /// Extract a struct, enum, trait, or type alias definition.
    fn extract_type_def(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
        kind: ChunkKind,
    ) -> Option<StructuralElement> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source).to_string();

        let symbol_path = build_symbol_path(module_name, scope_path, &name);
        let visibility = extract_rust_visibility(node, source);
        let doc_comment = extract_rust_doc_comment(node, source);

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

    /// Extract an `impl` block.
    fn extract_impl(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
    ) -> Option<StructuralElement> {
        // impl blocks don't have a "name" field -- we construct it from the type
        let type_node = node.child_by_field_name("type")?;
        let type_name = node_text(type_node, source).to_string();

        // Check if this is a trait impl: `impl Trait for Type`
        let name = if let Some(trait_node) = node.child_by_field_name("trait") {
            let trait_name = node_text(trait_node, source);
            format!("impl {trait_name} for {type_name}")
        } else {
            format!("impl {type_name}")
        };

        let symbol_path = build_symbol_path(module_name, scope_path, &name);

        Some(StructuralElement {
            symbol_path,
            name,
            kind: ChunkKind::Impl,
            visibility: Visibility::Public, // impl blocks are always accessible
            line_start: node.start_position().row as u32 + 1,
            line_end: node.end_position().row as u32 + 1,
            content: node_text(node, source).to_string(),
            doc_comment: None,
            references: Vec::new(),
        })
    }

    /// Extract a const/static item.
    fn extract_const(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
    ) -> Option<StructuralElement> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source).to_string();

        let symbol_path = build_symbol_path(module_name, scope_path, &name);
        let visibility = extract_rust_visibility(node, source);
        let doc_comment = extract_rust_doc_comment(node, source);

        Some(StructuralElement {
            symbol_path,
            name,
            kind: ChunkKind::Const,
            visibility,
            line_start: node.start_position().row as u32 + 1,
            line_end: node.end_position().row as u32 + 1,
            content: node_text(node, source).to_string(),
            doc_comment,
            references: Vec::new(),
        })
    }

    /// Handle a `mod` item -- might be inline (`mod x { ... }`) or external (`mod x;`).
    fn handle_mod_item(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
        elements: &mut Vec<StructuralElement>,
    ) {
        let name_node = match node.child_by_field_name("name") {
            Some(n) => n,
            None => return,
        };
        let name = node_text(name_node, source).to_string();

        // Check if this is a #[cfg(test)] mod
        let is_test_mod = has_attribute(node, source, "cfg(test)");

        // Emit the module itself
        let symbol_path = build_symbol_path(module_name, scope_path, &name);
        elements.push(StructuralElement {
            symbol_path,
            name: name.clone(),
            kind: ChunkKind::Module,
            visibility: extract_rust_visibility(node, source),
            line_start: node.start_position().row as u32 + 1,
            line_end: node.end_position().row as u32 + 1,
            content: node_text(node, source).to_string(),
            doc_comment: extract_rust_doc_comment(node, source),
            references: Vec::new(),
        });

        // If inline module, recurse into body
        if let Some(body) = node.child_by_field_name("body") {
            let mut inner_scope = scope_path.to_vec();
            inner_scope.push(name);
            self.walk_node(body, source, module_name, &inner_scope, elements, is_test_mod);
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Get the text content of a tree-sitter node.
fn node_text<'a>(node: tree_sitter::Node<'_>, source: &'a [u8]) -> &'a str {
    let start = node.start_byte();
    let end = node.end_byte();
    std::str::from_utf8(&source[start..end]).unwrap_or("")
}

/// Build a fully-qualified symbol path.
fn build_symbol_path(module_name: &str, scope_path: &[String], name: &str) -> String {
    let mut parts = vec![module_name.to_string()];
    parts.extend_from_slice(scope_path);
    parts.push(name.to_string());
    parts.join("::")
}

/// Extract Rust visibility from the node's `visibility_modifier` child.
fn extract_rust_visibility(node: tree_sitter::Node<'_>, source: &[u8]) -> Visibility {
    // Look for visibility_modifier as a sibling before the node
    // or as a child of the node
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            let text = node_text(child, source);
            return match text {
                "pub" => Visibility::Public,
                "pub(crate)" => Visibility::Crate,
                "pub(super)" => Visibility::Protected,
                _ if text.starts_with("pub(") => Visibility::Crate,
                _ => Visibility::Private,
            };
        }
    }

    Visibility::Private
}

/// Extract doc comments (/// and //!) preceding a node.
///
/// tree-sitter-rust puts doc comments as `line_comment` nodes preceding
/// the item. We check the previous siblings.
fn extract_rust_doc_comment(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    let mut doc_lines = Vec::new();
    let mut current = node.prev_sibling();

    while let Some(sibling) = current {
        let text = node_text(sibling, source).trim();

        if text.starts_with("///") {
            let line = text.strip_prefix("///").unwrap_or("").trim();
            doc_lines.push(line.to_string());
            current = sibling.prev_sibling();
        } else if text.starts_with("//!") {
            let line = text.strip_prefix("//!").unwrap_or("").trim();
            doc_lines.push(line.to_string());
            current = sibling.prev_sibling();
        } else if sibling.kind() == "attribute_item" {
            // Attributes are between doc comments and the item, skip through
            current = sibling.prev_sibling();
        } else {
            break;
        }
    }

    if doc_lines.is_empty() {
        return None;
    }

    doc_lines.reverse(); // we collected bottom-up
    Some(doc_lines.join("\n"))
}

/// Check if a node has a specific attribute (e.g., `#[test]`, `#[cfg(test)]`).
fn has_attribute(node: tree_sitter::Node<'_>, source: &[u8], attr_name: &str) -> bool {
    // Check previous siblings for attribute_item nodes
    let mut current = node.prev_sibling();
    while let Some(sibling) = current {
        if sibling.kind() == "attribute_item" {
            let text = node_text(sibling, source);
            if text.contains(attr_name) {
                return true;
            }
            current = sibling.prev_sibling();
        } else if sibling.kind() == "line_comment" {
            // Skip doc comments while searching for attributes
            current = sibling.prev_sibling();
        } else {
            break;
        }
    }
    false
}

/// Quick reference extraction from use declarations within a node.
fn extract_use_references(_node: tree_sitter::Node<'_>, _source: &[u8]) -> Vec<String> {
    // TODO: Extract `use` paths from function bodies
    Vec::new()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_file;
    use crate::types::Language;
    use std::path::Path;

    /// Helper: parse Rust source and return elements.
    fn parse_rust(source: &str) -> Vec<StructuralElement> {
        parse_file(Path::new("test.rs"), source.as_bytes(), Language::Rust)
            .expect("parse should succeed")
    }

    #[test]
    fn test_simple_function() {
        let src = r#"
fn hello(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#;
        let elements = parse_rust(src);
        let func = elements.iter().find(|e| e.name == "hello");
        assert!(func.is_some());
        let func = func.expect("hello");
        assert_eq!(func.kind, ChunkKind::Function);
        assert_eq!(func.visibility, Visibility::Private);
    }

    #[test]
    fn test_pub_function() {
        let src = r#"
pub fn public_api(x: i32) -> i32 {
    x * 2
}
"#;
        let elements = parse_rust(src);
        let func = elements.iter().find(|e| e.name == "public_api");
        assert!(func.is_some());
        assert_eq!(func.expect("public_api").visibility, Visibility::Public);
    }

    #[test]
    fn test_pub_crate_visibility() {
        let src = r#"
pub(crate) fn internal_fn() {}
"#;
        let elements = parse_rust(src);
        let func = elements.iter().find(|e| e.name == "internal_fn");
        assert!(func.is_some());
        assert_eq!(func.expect("internal_fn").visibility, Visibility::Crate);
    }

    #[test]
    fn test_struct() {
        let src = r#"
/// A configuration object.
pub struct Config {
    pub name: String,
    port: u16,
}
"#;
        let elements = parse_rust(src);
        let s = elements.iter().find(|e| e.name == "Config");
        assert!(s.is_some());
        let s = s.expect("Config");
        assert_eq!(s.kind, ChunkKind::Class);
        assert_eq!(s.visibility, Visibility::Public);
        assert_eq!(s.doc_comment.as_deref(), Some("A configuration object."));
    }

    #[test]
    fn test_enum() {
        let src = r#"
pub enum Color {
    Red,
    Green,
    Blue,
}
"#;
        let elements = parse_rust(src);
        let e = elements.iter().find(|e| e.name == "Color");
        assert!(e.is_some());
        assert_eq!(e.expect("Color").kind, ChunkKind::Class);
    }

    #[test]
    fn test_trait() {
        let src = r#"
pub trait Drawable {
    fn draw(&self);
    fn area(&self) -> f64;
}
"#;
        let elements = parse_rust(src);
        let t = elements.iter().find(|e| e.name == "Drawable");
        assert!(t.is_some());
        assert_eq!(t.expect("Drawable").kind, ChunkKind::Trait);
    }

    #[test]
    fn test_impl_block() {
        let src = r#"
impl Config {
    pub fn new() -> Self {
        Config { name: String::new(), port: 8080 }
    }

    fn validate(&self) -> bool {
        true
    }
}
"#;
        let elements = parse_rust(src);

        let imp = elements.iter().find(|e| e.name == "impl Config");
        assert!(imp.is_some());
        assert_eq!(imp.expect("impl Config").kind, ChunkKind::Impl);

        let new_fn = elements.iter().find(|e| e.name == "new");
        assert!(new_fn.is_some());
        assert_eq!(new_fn.expect("new").visibility, Visibility::Public);

        let validate_fn = elements.iter().find(|e| e.name == "validate");
        assert!(validate_fn.is_some());
        assert_eq!(validate_fn.expect("validate").visibility, Visibility::Private);
    }

    #[test]
    fn test_trait_impl() {
        let src = r#"
impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
"#;
        let elements = parse_rust(src);
        let imp = elements
            .iter()
            .find(|e| e.name.contains("Display") && e.name.contains("Config"));
        assert!(imp.is_some());
        assert_eq!(imp.expect("impl Display for Config").kind, ChunkKind::Impl);
    }

    #[test]
    fn test_test_function() {
        let src = r#"
#[test]
fn test_addition() {
    assert_eq!(1 + 1, 2);
}
"#;
        let elements = parse_rust(src);
        let test = elements.iter().find(|e| e.name == "test_addition");
        assert!(test.is_some());
        assert_eq!(test.expect("test_addition").kind, ChunkKind::Test);
    }

    #[test]
    fn test_const_and_static() {
        let src = r#"
pub const MAX_SIZE: usize = 1024;
static COUNTER: AtomicU64 = AtomicU64::new(0);
"#;
        let elements = parse_rust(src);

        let c = elements.iter().find(|e| e.name == "MAX_SIZE");
        assert!(c.is_some());
        assert_eq!(c.expect("MAX_SIZE").kind, ChunkKind::Const);
        assert_eq!(c.expect("MAX_SIZE").visibility, Visibility::Public);

        let s = elements.iter().find(|e| e.name == "COUNTER");
        assert!(s.is_some());
        assert_eq!(s.expect("COUNTER").kind, ChunkKind::Const);
    }

    #[test]
    fn test_type_alias() {
        let src = r#"
pub type Result<T> = std::result::Result<T, Error>;
"#;
        let elements = parse_rust(src);
        let t = elements.iter().find(|e| e.name == "Result");
        assert!(t.is_some());
        assert_eq!(t.expect("Result").kind, ChunkKind::TypeDef);
    }

    #[test]
    fn test_module() {
        let src = r#"
mod tests {
    fn helper() {}
}
"#;
        let elements = parse_rust(src);
        let m = elements.iter().find(|e| e.name == "tests");
        assert!(m.is_some());
        assert_eq!(m.expect("tests").kind, ChunkKind::Module);

        let h = elements.iter().find(|e| e.name == "helper");
        assert!(h.is_some());
        assert!(
            h.expect("helper")
                .symbol_path
                .contains("tests::helper")
        );
    }

    #[test]
    fn test_multiline_doc_comment() {
        let src = r#"
/// Perform the computation.
///
/// This function does amazing things.
/// It takes a value and doubles it.
pub fn compute(x: i32) -> i32 {
    x * 2
}
"#;
        let elements = parse_rust(src);
        let func = elements.iter().find(|e| e.name == "compute");
        assert!(func.is_some());
        let doc = func.expect("compute").doc_comment.as_ref();
        assert!(doc.is_some());
        let doc_text = doc.expect("doc");
        assert!(doc_text.contains("Perform the computation"));
        assert!(doc_text.contains("doubles it"));
    }

    #[test]
    fn test_empty_file() {
        let elements = parse_rust("");
        assert!(elements.is_empty());
    }

    #[test]
    fn test_symbol_path_uses_double_colon() {
        let src = r#"
impl Config {
    pub fn new() -> Self {
        Config {}
    }
}
"#;
        let elements = parse_rust(src);
        let new_fn = elements.iter().find(|e| e.name == "new");
        assert!(new_fn.is_some());
        let path = &new_fn.expect("new").symbol_path;
        // Rust paths use :: not .
        assert!(path.contains("::"), "path should use '::' separator: {path}");
    }
}
