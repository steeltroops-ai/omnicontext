---
description: How to debug search relevance issues in OmniContext
---

# Debug Search Relevance Workflow

When search results are poor or unexpected, follow this systematic approach.

## Symptoms

- Relevant code not appearing in top 10 results
- Irrelevant results ranked highly
- Empty results for queries that should match
- Duplicate/redundant results from same file

## Diagnostic Steps

### 1. Check Index Health

```bash
omnicontext status --repo /path/to/repo
```

Verify:

- Total files indexed matches expected count
- Total chunks count is reasonable (avg 5-15 chunks per file)
- Last indexed timestamp is recent
- No files listed under "failed to parse"

### 2. Verify Chunk Quality

```bash
omnicontext debug chunks --file <problematic_file>
```

Check:

- Is the relevant code actually in a chunk?
- Are chunk boundaries correct (not splitting mid-function)?
- Is the symbol_path correct?
- Is the chunk kind correct (function vs impl vs struct)?
- Is the weight appropriate?

### 3. Test Individual Retrieval Signals

```bash
# Keyword search only
omnicontext debug search --query "<query>" --mode keyword

# Semantic search only
omnicontext debug search --query "<query>" --mode semantic

# Symbol lookup
omnicontext debug search --query "<query>" --mode symbol
```

Compare results. Common issues:

- **Keyword finds it, semantic doesn't**: Embedding model issue (check model loaded, check embedding dimensions)
- **Semantic finds it, keyword doesn't**: FTS5 tokenizer issue (check tokenize configuration)
- **Neither finds it**: Chunk doesn't exist or indexing failed

### 4. Check Scoring Breakdown

```bash
omnicontext debug explain --query "<query>" --chunk-id <id>
```

This should show:

- RRF score components (semantic_rank, keyword_rank)
- Structural weight applied
- Dependency proximity boost
- Recency boost
- Final combined score

### 5. Verify Embedding Quality

```bash
omnicontext debug similarity --chunk-a <id1> --chunk-b <id2>
```

Check cosine similarity between chunks that should be related. Expected:

- Same concept, different files: > 0.7
- Related concepts: > 0.5
- Unrelated code: < 0.3

### 6. Check Dependency Graph

```bash
omnicontext debug graph --symbol <name> --depth 2
```

Verify:

- Dependencies are correctly resolved
- No orphaned nodes
- Import edges exist

## Common Fixes

| Issue                   | Fix                                                   |
| ----------------------- | ----------------------------------------------------- |
| File not indexed        | Check `.omnicontext/config.toml` exclude patterns     |
| Wrong chunk boundaries  | Check language analyzer for the file's language       |
| Low semantic similarity | Consider switching to a code-specific embedding model |
| Missing FTS results     | Check if content is too short for FTS5 tokenizer      |
| No dependency boost     | Check import resolver for the file's language         |
| Stale results           | Run `omnicontext reindex --file <path>`               |

## Nuclear Option

If all else fails:

```bash
omnicontext reindex --repo /path/to/repo --force
```

This rebuilds the entire index from scratch.
