//! Python structural extractor for OmniContext.
//!
//! Extracts functions, classes, methods, decorators, imports, docstrings,
//! and nested definitions from Python source files using tree-sitter.
//!
//! ## Python AST Node Types (tree-sitter-python)
//!
//! - `function_definition` -> Function/Test
//! - `class_definition` -> Class
//! - `decorated_definition` -> wraps function/class with decorators
//! - `import_statement`, `import_from_statement` -> references
//! - `expression_statement > string` (first child of body) -> docstring

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::{ChunkKind, DependencyKind, ImportStatement, Visibility};

/// Analyzer for Python source files.
pub struct PythonAnalyzer;

impl LanguageAnalyzer for PythonAnalyzer {
    fn language_id(&self) -> &str {
        "python"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn extract_structure(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> Vec<StructuralElement> {
        let mut elements = Vec::new();
        let module_name_str =
            crate::parser::build_module_name_from_path(file_path).replace("/", ".");
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
                // `import foo` or `import foo.bar`
                "import_statement" => {
                    let mut inner = child.walk();
                    for name_node in child.children(&mut inner) {
                        if name_node.kind() == "dotted_name" || name_node.kind() == "aliased_import"
                        {
                            let text = if name_node.kind() == "aliased_import" {
                                // `import foo as f` -> extract "foo"
                                name_node
                                    .child_by_field_name("name")
                                    .map(|n| node_text(n, source))
                                    .unwrap_or("")
                            } else {
                                node_text(name_node, source)
                            };
                            if !text.is_empty() {
                                imports.push(ImportStatement {
                                    import_path: text.to_string(),
                                    imported_names: vec![],
                                    line,
                                    kind: DependencyKind::Imports,
                                });
                            }
                        }
                    }
                }
                // `from foo import bar, baz` or `from foo.bar import *`
                "import_from_statement" => {
                    let module_path = child
                        .child_by_field_name("module_name")
                        .map(|n| node_text(n, source).to_string())
                        .unwrap_or_default();

                    if module_path.is_empty() {
                        continue;
                    }

                    let mut names = Vec::new();
                    let mut inner = child.walk();
                    for name_node in child.children(&mut inner) {
                        if name_node.kind() == "dotted_name"
                            && name_node
                                != child
                                    .child_by_field_name("module_name")
                                    .unwrap_or(name_node)
                        {
                            names.push(node_text(name_node, source).to_string());
                        } else if name_node.kind() == "aliased_import" {
                            if let Some(n) = name_node.child_by_field_name("name") {
                                names.push(node_text(n, source).to_string());
                            }
                        } else if name_node.kind() == "wildcard_import" {
                            names.push("*".to_string());
                        }
                    }

                    imports.push(ImportStatement {
                        import_path: module_path,
                        imported_names: names,
                        line,
                        kind: DependencyKind::Imports,
                    });
                }
                _ => {}
            }
        }

        imports
    }
}

