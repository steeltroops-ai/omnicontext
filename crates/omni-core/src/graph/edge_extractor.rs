//! AST-based edge extraction for file dependency graph.
//!
//! This module analyzes parsed AST to extract structural dependencies:
//! - IMPORTS: Import/require/use statements
//! - INHERITS: Class inheritance (extends/implements)
//! - CALLS: Function calls between files
//! - INSTANTIATES: Class instantiation (new/constructor calls)
//!
//! ## Language Support
//! - Rust: use statements, trait implementations, function calls
//! - Python: import/from statements, class inheritance, function calls
//! - TypeScript/JavaScript: import/require, extends/implements, function calls
//! - Go: import statements, interface implementations, function calls
//! - Java: import statements, extends/implements, method calls
//! - C/C++: #include directives, inheritance, function calls
//!
//! ## Performance Target
//! - Extract edges for 1000+ files in <5 seconds
//! - Parallel processing of independent files

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::OmniResult;
use crate::graph::dependencies::{DependencyEdge, EdgeType};
use crate::parser::StructuralElement;
use crate::types::{ChunkKind, Language};

// ---------------------------------------------------------------------------
// SymbolIndex — cross-file callee resolution
// ---------------------------------------------------------------------------

/// In-process reverse map: symbol short name → defining file paths.
///
/// Built once after the Rayon parse phase, before the sequential store phase.
/// Enables `EdgeExtractor` to resolve CALLS/INSTANTIATES callees to source
/// files even when `ImportResolver` has no registered entry for them.
///
/// ## Accuracy
/// ~70% — misses overloaded names and dynamic dispatch, sufficient for graph
/// heuristics.  Disambiguation order: (1) same directory as caller,
/// (2) first alphabetically.
pub struct SymbolIndex {
    /// Short symbol name → sorted list of defining file paths.
    symbol_to_files: HashMap<String, Vec<PathBuf>>,
}

impl SymbolIndex {
    /// Build from all `(path, elements)` pairs produced during the indexing run.
    pub fn build(all_elements: &[(PathBuf, &[StructuralElement])]) -> Self {
        let mut symbol_to_files: HashMap<String, Vec<PathBuf>> = HashMap::new();

        for (path, elements) in all_elements {
            for elem in *elements {
                if elem.name.is_empty() {
                    continue;
                }
                symbol_to_files
                    .entry(elem.name.clone())
                    .or_default()
                    .push(path.clone());
            }
        }

        // Sort each candidate list for deterministic disambiguation.
        for candidates in symbol_to_files.values_mut() {
            candidates.sort();
            candidates.dedup();
        }

        Self { symbol_to_files }
    }

    /// Resolve a callee short name to the most likely defining source file.
    ///
    /// Disambiguation order:
    /// 1. Same directory as `caller_path` (locality bias).
    /// 2. First path alphabetically (deterministic fallback).
    ///
    /// Returns `None` when the name is unknown or the only candidate is the
    /// caller itself.
    pub fn resolve(&self, callee: &str, caller_path: &Path) -> Option<&PathBuf> {
        let candidates = self.symbol_to_files.get(callee)?;

        let caller_dir = caller_path.parent();

        // Prefer same-directory candidates.
        if let Some(caller_dir) = caller_dir {
            for candidate in candidates {
                if candidate.parent() == Some(caller_dir) && candidate != caller_path {
                    return Some(candidate);
                }
            }
        }

        // Fall back to first alphabetically that isn't the caller.
        candidates.iter().find(|p| p.as_path() != caller_path)
    }
}

// ---------------------------------------------------------------------------
// EdgeExtractor
// ---------------------------------------------------------------------------

/// Extracts dependency edges from parsed AST elements.
pub struct EdgeExtractor {
    /// Map from import path to resolved file path
    import_resolver: ImportResolver,
    /// Optional symbol index for cross-file callee resolution.
    /// Read via `self.symbol_index.as_ref()` in CALLS/INSTANTIATES extraction.
    #[allow(dead_code)]
    symbol_index: Option<Arc<SymbolIndex>>,
}

