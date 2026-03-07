# GitHub Actions CI/CD Architecture

This document defines the operational pipeline and automation state machine for the OmniContext repository. All workflows strictly enforce production-grade continuous integration, deployment, security auditing, and performance tracking.

## Pipeline Topologies

### 1. Verification Engine (`ci.yml`)

**Purpose**: Pre-merge gatekeeper. Enforces logic correctness and style strictness.
**Hook**: `push` / `pull_request` (target: `main`)

**Execution Stages**:

1. **Formatting (`cargo fmt`)**: Restrictive code style validation. Non-compliant formatting blocks the pipeline immediately.
2. **Static Analysis (`cargo clippy`)**: Lints the core execution paths (`omni-mcp`, `omni-cli`, `omni-daemon`) enforcing zero warnings (`-D warnings`).
3. **Compilation Check (`cargo check`)**: Verifies build viability across the workspace.
4. **Test Matrix**: Full workspace unit tests across Windows, Ubuntu, and macOS targets.
5. **State Aggregation**: `CI Success` job aggregates matrix results to satisfy GitHub branch protection rules seamlessly.

### 2. Release & Distribution (`release.yml`)

**Purpose**: Deterministic compilation, packaging, and versioned distribution.
**Hook**: Tag push matching `v*` | Manual `workflow_dispatch`

**Execution Stages**:

1. **Source Resolution**: Extracts version data strictly from Cargo.toml or git tags.
2. **Cross-Platform Compilation Matrix**:
   - Compiles native binaries for `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-unknown-linux-gnu`.
   - Generates standardized archives (`.zip` for Windows, `.tar.gz` for UNIX).
3. **Cryptographic Integrity**: Computes strictly deterministic `sha256` checksums for all distribution binaries.
4. **Package Manager Registration**: Synchronizes remote Hombrew formula (`distribution/homebrew/omnicontext.rb`) and Scoop manifests (`distribution/scoop/omnicontext.json`) with the newly verified checksums.
5. **Release Artifact Generation**: Publishes immutable release state to GitHub Releases.

### 3. Security Hardening (`security.yml`)

**Purpose**: Proactive identification of supply chain and runtime vulnerabilities.
**Hook**: `push` / `pull_request` / `schedule` (00:00 UTC)

**Execution Stages**:

1. **Cargo Audit**: Checks `Cargo.lock` against the RustSec Advisory Database.
2. **Cargo Deny**: Enforces structural project rules (dependency licensing limits, bans on unmaintained dependencies, cyclic dependency avoidance).
3. **CodeQL SAST**: Performs deep static analysis on the C/C++/Rust logic trees to flag potential memory safety anomalies and vulnerabilities.
4. **Dependency Auditing**: Triggers automatic security review of net-new dependencies added in pull requests.

### 4. Performance Telemetry (`benchmark.yml`)

**Purpose**: Prevent algorithmic degradation and compute regressions.
**Hook**: `push` / `pull_request` / `workflow_dispatch`

**Execution Stages**:

1. **Benchmark Execution**: Triggers `cargo bench` natively.
2. **Regression Differential**: Evaluates the computational overhead of the PR diff relative to the `main` baseline. If a regression exceeds defined error margins, the step fails.

## Developer Execution Mandate

To bypass remote CI failures and maintain efficiency, run these checks precisely before submission:

```bash
cargo fmt --all
cargo clippy -p omni-mcp -p omni-daemon -p omni-cli --bins -- -D warnings
cargo test --workspace
```

## Maintenance Doctrine

- **Toolchain Anchoring**: The remote CI toolchain must mirror the deterministic local toolchain defined in `rust-toolchain.toml`.
- **Cache Optimization**: Build graphs and cargo caches (`Swatinem/rust-cache`) are to be invalidated accurately during crate lock changes to limit stale behavior.
- **Fail-Fast**: Ensure independent and failing tasks abort the workflow matrix immediately to curtail wasted compute time unless differential debugging is underway.
