---
title: Supported Languages
description: Languages with full AST parsing support
category: Reference
order: 20
---

# Supported Languages

OmniContext provides full AST parsing for 16 languages.

## Production Support

| Language | Parser | Status |
|----------|--------|--------|
| Rust | tree-sitter-rust | ✓ Stable |
| TypeScript | tree-sitter-typescript | ✓ Stable |
| JavaScript | tree-sitter-javascript | ✓ Stable |
| Python | tree-sitter-python | ✓ Stable |
| Go | tree-sitter-go | ✓ Stable |
| Java | tree-sitter-java | ✓ Stable |
| C++ | tree-sitter-cpp | ✓ Stable |
| C# | tree-sitter-c-sharp | ✓ Stable |
| C | tree-sitter-c | ✓ Stable |
| Ruby | tree-sitter-ruby | ✓ Stable |
| PHP | tree-sitter-php | ✓ Stable |
| Kotlin | tree-sitter-kotlin | ✓ Stable |
| Swift | tree-sitter-swift | ✓ Stable |
| CSS | tree-sitter-css | ✓ Stable |
| HTML | tree-sitter-html | ✓ Stable |
| Markdown | tree-sitter-md | ✓ Stable |

## What gets indexed

For each language, OmniContext extracts:

- Function definitions and calls
- Class definitions and inheritance
- Module imports and exports
- Type definitions
- Variable declarations
- Comments and documentation

## Performance

Indexing speed: **> 500 files/sec** on reference hardware.

## Adding new languages

OmniContext uses tree-sitter for parsing. To add a new language, implement the `LanguageParser` trait in `crates/omni-core/src/parser/languages/`.