impl EdgeExtractor {
    /// Create a new edge extractor.
    pub fn new() -> Self {
        Self {
            import_resolver: ImportResolver::new(),
            symbol_index: None,
        }
    }

    /// Create an edge extractor with a pre-built symbol index for cross-file
    /// CALLS/INSTANTIATES resolution.
    pub fn with_symbol_index(symbol_index: Arc<SymbolIndex>) -> Self {
        Self {
            import_resolver: ImportResolver::new(),
            symbol_index: Some(symbol_index),
        }
    }

    /// Extract all dependency edges from a file's AST elements.
    ///
    /// Returns edges with source = current file, target = dependency file.
    pub fn extract_edges(
        &mut self,
        file_path: &Path,
        language: Language,
        elements: &[StructuralElement],
    ) -> OmniResult<Vec<DependencyEdge>> {
        let mut edges = Vec::new();

        // Note: IMPORTS edges are extracted separately in pipeline/mod.rs using parse_imports()
        // This avoids duplication since StructuralElement doesn't have an imports field

        // Extract INHERITS edges from class inheritance
        edges.extend(self.extract_inheritance_edges(file_path, language, elements)?);

        // Extract CALLS edges from function calls
        edges.extend(self.extract_call_edges(file_path, language, elements)?);

        // Extract INSTANTIATES edges from class instantiation
        edges.extend(self.extract_instantiation_edges(file_path, language, elements)?);

        Ok(edges)
    }

    /// Extract IMPORTS edges from import/require/use statements.
    /// Extract INHERITS edges from class inheritance (extends/implements).
    fn extract_inheritance_edges(
        &mut self,
        file_path: &Path,
        language: Language,
        elements: &[StructuralElement],
    ) -> OmniResult<Vec<DependencyEdge>> {
        let mut edges = Vec::new();

        for element in elements {
            if element.kind != ChunkKind::Class {
                continue;
            }

            // Process extends relationships
            for parent_class in &element.extends {
                if let Some(target_path) = self
                    .import_resolver
                    .resolve_type(file_path, parent_class, language)
                {
                    edges.push(DependencyEdge {
                        source: file_path.to_path_buf(),
                        target: target_path,
                        edge_type: EdgeType::Inherits,
                        weight: 1.0,
                    });
                }
            }

            // Process implements relationships
            for interface in &element.implements {
                if let Some(target_path) = self
                    .import_resolver
                    .resolve_type(file_path, interface, language)
                {
                    edges.push(DependencyEdge {
                        source: file_path.to_path_buf(),
                        target: target_path,
                        edge_type: EdgeType::Inherits,
                        weight: 1.0,
                    });
                }
            }
        }

        Ok(edges)
    }

    /// Extract CALLS edges from function calls.
    fn extract_call_edges(
        &mut self,
        file_path: &Path,
        language: Language,
        elements: &[StructuralElement],
    ) -> OmniResult<Vec<DependencyEdge>> {
        let mut edges = Vec::new();
        let mut seen_targets: HashSet<PathBuf> = HashSet::new();

        for element in elements {
            if element.references.is_empty() {
                continue;
            }

            for reference in &element.references {
                // Primary resolution: ImportResolver module-prefix lookup.
                let resolved = self
                    .import_resolver
                    .resolve_reference(file_path, reference, language);

                // Fallback: SymbolIndex short-name lookup when ImportResolver
                // has no entry (the common case for cross-file calls in
                // repos where the module registry is sparse).
                let target_path = resolved.or_else(|| {
                    let callee = reference.split('.').next_back().unwrap_or(reference);
                    self.symbol_index
                        .as_ref()
                        .and_then(|idx| idx.resolve(callee, file_path))
                        .cloned()
                });

                if let Some(target_path) = target_path {
                    if target_path != file_path && seen_targets.insert(target_path.clone()) {
                        edges.push(DependencyEdge {
                            source: file_path.to_path_buf(),
                            target: target_path,
                            edge_type: EdgeType::Calls,
                            weight: 1.0,
                        });
                    }
                }
            }
        }

        Ok(edges)
    }

