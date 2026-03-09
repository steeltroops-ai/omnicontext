---
title: Get Started
description: Install and configure OmniContext in minutes
category: Guides
order: 1
---

# Get Started

Install OmniContext and connect it to your AI coding assistant.

## Install OmniContext

Choose your platform:

```bash
# macOS
brew install omnicontext

# Windows
scoop install omnicontext

# Linux
curl -fsSL https://omnicontext.dev/install.sh | bash
```

## Index your codebase

```bash
cd your-project
omnicontext index .
```

## Connect to your AI assistant

Add OmniContext to your AI assistant's MCP configuration:

```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "omnicontext-mcp"
    }
  }
}
```

Restart your AI assistant. You're ready to go.
