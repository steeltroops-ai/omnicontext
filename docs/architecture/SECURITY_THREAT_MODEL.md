# OmniContext Security & Threat Model

## Threat Model (Extended)

### T1: Path Traversal via Symlinks

**Threat**: A repository contains symlinks that point outside the indexed directory (e.g., `../../.ssh/id_rsa`). The indexer follows the symlink and indexes sensitive files.

**Mitigation**:

- `fs::canonicalize` all paths before indexing
- Verify canonical path is within the configured repo root
- Default: don't follow symlinks (configurable)
- Log a warning when symlinks are encountered

```rust
fn is_safe_path(path: &Path, repo_root: &Path) -> bool {
    match path.canonicalize() {
        Ok(canonical) => canonical.starts_with(repo_root),
        Err(_) => false, // broken symlink, skip
    }
}
```

### T2: AST-Based Denial of Service

**Threat**: A file with deeply nested structures (e.g., 1000 levels of nested functions) causes tree-sitter to consume excessive memory or time.

**Mitigation**:

- Set max parse time: 10 seconds per file
- Set max file size: 5MB (configurable)
- Set max AST depth for traversal: 50 levels
- If limits exceeded: skip file, log warning

### T3: Index Metadata Leakage

**Threat**: The `~/.omnicontext/repos/<hash>/` directory contains structural information about the codebase (symbol names, file paths, dependencies) even though file contents are local.

**Mitigation**:

- Index directory permissions: `0700` (owner only)
- Optional: encrypt index at rest with user password
- When sharing index (enterprise): strip file paths, keep only relative paths
- `.omnicontext/` should be in `.gitignore`

### T4: MCP Protocol Injection

**Threat**: A malicious MCP client sends crafted queries that exploit SQLite injection or cause resource exhaustion.

**Mitigation**:

- All SQL queries use parameterized statements (rusqlite enforces this)
- Rate limiting: max 100 queries/second per client
- Query timeout: 5 seconds max
- Input validation: max query length 10,000 characters
- Reject queries with SQL-like syntax in keyword search

### T5: Supply Chain Attack via tree-sitter Grammars

**Threat**: A community-contributed tree-sitter grammar contains malicious native code that executes during parsing.

**Mitigation**:

- Vendor all tree-sitter grammars (don't fetch at runtime)
- Pin grammar versions in Cargo.toml
- Only use grammars from official tree-sitter organization or well-audited sources
- Future: run grammars in WASM sandbox

### T6: ONNX Model Tampering

**Threat**: A modified ONNX model could produce embeddings that leak information or execute arbitrary code.

**Mitigation**:

- Verify model SHA256 hash on load
- Ship known-good hashes in binary
- Download models from verified sources only
- ONNX Runtime has sandboxing for graph execution

### T7: Multi-Tenant Data Isolation (Enterprise)

**Threat**: In hosted enterprise deployment, one tenant's data leaks to another.

**Mitigation**:

- Each tenant gets isolated SQLite database and vector index
- No shared state between tenant processes
- Container-level isolation (separate process per tenant)
- Customer-managed encryption keys (CMEK)

## Security Practices

### Dependency Audit

```bash
# Run before every release
cargo audit

# Run on every PR (CI)
cargo deny check advisories
cargo deny check licenses
```

### Filesystem Security

| Platform    | Index Directory               | Permissions         |
| ----------- | ----------------------------- | ------------------- |
| Linux/macOS | `~/.local/share/omnicontext/` | `drwx------` (0700) |
| Windows     | `%LOCALAPPDATA%\omnicontext\` | User-only ACL       |

### Network Security (Enterprise Tier)

- TLS 1.3 only (no TLS 1.2 fallback)
- API key rotation every 90 days
- JWT with `RS256` signing
- CORS: allowlist only
- Rate limiting: token bucket per API key

### Content Security

- Never log file contents at `info` level or above
- Structured logs must not contain source code
- Error messages must not reveal file system paths in enterprise mode