    /// Extract INSTANTIATES edges from class instantiation (new/constructor calls).
    fn extract_instantiation_edges(
        &mut self,
        file_path: &Path,
        language: Language,
        elements: &[StructuralElement],
    ) -> OmniResult<Vec<DependencyEdge>> {
        let mut edges = Vec::new();
        let mut seen_targets: HashSet<PathBuf> = HashSet::new();

        for element in elements {
            // Look for "new ClassName()" patterns in references
            for reference in &element.references {
                if self.is_instantiation(reference, language) {
                    let class_name = self.extract_class_name(reference);

                    // Primary resolution: ImportResolver type lookup.
                    let resolved = self
                        .import_resolver
                        .resolve_type(file_path, &class_name, language);

                    // Fallback: SymbolIndex when class is defined in another
                    // file not yet in the ImportResolver registry.
                    let target_path = resolved.or_else(|| {
                        self.symbol_index
                            .as_ref()
                            .and_then(|idx| idx.resolve(&class_name, file_path))
                            .cloned()
                    });

                    if let Some(target_path) = target_path {
                        if target_path != file_path && seen_targets.insert(target_path.clone()) {
                            edges.push(DependencyEdge {
                                source: file_path.to_path_buf(),
                                target: target_path,
                                edge_type: EdgeType::Instantiates,
                                weight: 1.0,
                            });
                        }
                    }
                }
            }
        }

        Ok(edges)
    }

    /// Check if a reference is a class instantiation.
    fn is_instantiation(&self, reference: &str, language: Language) -> bool {
        match language {
            Language::JavaScript | Language::TypeScript => {
                reference.starts_with("new ") || reference.contains("new ")
            }
            Language::Python => {
                reference.ends_with("()")
                    && reference.chars().next().is_some_and(|c| c.is_uppercase())
            }
            Language::Java | Language::Kotlin => reference.starts_with("new "),
            Language::Rust => false, // Rust uses struct literals, not "new"
            _ => false,
        }
    }

    /// Extract class name from instantiation reference.
    fn extract_class_name(&self, reference: &str) -> String {
        // Remove "new " prefix and "()" suffix
        reference
            .trim_start_matches("new ")
            .trim_end_matches("()")
            .split('(')
            .next()
            .unwrap_or(reference)
            .trim()
            .to_string()
    }

    /// Register a file in the import resolver for future resolution.
    pub fn register_file(&mut self, file_path: PathBuf, module_name: String) {
        self.import_resolver.register_file(file_path, module_name);
    }
}

impl Default for EdgeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolves import paths to actual file paths.
///
/// Maintains a registry of files and their module names for cross-file resolution.
pub struct ImportResolver {
    /// Map from module name to file path
    module_to_file: HashMap<String, PathBuf>,
    /// Map from file path to module name
    file_to_module: HashMap<PathBuf, String>,
}

impl ImportResolver {
    /// Create a new import resolver.
    pub fn new() -> Self {
        Self {
            module_to_file: HashMap::new(),
            file_to_module: HashMap::new(),
        }
    }

    /// Register a file with its module name.
    pub fn register_file(&mut self, file_path: PathBuf, module_name: String) {
        self.module_to_file
            .insert(module_name.clone(), file_path.clone());
        self.file_to_module.insert(file_path, module_name);
    }

    /// Resolve an import statement to a file path.
    pub fn resolve_import(
        &self,
        source_file: &Path,
        import_path: &str,
        language: Language,
    ) -> Option<PathBuf> {
        match language {
            Language::Rust => self.resolve_rust_import(source_file, import_path),
            Language::Python => self.resolve_python_import(source_file, import_path),
            Language::JavaScript | Language::TypeScript => {
                self.resolve_js_import(source_file, import_path)
            }
            Language::Go => self.resolve_go_import(source_file, import_path),
            Language::Java | Language::Kotlin => self.resolve_java_import(source_file, import_path),
            Language::C | Language::Cpp => self.resolve_c_import(source_file, import_path),
            _ => None,
        }
    }

