# OmniContext Native Language Support Matrix

This document outlines the strict Abstract Syntax Tree (AST) parsing boundaries currently instantiated in the OmniContext heuristic engine.

## Execution Capabilities

| Language Matrix | `tree-sitter` Grammar Integration | Ast Extractor | Graph Resolver | Unit Test Fixture | Engine Boundary  |
| :-------------- | :-------------------------------- | :------------ | :------------- | :---------------- | :--------------- |
| **Python**      | `tree-sitter-python`              | `[v]`         | `[v]`          | `[v]`             | Core Baseline    |
| **TypeScript**  | `tree-sitter-typescript`          | `[v]`         | `[v]`          | `[v]`             | Core Baseline    |
| **JavaScript**  | `tree-sitter-javascript`          | `[v]`         | `[v]`          | `[v]`             | Core Baseline    |
| **Rust**        | `tree-sitter-rust`                | `[v]`         | `[v]`          | `[v]`             | Core Baseline    |
| **Go**          | `tree-sitter-go`                  | `[v]`         | `[v]`          | `[v]`             | Core Baseline    |
| **Java**        | `tree-sitter-java`                | `[v]`         | `[v]`          | `[v]`             | Extended         |
| **C / C++**     | `tree-sitter-c` / `cpp`           | `[v]`         | `[v]`          | `[v]`             | Extended         |
| **C#**          | `tree-sitter-c-sharp`             | `[v]`         | `[v]`          | `[v]`             | Extended         |
| **CSS**         | `tree-sitter-css`                 | `[v]`         | `[-]`          | `[v]`             | Extended         |
| **Ruby**        | `tree-sitter-ruby`                | `[v]`         | `[v]`          | `[v]`             | Pending Baseline |
| **PHP**         | `tree-sitter-php`                 | `[v]`         | `[v]`          | `[v]`             | Pending Baseline |
| **Swift**       | `tree-sitter-swift`               | `[v]`         | `[v]`          | `[v]`             | Pending Baseline |
| **Kotlin**      | `tree-sitter-kotlin-ng`           | `[v]`         | `[v]`          | `[v]`             | Pending Baseline |

## Architectural Constraints Per Language

Integration of a semantic parser into the engine requires hardcoding boundaries against four distinct architectural capabilities.

### 1. AST Structural Mapping

Nodes emitted by `tree-sitter` must be definitively translated into one of the core internal representations inside the Rust backend:
`function`, `class`, `trait`, `impl`, `const`, `type`, `module`, or `test`.

### 2. Graph Importer Resolution

The dependency graph builder demands language-specific heuristics for cross-file graph edges:

- `Python`: `import | from | importlib`
- `TS/JS`: `import | require() | barrel bounds`
- `Rust`: `use | mod | pub use`
- `Go`: `import`

### 3. Visibility Boundaries

Search retrieval limits depend on encapsulation flags:

- `Python`: Heuristic (No `_` prefix = Public)
- `TS/JS`: Static (`export` tokens)
- `Rust`: Explicit (`pub / pub(crate)`)
- `Go`: Syntactic (Capitalized = Public)

### 4. Doc-String Extraction

The engine correlates dense semantic chunks against comment prefixes: `"""..."""` (Py), `/**` (TS/Java), `///` (Rust).

> **Integration Request Protocol**: To instantiate support for a new AST boundary, explicitly execute the deterministic workflow path specified in `/.agents/workflows/add-language.md`. Average integration timeline spans `3` continuous engineering days.