impl PythonAnalyzer {
    /// Recursively walk the AST and extract structural elements.
    ///
    /// `scope_path` tracks the current nesting (e.g., ["module", "ClassName"])
    /// so we can build fully qualified symbol paths.
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
                        self.extract_function(child, source, module_name, scope_path, &[])
                    {
                        // Recurse into function body for nested defs
                        let mut inner_scope = scope_path.to_vec();
                        inner_scope.push(elem.name.clone());
                        if let Some(body) = child.child_by_field_name("body") {
                            self.walk_node(body, source, module_name, &inner_scope, elements);
                        }
                        elements.push(elem);
                    }
                }
                "class_definition" => {
                    if let Some(elem) =
                        self.extract_class(child, source, module_name, scope_path, &[])
                    {
                        // Recurse into class body for methods and nested classes
                        let mut inner_scope = scope_path.to_vec();
                        inner_scope.push(elem.name.clone());
                        if let Some(body) = child.child_by_field_name("body") {
                            self.walk_node(body, source, module_name, &inner_scope, elements);
                        }
                        elements.push(elem);
                    }
                }
                "decorated_definition" => {
                    let decorators = self.extract_decorators(child, source);
                    // The actual definition is a child of decorated_definition
                    let mut inner_cursor = child.walk();
                    for inner_child in child.children(&mut inner_cursor) {
                        match inner_child.kind() {
                            "function_definition" => {
                                if let Some(elem) = self.extract_function(
                                    inner_child,
                                    source,
                                    module_name,
                                    scope_path,
                                    &decorators,
                                ) {
                                    let mut inner_scope = scope_path.to_vec();
                                    inner_scope.push(elem.name.clone());
                                    if let Some(body) = inner_child.child_by_field_name("body") {
                                        self.walk_node(
                                            body,
                                            source,
                                            module_name,
                                            &inner_scope,
                                            elements,
                                        );
                                    }
                                    // Use the decorated_definition's span for full content
                                    let mut elem = elem;
                                    elem.line_start = child.start_position().row as u32 + 1;
                                    elem.content = node_text(child, source).to_string();
                                    elements.push(elem);
                                }
                            }
                            "class_definition" => {
                                if let Some(elem) = self.extract_class(
                                    inner_child,
                                    source,
                                    module_name,
                                    scope_path,
                                    &decorators,
                                ) {
                                    let mut inner_scope = scope_path.to_vec();
                                    inner_scope.push(elem.name.clone());
                                    if let Some(body) = inner_child.child_by_field_name("body") {
                                        self.walk_node(
                                            body,
                                            source,
                                            module_name,
                                            &inner_scope,
                                            elements,
                                        );
                                    }
                                    let mut elem = elem;
                                    elem.line_start = child.start_position().row as u32 + 1;
                                    elem.content = node_text(child, source).to_string();
                                    elements.push(elem);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                "import_statement" | "import_from_statement" => {
                    // We don't emit these as elements, but we could
                    // extract references for the dependency graph later.
                }
                _ => {
                    // Recurse into compound statements (if/for/with/try blocks)
                    // that might contain definitions
                    if child.child_count() > 0 {
                        self.walk_node(child, source, module_name, scope_path, elements);
                    }
                }
            }
        }
    }

    /// Extract a function/method definition.
    fn extract_function(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
        decorators: &[String],
    ) -> Option<StructuralElement> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source).to_string();

        let symbol_path = build_symbol_path(module_name, scope_path, &name);
        let visibility = python_visibility(&name);
        let kind = determine_function_kind(&name, decorators);
        let doc_comment = self.extract_docstring(node, source);
        let references = self.extract_function_references(node, source);

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
            extends: Vec::new(),
            implements: Vec::new(),
        })
    }

    /// Extract a class definition.
    fn extract_class(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
        module_name: &str,
        scope_path: &[String],
        _decorators: &[String],
    ) -> Option<StructuralElement> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source).to_string();

        let symbol_path = build_symbol_path(module_name, scope_path, &name);
        let visibility = python_visibility(&name);
        let doc_comment = self.extract_docstring(node, source);

        // Extract base classes as references
        let mut references = Vec::new();
        if let Some(args) = node.child_by_field_name("superclasses") {
            let mut cursor = args.walk();
            for child in args.children(&mut cursor) {
                if child.kind() == "identifier" || child.kind() == "attribute" {
                    references.push(node_text(child, source).to_string());
                }
            }
        }

        Some(StructuralElement {
            symbol_path,
            name,
            kind: ChunkKind::Class,
            visibility,
            line_start: node.start_position().row as u32 + 1,
            line_end: node.end_position().row as u32 + 1,
            content: node_text(node, source).to_string(),
            doc_comment,
            references,
            extends: Vec::new(),
            implements: Vec::new(),
        })
    }

    /// Extract decorator names from a `decorated_definition` node.
    fn extract_decorators(&self, node: tree_sitter::Node<'_>, source: &[u8]) -> Vec<String> {
        let mut decorators = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "decorator" {
                // The decorator content is `@name` or `@name(args)`
                // We want just the name part
                let text = node_text(child, source);
                let name = text
                    .strip_prefix('@')
                    .unwrap_or(text)
                    .split('(')
                    .next()
                    .unwrap_or(text)
                    .trim();
                decorators.push(name.to_string());
            }
        }

        decorators
    }

    /// Extract docstring from the first statement in a function/class body.
    ///
    /// Python docstrings are the first `expression_statement` containing
    /// a string literal in the body block.
    fn extract_docstring(&self, node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
        let body = node.child_by_field_name("body")?;
        let first_stmt = body.child(0)?;

        if first_stmt.kind() != "expression_statement" {
            return None;
        }

        let expr = first_stmt.child(0)?;
        if expr.kind() != "string" && expr.kind() != "concatenated_string" {
            return None;
        }

        let raw = node_text(expr, source);
        Some(clean_docstring(raw))
    }

    /// Extract symbol references from function body (calls, attribute access).
    fn extract_function_references(
        &self,
        node: tree_sitter::Node<'_>,
        source: &[u8],
    ) -> Vec<String> {
        let mut refs = Vec::new();
        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => return refs,
        };

        self.collect_calls(body, source, &mut refs);
        refs.sort();
        refs.dedup();
        refs
    }

    /// Recursively collect function call targets.
    fn collect_calls(&self, node: tree_sitter::Node<'_>, source: &[u8], refs: &mut Vec<String>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "call" {
                if let Some(func) = child.child_by_field_name("function") {
                    let text = node_text(func, source);
                    if !text.is_empty() {
                        refs.push(text.to_string());
                    }
                }
            }
            // Recurse
            if child.child_count() > 0 {
                self.collect_calls(child, source, refs);
            }
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
    parts.join(".")
}

