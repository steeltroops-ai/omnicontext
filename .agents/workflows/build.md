---
description: Build, test, and verify the OmniContext project
---

# Build Workflow

## Prerequisites

- Rust stable toolchain (1.80+)
- ONNX Runtime (bundled via `ort` crate)
- SQLite3 development headers (usually bundled via `rusqlite` feature `bundled`)

## Steps

// turbo-all

1. Check toolchain

```bash
rustup show active-toolchain
```

2. Format check

```bash
cargo fmt --all --check
```

3. Lint

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

4. Build all crates

```bash
cargo build --workspace
```

5. Run unit tests

```bash
cargo test --workspace --lib
```

6. Run integration tests

```bash
cargo test --workspace --test '*'
```

7. Run doc tests

```bash
cargo test --workspace --doc
```

8. Check for security vulnerabilities

```bash
cargo audit
```

## Quick Build (development only)

```bash
cargo build --workspace 2>&1 | head -20
```

## Release Build

```bash
cargo build --workspace --release
```

## Cross-Platform Build

```bash
# Linux (from any host with cross installed)
cross build --target x86_64-unknown-linux-gnu --release

# macOS (from macOS host)
cargo build --target x86_64-apple-darwin --release
cargo build --target aarch64-apple-darwin --release

# Windows
cargo build --target x86_64-pc-windows-msvc --release
```
