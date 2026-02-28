//! JavaScript structural extractor for OmniContext.
//!
//! Shares most logic with the TypeScript analyzer but uses the JavaScript
//! grammar (no type annotations, no interfaces/type aliases).

use std::path::Path;

use crate::parser::{LanguageAnalyzer, StructuralElement};
use crate::types::ImportStatement;

/// Analyzer for JavaScript source files.
pub struct JavaScriptAnalyzer;

impl LanguageAnalyzer for JavaScriptAnalyzer {
    fn language_id(&self) -> &str {
        "javascript"
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_javascript::LANGUAGE.into()
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
        // Reuse the TS walker -- JS is a subset of TS AST node types
        super::typescript::walk_ts_node(root, source, module_name, &[], &mut elements);
        elements
    }

    fn extract_imports(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        _file_path: &Path,
    ) -> Vec<ImportStatement> {
        let mut imports = Vec::new();
        super::typescript::collect_ts_imports(tree.root_node(), source, &mut imports);
        imports
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_file;
    use crate::types::{ChunkKind, Language, Visibility};
    use std::path::Path;

    fn parse_js(source: &str) -> Vec<StructuralElement> {
        parse_file(
            Path::new("test.js"),
            source.as_bytes(),
            Language::JavaScript,
        )
        .expect("parse should succeed")
    }

    #[test]
    fn test_js_function() {
        let src = "function greet(name) {\n  return `Hello, ${name}`;\n}\n";
        let elements = parse_js(src);
        let func = elements.iter().find(|e| e.name == "greet");
        assert!(func.is_some());
        assert_eq!(func.expect("greet").kind, ChunkKind::Function);
    }

    #[test]
    fn test_js_class() {
        let src = r#"
class Animal {
    constructor(name) {
        this.name = name;
    }
    speak() {
        console.log(this.name);
    }
}
"#;
        let elements = parse_js(src);
        let class = elements.iter().find(|e| e.name == "Animal");
        assert!(class.is_some());
        assert_eq!(class.expect("Animal").kind, ChunkKind::Class);
    }

    #[test]
    fn test_js_arrow_function() {
        let src = "const double = (x) => x * 2;\n";
        let elements = parse_js(src);
        let func = elements.iter().find(|e| e.name == "double");
        assert!(func.is_some());
        assert_eq!(func.expect("double").kind, ChunkKind::Function);
    }

    #[test]
    fn test_js_exported() {
        let src = "export function handler(req, res) { }\n";
        let elements = parse_js(src);
        let func = elements.iter().find(|e| e.name == "handler");
        assert!(func.is_some());
        assert_eq!(func.expect("handler").visibility, Visibility::Public);
    }

    #[test]
    fn test_js_const() {
        let src = "const CONFIG = { port: 3000 };\n";
        let elements = parse_js(src);
        let c = elements.iter().find(|e| e.name == "CONFIG");
        assert!(c.is_some());
        assert_eq!(c.expect("CONFIG").kind, ChunkKind::Const);
    }
}
