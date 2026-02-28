---
description: How to scaffold a new crate in the OmniContext workspace
---

# Scaffold New Crate Workflow

## Steps

### 1. Create the Crate Directory

```bash
mkdir -p crates/<crate-name>/src
```

### 2. Create Cargo.toml

```toml
[package]
name = "<crate-name>"
version = "0.1.0"
edition = "2021"
description = "<description>"
license = "Apache-2.0"

[dependencies]
# Add dependencies here

[dev-dependencies]
# Add test dependencies here
```

### 3. Create lib.rs or main.rs

For library crate (`lib.rs`):

```rust
//! <crate-name> -- <one-line description>
//!
//! <longer description of what this crate does and how it fits
//!  into the OmniContext architecture>

#![warn(missing_docs)]
#![warn(clippy::all)]
#![deny(clippy::unwrap_used)]

pub mod error;
// ... other modules
```

For binary crate (`main.rs`):

```rust
//! <crate-name> -- <one-line description>

use anyhow::Result;

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // ...
    Ok(())
}
```

### 4. Create Error Module

`src/error.rs`:

```rust
//! Error types for <crate-name>.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum <CrateName>Error {
    // Add variants
}
```

### 5. Add to Workspace

Edit root `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/<crate-name>",
    # ... existing crates
]
```

### 6. Verify

```bash
cargo build -p <crate-name>
cargo test -p <crate-name>
cargo clippy -p <crate-name> -- -D warnings
```
