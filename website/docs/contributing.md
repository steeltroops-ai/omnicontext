---
title: Contributing
description: Guidelines for contributing to OmniContext
category: Contributing
order: 40
---

# Contributing

OmniContext is open source under Apache 2.0 license. We welcome contributions from the community.

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

## Development Workflow

1. **Fork** the repository on GitHub
2. **Branch** from main with descriptive name
3. **Code** following project standards
4. **Test** thoroughly with unit and integration tests
5. **CI** must pass all checks
6. **PR** with clear description
7. **Review** address feedback promptly
8. **Merge** once approved

## Requirements

Before submitting a PR, ensure:

| Check | Command |
|-------|---------|
| Format | `cargo fmt` |
| Lint | `cargo clippy -- -D warnings` |
| Test | `cargo test --workspace` |
| Build | `cargo build --workspace --release` |
| Docs | Add doc comments for public APIs |

## Project Structure

```
omnicontext/
├── crates/
│   ├── omni-core/      # Core library
│   ├── omni-cli/       # CLI binary
│   ├── omni-daemon/    # Background daemon
│   └── omni-mcp/       # MCP server
├── docs/               # Documentation
├── editors/vscode/     # VS Code extension
└── distribution/       # Installation scripts
```

## Contribution Areas

### Bug Fixes

- Check [Issues](https://github.com/steeltroops-ai/omnicontext/issues)
- Look for `good first issue` label
- Write failing test → Fix → Verify

### Features

- Discuss in [Discussions](https://github.com/steeltroops-ai/omnicontext/discussions)
- Check roadmap for planned features
- Design proposal for large features

### Documentation

- Fix errors and typos
- Add examples and clarifications
- Use clear, concise language
- Include code examples where helpful

### Testing

- Coverage targets: 75-90%
- Add benchmarks for performance-critical paths
- Test error paths explicitly
- Use property-based tests for parsers

### Language Support

- See workflow in `.agents/workflows/add-language.md`
- Implement `LanguageParser` trait
- Add tree-sitter grammar dependency
- Create unit test fixtures
- Average timeline: 3 days

## PR Checklist

Before submitting:

- [ ] Builds without errors
- [ ] All tests pass
- [ ] Linters pass (fmt, clippy)
- [ ] Documentation updated
- [ ] Commits follow conventions
- [ ] Branch up-to-date with main

## PR Template

```markdown
## What
Brief description of changes

## Why
Reason for this change

## How
Implementation approach

## Testing
How you tested the changes

## Breaking Changes
Any breaking changes? If yes, describe migration path
```

## Commit Conventions

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```bash
feat(parser): add support for Ruby language
fix(embedder): handle model download failures gracefully
docs: update installation guide for Windows
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `perf`: Performance improvement
- `refactor`: Code restructure
- `test`: Add or fix tests

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on code, not person
- Welcome newcomers

## Getting Help

- **Questions**: [Discussions](https://github.com/steeltroops-ai/omnicontext/discussions)
- **Bugs**: [Issues](https://github.com/steeltroops-ai/omnicontext/issues)
- **Chat**: Join our community channels
