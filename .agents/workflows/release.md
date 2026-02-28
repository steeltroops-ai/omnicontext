---
description: How to prepare and publish a release of OmniContext
---

# Release Workflow

## Pre-Release Checklist

- [ ] All CI checks pass on `main`
- [ ] CHANGELOG.md updated with release notes
- [ ] Version bumped in all `Cargo.toml` files
- [ ] Benchmark suite shows no regressions
- [ ] Integration tests pass against reference repos (Python, TS, Rust, Go, Java)
- [ ] `cargo audit` clean
- [ ] README reflects current features

## Steps

### 1. Version Bump

Edit workspace `Cargo.toml` and all crate `Cargo.toml` files:

```bash
# Use cargo-release or manual bump
cargo set-version --workspace <new_version>
```

### 2. Update Changelog

Following Keep a Changelog format in `CHANGELOG.md`:

```markdown
## [<version>] - YYYY-MM-DD

### Added

### Changed

### Fixed

### Performance

### Breaking Changes
```

### 3. Final CI Run

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

### 4. Tag Release

```bash
git add -A
git commit -m "chore: release v<version>"
git tag -a v<version> -m "Release v<version>"
git push origin main --tags
```

### 5. Build Release Binaries

CI handles this via GitHub Actions on tag push:

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

### 6. Publish to crates.io

```bash
# Publish in dependency order
cargo publish -p omni-core
cargo publish -p omni-mcp
cargo publish -p omni-cli
```

### 7. Update Package Managers

- **Homebrew**: Update formula in `homebrew-tap` repo
- **Scoop**: Update manifest in `scoop-bucket` repo
- **AUR**: Update PKGBUILD

### 8. GitHub Release

Create GitHub Release from the tag:

- Attach compiled binaries
- Copy changelog entry as release notes
- Mark pre-release if version < 1.0

### 9. Notify

- Update documentation site
- Post announcement (if major release)