/// Determine Python visibility from naming convention.
///
/// - `__dunder__` methods are public (special methods)
/// - `__private` (name-mangled) is private
/// - `_protected` is protected
/// - Everything else is public
fn python_visibility(name: &str) -> Visibility {
    if name.starts_with("__") && name.ends_with("__") {
        Visibility::Public // dunder methods
    } else if name.starts_with("__") {
        Visibility::Private // name-mangled
    } else if name.starts_with('_') {
        Visibility::Protected // convention-private
    } else {
        Visibility::Public
    }
}

/// Determine function kind from name and decorators.
fn determine_function_kind(name: &str, decorators: &[String]) -> ChunkKind {
    // Check if it's a test function
    if name.starts_with("test_") || name == "test" {
        return ChunkKind::Test;
    }

    // Check decorators for special kinds
    for dec in decorators {
        if dec == "pytest.fixture" || dec == "fixture" {
            return ChunkKind::Test;
        }
    }

    ChunkKind::Function
}

/// Clean a Python docstring by stripping triple quotes and normalizing whitespace.
fn clean_docstring(raw: &str) -> String {
    let stripped = raw
        .trim()
        .strip_prefix("\"\"\"")
        .or_else(|| raw.trim().strip_prefix("'''"))
        .unwrap_or(raw);

    let stripped = stripped
        .strip_suffix("\"\"\"")
        .or_else(|| stripped.strip_suffix("'''"))
        .unwrap_or(stripped);

    stripped.trim().to_string()
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

    /// Helper: parse Python source and return elements.
    fn parse_python(source: &str) -> Vec<StructuralElement> {
        parse_file(Path::new("test.py"), source.as_bytes(), Language::Python)
            .expect("parse should succeed")
    }

    #[test]
    fn test_simple_function() {
        let src = r#"
def hello(name):
    """Greet someone."""
    print(f"Hello, {name}!")
"#;
        let elements = parse_python(src);
        assert_eq!(elements.len(), 1);

        let func = &elements[0];
        assert_eq!(func.name, "hello");
        assert_eq!(func.kind, ChunkKind::Function);
        assert_eq!(func.visibility, Visibility::Public);
        assert_eq!(func.doc_comment.as_deref(), Some("Greet someone."));
        assert!(func.symbol_path.ends_with(".hello"));
    }

    #[test]
    fn test_private_function() {
        let src = r#"
def _private_helper():
    pass

def __mangled_name():
    pass
"#;
        let elements = parse_python(src);
        assert_eq!(elements.len(), 2);

        assert_eq!(elements[0].name, "_private_helper");
        assert_eq!(elements[0].visibility, Visibility::Protected);

        assert_eq!(elements[1].name, "__mangled_name");
        assert_eq!(elements[1].visibility, Visibility::Private);
    }

    #[test]
    fn test_class_with_methods() {
        let src = r#"
class UserService:
    """Service for managing users."""

    def __init__(self, db):
        self.db = db

    def get_user(self, user_id):
        """Retrieve a user by ID."""
        return self.db.find(user_id)

    def _validate(self, data):
        pass
"#;
        let elements = parse_python(src);

        // Should have: class UserService, __init__, get_user, _validate
        let class = elements.iter().find(|e| e.name == "UserService");
        assert!(class.is_some(), "should find UserService class");
        let class = class.expect("class exists");
        assert_eq!(class.kind, ChunkKind::Class);
        assert_eq!(
            class.doc_comment.as_deref(),
            Some("Service for managing users.")
        );

        let init = elements.iter().find(|e| e.name == "__init__");
        assert!(init.is_some(), "should find __init__");
        let init = init.expect("init exists");
        assert_eq!(init.visibility, Visibility::Public); // dunder = public

        let validate = elements.iter().find(|e| e.name == "_validate");
        assert!(validate.is_some(), "should find _validate");
        assert_eq!(
            validate.expect("validate exists").visibility,
            Visibility::Protected
        );
    }

    #[test]
    fn test_test_function_detection() {
        let src = r#"
def test_addition():
    assert 1 + 1 == 2

def test():
    pass

def helper_function():
    pass
"#;
        let elements = parse_python(src);

        let test_add = elements.iter().find(|e| e.name == "test_addition");
        assert_eq!(test_add.expect("test_addition").kind, ChunkKind::Test);

        let test_bare = elements.iter().find(|e| e.name == "test");
        assert_eq!(test_bare.expect("test").kind, ChunkKind::Test);

        let helper = elements.iter().find(|e| e.name == "helper_function");
        assert_eq!(helper.expect("helper_function").kind, ChunkKind::Function);
    }

    #[test]
    fn test_decorated_function() {
        let src = r#"
@staticmethod
def create_default():
    return Config()

@app.route("/api/users")
def list_users():
    """List all users."""
    pass
"#;
        let elements = parse_python(src);
        assert_eq!(elements.len(), 2);

        assert_eq!(elements[0].name, "create_default");
        assert_eq!(elements[1].name, "list_users");
        assert_eq!(elements[1].doc_comment.as_deref(), Some("List all users."));
    }

    #[test]
    fn test_class_inheritance() {
        let src = r#"
class Dog(Animal):
    def bark(self):
        pass

class ServiceError(ValueError, CustomMixin):
    pass
"#;
        let elements = parse_python(src);

        let dog = elements.iter().find(|e| e.name == "Dog");
        assert!(dog.is_some());
        assert!(dog.expect("Dog").references.contains(&"Animal".to_string()));

        let err = elements.iter().find(|e| e.name == "ServiceError");
        assert!(err.is_some());
        let err = err.expect("ServiceError");
        assert!(err.references.contains(&"ValueError".to_string()));
    }

    #[test]
    fn test_nested_class() {
        let src = r#"
class Outer:
    class Inner:
        def method(self):
            pass
"#;
        let elements = parse_python(src);

        let inner = elements.iter().find(|e| e.name == "Inner");
        assert!(inner.is_some());
        assert!(inner.expect("Inner").symbol_path.contains("Outer.Inner"));

        let method = elements.iter().find(|e| e.name == "method");
        assert!(method.is_some());
        assert!(method
            .expect("method")
            .symbol_path
            .contains("Outer.Inner.method"));
    }

    #[test]
    fn test_multiline_docstring() {
        let src = r#"
def complex_function(a, b, c):
    """
    Perform a complex computation.

    Args:
        a: First argument
        b: Second argument
        c: Third argument

    Returns:
        The computed result
    """
    return a + b + c
"#;
        let elements = parse_python(src);
        assert_eq!(elements.len(), 1);

        let doc = elements[0].doc_comment.as_ref().expect("has docstring");
        assert!(doc.contains("Perform a complex computation"));
        assert!(doc.contains("Args:"));
        assert!(doc.contains("Returns:"));
    }

    #[test]
    fn test_function_references() {
        let src = r#"
def process_data(items):
    validated = validate_input(items)
    result = transform(validated)
    logger.info("done")
    return serialize(result)
"#;
        let elements = parse_python(src);
        assert_eq!(elements.len(), 1);

        let refs = &elements[0].references;
        assert!(refs.contains(&"validate_input".to_string()));
        assert!(refs.contains(&"transform".to_string()));
        assert!(refs.contains(&"serialize".to_string()));
        assert!(refs.contains(&"logger.info".to_string()));
    }

    #[test]
    fn test_line_numbers() {
        let src = "def first():\n    pass\n\ndef second():\n    pass\n";
        let elements = parse_python(src);
        assert_eq!(elements.len(), 2);

        assert_eq!(elements[0].name, "first");
        assert_eq!(elements[0].line_start, 1);
        assert_eq!(elements[0].line_end, 2);

        assert_eq!(elements[1].name, "second");
        assert_eq!(elements[1].line_start, 4);
        assert_eq!(elements[1].line_end, 5);
    }

    #[test]
    fn test_empty_file() {
        let elements = parse_python("");
        assert!(elements.is_empty());
    }

    #[test]
    fn test_comments_only_file() {
        let src = "# This is a comment\n# Another comment\n";
        let elements = parse_python(src);
        assert!(elements.is_empty());
    }

    #[test]
    fn test_visibility_helpers() {
        assert_eq!(python_visibility("public_func"), Visibility::Public);
        assert_eq!(python_visibility("_protected"), Visibility::Protected);
        assert_eq!(python_visibility("__private"), Visibility::Private);
        assert_eq!(python_visibility("__init__"), Visibility::Public);
        assert_eq!(python_visibility("__str__"), Visibility::Public);
    }

    #[test]
    fn test_clean_docstring() {
        assert_eq!(clean_docstring(r#""""hello""""#), "hello");
        assert_eq!(clean_docstring("'''hello'''"), "hello");
        assert_eq!(clean_docstring("\"\"\"  spaced  \"\"\""), "spaced");
    }
}
