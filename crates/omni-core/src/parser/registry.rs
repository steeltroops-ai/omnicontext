//! Language analyzer registry.
//!
//! Central registration point for all language analyzers. The registry
//! is initialized once at startup and provides thread-safe access to
//! language-specific analyzers.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::types::Language;
use super::LanguageAnalyzer;

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

        // Register Phase 1 languages
        analyzers.insert(Language::Python, Box::new(super::languages::python::PythonAnalyzer));
        analyzers.insert(Language::Rust, Box::new(super::languages::rust::RustAnalyzer));
        analyzers.insert(Language::TypeScript, Box::new(super::languages::typescript::TypeScriptAnalyzer));
        analyzers.insert(Language::JavaScript, Box::new(super::languages::javascript::JavaScriptAnalyzer));
        analyzers.insert(Language::Go, Box::new(super::languages::go::GoAnalyzer));

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
    fn test_registry_has_all_phase1_languages() {
        let reg = global_registry();
        assert!(reg.get(Language::Python).is_some());
        assert!(reg.get(Language::Rust).is_some());
        assert!(reg.get(Language::TypeScript).is_some());
        assert!(reg.get(Language::JavaScript).is_some());
        assert!(reg.get(Language::Go).is_some());
    }

    #[test]
    fn test_registry_returns_none_for_unknown() {
        let reg = global_registry();
        assert!(reg.get(Language::Unknown).is_none());
    }
}
