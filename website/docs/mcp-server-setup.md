---
title: MCP Server Setup
description: Configure OmniContext MCP server for AI clients
category: MCP Integration
order: 8
---

# MCP Server Setup

OmniContext exposes its functionality through the Model Context Protocol (MCP), allowing AI agents to search your codebase, navigate dependencies, and assemble context.

## What is MCP?

The Model Context Protocol is an open standard for connecting AI assistants to external data sources and tools. OmniContext implements MCP to provide semantic code search capabilities to any compatible AI client.

## Supported Clients

OmniContext works with all MCP-compatible clients:

| Client | Configuration File | Auto-Configured |
|--------|-------------------|-----------------|
| Claude Desktop | `claude_desktop_config.json` | ✅ Yes |
| Cursor | `.cursor/mcp/config.json` | ✅ Yes |
| Windsurf | `mcp_config.json` | ✅ Yes |
| Cline / RooCode | `mcp_settings.json` | ✅ Yes |
| Kiro | `~/.kiro/settings/mcp.json` | ✅ Yes |
| Continue.dev | `config.json` | ⚠️ Manual |
| Trae IDE | `.trae/mcp.json` | ⚠️ Manual |
| Custom | Your config | ⚠️ Manual |

## Transport Protocols

OmniContext supports two MCP transport protocols:

### stdio (Default)

Standard input/output communication. Recommended for local development.

```bash
omnicontext-mcp
```

Configuration:
```json
{
  "command": "omnicontext-mcp",
  "args": []
}
```

### SSE (Server-Sent Events)

HTTP-based communication. Required for remote/cloud deployments.

```bash
omnicontext-mcp --transport sse --port 3000
```

Configuration:
```json
{
  "url": "http://localhost:3000/sse",
  "transport": "sse"
}
```

## Configuration by Client

### Claude Desktop

**Location**:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`

**Configuration**:
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

**Restart**: Quit Claude Desktop completely and relaunch.

### Cursor

**Location**: `.cursor/mcp/config.json` in your home directory

**Configuration**:
```json
{
  "mcpServers": {
    "omnicontext": {
      "command": "omnicontext-mcp",
      "args": [],
      "env": {}
    }
  }
}
```

**Restart**: Reload Cursor window (Cmd/Ctrl + Shift + P → "Reload Window")

### Windsurf

**Location**: `mcp_config.json` in Windsurf config directory

**Configuration**:
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

**Restart**: Restart Windsurf application

### Kiro

**Location**: `~/.kiro/settings/mcp.json`

**Configuration**:
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

**Restart**: Reload Kiro or restart VS Code

### Continue.dev

**Location**: `~/.continue/config.json`

**Configuration**:
```json
{
  "experimental": {
    "modelContextProtocolServers": [
      {
        "transport": {
          "type": "stdio",
          "command": "omnicontext-mcp",
          "args": []
        }
      }
    ]
  }
}
```

**Restart**: Reload VS Code window

## Advanced Configuration

### Custom Workspace

Index a specific directory:

```json
{
  "command": "omnicontext-mcp",
  "args": ["--workspace", "/path/to/project"]
}
```

### Custom Model Path

Use a different embedding model:

```json
{
  "command": "omnicontext-mcp",
  "args": ["--model-path", "/path/to/model"]
}
```

### Debug Mode

Enable verbose logging:

```json
{
  "command": "omnicontext-mcp",
  "args": ["--log-level", "debug"],
  "env": {
    "RUST_LOG": "omni_mcp=debug"
  }
}
```

### Multiple Workspaces

Configure separate MCP servers for different projects:

```json
{
  "mcpServers": {
    "omnicontext-project-a": {
      "command": "omnicontext-mcp",
      "args": ["--workspace", "/path/to/project-a"]
    },
    "omnicontext-project-b": {
      "command": "omnicontext-mcp",
      "args": ["--workspace", "/path/to/project-b"]
    }
  }
}
```

## Verification

### Check Server Status

```bash
# Start server manually
omnicontext-mcp

# Should output:
# MCP Server listening on stdio...
# Ready. Connect any MCP-compatible agent.
```

### Test from AI Client

Ask your AI agent:

> "What MCP tools are available?"

You should see 6 OmniContext tools listed:
- search_codebase
- get_architectural_context
- get_dependencies
- get_commit_context
- get_workspace_stats
- context_window

### Check Logs

```bash
# macOS/Linux
tail -f ~/.omnicontext/logs/mcp.log

# Windows
Get-Content $env:LOCALAPPDATA\omnicontext\logs\mcp.log -Wait
```

## Troubleshooting

### Server not starting

**Check binary exists**:
```bash
which omnicontext-mcp  # macOS/Linux
where omnicontext-mcp  # Windows
```

**Check permissions**:
```bash
chmod +x $(which omnicontext-mcp)
```

### Client not connecting

**Verify configuration syntax**:
```bash
# Validate JSON
cat ~/.config/Claude/claude_desktop_config.json | jq .
```

**Check client logs**:
- Claude Desktop: Help → View Logs
- Cursor: Output panel → MCP
- VS Code: Output panel → Continue

### Tools not appearing

**Re-index workspace**:
```bash
cd /path/to/project
omnicontext index .
```

**Restart MCP server**:
```bash
# Kill existing process
pkill omnicontext-mcp

# Restart client to spawn new server
```

### Performance issues

**Increase timeout**:
```json
{
  "command": "omnicontext-mcp",
  "args": ["--timeout", "30"]
}
```

**Use daemon mode**:
```bash
# Start daemon
omnicontext-daemon

# Configure MCP to use daemon
{
  "command": "omnicontext-mcp",
  "args": ["--use-daemon"]
}
```

## Next Steps

- [Available Tools](/docs/available-tools) - Learn about all 6 MCP tools
- [Integration Guides](/docs/integration-guides) - Client-specific tutorials
- [Configuration](/docs/configuration) - Advanced MCP server options
