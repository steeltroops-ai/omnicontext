//! Language analyzer registry.
//!
//! Central registration point for all language analyzers. The registry
//! is initialized once at startup and provides thread-safe access to
//! language-specific analyzers.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::LanguageAnalyzer;
use crate::types::Language;

/// Global registry instance.
static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Get the global language analyzer registry.
pub fn global_registry() -> &'static Registry {
    REGISTRY.get_or_init(Registry::new)
}

/// Registry of language analyzers.
pub struct Registry {
    analyzers: HashMap<Language, Box<dyn LanguageAnalyzer>>,
}

impl Registry {
    /// Create a new registry with all supported languages registered.
    fn new() -> Self {
        let mut analyzers: HashMap<Language, Box<dyn LanguageAnalyzer>> = HashMap::new();

        // Phase 1 languages (code -- full tree-sitter AST)
        analyzers.insert(
            Language::Python,
            Box::new(super::languages::python::PythonAnalyzer),
        );
        analyzers.insert(
            Language::Rust,
            Box::new(super::languages::rust::RustAnalyzer),
        );
        analyzers.insert(
            Language::TypeScript,
            Box::new(super::languages::typescript::TypeScriptAnalyzer),
        );
        analyzers.insert(
            Language::JavaScript,
            Box::new(super::languages::javascript::JavaScriptAnalyzer),
        );
        analyzers.insert(Language::Go, Box::new(super::languages::go::GoAnalyzer));

        // Phase 2 languages (code -- full tree-sitter AST)
        analyzers.insert(
            Language::Java,
            Box::new(super::languages::java::JavaAnalyzer),
        );
        analyzers.insert(Language::C, Box::new(super::languages::c::CAnalyzer));
        analyzers.insert(Language::Cpp, Box::new(super::languages::cpp::CppAnalyzer));
        analyzers.insert(
            Language::CSharp,
            Box::new(super::languages::csharp::CSharpAnalyzer),
        );
        analyzers.insert(Language::Css, Box::new(super::languages::css::CssAnalyzer));

        // Phase 4 languages (code -- full tree-sitter AST)
        analyzers.insert(
            Language::Ruby,
            Box::new(super::languages::ruby::RubyAnalyzer),
        );
        analyzers.insert(Language::Php, Box::new(super::languages::php::PhpAnalyzer));
        analyzers.insert(
            Language::Swift,
            Box::new(super::languages::swift::SwiftAnalyzer),
        );
        analyzers.insert(
            Language::Kotlin,
            Box::new(super::languages::kotlin::KotlinAnalyzer),
        );

        // Document and config formats (section-based text chunking)
        analyzers.insert(
            Language::Markdown,
            Box::new(super::languages::document::DocumentAnalyzer::new(
                Language::Markdown,
            )),
        );
        analyzers.insert(
            Language::Toml,
            Box::new(super::languages::document::DocumentAnalyzer::new(
                Language::Toml,
            )),
        );
        analyzers.insert(
            Language::Yaml,
            Box::new(super::languages::document::DocumentAnalyzer::new(
                Language::Yaml,
            )),
        );
        analyzers.insert(
            Language::Json,
            Box::new(super::languages::document::DocumentAnalyzer::new(
                Language::Json,
            )),
        );
        analyzers.insert(
            Language::Html,
            Box::new(super::languages::document::DocumentAnalyzer::new(
                Language::Html,
            )),
        );
        analyzers.insert(
            Language::Shell,
            Box::new(super::languages::document::DocumentAnalyzer::new(
                Language::Shell,
            )),
        );

        Self { analyzers }
    }

    /// Get the analyzer for a given language.
    pub fn get(&self, language: Language) -> Option<&dyn LanguageAnalyzer> {
        self.analyzers.get(&language).map(|a| a.as_ref())
    }

    /// List all registered languages.
    pub fn languages(&self) -> Vec<Language> {
        self.analyzers.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_all_code_languages() {
        let reg = global_registry();
        // Phase 1
        assert!(reg.get(Language::Python).is_some());
        assert!(reg.get(Language::Rust).is_some());
        assert!(reg.get(Language::TypeScript).is_some());
        assert!(reg.get(Language::JavaScript).is_some());
        assert!(reg.get(Language::Go).is_some());
        // Phase 2
        assert!(reg.get(Language::Java).is_some());
        assert!(reg.get(Language::C).is_some());
        assert!(reg.get(Language::Cpp).is_some());
        assert!(reg.get(Language::CSharp).is_some());
        assert!(reg.get(Language::Css).is_some());
        // Phase 4 - Additional Languages
        assert!(reg.get(Language::Ruby).is_some());
        assert!(reg.get(Language::Php).is_some());
        assert!(reg.get(Language::Swift).is_some());
        assert!(reg.get(Language::Kotlin).is_some());
    }

    #[test]
    fn test_registry_has_all_document_formats() {
        let reg = global_registry();
        assert!(reg.get(Language::Markdown).is_some());
        assert!(reg.get(Language::Toml).is_some());
        assert!(reg.get(Language::Yaml).is_some());
        assert!(reg.get(Language::Json).is_some());
        assert!(reg.get(Language::Html).is_some());
        assert!(reg.get(Language::Shell).is_some());
    }

    #[test]
    fn test_registry_returns_none_for_unknown() {
        let reg = global_registry();
        assert!(reg.get(Language::Unknown).is_none());
    }
}
