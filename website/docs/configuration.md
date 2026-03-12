---
title: Configuration
description: Configure OmniContext indexing, search, and embedding behavior
category: Guides
order: 30
---

# Configuration

OmniContext works with zero configuration out of the box — it auto-downloads models, auto-detects languages, and applies sensible defaults. For advanced use cases, you can customize behavior through a project-level configuration file, a user-level configuration file, and environment variables.

---

## Configuration File

The project configuration file lives at `.omnicontext/config.toml` in your repository root. Create it with:

```bash
omnicontext config --init
```

Or create it manually. The full set of available options is shown below:

```toml
[indexing]
# Glob patterns for paths to exclude from indexing
exclude_patterns = [
    "node_modules",
    "dist",
    "build",
    "target",
    "__pycache__",
    "*.test.ts",
    "*.spec.js"
]

# Maximum file size to index, in bytes (default: 1 MB)
max_file_size = 1048576

# Maximum tokens per chunk (default: 512)
max_chunk_tokens = 512

[embedding]
# Dimensions of the embedding model (default: 768 for jina-v2-base-code)
dimensions = 768

[search]
# Default number of results to return
default_limit = 10

# RRF constant for reciprocal rank fusion (higher = less aggressive merging)
rrf_k = 60

# Default token budget for context_window tool
token_budget = 8192

[watcher]
# Debounce delay in milliseconds before re-indexing changed files
debounce_ms = 100

# Polling interval in seconds for periodic full-index refresh
poll_interval_secs = 300
```

---

## Viewing the Effective Configuration

To see the configuration currently in effect (merged from all sources):

```bash
omnicontext config --show
```

---

## Environment Variables

Override any configuration value at runtime using environment variables:

```bash
# Model configuration
export OMNI_MODEL_PATH=/custom/path/to/model

# Index location (defaults to .omnicontext/ in the repo root)
export OMNI_INDEX_PATH=/custom/index/location

# Logging
export OMNI_LOG_LEVEL=debug        # trace | debug | info | warn | error
export RUST_LOG=omni_core=debug

# Skip embedding model download (starts in keyword-only mode)
export OMNI_SKIP_MODEL_DOWNLOAD=1

# Repository path for the MCP server (used by IDE launchers)
export OMNICONTEXT_REPO=/path/to/project
```

---

## Configuration Precedence

Settings are resolved in this order (highest priority first):

1. CLI flags (e.g., `--repo`, `--log-level`)
2. Environment variables (`OMNI_*` prefix)
3. Project config (`.omnicontext/config.toml` in repo root)
4. User config (`~/.config/omnicontext/config.toml`)
5. Built-in defaults

---

## User-Level Configuration

Create `~/.config/omnicontext/config.toml` to apply global defaults across all repositories:

```toml
[search]
default_limit = 20

[watcher]
debounce_ms = 200
```

---

## Common Configuration Recipes

### Large Codebase (>100 K files)

Reduce memory pressure and speed up indexing:

```toml
[indexing]
max_file_size = 524288   # 512 KB
exclude_patterns = ["node_modules", "vendor", "*.min.js", "*.map"]
max_chunk_tokens = 256

[search]
default_limit = 10
```

### Monorepo

Limit graph traversal to avoid cross-package noise:

```toml
[indexing]
exclude_patterns = [
    "*/node_modules",
    "*/dist",
    "*/build",
    "*/.next"
]

[search]
rrf_k = 80   # Slightly more conservative fusion
```

### Active Development (Frequent Changes)

Faster incremental updates with file watching:

```toml
[watcher]
debounce_ms = 100    # Respond quickly to saves
poll_interval_secs = 60
```

### CI / CD (No File Watching)

Disable the file watcher to avoid hanging processes in CI:

```toml
[watcher]
debounce_ms = 0
poll_interval_secs = 0
```

You can also set `OMNI_SKIP_MODEL_DOWNLOAD=1` to skip the embedding model download and run keyword-only search in CI:

```bash
OMNI_SKIP_MODEL_DOWNLOAD=1 omnicontext index .
```

### Accuracy Optimization

Maximize search quality at the expense of latency:

```toml
[search]
default_limit = 20
token_budget = 16384
rrf_k = 60
```

---

## Performance Tuning

### Memory Optimization

- Reduce `max_file_size` to skip very large generated files.
- Reduce `max_chunk_tokens` to produce more, smaller chunks.
- Lower `default_limit` to return fewer results per query.

### Speed Optimization

- Increase `max_chunk_tokens` to produce fewer, larger chunks (faster indexing).
- Use `OMNI_SKIP_MODEL_DOWNLOAD=1` for keyword-only mode when embeddings are not needed.
- Increase `debounce_ms` to reduce re-index frequency during heavy editing.

### Search Quality Optimization

- Increase `token_budget` in `[search]` to pack more context for the LLM.
- Increase `default_limit` to retrieve more candidates before reranking.
- Lower `rrf_k` (e.g., 30) for more aggressive score fusion.

---

## Troubleshooting

### Indexing Is Too Slow

- Increase `max_chunk_tokens` to reduce the total number of chunks generated.
- Add more `exclude_patterns` for generated or vendored directories.
- Reduce `max_file_size` to skip large auto-generated files.

### High Memory Usage

- Reduce `max_file_size`.
- Reduce `max_chunk_tokens`.
- Exclude large asset directories (`*.png`, `*.svg`, etc.).

### Poor Search Results

- Increase `token_budget` so the `context_window` tool packs more context.
- Increase `default_limit` to retrieve more candidates.
- Lower `rrf_k` for more aggressive fusion of keyword and semantic results.
- Check embedding coverage with `omnicontext status`; if coverage is low, run:

```bash
omnicontext setup model-download
omnicontext index . --force
```
