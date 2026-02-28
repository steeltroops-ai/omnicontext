# OmniContext Development Rules

## Mandatory Rules (Never Violate)

### R1: No Unsafe Without Justification

Every `unsafe` block must have a `// SAFETY:` comment explaining:

1. What invariant is being upheld
2. Why safe alternatives are insufficient
3. What happens if the invariant is violated

### R2: Error Handling

- Library code: Return `Result<T, OmniError>`
- Binary code: Use `anyhow::Result` at the top level only
- Never swallow errors silently -- at minimum `tracing::warn!`
- All external calls (file I/O, FFI, network) must have timeout guards
- Use the hierarchical error taxonomy: Recoverable -> Degraded -> Fatal

### R3: Testing Requirements

- Every public function needs at least one unit test
- Every MCP tool needs an integration test with a real fixture repo
- Performance-critical paths need criterion benchmarks
- Parser correctness: property-based tests with proptest
- Search relevance: NDCG benchmark suite against reference dataset

### R4: Dependency Policy

Before adding any new crate:

1. Check crates.io -- minimum 100k downloads or known-good ecosystem crate
2. Check `cargo audit` -- no known vulnerabilities
3. Check maintenance -- last commit within 6 months
4. Prefer crates with `#![forbid(unsafe_code)]` when possible
5. All dependencies go in `[workspace.dependencies]` first, then referenced
6. Verify actual published version before pinning (RC/pre-release awareness)

### R5: Performance Invariants

These must never regress (enforced by CI benchmarks):

- File indexing: > 500 files/sec on reference hardware
- Embedding: > 800 chunks/sec on CPU
- Search: < 50ms P99 for 100k chunk index
- Memory: < 2KB per indexed chunk (metadata only, vectors mmap'd)

### R6: Platform Compatibility

- Test on Linux (primary), macOS, Windows
- Use `std::path::PathBuf`, never string-based paths
- Use `dirs::data_dir()` for platform-appropriate storage
- File operations: always use `std::fs` with proper error handling
- Line endings: handle both `\n` and `\r\n`
- Path separators: never assume `/`, use `Path::join()`

### R7: Configuration Precedence

Configuration is resolved in this order (highest wins):

1. CLI flags
2. Environment variables (`OMNI_*`)
3. Project config (`.omnicontext/config.toml`)
4. User config (`~/.config/omnicontext/config.toml`)
5. Defaults (hardcoded in `Config::defaults()`)

### R8: Logging Standards

```rust
// Use tracing with structured fields
tracing::info!(file = %path, chunks = count, "indexed file");
tracing::warn!(error = %err, file = %path, "parse failed, skipping");
tracing::debug!(query = %q, results = count, latency_ms = elapsed, "search completed");
```

Log levels:

- `error`: Unrecoverable failures (index corruption, startup failure)
- `warn`: Recoverable failures (parse error on one file, embedding timeout)
- `info`: Major operations (indexing started/completed, config loaded)
- `debug`: Per-query/per-file details
- `trace`: Internal algorithm state (search scores, chunk boundaries)

### R9: MCP Protocol Compliance

- Strictly follow the MCP specification
- All tool responses must be valid JSON
- Error responses must use standard MCP error codes
- Tool descriptions must be accurate and helpful (agents rely on them)
- Never return more than the requested `limit` results

### R10: Git Hygiene

- Never commit to `main` directly -- always branch
- Branch naming: `<type>/<scope>-<description>` (e.g., `feat/parser-python-support`)
- Every PR must pass CI (build, test, clippy, fmt)
- Squash merge to keep history clean

### R11: Module Decoupling

- Every subsystem module (parser, chunker, embedder, index, vector, graph, search, watcher, pipeline) must be independently compilable and testable
- Cross-module communication happens ONLY through types defined in `omni_core::types`
- No module should import another module's internal types
- Tests for module X should not require module Y to be fully implemented (use stubs/mocks)

### R12: Borrow Checker Patterns

Learned from Phase 0 graph implementation:

- Never use `HashMap::entry().or_insert_with(|| ...)` when the closure needs mutable access to the same struct
- Split check-then-insert into `contains_key()` + `insert()` when the insertion value requires borrowing the parent struct
- Prefer `RwLock` over `Mutex` for read-heavy data structures (dependency graph, symbol table)
- Always handle lock poisoning with `.map_err()` -- never `.unwrap()` on locks

### R13: Workspace Dependency Management

- All crate versions are defined in the root `Cargo.toml` under `[workspace.dependencies]`
- Individual crates reference workspace dependencies with `{ workspace = true }`
- Pre-release crates (RC, alpha, beta) must use explicit version strings (e.g., `"2.0.0-rc.11"`)
- When a dependency is not ready for integration, comment it out with a TODO note explaining when to revisit

---

## When to Create New Rules

Create a new rule when:

1. A bug is caused by a pattern that could recur
2. A code review catches a systemic issue
3. A performance regression slips through
4. A new external system is integrated (add integration-specific rules)
5. Platform-specific behavior causes unexpected failures
6. A borrow checker or lifetime issue takes > 5 minutes to resolve (document the pattern)
7. A dependency version mismatch causes build failures (document the resolution)

Rules should be:

- Actionable (tells you exactly what to do)
- Enforceable (can be checked by CI or code review)
- Justified (explains WHY, not just WHAT)

## When to Create New Workflows

Create a new workflow when:

1. A multi-step process is repeated more than twice
2. A task requires coordination between multiple components
3. A deployment or release process is established
4. A debugging session reveals a systematic diagnostic approach
5. A new language support is added (template workflow)
6. A new subsystem is being implemented (create a subsystem-specific build/test workflow)

---

## Code Review Checklist

Before approving any change:

- [ ] `cargo check --workspace` passes (fast first gate)
- [ ] `cargo build --all-targets` passes
- [ ] `cargo test --workspace` passes (all crates)
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] No new `unwrap()` in library code
- [ ] New public APIs have doc comments
- [ ] Performance-critical changes have benchmarks
- [ ] Error paths are tested
- [ ] Platform-specific code has `#[cfg]` guards
- [ ] No unused imports or dead code warnings
- [ ] Module boundaries respected (no cross-module internal imports)