    /// Resolve a type reference to a file path.
    pub fn resolve_type(
        &self,
        source_file: &Path,
        type_name: &str,
        language: Language,
    ) -> Option<PathBuf> {
        // Try module registry first
        if let Some(path) = self.module_to_file.get(type_name) {
            return Some(path.clone());
        }

        // Try relative resolution based on naming conventions
        self.resolve_by_convention(source_file, type_name, language)
    }

    /// Resolve a function/variable reference to a file path.
    pub fn resolve_reference(
        &self,
        _source_file: &Path,
        reference: &str,
        _language: Language,
    ) -> Option<PathBuf> {
        // Extract module prefix if present (e.g., "module.function" -> "module")
        let module_name = reference.split('.').next()?;
        self.module_to_file.get(module_name).cloned()
    }

    // Language-specific import resolution

    fn resolve_rust_import(&self, source_file: &Path, import_path: &str) -> Option<PathBuf> {
        // Rust: "use crate::module::Type" or "use super::module"
        if import_path.starts_with("crate::") {
            let module_path = import_path.strip_prefix("crate::")?;
            return self.resolve_crate_relative(source_file, module_path);
        }

        if import_path.starts_with("super::") {
            let module_path = import_path.strip_prefix("super::")?;
            return self.resolve_parent_relative(source_file, module_path);
        }

        // External crate or std library - skip
        None
    }

    fn resolve_python_import(&self, source_file: &Path, import_path: &str) -> Option<PathBuf> {
        // Python: "from module import Class" or "import module"
        let module_name = import_path.split('.').next()?;

        // Try module registry
        if let Some(path) = self.module_to_file.get(module_name) {
            return Some(path.clone());
        }

        // Try relative import
        if import_path.starts_with('.') {
            return self.resolve_python_relative(source_file, import_path);
        }

        None
    }

    fn resolve_js_import(&self, source_file: &Path, import_path: &str) -> Option<PathBuf> {
        // JavaScript/TypeScript: "./module" or "../module" or "module"
        if import_path.starts_with("./") || import_path.starts_with("../") {
            return self.resolve_relative_path(source_file, import_path);
        }

        // Try module registry for absolute imports
        self.module_to_file.get(import_path).cloned()
    }

    fn resolve_go_import(&self, _source_file: &Path, import_path: &str) -> Option<PathBuf> {
        // Go: "github.com/user/repo/package"
        // Extract package name (last component)
        let package_name = import_path.split('/').next_back()?;
        self.module_to_file.get(package_name).cloned()
    }

    fn resolve_java_import(&self, _source_file: &Path, import_path: &str) -> Option<PathBuf> {
        // Java: "com.example.package.Class"
        // Extract class name (last component)
        let class_name = import_path.split('.').next_back()?;
        self.module_to_file.get(class_name).cloned()
    }

    fn resolve_c_import(&self, source_file: &Path, import_path: &str) -> Option<PathBuf> {
        // C/C++: #include "header.h" or #include <header.h>
        let header_name = import_path.trim_matches(|c| c == '"' || c == '<' || c == '>');

        // Try relative to source file
        if let Some(parent) = source_file.parent() {
            let candidate = parent.join(header_name);
            if self.file_to_module.contains_key(&candidate) {
                return Some(candidate);
            }
        }

        None
    }

    // Helper methods for path resolution

    fn resolve_crate_relative(&self, source_file: &Path, module_path: &str) -> Option<PathBuf> {
        // Find crate root (directory containing Cargo.toml or src/)
        let mut current = source_file.parent()?;
        while let Some(parent) = current.parent() {
            if parent.join("Cargo.toml").exists() || parent.join("src").exists() {
                let src_dir = parent.join("src");
                let module_file = src_dir
                    .join(module_path.replace("::", "/"))
                    .with_extension("rs");
                if self.file_to_module.contains_key(&module_file) {
                    return Some(module_file);
                }
                break;
            }
            current = parent;
        }
        None
    }

