//! Per-language tree-sitter analyzers.
//!
//! Each language module implements the `LanguageAnalyzer` trait.
//! They are registered in the `registry` module at startup.

pub mod c;
pub mod cpp;
pub mod csharp;
pub mod css;
pub mod document;
pub mod go;
pub mod java;
pub mod javascript;
pub mod kotlin;
pub mod php;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod swift;
pub mod typescript;
