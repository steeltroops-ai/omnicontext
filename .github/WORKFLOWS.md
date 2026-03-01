# GitHub Actions Workflows

This document describes all GitHub Actions workflows in the OmniContext project.

## Workflows Overview

### 1. CI (Continuous Integration)

**File**: `.github/workflows/ci.yml`

**Triggers**:
- Push to `main` branch
- Pull requests to `main` branch

**Jobs**:
- **Check**: Runs `cargo check` on all targets
- **Test**: Runs test suite on Ubuntu, Windows, and macOS
- **Format**: Checks code formatting with `cargo fmt`
- **Clippy**: Lints code with `cargo clippy` (main crates only)
- **CI Success**: Gates all checks - required for merge

**Purpose**: Ensures all code meets quality standards before merging.

**Branch Protection**: Enable "Require status checks to pass before merging" and select "CI Success" as required check.

---

### 2. Release

**File**: `.github/workflows/release.yml`

**Triggers**:
- Push tags matching `v*` (e.g., `v0.1.0`)
- Manual workflow dispatch with version input

**Jobs**:
- **Create Release**: Creates GitHub release with notes
- **Build**: Builds binaries for all platforms (Windows, Linux, macOS x64/ARM64)
- **Update Package Manifests**: Updates Homebrew and Scoop manifests with SHA256 hashes

**Artifacts**:
- Platform-specific archives (`.zip` for Windows, `.tar.gz` for Unix)
- SHA256 checksums for verification
- Updated package manager manifests

**Usage**:
```bash
# Bump version and create tag
./scripts/bump-version.sh patch  # or minor, major, or custom version
git push origin main
git push origin v0.1.0

# Or trigger manually from GitHub Actions UI
```

---

### 3. Security Audit

**File**: `.github/workflows/security.yml`

**Triggers**:
- Push to `main` branch
- Pull requests to `main` branch
- Daily schedule (00:00 UTC)
- Manual workflow dispatch

**Jobs**:
- **Audit**: Runs `cargo audit` to check for known vulnerabilities
- **Dependency Review**: Reviews dependency changes in PRs
- **Supply Chain**: Checks licenses and advisories with `cargo deny`
- **CodeQL**: Static analysis for security issues

**Purpose**: Proactive security monitoring and vulnerability detection.

---

### 4. Benchmark

**File**: `.github/workflows/benchmark.yml`

**Triggers**:
- Push to `main` branch
- Pull requests to `main` branch
- Manual workflow dispatch

**Jobs**:
- **Benchmark**: Runs all benchmarks and uploads results
- **Performance Regression**: Compares PR performance against main branch

**Artifacts**:
- Criterion benchmark results (retained for 30 days)

**Purpose**: Track performance metrics and detect regressions.

---

## Workflow Best Practices

### For Contributors

1. **Before Committing**: Run local checks
   ```bash
   cargo fmt --all
   cargo clippy -p omni-mcp -p omni-daemon -p omni-cli --bins -- -D warnings
   cargo test --workspace
   ```

2. **Install Git Hooks**: Automate local checks
   ```bash
   ./scripts/setup-dev.sh  # or setup-dev.ps1 on Windows
   ```

3. **Check CI Status**: Ensure all checks pass before requesting review

### For Maintainers

1. **Enable Branch Protection**:
   - Go to Settings → Branches → Add rule for `main`
   - Enable "Require status checks to pass before merging"
   - Select "CI Success" as required check
   - Enable "Require branches to be up to date before merging"

2. **Release Process**:
   ```bash
   # 1. Bump version
   ./scripts/bump-version.sh minor
   
   # 2. Review changes
   git show
   
   # 3. Push commit and tag
   git push origin main
   git push origin v0.2.0
   
   # 4. GitHub Actions automatically creates release
   ```

3. **Security Monitoring**:
   - Review daily security audit results
   - Address vulnerabilities promptly
   - Update dependencies regularly

4. **Performance Tracking**:
   - Review benchmark results on main branch
   - Investigate performance regressions in PRs
   - Set performance budgets for critical paths

---

## Workflow Secrets

No secrets are required for public repositories. For private repositories or additional features:

- `GITHUB_TOKEN`: Automatically provided by GitHub Actions
- `CARGO_REGISTRY_TOKEN`: (Optional) For publishing to crates.io

---

## Troubleshooting

### CI Failures

**Format Check Failed**:
```bash
cargo fmt --all
git add .
git commit --amend --no-edit
git push --force-with-lease
```

**Clippy Warnings**:
```bash
cargo clippy -p omni-mcp -p omni-daemon -p omni-cli --bins -- -D warnings
# Fix warnings, then commit
```

**Test Failures**:
```bash
cargo test --workspace -- --nocapture
# Debug and fix failing tests
```

### Release Failures

**Build Failed on Platform**:
- Check platform-specific dependencies
- Verify cross-compilation setup
- Test locally with `cargo build --release --target <target>`

**Package Manifest Update Failed**:
- Verify SHA256 files were generated
- Check sed commands in workflow
- Manually update manifests if needed

### Security Audit Failures

**Known Vulnerability**:
```bash
cargo audit
# Review advisory details
cargo update <crate>  # Update vulnerable dependency
```

**License Issue**:
```bash
cargo deny check licenses
# Review license compatibility
# Update or replace incompatible dependency
```

---

## Adding New Workflows

When adding new workflows:

1. Create workflow file in `.github/workflows/`
2. Use descriptive name (e.g., `deploy.yml`, `docs.yml`)
3. Add concurrency control to prevent duplicate runs
4. Document in this file
5. Test with workflow dispatch before enabling automatic triggers

Example template:
```yaml
name: New Workflow

on:
  push:
    branches: [main]
  workflow_dispatch:

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always

jobs:
  job-name:
    name: Job Description
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build
```

---

## Workflow Maintenance

### Regular Updates

- Update action versions quarterly
- Review and update Rust toolchain version
- Monitor GitHub Actions changelog for breaking changes
- Test workflows after major updates

### Performance Optimization

- Use caching for cargo registry and build artifacts
- Enable `fail-fast: false` for matrix builds when appropriate
- Use `concurrency` to cancel outdated runs
- Minimize workflow run time to reduce costs

---

## Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust GitHub Actions](https://github.com/actions-rs)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)
- [OmniContext Contributing Guide](../CONTRIBUTING.md)
