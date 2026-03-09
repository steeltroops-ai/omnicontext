---
title: MCP Tools
description: Available tools for AI agents
category: API
order: 10
---

# MCP Tools

OmniContext exposes 6 tools through the Model Context Protocol.

## search_codebase

Search your codebase semantically.

```typescript
{
  query: string;
  limit?: number;
}
```

Returns ranked code chunks with relevance scores.

## get_architectural_context

Get dependency relationships for a file.

```typescript
{
  file_path: string;
  max_hops?: number;
}
```

Returns files connected through imports, inheritance, and function calls.

## get_dependencies

Get dependencies for a specific symbol.

```typescript
{
  symbol_path: string;
  depth?: number;
}
```

Returns upstream and downstream dependencies.

## get_commit_context

Get relevant commits for understanding code evolution.

```typescript
{
  query?: string;
  file_paths?: string[];
  limit?: number;
}
```

Returns commit history with generated summaries.

## get_workspace_stats

Get repository statistics.

```typescript
{}
```

Returns files indexed, chunks, vectors, and health metrics.

## context_window

Assemble token-optimized context for LLMs.

```typescript
{
  query: string;
  token_budget?: number;
  priority_files?: string[];
}
```

Returns prioritized code chunks within token budget.
