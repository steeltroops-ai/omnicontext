---
title: Configuration
description: Configure OmniContext indexing, search, and embedding behavior
category: Guides
order: 30
---

# Configuration

OmniContext works with zero configuration, but you can customize behavior through configuration files and environment variables.

## Configuration File

Create `.omnicontext/config.toml` in your project root:

```toml
[index]
# Exclude patterns (glob syntax)
exclude = [
    "node_modules/**",
    "dist/**",
    "build/**",
    "*.test.ts",
    "*.spec.js"
]

# Maximum file size to index (bytes)
max_file_size = 1048576  # 1MB

# File extensions to index
include_extensions = [
    "rs", "py", "ts", "js", "go", "java",
    "c", "cpp", "cs", "rb", "php", "swift", "kt"
]

[embedder]
# Model path (auto-downloads if not present)
model_path = "~/.omnicontext/models/jina-embeddings-v2-base-code"

# Batch size for embedding generation
batch_size = 32

# Use quantization (INT8) for memory efficiency
quantize = true

[search]
# Number of results to return
limit = 20

# Minimum similarity score (0.0 - 1.0)
min_score = 0.3

# Enable graph boosting
graph_boost = true

# Enable cross-encoder reranking
rerank = true

[graph]
# Maximum hops for dependency traversal
max_hops = 3

# Edge types to include
edge_types = ["IMPORTS", "INHERITS", "CALLS", "INSTANTIATES", "HISTORICAL_CO_CHANGE"]

[watcher]
# Enable file watching for incremental updates
enabled = true

# Debounce delay (milliseconds)
debounce_ms = 500
```

## Environment Variables

Override configuration with environment variables:

```bash
# Model configuration
export OMNI_MODEL_PATH=/custom/path/to/model
export OMNI_BATCH_SIZE=64

# Index configuration
export OMNI_INDEX_PATH=/custom/index/location
export OMNI_MAX_FILE_SIZE=2097152  # 2MB

# Search configuration
export OMNI_SEARCH_LIMIT=50
export OMNI_MIN_SCORE=0.5

# Logging
export OMNI_LOG_LEVEL=debug  # trace, debug, info, warn, error
export RUST_LOG=omni_core=debug

# Performance
export OMNI_THREAD_POOL_SIZE=8
```

## Configuration Precedence

Configuration is resolved in this order (highest to lowest):

1. CLI flags (e.g., `--model-path`)
2. Environment variables (`OMNI_*` prefix)
3. Project config (`.omnicontext/config.toml`)
4. User config (`~/.config/omnicontext/config.toml`)
5. Hardcoded defaults

## Common Configurations

### Large Codebase

For repositories with >100K files:

```toml
[index]
max_file_size = 524288  # 512KB
exclude = ["node_modules/**", "vendor/**", "*.min.js"]

[embedder]
batch_size = 64
quantize = true

[search]
limit = 10
graph_boost = true
```

### Monorepo

For monorepos with multiple projects:

```toml
[index]
exclude = [
    "*/node_modules/**",
    "*/dist/**",
    "*/build/**",
    "*/.next/**"
]

[graph]
max_hops = 2  # Limit cross-project traversal
```

### Development

For active development with frequent changes:

```toml
[watcher]
enabled = true
debounce_ms = 200  # Faster updates

[index]
max_file_size = 2097152  # 2MB for larger files

[search]
rerank = true  # Better accuracy
```

### CI/CD

For continuous integration:

```toml
[watcher]
enabled = false  # No file watching in CI

[embedder]
batch_size = 128  # Faster indexing

[search]
rerank = false  # Faster queries
```

## User-Level Configuration

Create `~/.config/omnicontext/config.toml` for global settings:

```toml
[embedder]
model_path = "~/.omnicontext/models/jina-embeddings-v2-base-code"

[logging]
level = "info"
format = "json"
```

## Validation

Validate your configuration:

```bash
omni config validate
```

View effective configuration:

```bash
omni config show
```

## Performance Tuning

### Memory Optimization

```toml
[embedder]
quantize = true  # 4x memory reduction
batch_size = 16  # Lower memory usage

[search]
limit = 10  # Fewer results
```

### Speed Optimization

```toml
[embedder]
batch_size = 128  # Faster indexing

[search]
rerank = false  # Skip reranking
graph_boost = false  # Skip graph boost
```

### Accuracy Optimization

```toml
[search]
rerank = true  # Enable reranking
graph_boost = true  # Enable graph boost
min_score = 0.5  # Higher threshold

[graph]
max_hops = 3  # Deeper traversal
```

## Troubleshooting

### Indexing Too Slow

- Increase `batch_size`
- Reduce `max_file_size`
- Add more exclusions

### High Memory Usage

- Enable `quantize = true`
- Reduce `batch_size`
- Lower `max_file_size`

### Poor Search Results

- Enable `rerank = true`
- Enable `graph_boost = true`
- Lower `min_score`
- Increase `limit`
