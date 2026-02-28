---
description: How to add support for a new programming language to OmniContext
---

# Add Language Support Workflow

Adding a new language requires 4 components. Follow this checklist exactly.

## Prerequisites

- Tree-sitter grammar crate exists for the language
- You have at least one reference repository in that language for testing

## Steps

### 1. Add Tree-sitter Grammar Dependency

Edit `crates/omni-core/Cargo.toml`:

```toml
[dependencies]
tree-sitter-<language> = "<version>"
```

### 2. Create Language Analyzer Module

Create `crates/omni-core/src/parser/languages/<language>.rs`:

```rust
//! <Language> structural extractor for OmniContext.
//!
//! Maps tree-sitter AST nodes to OmniContext chunk kinds.

use crate::parser::{ChunkKind, LanguageAnalyzer, StructuralElement};
use tree_sitter::Node;

pub struct <Language>Analyzer;

impl LanguageAnalyzer for <Language>Analyzer {
    fn language_id(&self) -> &str { "<language>" }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_<language>::LANGUAGE.into()
    }

    fn extract_structure(&self, node: Node, source: &[u8]) -> Vec<StructuralElement> {
        // Map AST nodes to structural elements
        // Functions, classes, imports, types, etc.
        todo!()
    }

    fn resolve_import(&self, import_path: &str, file_path: &Path) -> Option<ResolvedImport> {
        // Language-specific import resolution
        todo!()
    }
}
```

### 3. Register the Language

Edit `crates/omni-core/src/parser/registry.rs`:

```rust
registry.register("<language>", Box::new(<Language>Analyzer));
```

Also add file extension mappings:

```rust
extensions.insert("<ext>", "<language>");
```

### 4. Create Test Fixture

Create `tests/fixtures/<language>/` with:

- A small but representative code sample (100-500 lines)
- Known expected chunks (document in `expected.json`)
- Edge cases: nested functions, complex imports, generics/templates

### 5. Write Tests

Create `crates/omni-core/src/parser/languages/<language>_test.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_functions() { /* ... */ }

    #[test]
    fn test_extract_classes() { /* ... */ }

    #[test]
    fn test_resolve_imports() { /* ... */ }

    #[test]
    fn test_complex_nesting() { /* ... */ }
}
```

### 6. Integration Test

Add to `tests/integration/parser_test.rs`:

```rust
#[test]
fn test_<language>_full_pipeline() {
    let repo = fixture_repo("<language>");
    let chunks = index_repository(&repo);
    assert!(chunks.len() > 0);
    // Verify chunk kinds, boundaries, metadata
}
```

### 7. Update Documentation

- Add language to supported languages table in README
- Update `docs/SUPPORTED_LANGUAGES.md`
- Add language-specific configuration notes if any

### 8. Verify

```bash
cargo test --workspace -k "<language>"
cargo clippy --workspace -- -D warnings
```

## Estimated Time: 2-3 days per language
