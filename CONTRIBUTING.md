# Contributing to OmniContext

Thank you for your interest in contributing to OmniContext! This document provides guidelines and instructions for contributors.

## Commit Message Format

OmniContext uses [Conventional Commits](https://www.conventionalcommits.org/) for automatic version bumping and changelog generation.

**Format**: `<type>[optional scope]: <description>`

**Types**:
- `feat:` - New feature (triggers minor version bump)
- `fix:` - Bug fix (triggers patch version bump)
- `feat!:` or `BREAKING CHANGE:` - Breaking change (triggers major version bump)
- `chore:`, `docs:`, `style:`, `refactor:`, `perf:`, `test:`, `ci:` - No version bump

**Examples**:
```bash
git commit -m "feat: add cross-encoder reranking"
git commit -m "fix(parser): handle empty files"
git commit -m "feat!: change MCP API (breaking)"
```

See [docs/CONVENTIONAL_COMMITS.md](docs/CONVENTIONAL_COMMITS.md) for detailed guide.

## Development Setup

After cloning the repository, run the setup script to install git hooks:

**Unix/Linux/macOS:**
```bash
./scripts/setup-dev.sh
```

**Windows:**
```powershell
.\scripts\setup-dev.ps1
```

This will configure git hooks that automatically check code quality before commits and pushes.

## Code Quality Standards

All code must pass the following checks before being merged:

1. **Formatting**: `cargo fmt --all -- --check`
2. **Linting**: `cargo clippy -- -D warnings` (on main crates)
3. **Tests**: `cargo test --workspace`

### Pre-commit Checks

The pre-commit hook automatically runs:
- Code formatting check
- Clippy linting on main crates (omni-mcp, omni-daemon, omni-cli, omni-core)

If any check fails, the commit will be blocked. Fix the issues before committing.

### Pre-push Checks

The pre-push hook automatically runs:
- Full test suite

If tests fail, the push will be blocked.

## Manual Checks

You can run these checks manually at any time:

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check

# Run clippy
cargo clippy -p omni-mcp -p omni-daemon -p omni-cli --bins -- -D warnings
cargo clippy -p omni-core --lib -- -D warnings

# Run tests
cargo test --workspace

# Run all checks
cargo fmt --all -- --check && \
cargo clippy -p omni-mcp -p omni-daemon -p omni-cli --bins -- -D warnings && \
cargo clippy -p omni-core --lib -- -D warnings && \
cargo test --workspace
```

## CI Pipeline

All pull requests must pass the CI pipeline, which enforces:

1. **Check**: `cargo check --workspace --all-targets`
2. **Test**: `cargo test --workspace` (on Ubuntu, Windows, macOS)
3. **Format**: `cargo fmt --all -- --check`
4. **Clippy**: Linting on main crates with `-D warnings`
5. **CI Success**: All above checks must pass

The CI pipeline cannot be bypassed. Even if you bypass local hooks, CI will catch issues.

## Bypassing Hooks (Not Recommended)

In rare cases where you need to bypass hooks:

```bash
git commit --no-verify
git push --no-verify
```

**Warning**: Bypassing hooks may result in CI failures. Only use this for work-in-progress commits.

## Branch Protection

The `main` branch is protected and requires:
- All CI checks to pass
- Code review approval
- Up-to-date branch before merging

## Project Structure

See `.kiro/steering/structure.md` for detailed project structure and module organization guidelines.

## Tech Stack

See `.kiro/steering/tech.md` for technology stack details and common commands.

## Questions?

If you have questions about contributing, please open an issue or discussion on GitHub.
