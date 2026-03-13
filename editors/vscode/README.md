# OmniContext — VS Code Extension

> Intelligent pre-fetch caching and automatic semantic context injection for Visual Studio Code, bridging the OmniContext daemon and your AI assistant.

---

[![Version](https://img.shields.io/badge/version-v1.2.0-blue)](https://github.com/steeltroops-ai/omnicontext/releases/tag/v1.2.0)
[![VS Code Marketplace](https://img.shields.io/badge/VS%20Code-Marketplace-blue?logo=visual-studio-code)](https://marketplace.visualstudio.com/items?itemName=steeltroops.omnicontext&ssr=false#overview)
[![Open VSX](https://img.shields.io/badge/Open%20VSX-Registry-purple)](https://open-vsx.org/extension/steeltroops/omnicontext)

---

## Overview

The OmniContext VS Code extension connects your editor to the OmniContext daemon over a local IPC socket, enabling:

- **Predictive context pre-fetching** — debounced on every cursor move and document change, so relevant context is ready before you ask
- **Automatic semantic injection** — feeds the right code snippets and symbol definitions directly into compatible AI assistants
- **Sidebar workspace panel** — browse index status, cache statistics, and active workspace at a glance
- **14 commands** for manual control over indexing, cache, model, and server lifecycle

---

## Installation

### From the Marketplace (recommended)

Search for **OmniContext** in the VS Code Extensions panel (`Ctrl+Shift+X` / `Cmd+Shift+X`) and click **Install**.

Or install in one command:

```bash
code --install-extension steeltroops.omnicontext
```

### From a VSIX file

```bash
code --install-extension omnicontext-1.2.0.vsix
```

### Build from source

```bash
cd editors/vscode
bun install
bun run compile
bun run package
code --install-extension omnicontext-1.2.0.vsix
```

---

## Supported AI Clients

OmniContext v1.2.0 works alongside **16 AI clients**:

| Client | Type |
|--------|------|
| Claude Desktop | Desktop app |
| Claude Code | CLI agent |
| Cursor | IDE |
| Windsurf | IDE |
| VS Code (GitHub Copilot, etc.) | Editor |
| VS Code Insiders | Editor |
| Cline | VS Code extension |
| RooCode | VS Code extension |
| Continue.dev | VS Code / JetBrains extension |
| Zed | Editor |
| Kiro | IDE |
| PearAI | IDE |
| Trae | IDE |
| Antigravity | IDE *(new in v1.2.0)* |
| Gemini CLI | CLI agent |
| Amazon Q CLI | CLI agent |
| Augment Code | IDE extension |

---

## Commands

All commands are accessible from the Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`) under the `OmniContext:` prefix.

| Command | Description |
|---------|-------------|
| `OmniContext: Index Workspace` | Trigger a full index of the current workspace |
| `OmniContext: Re-index Workspace` | Force a complete re-index, discarding the existing index |
| `OmniContext: Show Index Status` | Display index health, coverage %, and staleness |
| `OmniContext: Search Code` | Open the semantic search input box |
| `OmniContext: Search by Symbol` | Jump to a symbol by name across the codebase |
| `OmniContext: Get File Context` | Show context panel for the active file |
| `OmniContext: Download Model` | Run the model downloader (default model) |
| `OmniContext: Download Small Model` | Download `jina-embeddings-v2-small-en` for lower memory usage |
| `OmniContext: Select Model` | Pick from installed models |
| `OmniContext: Start MCP Server` | Start the `omnicontext-mcp` process |
| `OmniContext: Stop MCP Server` | Gracefully stop the MCP server |
| `OmniContext: Clear Cache` | Evict all cached search results |
| `OmniContext: Show Cache Stats` | Display hit rate, entry count, and memory usage |
| `OmniContext: Open Output Log` | Focus the OmniContext output channel |

---

## Configuration

All settings are under the `omnicontext` namespace. Configure them in **Settings** (`Ctrl+,`) or in `settings.json`.

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `omnicontext.binaryPath` | `string` | `"omnicontext"` | Absolute path to the `omnicontext` binary. Set this if the binary is not on `PATH`. |
| `omnicontext.mcpBinaryPath` | `string` | `"omnicontext-mcp"` | Absolute path to the `omnicontext-mcp` binary. |
| `omnicontext.model` | `string` | `"jina-embeddings-v2-base-code"` | Embedding model to use. Accepts any model name recognised by `omnicontext setup model-download --model`. |
| `omnicontext.autoStartMcp` | `boolean` | `true` | Automatically start the MCP server when the workspace opens. |
| `omnicontext.autoIndex` | `boolean` | `false` | Automatically index the workspace on first open. |
| `omnicontext.prefetch.enabled` | `boolean` | `true` | Master toggle for predictive context pre-fetching. |
| `omnicontext.prefetch.cacheSize` | `number` | `100` | Maximum number of entries in the LRU pre-fetch cache. |
| `omnicontext.prefetch.cacheTtlSeconds` | `number` | `300` | Time-to-live for pre-fetched cache entries (seconds). |
| `omnicontext.prefetch.debounceMs` | `number` | `150` | Debounce window (ms) applied to editor events before triggering a pre-fetch query. |
| `omnicontext.ipc.socketPath` | `string` | *(platform default)* | Override the IPC socket path. Defaults to `\\.\pipe\omnicontext-ipc` on Windows and `/tmp/omnicontext-ipc.sock` on Unix. |
| `omnicontext.log.level` | `string` | `"info"` | Log verbosity for the Output channel. One of `"error"`, `"warn"`, `"info"`, `"debug"`. |

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  VS Code Extension                  │
│                                                     │
│  ┌─────────────┐   ┌──────────────┐                │
│  │ EventTracker│──▶│SymbolExtractor│               │
│  └──────┬──────┘   └──────┬───────┘               │
│         │                 │                         │
│         ▼                 ▼                         │
│      ┌──────────────────────┐                      │
│      │      IPC Client      │                      │
│      │  (named pipe / sock) │                      │
│      └──────────┬───────────┘                      │
│                 │                                   │
│      ┌──────────▼───────────┐                      │
│      │  Bootstrap Service   │                      │
│      │  (binary lifecycle)  │                      │
│      └──────────────────────┘                      │
└─────────────────────────────────────────────────────┘
                     │ IPC
                     ▼
          ┌──────────────────┐
          │ omnicontext-mcp  │
          │  (Rust daemon)   │
          └──────────────────┘
```

### EventTracker

Subscribes to `vscode.workspace.onDidChangeTextDocument` and `vscode.window.onDidChangeTextEditorSelection`. Applies a configurable debounce (`omnicontext.prefetch.debounceMs`) to filter high-frequency keystrokes, emitting a stable event only when the cursor has been idle for the debounce window. This prevents IPC flooding while keeping pre-fetch latency low.

### SymbolExtractor

Calls `vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', uri)` to discover the symbol at the current cursor position. The resulting symbol path is appended to pre-fetch queries so the daemon returns context that is scoped to the active function or class, not just the file.

### IPC Client

Communicates with the OmniContext daemon over:
- **Windows**: `\\.\pipe\omnicontext-ipc` (named pipe)
- **Unix/macOS**: `/tmp/omnicontext-ipc.sock` (Unix domain socket)

Implements an `isDaemonConnected` circuit breaker: if the socket is unavailable, pre-fetch queries are dropped silently rather than queuing, preventing the extension host from blocking the editor UI thread.

### Bootstrap Service

Manages the lifecycle of the `omnicontext` and `omnicontext-mcp` child processes. On workspace open (when `omnicontext.autoStartMcp` is `true`), it spawns `omnicontext-mcp`, monitors its stdout/stderr, and forwards output to the OmniContext output channel. On VS Code shutdown, it sends a graceful `shutdown` signal and waits up to 5 seconds before force-killing.

---

## Model Selection

### Default model (higher accuracy)

Run from the Command Palette:

```
OmniContext: Download Model
```

Or via terminal:

```bash
omnicontext setup model-download
```

### Smaller model (lower memory, faster startup)

```
OmniContext: Download Small Model
```

Or via terminal:

```bash
omnicontext setup model-download --model jina-embeddings-v2-small-en
```

After downloading, set the active model in settings:

```json
{
  "omnicontext.model": "jina-embeddings-v2-small-en"
}
```

---

## Performance Targets

| Operation | Target |
|-----------|--------|
| Editor event processing (debounced) | < 5 ms |
| IPC round-trip to daemon | < 10 ms |
| Sidebar panel refresh | < 100 ms |
| Extension host memory footprint | < 50 MB |
| Pre-fetch cache hit rate (warm) | ≥ 80 % |

---

## Troubleshooting

### IPC not connected

**Symptom**: Status bar shows `OmniContext: Not Connected` and pre-fetch is silently disabled.

**Steps**:
1. Open the Output channel: `OmniContext: Open Output Log`
2. Check for `[Bootstrap]` errors — the daemon may have failed to start
3. Verify the binary path: `omnicontext --version` in a terminal
4. If `omnicontext.autoStartMcp` is `false`, run `OmniContext: Start MCP Server` manually
5. On Windows, confirm no other process holds the named pipe: `handle \\.\pipe\omnicontext-ipc`

### Model not found

**Symptom**: Indexing fails with `Model file not found` or embeddings are skipped.

**Steps**:
1. Run `OmniContext: Show Index Status` — look for `model: missing`
2. Run `OmniContext: Download Model` to fetch the default model
3. If you prefer the smaller model, run `OmniContext: Download Small Model` and set `omnicontext.model` to `jina-embeddings-v2-small-en`
4. Confirm the model directory is writable: `omnicontext preflight-check`

### Binary not found

**Symptom**: All commands fail with `spawn omnicontext ENOENT`.

**Steps**:
1. Confirm `omnicontext` is on your system `PATH`: `which omnicontext` (Unix) or `where omnicontext` (Windows)
2. If installed to a non-standard location, set `omnicontext.binaryPath` to the absolute path, e.g.:
   ```json
   {
     "omnicontext.binaryPath": "/usr/local/bin/omnicontext",
     "omnicontext.mcpBinaryPath": "/usr/local/bin/omnicontext-mcp"
   }
   ```
3. Reload the VS Code window (`Ctrl+Shift+P` → `Developer: Reload Window`) after updating settings

---

## Development

### Prerequisites

- Node.js 20+ and [Bun](https://bun.sh/)
- VS Code 1.85+

### Build & test

```bash
cd editors/vscode
bun install
bun run compile
bun run test
```

### Package a VSIX

```bash
bun run package
```

### Launch Extension Development Host

Press `F5` in VS Code with the `editors/vscode` folder open, or run:

```bash
code --extensionDevelopmentPath=./editors/vscode
```

---

## Contributing

1. Fork the repository and create a feature branch
2. Follow the existing TypeScript style — strict mode, no `any`, Bun for all tooling
3. Run `bun run compile && bun run test` before opening a PR
4. Include a short description of the change and, where relevant, a screenshot or GIF

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for the full contribution guide.

---

## License

MIT — see [LICENSE](../../LICENSE) for details.
