---
title: Supported Languages
description: Complete language support matrix with AST parsing capabilities
category: API Reference
order: 11
---

# Supported Languages

OmniContext provides full AST parsing support for 16+ programming languages using tree-sitter grammars.

## Language Support Matrix

| Language | tree-sitter Grammar | AST Extractor | Graph Resolver | Unit Tests | Status |
|----------|-------------------|---------------|----------------|------------|--------|
| **Python** | `tree-sitter-python` | ✓ | ✓ | ✓ | Core Baseline |
| **TypeScript** | `tree-sitter-typescript` | ✓ | ✓ | ✓ | Core Baseline |
| **JavaScript** | `tree-sitter-javascript` | ✓ | ✓ | ✓ | Core Baseline |
| **Rust** | `tree-sitter-rust` | ✓ | ✓ | ✓ | Core Baseline |
| **Go** | `tree-sitter-go` | ✓ | ✓ | ✓ | Core Baseline |
| **Java** | `tree-sitter-java` | ✓ | ✓ | ✓ | Extended |
| **C / C++** | `tree-sitter-c` / `cpp` | ✓ | ✓ | ✓ | Extended |
| **C#** | `tree-sitter-c-sharp` | ✓ | ✓ | ✓ | Extended |
| **CSS** | `tree-sitter-css` | ✓ | - | ✓ | Extended |
| **Ruby** | `tree-sitter-ruby` | ✓ | ✓ | ✓ | Pending Baseline |
| **PHP** | `tree-sitter-php` | ✓ | ✓ | ✓ | Pending Baseline |
| **Swift** | `tree-sitter-swift` | ✓ | ✓ | ✓ | Pending Baseline |
| **Kotlin** | `tree-sitter-kotlin-ng` | ✓ | ✓ | ✓ | Pending Baseline |

## Language Capabilities

### 1. AST Structural Mapping

Each language parser maps tree-sitter nodes to internal representations:
- `function`: Function declarations and definitions
- `class`: Class declarations
- `trait`: Trait/interface definitions
- `impl`: Implementation blocks
- `const`: Constants and static values
- `type`: Type definitions
- `module`: Module/namespace declarations
- `test`: Test functions

### 2. Graph Import Resolution

Language-specific import resolution for dependency graphs:

- **Python**: `import`, `from`, `importlib`
- **TypeScript/JavaScript**: `import`, `require()`, barrel exports
- **Rust**: `use`, `mod`, `pub use`
- **Go**: `import`
- **Java**: `import`, `package`
- **C/C++**: `#include`
- **C#**: `using`

### 3. Visibility Boundaries

Search respects language-specific visibility rules:

- **Python**: Heuristic (no `_` prefix = public)
- **TypeScript/JavaScript**: Static (`export` tokens)
- **Rust**: Explicit (`pub`, `pub(crate)`)
- **Go**: Syntactic (capitalized = public)
- **Java**: Keywords (`public`, `private`, `protected`)
- **C#**: Keywords (`public`, `private`, `internal`)

### 4. Documentation Extraction

Extracts documentation from language-specific comment formats:

- **Python**: `"""docstring"""`
- **TypeScript/JavaScript**: `/** JSDoc */`
- **Rust**: `/// doc comments`
- **Go**: `// Package comments`
- **Java**: `/** Javadoc */`
- **C#**: `/// XML comments`

## Adding New Languages

To add support for a new language, follow the workflow in `.agents/workflows/add-language.md`. Average integration time: 3 engineering days.

Required steps:
1. Add tree-sitter grammar dependency
2. Implement `LanguageParser` trait
3. Add graph import resolution logic
4. Create unit test fixtures
5. Update language registry

## Performance

All language parsers maintain consistent performance:
- **Parsing**: > 500 files/sec
- **AST extraction**: < 5ms per file
- **Graph resolution**: < 10ms per file
- **Memory**: < 2KB per indexed chunk
