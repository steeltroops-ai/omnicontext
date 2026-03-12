---
title: Supported Languages
description: Complete language support matrix with AST parsing capabilities
category: API Reference
order: 11
---

# Supported Languages

OmniContext provides full AST parsing support for 13+ programming languages using [tree-sitter](https://tree-sitter.github.io/tree-sitter/) grammars. Each language includes symbol extraction, import resolution for dependency graphs, visibility inference, and documentation extraction.

---

## Language Support Matrix

| Language | tree-sitter Grammar | AST Extractor | Graph Resolver | Tests | Status |
|----------|-------------------|:---:|:---:|:---:|--------|
| **Python** | `tree-sitter-python` | ✓ | ✓ | ✓ | Core Baseline |
| **TypeScript** | `tree-sitter-typescript` | ✓ | ✓ | ✓ | Core Baseline |
| **JavaScript** | `tree-sitter-javascript` | ✓ | ✓ | ✓ | Core Baseline |
| **Rust** | `tree-sitter-rust` | ✓ | ✓ | ✓ | Core Baseline |
| **Go** | `tree-sitter-go` | ✓ | ✓ | ✓ | Core Baseline |
| **Java** | `tree-sitter-java` | ✓ | ✓ | ✓ | Extended |
| **C** | `tree-sitter-c` | ✓ | ✓ | ✓ | Extended |
| **C++** | `tree-sitter-cpp` | ✓ | ✓ | ✓ | Extended |
| **C#** | `tree-sitter-c-sharp` | ✓ | ✓ | ✓ | Extended |
| **CSS** | `tree-sitter-css` | ✓ | — | ✓ | Extended |
| **Ruby** | `tree-sitter-ruby` | ✓ | ✓ | ✓ | Pending Baseline |
| **PHP** | `tree-sitter-php` | ✓ | ✓ | ✓ | Pending Baseline |
| **Swift** | `tree-sitter-swift` | ✓ | ✓ | ✓ | Pending Baseline |
| **Kotlin** | `tree-sitter-kotlin-ng` | ✓ | ✓ | ✓ | Pending Baseline |

> **Status meanings**:
> - **Core Baseline** — full support, thoroughly tested, production-ready.
> - **Extended** — full support, tested, actively maintained.
> - **Pending Baseline** — functional, test suite being expanded.

---

## Language Capabilities

### 1. AST Structural Mapping

Each language parser maps tree-sitter nodes to OmniContext's internal chunk representation. The following symbol kinds are extracted:

| Kind | Description |
|------|-------------|
| `function` | Function declarations and definitions |
| `class` | Class declarations |
| `trait` | Trait and interface definitions |
| `impl` | Implementation blocks (Rust) |
| `const` | Constants and static values |
| `type` | Type aliases and definitions |
| `module` | Module and namespace declarations |
| `test` | Test functions and test blocks |

---

### 2. Graph Import Resolution

OmniContext builds a dependency graph by resolving language-specific import statements at index time:

| Language | Resolved Constructs |
|----------|-------------------|
| **Python** | `import`, `from ... import`, `importlib` |
| **TypeScript / JavaScript** | `import`, `require()`, barrel re-exports (`export * from`) |
| **Rust** | `use`, `mod`, `pub use` |
| **Go** | `import` |
| **Java** | `import`, `package` |
| **C / C++** | `#include` |
| **C#** | `using` |

---

### 3. Visibility Inference

Search results and dependency graphs respect language-specific visibility rules:

| Language | Visibility Mechanism |
|----------|---------------------|
| **Python** | Heuristic: names without a leading `_` are treated as public |
| **TypeScript / JavaScript** | Static: presence of `export` token |
| **Rust** | Explicit: `pub`, `pub(crate)`, `pub(super)` |
| **Go** | Syntactic: capitalized identifiers are public |
| **Java** | Keywords: `public`, `private`, `protected`, package-private |
| **C#** | Keywords: `public`, `private`, `internal`, `protected` |

---

### 4. Documentation Extraction

OmniContext extracts documentation comments and makes them searchable alongside symbol names:

| Language | Comment Format |
|----------|---------------|
| **Python** | `"""docstring"""` |
| **TypeScript / JavaScript** | `/** JSDoc */` |
| **Rust** | `/// doc comment` and `//! inner doc comment` |
| **Go** | `// Package / function comment` (preceding the declaration) |
| **Java** | `/** Javadoc */` |
| **C#** | `/// <summary>XML doc comment</summary>` |

---

## Adding New Languages

To add support for a new language, follow the workflow defined in `.agents/workflows/add-language.md`. The average integration time is approximately 3 engineering days.

**Required steps**:

1. Add the tree-sitter grammar crate to `Cargo.toml` in `crates/omni-core`.
2. Implement the `LanguageParser` trait in `crates/omni-core/src/parser/languages/`.
3. Add graph import resolution logic.
4. Register the language in `crates/omni-core/src/parser/registry.rs`.
5. Create unit test fixtures in `crates/omni-core/tests/`.
6. Submit a pull request with the new language and test fixtures.

---

## Performance

All language parsers are designed to maintain consistent throughput:

| Metric | Target |
|--------|--------|
| File parsing | > 500 files / second |
| AST extraction | < 5 ms per file |
| Graph resolution | < 10 ms per file |
| Chunk memory overhead | < 2 KB per indexed chunk |

These figures are measured on modern commodity hardware (8-core CPU, NVMe storage) using the core baseline languages.
