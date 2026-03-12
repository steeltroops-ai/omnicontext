---
title: Contributing
description: Guidelines for contributing to OmniContext
category: Contributing
order: 40
---

# Contributing

OmniContext is open source under the [Apache 2.0 license](https://github.com/steeltroops-ai/omnicontext/blob/main/LICENSE). Contributions of all kinds are welcome — bug fixes, new features, documentation improvements, new language parsers, and test coverage.

---

## Quick Start

```bash
# Fork on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/omnicontext.git
cd omnicontext

# Create a feature branch
git checkout -b feat/your-feature

# Build and test
cargo build --workspace --release
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
```

> **Rust version requirement**: 1.80 or later.

---

## Development Workflow

1. **Fork** the repository at [github.com/steeltroops-ai/omnicontext](https://github.com/steeltroops-ai/omnicontext)
2. **Branch** from `main` with a descriptive name (e.g., `feat/add-ruby-parser`, `fix/embedder-timeout`)
3. **Code** following project standards (see below)
4. **Test** thoroughly with unit and integration tests
5. **CI** must pass all checks before merge is considered
6. **PR** with a clear description using the template below
7. **Review** address feedback promptly
8. **Merge** once approved by a maintainer

---

## Requirements

Every pull request must pass the following checks before review:

| Check | Command |
|-------|---------|
| Format | `cargo fmt --all` |
| Lint | `cargo clippy --workspace -- -D warnings` |
| Test | `cargo test --workspace` |
| Build | `cargo build --workspace --release` |
| Docs | Public API items must have doc comments |

Run all checks at once:

```bash
cargo fmt --all && \
cargo clippy --workspace -- -D warnings && \
cargo test --workspace && \
cargo build --workspace --release
```

---

## Project Structure

```
omnicontext/
├── crates/
│   ├── omni-core/       # Core library: parser, chunker, embedder, search, graph
│   │   ├── src/
│   │   │   ├── parser/languages/   # One file per language (python.rs, rust.rs, ...)
│   │   │   ├── embedder/           # ONNX model loading, batching, quantization
│   │   │   ├── search/             # Query engine, HyDE, synonyms, intent classifier
│   │   │   ├── graph/              # Dependency graph, community detection, PageRank
│   │   │   └── commits.rs          # Git history indexing and co-change analysis
│   ├── omni-cli/        # omnicontext CLI binary
│   │   └── src/
│   │       └── orchestrator.rs     # Universal IDE orchestrator (setup --all)
│   ├── omni-mcp/        # omnicontext-mcp binary (MCP server, 19 tools)
│   │   └── src/
│   │       └── tools.rs            # All 16 MCP tool implementations
│   ├── omni-daemon/     # omnicontext-daemon (background file watcher)
│   └── omni-ffi/        # C FFI bindings for external integrations
├── editors/
│   └── vscode/          # VS Code extension
├── website/
│   └── docs/            # This documentation
├── distribution/        # Install scripts (install.sh, install.ps1)
└── .agents/
    └── workflows/       # Agent-readable workflow documents
        └── add-language.md
```

---

## Contribution Areas

### Bug Fixes

- Browse [open issues](https://github.com/steeltroops-ai/omnicontext/issues) on GitHub.
- Look for the `good first issue` label for approachable entry points.
- Standard workflow: write a failing test → fix the bug → verify the test passes.

### New Features

- Open a [discussion](https://github.com/steeltroops-ai/omnicontext/discussions) before starting large features.
- Check the roadmap for planned work to avoid duplication.
- For architectural changes, write a short design proposal in the PR description.

### Documentation

- Fix factual errors, typos, and broken examples.
- Add clarifications and real-world examples.
- Keep language precise and concise.
- All code blocks must be accurate and runnable.

### Testing

- Coverage targets: 75–90% for core logic.
- Add benchmarks for performance-critical paths using `cargo bench`.
- Test error paths explicitly — not just the happy path.
- Use property-based testing (e.g., `proptest`) for parsers and chunkers where applicable.

### Language Support

Adding a new language parser is one of the most impactful contributions. Follow the workflow in `.agents/workflows/add-language.md`. The required steps are:

1. Add the tree-sitter grammar crate to `crates/omni-core/Cargo.toml`.
2. Implement the `LanguageParser` trait in `crates/omni-core/src/parser/languages/<lang>.rs`.
3. Add graph import resolution logic.
4. Register the language in `crates/omni-core/src/parser/registry.rs`.
5. Create unit test fixtures under `crates/omni-core/tests/`.
6. Update the [Supported Languages](/docs/supported-languages) documentation.

Average integration time: approximately 3 engineering days.

---

## PR Checklist

Before submitting your pull request:

- [ ] Builds without errors (`cargo build --workspace --release`)
- [ ] All tests pass (`cargo test --workspace`)
- [ ] Linters pass (`cargo fmt --all`, `cargo clippy -- -D warnings`)
- [ ] Documentation updated for any public API changes
- [ ] Commits follow Conventional Commits format
- [ ] Branch is up to date with `main`
- [ ] PR description explains what, why, and how

---

## PR Template

```markdown
## What
Brief description of the changes in this PR.

## Why
The reason for this change — what problem does it solve or what improvement does it make?

## How
A summary of the implementation approach and any non-obvious design decisions.

## Testing
How you tested the changes — unit tests, integration tests, manual verification steps.

## Breaking Changes
Does this change break any existing behavior or public API? If yes, describe the migration path.
```

---

## Commit Conventions

OmniContext follows [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short description>

[optional body]

[optional footer]
```

**Examples**:
```bash
feat(parser): add support for Ruby language
fix(embedder): handle model download failures gracefully
docs(installation): add cargo install instructions
perf(search): reduce RRF fusion latency by 30%
refactor(chunker): simplify context prefix generation
test(graph): add unit tests for blast radius with cycles
```

**Types**:
| Type | When to use |
|------|------------|
| `feat` | A new feature |
| `fix` | A bug fix |
| `docs` | Documentation-only changes |
| `perf` | A change that improves performance |
| `refactor` | Code restructuring with no behavior change |
| `test` | Adding or fixing tests |
| `chore` | Build process, dependency updates, tooling |

---

## Code Style

- All public items (`pub fn`, `pub struct`, `pub trait`) must have doc comments (`///`).
- Use `anyhow` for application-level errors, `thiserror` for library errors.
- Prefer explicit error handling over `.unwrap()` — the project denies `clippy::unwrap_used`.
- Keep functions focused; large functions should be decomposed.
- Write tests inline in the same file using `#[cfg(test)]` modules.

---

## Code of Conduct

- Be respectful and inclusive in all interactions.
- Provide constructive, specific feedback on code — not on people.
- Welcome newcomers; everyone starts somewhere.
- Focus reviews on correctness, clarity, and performance — in that order.

---

## Getting Help

| Need | Where to go |
|------|------------|
| Questions about the codebase | [GitHub Discussions](https://github.com/steeltroops-ai/omnicontext/discussions) |
| Bug reports | [GitHub Issues](https://github.com/steeltroops-ai/omnicontext/issues) |
| Feature ideas | [GitHub Discussions](https://github.com/steeltroops-ai/omnicontext/discussions) |
| Real-time chat | Community channels (see website) |
