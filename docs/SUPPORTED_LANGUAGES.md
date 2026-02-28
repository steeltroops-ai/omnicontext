# OmniContext Supported Languages

## Current Status

| Language   | Grammar                             | Structural Extractor | Import Resolver | Test Fixture | Status  |
| ---------- | ----------------------------------- | -------------------- | --------------- | ------------ | ------- |
| Python     | `tree-sitter-python`                | Planned              | Planned         | Planned      | Phase 1 |
| TypeScript | `tree-sitter-typescript`            | Planned              | Planned         | Planned      | Phase 1 |
| JavaScript | `tree-sitter-javascript`            | Planned              | Planned         | Planned      | Phase 1 |
| Rust       | `tree-sitter-rust`                  | Planned              | Planned         | Planned      | Phase 1 |
| Go         | `tree-sitter-go`                    | Planned              | Planned         | Planned      | Phase 1 |
| Java       | `tree-sitter-java`                  | Planned              | Planned         | Planned      | Phase 2 |
| C/C++      | `tree-sitter-c` / `tree-sitter-cpp` | --                   | --              | --           | Phase 2 |
| C#         | `tree-sitter-c-sharp`               | --                   | --              | --           | Phase 2 |
| Ruby       | `tree-sitter-ruby`                  | --                   | --              | --           | Phase 3 |
| PHP        | `tree-sitter-php`                   | --                   | --              | --           | Phase 3 |
| Swift      | `tree-sitter-swift`                 | --                   | --              | --           | Phase 3 |
| Kotlin     | `tree-sitter-kotlin`                | --                   | --              | --           | Phase 3 |

## Language Analyzer Components

Each supported language requires:

### 1. Structural Extractor

Maps tree-sitter AST nodes to OmniContext chunk kinds:

- `function` -- function/method definitions
- `class` -- class/struct/enum definitions
- `trait` -- trait/interface/protocol definitions
- `impl` -- implementation blocks
- `const` -- constants and static values
- `type` -- type aliases and definitions
- `module` -- module/namespace declarations
- `test` -- test functions

### 2. Import Resolver

Language-specific dependency resolution:

- **Python**: `import foo`, `from foo.bar import baz`, `importlib.import_module()`
- **TypeScript/JS**: `import { x } from './y'`, `require()`, barrel re-exports
- **Rust**: `use crate::foo::bar`, `mod baz`, `pub use`
- **Go**: `import "pkg/path"`
- **Java**: `import com.foo.Bar`

### 3. Visibility Detector

- **Python**: Convention-based (`_private`, `__mangled`, no underscore = public)
- **TypeScript**: `export`, `export default`
- **Rust**: `pub`, `pub(crate)`, `pub(super)`, private (default)
- **Go**: Capitalized = public, lowercase = private
- **Java**: `public`, `protected`, `private`, package-private

### 4. Doc Comment Extractor

- **Python**: Docstrings ("""...""")
- **TypeScript/JS**: JSDoc (`/** ... */`)
- **Rust**: Doc comments (`///`, `//!`, `/** */`)
- **Go**: Comment blocks preceding declarations
- **Java**: Javadoc (`/** ... */`)

## Adding a New Language

See workflow: [/add-language](../.agents/workflows/add-language.md)

Estimated effort per language: **2-3 days** of engineering.
