//! Per-language tree-sitter analyzers.
//!
//! Each language module implements the `LanguageAnalyzer` trait.
//! They are registered in the `registry` module at startup.

pub mod python;
pub mod rust;
pub mod typescript;
pub mod javascript;
pub mod go;