    fn resolve_parent_relative(&self, source_file: &Path, module_path: &str) -> Option<PathBuf> {
        let parent = source_file.parent()?.parent()?;
        let module_file = parent
            .join(module_path.replace("::", "/"))
            .with_extension("rs");
        if self.file_to_module.contains_key(&module_file) {
            Some(module_file)
        } else {
            None
        }
    }

    fn resolve_python_relative(&self, source_file: &Path, import_path: &str) -> Option<PathBuf> {
        let parent = source_file.parent()?;
        let dots = import_path.chars().take_while(|&c| c == '.').count();
        let module_path = import_path.trim_start_matches('.');

        let mut target_dir = parent.to_path_buf();
        for _ in 1..dots {
            target_dir = target_dir.parent()?.to_path_buf();
        }

        let module_file = target_dir
            .join(module_path.replace('.', "/"))
            .with_extension("py");
        if self.file_to_module.contains_key(&module_file) {
            Some(module_file)
        } else {
            None
        }
    }

    fn resolve_relative_path(&self, source_file: &Path, import_path: &str) -> Option<PathBuf> {
        let parent = source_file.parent()?;
        let target = parent.join(import_path);

        // Try with common extensions
        for ext in &["", ".ts", ".tsx", ".js", ".jsx"] {
            let candidate = if ext.is_empty() {
                target.clone()
            } else {
                target.with_extension(ext.trim_start_matches('.'))
            };

            if self.file_to_module.contains_key(&candidate) {
                return Some(candidate);
            }
        }

        None
    }

    fn resolve_by_convention(
        &self,
        source_file: &Path,
        type_name: &str,
        language: Language,
    ) -> Option<PathBuf> {
        // Try common naming conventions
        let parent = source_file.parent()?;

        match language {
            Language::Python => {
                // Python: ClassName -> class_name.py
                let file_name = to_snake_case(type_name) + ".py";
                let candidate = parent.join(file_name);
                if self.file_to_module.contains_key(&candidate) {
                    return Some(candidate);
                }
            }
            Language::Java | Language::Kotlin => {
                // Java: ClassName -> ClassName.java
                let file_name = format!("{}.java", type_name);
                let candidate = parent.join(file_name);
                if self.file_to_module.contains_key(&candidate) {
                    return Some(candidate);
                }
            }
            _ => {}
        }

        None
    }
}

