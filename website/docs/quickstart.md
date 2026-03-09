---
title: Quickstart
description: Get started in 5 minutes
category: Getting Started
order: 2
---

# Quickstart

Index your codebase and connect to AI agents in 5 minutes.

## 1. Install

```bash
# macOS/Linux
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash

# Windows
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex
```

## 2. Index

```bash
cd /path/to/your/project
omnicontext index .
```

## 3. Configure MCP

Edit your AI client config:

**Claude Desktop**:
```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "omnicontext-mcp",
      "args": []
    }
  }
}
```

## 4. Test

Ask your AI agent:

> "Search for authentication logic in my codebase"

Done. Your agent now has semantic code search.
