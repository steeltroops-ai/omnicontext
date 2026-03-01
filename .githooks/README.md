# Git Hooks

This directory contains git hooks to enforce code quality before commits and pushes.

## Installation

To enable these hooks, run:

```bash
git config core.hooksPath .githooks
```

On Windows, you may need to make the scripts executable:

```bash
chmod +x .githooks/pre-commit
chmod +x .githooks/pre-push
```

## Hooks

- `pre-commit`: Runs `cargo fmt --check` and `cargo clippy` on main crates
- `pre-push`: Runs `cargo test --workspace` before pushing

## Bypassing Hooks

If you need to bypass hooks (not recommended), use:

```bash
git commit --no-verify
git push --no-verify
```

## CI Enforcement

Even if you bypass local hooks, the CI pipeline will enforce the same checks on GitHub.