impl Default for ImportResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert PascalCase to snake_case.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        if let Some(ch) = c.to_lowercase().next() {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("ClassName"), "class_name");
        assert_eq!(to_snake_case("HTTPServer"), "h_t_t_p_server");
        assert_eq!(to_snake_case("simple"), "simple");
    }

    #[test]
    fn test_is_instantiation() {
        let extractor = EdgeExtractor::new();

        assert!(extractor.is_instantiation("new ClassName()", Language::JavaScript));
        assert!(extractor.is_instantiation("new MyClass", Language::TypeScript));
        assert!(extractor.is_instantiation("MyClass()", Language::Python));
        assert!(!extractor.is_instantiation("myFunction()", Language::Python));
        assert!(!extractor.is_instantiation("someVar", Language::Rust));
    }

    #[test]
    fn test_extract_class_name() {
        let extractor = EdgeExtractor::new();

        assert_eq!(extractor.extract_class_name("new ClassName()"), "ClassName");
        assert_eq!(extractor.extract_class_name("new MyClass"), "MyClass");
        assert_eq!(extractor.extract_class_name("MyClass()"), "MyClass");
    }

    #[test]
    fn test_import_resolver_register() {
        let mut resolver = ImportResolver::new();
        let file_path = PathBuf::from("src/module.rs");
        resolver.register_file(file_path.clone(), "module".to_string());

        assert_eq!(resolver.module_to_file.get("module"), Some(&file_path));
        assert_eq!(
            resolver.file_to_module.get(&file_path),
            Some(&"module".to_string())
        );
    }

    #[test]
    fn test_resolve_rust_import() {
        let mut resolver = ImportResolver::new();
        let _source = PathBuf::from("src/main.rs");
        let target = PathBuf::from("src/config.rs");

        resolver.register_file(target.clone(), "config".to_string());

        // Note: Full resolution requires file system context
        // This test verifies the resolver structure
        assert!(resolver.module_to_file.contains_key("config"));
    }

    fn make_element(name: &str) -> StructuralElement {
        StructuralElement {
            symbol_path: name.to_string(),
            name: name.to_string(),
            kind: ChunkKind::Function,
            visibility: crate::types::Visibility::Public,
            line_start: 1,
            line_end: 10,
            content: String::new(),
            doc_comment: None,
            references: vec![],
            extends: vec![],
            implements: vec![],
        }
    }

    #[test]
    fn test_symbol_index_build_and_resolve_same_directory_preference() {
        // Two files define `authenticate` — one in the same dir as caller, one remote.
        // `resolve` should prefer the same-directory file.
        let auth_same = PathBuf::from("src/auth/service.rs");
        let auth_other = PathBuf::from("src/middleware/auth.rs");
        let caller = PathBuf::from("src/auth/handler.rs");

        let elem = make_element("authenticate");
        let pairs: Vec<(PathBuf, &[StructuralElement])> = vec![
            (auth_same.clone(), std::slice::from_ref(&elem)),
            (auth_other.clone(), std::slice::from_ref(&elem)),
        ];

        let idx = SymbolIndex::build(&pairs);
        let resolved = idx.resolve("authenticate", &caller);

        assert_eq!(
            resolved,
            Some(&auth_same),
            "should prefer same-directory file for callee resolution"
        );
    }

    #[test]
    fn test_symbol_index_resolve_falls_back_alphabetically() {
        // Two files defining `hash_password` in different directories, none same as caller.
        let file_a = PathBuf::from("src/crypto/hasher.rs");
        let file_b = PathBuf::from("src/utils/password.rs");
        let caller = PathBuf::from("src/api/endpoint.rs");

        let elem = make_element("hash_password");
        let pairs: Vec<(PathBuf, &[StructuralElement])> = vec![
            (file_a.clone(), std::slice::from_ref(&elem)),
            (file_b.clone(), std::slice::from_ref(&elem)),
        ];

        let idx = SymbolIndex::build(&pairs);
        let resolved = idx.resolve("hash_password", &caller);

        // Neither is same-dir; alphabetically file_a < file_b.
        assert_eq!(
            resolved,
            Some(&file_a),
            "should fall back to first alphabetically when no same-dir match"
        );
    }

    #[test]
    fn test_symbol_index_resolve_unknown_returns_none() {
        let pairs: Vec<(PathBuf, &[StructuralElement])> = vec![];
        let idx = SymbolIndex::build(&pairs);
        assert!(
            idx.resolve("unknown_func", &PathBuf::from("src/main.rs"))
                .is_none(),
            "unknown symbol should resolve to None"
        );
    }

    #[test]
    fn test_symbol_index_excludes_caller_self_reference() {
        // Caller file itself defines the symbol — should not resolve to itself.
        let caller = PathBuf::from("src/auth/service.rs");
        let other = PathBuf::from("src/util/helper.rs");

        let elem = make_element("process");
        let pairs: Vec<(PathBuf, &[StructuralElement])> = vec![
            (caller.clone(), std::slice::from_ref(&elem)),
            (other.clone(), std::slice::from_ref(&elem)),
        ];

        let idx = SymbolIndex::build(&pairs);
        let resolved = idx.resolve("process", &caller);

        assert_ne!(
            resolved,
            Some(&caller),
            "should not resolve a symbol back to the calling file itself"
        );
    }
}
