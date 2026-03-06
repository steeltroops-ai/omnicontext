# OmniContext VS Code Extension

Provides intelligent pre-fetch caching and automatic context injection capabilities bridging the OmniContext daemon and AI assistants within Visual Studio Code.

## Architecture

The extension operates as a high-performance IPC client to the `omnicontext-daemon`:

1. **Event Interception**: Silently monitors IDE workspace events (file selection, text mutations, cursor displacement).
2. **Context Pre-fetching**: Leverages background debouncing to proactively query the code index and hydrate a fast-access memory cache.
3. **AI Injection**: Hooks into assistant request pipelines (e.g., GitHub Copilot Chat) to append context payloads automatically upon trigger.

## Development Constraints

The extension uses TypeScript and targets standard VS Code APIs.

### Setup

```bash
cd editors/vscode
npm install
npm run compile
```

### Execution

Launch the Extension Development Host (`F5` in VS Code) from the `editors/vscode` workspace context.

### Configuration Namespace

- `omnicontext.prefetch.enabled`: Master toggle for predictive caching mechanism.
- `omnicontext.prefetch.cacheSize`: LRU cache entry limit.
- `omnicontext.prefetch.cacheTtlSeconds`: Entry eviction TTL.
- `omnicontext.prefetch.debounceMs`: Inter-event debouncing filter (ms).

## Implementation Details

Core modules:

- `EventTracker`: Filters VS Code `TextDocumentChangeEvent` via high-frequency debounce loops.
- `SymbolExtractor`: Bridges local language server semantic context via `vscode.executeDocumentSymbolProvider`.
- `IPC Client`: Pipes raw queries over `\\.\pipe\omnicontext-ipc` (Windows) or `/tmp/omnicontext-ipc.sock` (Unix).

For end-user installation, please refer to the main [OmniContext Documentation](../../INSTALL.md).
