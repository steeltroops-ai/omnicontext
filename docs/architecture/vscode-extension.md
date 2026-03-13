# VS Code Extension Architecture

**Version**: 1.2.1
**Last Updated**: 2026-03-13
**Status**: Production Ready

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture Diagram](#architecture-diagram)
3. [Core Components](#core-components)
4. [File Structure](#file-structure)
5. [Communication Flow](#communication-flow)
6. [Key Features](#key-features)
7. [Configuration](#configuration)
8. [Development](#development)

---

## Overview

The OmniContext VS Code extension provides seamless integration between VS Code and the OmniContext semantic code search engine. It enables AI agents (via MCP) and developers to access intelligent code context through a zero-configuration, auto-bootstrapping architecture.

### Key Characteristics

- **Zero Configuration**: Auto-downloads binaries, auto-detects repositories, auto-starts daemon
- **Cross-Platform**: Windows, macOS, Linux with platform-specific optimizations
- **IPC Communication**: Named pipes (Windows) / Unix sockets (Linux/macOS) for low-latency daemon communication
- **LSP Integration**: Leverages VS Code's Language Server Protocol for precise symbol extraction
- **MCP Ready**: Automatically configures MCP server for AI agent integration
- **Resilience**: Circuit breakers, health monitoring, automatic recovery

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        VS Code Extension                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  Extension   │  │   Sidebar    │  │    Event     │          │
│  │   (Main)     │  │   Provider   │  │   Tracker    │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                  │                  │                   │
│         └──────────────────┴──────────────────┘                   │
│                            │                                      │
│                    ┌───────▼────────┐                            │
│                    │  IPC Client    │                            │
│                    │ (Named Pipe /  │                            │
│                    │  Unix Socket)  │                            │
│                    └───────┬────────┘                            │
└────────────────────────────┼─────────────────────────────────────┘
                             │
                    ┌────────▼────────┐
                    │  omni-daemon    │
                    │  (Background    │
                    │   Process)      │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │   omni-core     │
                    │  (Rust Engine)  │
                    │                 │
                    │  • Parser       │
                    │  • Embedder     │
                    │  • Vector Index │
                    │  • Search       │
                    │  • Reranker     │
                    │  • Graph        │
                    └─────────────────┘
```

---


## Core Components

### 1. Extension Entry Point (`extension.ts`)

**Purpose**: Main extension activation and lifecycle management

**Responsibilities**:
- Extension activation/deactivation
- Binary bootstrapping and daemon lifecycle
- IPC connection management with automatic reconnection
- Command registration (search, index, sync MCP, etc.)
- Status bar management
- Repository detection and registration

**Key Functions**:
- `activate()`: Extension entry point, initializes all subsystems
- `deactivate()`: Cleanup on extension shutdown
- `ensureDaemonRunning()`: Starts daemon if not running
- `connectToIpc()`: Establishes IPC connection with retry logic
- `sendIpcRequest()`: Sends JSON-RPC requests to daemon
- `handleIpcMessage()`: Processes JSON-RPC responses

**State Management**:
- `daemonProcess`: Child process handle for daemon
- `ipcClient`: Socket connection to daemon
- `pendingRequests`: Map of in-flight IPC requests
- `reconnectAttempts`: Tracks reconnection attempts (max 10)

---

### 2. Bootstrap Service (`bootstrapService.ts`)

**Purpose**: Zero-friction binary resolution and auto-download

**Execution Order**:
1. Check `extensionPath/bin/<platform>` for bundled binary (fastest)
2. Check `~/.omnicontext/bin` (standalone installer)
3. Check `~/.cargo/bin` (developer path)
4. Check system PATH
5. If not found: Download latest release from GitHub

**Key Functions**:
- `bootstrap()`: Main entry point, orchestrates binary resolution
- `resolveBinaries()`: Searches all known locations
- `downloadAndExtractRelease()`: Downloads from GitHub releases
- `verifyBinary()`: Validates binary integrity and executability
- `getPlatformInfo()`: Returns platform-specific paths and names

**Download Strategy**:
- Fetches latest release from GitHub API
- Downloads platform-specific archive (`.tar.gz` or `.zip`)
- Extracts to `globalStoragePath/bin`
- Verifies SHA-256 checksums
- Sets executable permissions (Unix)

**Progress Reporting**:
```typescript
type BootstrapPhase = 
  | "checking"    // Searching for binaries
  | "downloading" // Downloading from GitHub
  | "extracting"  // Extracting archive
  | "verifying"   // Validating checksums
  | "ready"       // Binaries ready
  | "failed";     // Bootstrap failed
```

---

### 3. Sidebar Provider (`sidebarProvider.ts`)

**Purpose**: Webview-based sidebar UI for status, metrics, and controls

**UI Sections**:
1. **Connection Status**: Daemon connection state, uptime, version
2. **Repository Info**: Current repo, files/chunks indexed, embedding coverage
3. **Performance Metrics**: Search latency (P50/P95/P99), throughput
4. **Intelligence Layer**: Reranker metrics, graph metrics, intent classification
5. **Resilience Monitoring**: Circuit breakers, health status, deduplication, backpressure
6. **Settings**: Auto-index, auto-daemon, auto-sync toggles
7. **Integrations**: Quick search, MCP sync, update/repair
8. **Activity Log**: Recent operations and events

**Key Features**:
- Real-time metrics updates via IPC
- Interactive controls (index, search, sync)
- Color-coded status indicators
- Responsive design with VS Code theming
- Activity log with timestamps

**Communication**:
- Webview → Extension: `vscode.postMessage()`
- Extension → Webview: `webview.postMessage()`

---

### 4. Event Tracker (`eventTracker.ts`)

**Purpose**: IDE event tracking with debouncing for intelligent pre-fetch

**Tracked Events**:
- `file_opened`: User opens a file
- `cursor_moved`: Cursor position changes
- `text_edited`: File content modified

**Debouncing**:
- File open: Immediate (no debounce)
- Cursor move: 300ms debounce
- Text edit: 500ms debounce

**LSP Integration**:
- Extracts symbol at cursor via `SymbolExtractor`
- Includes fully qualified name, type signature, definition location
- Enables precise cross-file pre-fetch

**Queue Management**:
- Maintains event queue with configurable max size
- Sends events to daemon via `ide_event` IPC method
- Connection gate: Suppresses events when daemon disconnected

---

### 5. Symbol Extractor (`symbolExtractor.ts`)

**Purpose**: LSP-enhanced symbol extraction at cursor position

**Resolution Strategies** (in order of precision):
1. **DocumentSymbolProvider**: AST node + symbol kind
2. **HoverProvider**: Type signatures
3. **DefinitionProvider**: Definition location
4. **Word-at-cursor**: Fallback when LSP unavailable

**Extracted Information**:
```typescript
interface SymbolInfo {
  name: string;              // "validate_token"
  fqn?: string;              // "auth::middleware::validate_token"
  kind?: string;             // "Function", "Class", "Method"
  type_signature?: string;   // "fn validate_token(token: &str) -> Result<Claims>"
  definition_file?: string;  // "/path/to/auth.rs"
  definition_line?: number;  // 42
}
```

**Caching**:
- 500ms TTL cache to avoid redundant LSP queries
- Cache key: `${filePath}:${line}:${character}`

---

### 6. Repository Registry (`repoRegistry.ts`)

**Purpose**: Discovers and manages all indexed OmniContext repositories

**Registry Location**:
- Windows: `%LOCALAPPDATA%/omnicontext/registry.json`
- Linux/macOS: `~/.local/share/omnicontext/registry.json`

**Registry Schema**:
```typescript
interface IndexedRepo {
  repoPath: string;        // Absolute path
  name: string;            // Folder name
  hash: string;            // SHA-256 prefix (12 chars)
  filesIndexed: number;    // Last known count
  chunksIndexed: number;   // Last known count
  lastIndexedAt: number;   // Epoch milliseconds
  exists: boolean;         // Path still valid
}
```

**Key Functions**:
- `registerRepo()`: Adds repo to registry
- `listRepos()`: Returns all indexed repos
- `updateRepoStats()`: Updates file/chunk counts
- `removeRepo()`: Removes repo from registry
- `getActiveRepo()`: Returns currently active repo

---

### 7. Cache Stats Manager (`cacheStats.ts`)

**Purpose**: Retrieves and formats cache statistics from daemon

**Metrics**:
- `hits`: Number of cache hits
- `misses`: Number of cache misses
- `size`: Current cache size
- `capacity`: Maximum cache capacity
- `hit_rate`: Hit rate (0.0 to 1.0)

**Key Functions**:
- `getStats()`: Retrieves cache stats via IPC
- `clearCache()`: Clears pre-fetch cache
- `formatStats()`: Formats stats for display

---

### 8. Extension Utils (`extensionUtils.ts`)

**Purpose**: Pure-function utilities for testability

**Key Functions**:
- `derivePipeName()`: Computes IPC pipe name from repo path
- `assembleCliContext()`: Formats search results for CLI fallback
- `buildJsonRpcRequest()`: Constructs JSON-RPC 2.0 requests
- `parseJsonRpcResponse()`: Parses JSON-RPC 2.0 responses
- `calculateBackoffDelay()`: Exponential backoff for reconnection
- `deriveMcpBinaryPath()`: Resolves MCP binary path
- `buildMcpServerEntry()`: Constructs MCP server config
- `mergeMcpConfig()`: Merges MCP configurations

**Normalization**:
- Path normalization matches daemon's `default_pipe_name()`
- Strips `\\?\` prefix (Windows long paths)
- Converts backslashes to forward slashes
- Lowercases path
- Strips trailing separators

---


## File Structure

### Source Files (`src/`)

```
src/
├── extension.ts           # Main entry point, lifecycle management
├── bootstrapService.ts    # Binary resolution and auto-download
├── sidebarProvider.ts     # Webview sidebar UI
├── eventTracker.ts        # IDE event tracking with debouncing
├── symbolExtractor.ts     # LSP-enhanced symbol extraction
├── repoRegistry.ts        # Repository discovery and management
├── cacheStats.ts          # Cache statistics management
├── extensionUtils.ts      # Pure-function utilities
├── types.ts               # TypeScript type definitions
└── test/                  # Test suite
    ├── suite/
    │   ├── extension.test.ts
    │   ├── bootstrapService.test.ts
    │   └── extensionUtils.test.ts
    └── runTest.ts
```

### Configuration Files

```
editors/vscode/
├── package.json           # Extension manifest, commands, config
├── tsconfig.json          # TypeScript compiler configuration
├── .vscodeignore          # Files excluded from VSIX package
├── README.md              # User-facing documentation
├── CHANGELOG.md           # Version history
└── LICENSE                # MIT License
```

### Build Output (`out/`)

```
out/
├── extension.js           # Compiled main entry point
├── bootstrapService.js    # Compiled bootstrap service
├── sidebarProvider.js     # Compiled sidebar provider
├── eventTracker.js        # Compiled event tracker
├── symbolExtractor.js     # Compiled symbol extractor
├── repoRegistry.js        # Compiled repo registry
├── cacheStats.js          # Compiled cache stats
├── extensionUtils.js      # Compiled utilities
├── types.js               # Compiled type definitions
└── *.js.map               # Source maps for debugging
```

### Resources (`resources/`)

```
resources/
├── icon_horizontal.png           # Extension icon (horizontal)
├── icon-logo.svg                 # SVG logo
├── architecture-diagram.png      # Architecture diagram (PNG)
├── architecture-diagram.svg      # Architecture diagram (SVG)
├── extension-architecture-diagram.png  # Extension-specific diagram
└── extension-architecture-diagram.svg  # Extension-specific diagram (SVG)
```

---

## Communication Flow

### 1. Extension Activation Flow

```
User Opens VS Code
       ↓
Extension Activates (extension.ts)
       ↓
Bootstrap Service Resolves Binaries
       ↓
Daemon Starts (omnicontext-daemon)
       ↓
IPC Connection Established
       ↓
Repository Detected & Registered
       ↓
Sidebar UI Initialized
       ↓
Event Tracking Enabled
       ↓
Extension Ready
```

### 2. IPC Request/Response Flow

```
Extension (TypeScript)
       ↓
buildJsonRpcRequest()
       ↓
JSON-RPC 2.0 Request
       ↓
Named Pipe / Unix Socket
       ↓
Daemon (Rust)
       ↓
Handle Request (ipc.rs)
       ↓
Engine Operation (omni-core)
       ↓
JSON-RPC 2.0 Response
       ↓
Named Pipe / Unix Socket
       ↓
parseJsonRpcResponse()
       ↓
Extension (TypeScript)
```

### 3. Event Tracking Flow

```
User Action (cursor move, file open, edit)
       ↓
Event Tracker Captures Event
       ↓
Debounce (300ms cursor, 500ms edit)
       ↓
Symbol Extractor (LSP)
       ↓
Extract Symbol Info
       ↓
Build IdeEvent
       ↓
Send to Daemon (ide_event IPC)
       ↓
Daemon Pre-fetches Context
       ↓
Cache Warmed for Future Queries
```

### 4. Search Flow

```
User Triggers Search Command
       ↓
Extension Prompts for Query
       ↓
Send IPC Request (search method)
       ↓
Daemon Executes Hybrid Search
       ↓
Results Returned (JSON)
       ↓
Extension Formats Results
       ↓
Display in Quick Pick / Webview
```

---

## Key Features

### 1. Zero-Configuration Bootstrap

**Problem**: Users shouldn't need Rust, Cargo, or manual setup

**Solution**:
- Auto-detects binaries in multiple locations
- Downloads latest release from GitHub if not found
- Extracts and validates binaries automatically
- Works out-of-box for all users

**Fallback Chain**:
1. Bundled binary (extension package)
2. Standalone installer (`~/.omnicontext/bin`)
3. Developer path (`~/.cargo/bin`)
4. System PATH
5. Auto-download from GitHub

### 2. Automatic Daemon Management

**Problem**: Users shouldn't manually start/stop daemon

**Solution**:
- Extension starts daemon on activation
- Monitors daemon health via IPC heartbeat
- Automatic reconnection with exponential backoff
- Graceful shutdown on extension deactivation

**Reconnection Strategy**:
- Max 10 reconnection attempts
- Exponential backoff: 1s, 2s, 4s, 8s, 16s, 32s, 64s, 128s, 256s, 512s
- Resets counter on successful connection

### 3. Intelligent Pre-fetch

**Problem**: AI agents need context before user asks

**Solution**:
- Tracks file opens, cursor moves, text edits
- Extracts symbol info via LSP
- Pre-fetches related code into cache
- Reduces query latency from 50ms to <5ms

**Cache Strategy**:
- LRU cache with configurable capacity (default: 100)
- TTL-based expiration (default: 5 minutes)
- Hit rate tracking for optimization

### 4. MCP Auto-Configuration

**Problem**: AI agents need MCP server configured

**Solution**:
- Detects known MCP clients (Claude Desktop, Cline, Continue, etc.)
- Auto-generates MCP server entry
- Merges with existing config (preserves user settings)
- One-click sync via sidebar button

**Supported Clients**:
- Claude Desktop
- Claude Code
- Cline
- RooCode
- Continue.dev
- Windsurf
- Cursor
- Kiro
- PearAI
- Trae
- Antigravity
- Zed
- Gemini CLI
- Amazon Q CLI
- Augment Code
- VS Code
- VS Code Insiders

### 5. Real-Time Metrics Dashboard

**Problem**: Users need visibility into system performance

**Solution**:
- Sidebar displays real-time metrics
- Circuit breaker states
- Health monitoring
- Performance statistics
- Activity log

**Metrics Categories**:
- Connection: Status, uptime, version
- Repository: Files, chunks, coverage
- Performance: Latency (P50/P95/P99)
- Intelligence: Reranker, graph, intent
- Resilience: Circuit breakers, health

---

## Configuration

### Extension Settings (`package.json`)

```json
{
  "omnicontext.autoIndex": {
    "type": "boolean",
    "default": true,
    "description": "Automatically index repository on open"
  },
  "omnicontext.autoDaemon": {
    "type": "boolean",
    "default": true,
    "description": "Automatically start daemon on activation"
  },
  "omnicontext.prefetch.enabled": {
    "type": "boolean",
    "default": true,
    "description": "Enable intelligent pre-fetch"
  },
  "omnicontext.prefetch.cacheSize": {
    "type": "number",
    "default": 100,
    "description": "Pre-fetch cache capacity"
  },
  "omnicontext.prefetch.cacheTtlSeconds": {
    "type": "number",
    "default": 300,
    "description": "Cache TTL in seconds"
  },
  "omnicontext.search.limit": {
    "type": "number",
    "default": 10,
    "description": "Maximum search results"
  }
}
```

### Commands

```json
{
  "omnicontext.search": "Search codebase",
  "omnicontext.index": "Index repository",
  "omnicontext.reindex": "Re-index repository",
  "omnicontext.clearCache": "Clear pre-fetch cache",
  "omnicontext.syncMcp": "Sync MCP configuration",
  "omnicontext.showSidebar": "Show OmniContext sidebar",
  "omnicontext.restartDaemon": "Restart daemon",
  "omnicontext.showLogs": "Show extension logs"
}
```

---


## Development

### Prerequisites

- Node.js 18+ (for TypeScript compilation)
- VS Code 1.85+ (for extension development)
- Rust toolchain (for building binaries)

### Setup

```bash
# Navigate to extension directory
cd editors/vscode

# Install dependencies
bun install

# Compile TypeScript
bun run compile

# Watch mode (auto-recompile on changes)
bun run watch
```

### Testing

```bash
# Run all tests
bun test

# Run specific test suite
bun test --grep "Bootstrap"

# Run with coverage
bun run test:coverage
```

### Debugging

1. Open `editors/vscode` in VS Code
2. Press F5 to launch Extension Development Host
3. Set breakpoints in TypeScript source
4. Extension runs in debug mode with hot reload

**Launch Configuration** (`.vscode/launch.json`):
```json
{
  "name": "Run Extension",
  "type": "extensionHost",
  "request": "launch",
  "args": ["--extensionDevelopmentPath=${workspaceFolder}"],
  "outFiles": ["${workspaceFolder}/out/**/*.js"],
  "preLaunchTask": "npm: watch"
}
```

### Building VSIX Package

```bash
# Install vsce (VS Code Extension Manager)
bun add -g @vscode/vsce

# Package extension
vsce package

# Output: omnicontext-1.2.1.vsix
```

### Publishing

```bash
# Login to VS Code Marketplace
vsce login steeltroops-ai

# Publish extension
vsce publish

# Publish specific version
vsce publish 1.2.1
```

---

## IPC Protocol

### JSON-RPC 2.0 Format

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "search",
  "params": {
    "query": "authentication middleware",
    "limit": 10
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "results": [...],
    "elapsed_ms": 42
  }
}
```

**Error**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32603,
    "message": "Internal error"
  }
}
```

### Available Methods

#### Core Methods

- `status`: Get daemon status
- `search`: Hybrid semantic search
- `context_window`: Get context for query
- `index`: Index repository
- `clear_index`: Clear index

#### Intelligence Methods

- `reranker/get_metrics`: Get reranker metrics
- `graph/get_metrics`: Get graph metrics
- `search/get_intent`: Classify query intent

#### Resilience Methods

- `resilience/get_status`: Get circuit breaker and health status
- `resilience/reset_circuit_breaker`: Reset circuit breakers

#### Historical Methods

- `history/get_commit_context`: Get commit history for file
- `history/index_commits`: Index git commit history

#### Pre-fetch Methods

- `ide_event`: Send IDE event for pre-fetch
- `prefetch_stats`: Get cache statistics
- `clear_cache`: Clear pre-fetch cache
- `update_config`: Update cache configuration

---

## Error Handling

### Connection Errors

**Scenario**: Daemon not running or IPC connection lost

**Handling**:
1. Display error in status bar
2. Attempt automatic reconnection (max 10 attempts)
3. Exponential backoff between attempts
4. Fallback to CLI mode if reconnection fails
5. User notification with "Restart Daemon" action

### Bootstrap Errors

**Scenario**: Binary download or extraction fails

**Handling**:
1. Display error notification
2. Provide "Retry" and "Manual Setup" actions
3. Log detailed error to output channel
4. Fallback to system PATH binaries if available

### IPC Timeout

**Scenario**: Request takes too long (>30s)

**Handling**:
1. Cancel pending request
2. Log timeout warning
3. Return error to caller
4. Don't break IPC connection (allow recovery)

### LSP Unavailable

**Scenario**: Language server not running

**Handling**:
1. Fallback to word-at-cursor extraction
2. Continue event tracking with reduced precision
3. No error shown to user (graceful degradation)

---

## Performance Optimizations

### 1. Debouncing

**Problem**: Too many events overwhelm daemon

**Solution**:
- Cursor move: 300ms debounce
- Text edit: 500ms debounce
- File open: No debounce (immediate)

### 2. Caching

**Problem**: Redundant LSP queries

**Solution**:
- Symbol info cache (500ms TTL)
- Cache key: `${file}:${line}:${char}`
- Invalidate on file edit

### 3. Connection Pooling

**Problem**: Creating new IPC connections is expensive

**Solution**:
- Single persistent IPC connection
- Automatic reconnection on disconnect
- Multiplexed requests over single socket

### 4. Lazy Loading

**Problem**: Extension startup time

**Solution**:
- Defer sidebar initialization until first view
- Lazy-load event tracker until first event
- Async binary bootstrap (non-blocking)

---

## Security Considerations

### 1. Binary Verification

- SHA-256 checksum validation
- GitHub release signature verification
- Executable permission checks (Unix)

### 2. IPC Security

- Named pipes restricted to current user (Windows)
- Unix socket permissions: 0600 (owner only)
- No network exposure (local-only)

### 3. Path Sanitization

- All file paths normalized and validated
- No path traversal vulnerabilities
- Workspace-relative paths only

### 4. Configuration Validation

- All user inputs validated
- Numeric ranges enforced
- Boolean type checking
- No arbitrary code execution

---

## Troubleshooting

### Extension Not Activating

**Symptoms**: No status bar item, no sidebar

**Causes**:
- VS Code version too old (<1.85)
- Extension not installed correctly
- Activation event not triggered

**Solutions**:
1. Check VS Code version: `code --version`
2. Reinstall extension: `code --install-extension omnicontext-1.2.1.vsix`
3. Open a workspace (not single file)
4. Check extension logs: `Output > OmniContext`

### Daemon Not Starting

**Symptoms**: "Daemon not running" in status bar

**Causes**:
- Binary not found or not executable
- Port/pipe already in use
- Insufficient permissions

**Solutions**:
1. Check binary exists: `~/.omnicontext/bin/omnicontext-daemon`
2. Check permissions: `chmod +x ~/.omnicontext/bin/*` (Unix)
3. Kill existing daemon: `pkill omnicontext-daemon`
4. Check logs: `~/.omnicontext/logs/daemon.log`

### IPC Connection Failed

**Symptoms**: "Connection lost" notifications

**Causes**:
- Daemon crashed
- Network issues (rare, local-only)
- Pipe/socket file deleted

**Solutions**:
1. Restart daemon: Command Palette > "OmniContext: Restart Daemon"
2. Check daemon logs for crashes
3. Verify pipe/socket exists: `/tmp/omnicontext-*.sock` (Unix)
4. Restart VS Code

### Search Not Working

**Symptoms**: No results or errors

**Causes**:
- Repository not indexed
- Index corrupted
- Daemon not connected

**Solutions**:
1. Index repository: Command Palette > "OmniContext: Index Repository"
2. Check index status in sidebar
3. Re-index if corrupted: "OmniContext: Re-index Repository"
4. Verify daemon connection

---

## Future Enhancements

### Planned Features

1. **Graph Visualization**: Interactive dependency graph explorer
2. **Co-Change Analysis**: Files that frequently change together
3. **Bug-Prone File Detection**: Identify high-risk files
4. **Multi-Repository Support**: Search across multiple repos
5. **Custom Embeddings**: User-provided embedding models
6. **Incremental Indexing**: Real-time index updates on file save

### Performance Improvements

1. **Streaming Search**: Stream results as they arrive
2. **Parallel Indexing**: Multi-threaded file processing
3. **Delta Indexing**: Only re-index changed files
4. **Compression**: Compress IPC messages for large payloads

### UX Enhancements

1. **Inline Search**: Search results in editor gutter
2. **Hover Context**: Show related code on hover
3. **Code Lens**: Display usage counts and references
4. **Diff View**: Compare search results side-by-side

---

## References

- [VS Code Extension API](https://code.visualstudio.com/api)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
- [Model Context Protocol](https://modelcontextprotocol.io/)
- [OmniContext Core Documentation](../index.md)
- [Daemon IPC Protocol](./daemon-ipc.md)

---

**Last Updated**: 2026-03-13
**Maintainer**: SteelTroops AI
**License**: MIT
