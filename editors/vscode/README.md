# OmniContext VS Code Extension

> Intelligent code context engine with pre-fetch caching and automatic context injection for AI coding assistants

## Features

### Pre-Fetch Context Caching

The extension intelligently tracks your IDE activity and pre-fetches relevant code context **before you even ask**. This dramatically reduces response time when working with AI assistants.

> **Screenshot Coming Soon**: Sidebar showing cache statistics with real-time hit rate, hits, misses, and cache size display

**How it works:**
- Monitors file opens, cursor movements, and text edits
- Extracts symbols at cursor position using VS Code's language features
- Pre-fetches relevant context from the OmniContext daemon
- Caches results with configurable TTL (time-to-live)
- Automatically injects cached context into AI chat requests

**Cache Indicators:**
- **[cached]** - Context retrieved from cache (instant response)
- **[fresh search]** - Context fetched from index (first time or cache expired)

> **Screenshots Coming Soon**: 
> - AI chat response showing cached indicator
> - AI chat response showing fresh search indicator

### Real-Time Cache Statistics

Monitor pre-fetch performance directly in the VS Code sidebar:
- **Hit Rate** - Percentage of requests served from cache
- **Cache Hits** - Number of successful cache retrievals
- **Cache Misses** - Number of fresh searches performed
- **Cache Size** - Current entries / Maximum capacity

> **Screenshot Coming Soon**: Sidebar cache statistics section showing active status with performance metrics

### Automatic Context Injection

When you interact with AI assistants (GitHub Copilot Chat, etc.), the extension automatically:
1. Detects your current file and cursor position
2. Checks the pre-fetch cache for relevant context
3. Injects context into the AI request (invisible to you)
4. Shows cache hit/miss indicator in the response

### Flexible Configuration

Fine-tune pre-fetch behavior to match your workflow:
- Enable/disable pre-fetch functionality
- Adjust cache size (10-1000 entries)
- Configure cache TTL (60-3600 seconds)
- Set debounce delay for IDE events (50-1000ms)

## Installation

1. Install the extension from VS Code Marketplace (coming soon) or build from source
2. Install OmniContext daemon: See [main installation guide](../../INSTALL.md)
3. Open a workspace - the extension will auto-start the daemon and begin indexing

## Configuration

### Pre-Fetch Settings

Access via VS Code Settings (`Ctrl+,` or `Cmd+,`) and search for "OmniContext"

> **Screenshot Coming Soon**: VS Code settings page showing all OmniContext pre-fetch configuration options

#### `omnicontext.prefetch.enabled`
- **Type**: Boolean
- **Default**: `true`
- **Description**: Enable pre-fetch functionality to cache context based on IDE events (file open, cursor movement, text edits)

#### `omnicontext.prefetch.cacheSize`
- **Type**: Number
- **Default**: `100`
- **Range**: 10-1000
- **Description**: Maximum number of entries in the pre-fetch cache. Higher values use more memory but improve hit rate.

**Recommendations:**
- Small projects (<1000 files): 50-100 entries
- Medium projects (1000-5000 files): 100-200 entries
- Large projects (>5000 files): 200-500 entries

#### `omnicontext.prefetch.cacheTtlSeconds`
- **Type**: Number
- **Default**: `300` (5 minutes)
- **Range**: 60-3600 seconds
- **Description**: Time-to-live for cached entries. Entries older than this are evicted.

**Recommendations:**
- Fast-changing codebases: 180-300 seconds (3-5 minutes)
- Stable codebases: 600-1800 seconds (10-30 minutes)
- Read-only exploration: 1800-3600 seconds (30-60 minutes)

#### `omnicontext.prefetch.debounceMs`
- **Type**: Number
- **Default**: `200`
- **Range**: 50-1000 milliseconds
- **Description**: Debounce delay for IDE events (cursor movement, text edits) to avoid excessive pre-fetch requests

**Recommendations:**
- Fast typists: 300-500ms (reduce noise)
- Slow/deliberate navigation: 100-200ms (more responsive)
- High-latency systems: 500-1000ms (reduce load)

### Other Settings

#### `omnicontext.autoStartDaemon`
- **Type**: Boolean
- **Default**: `true`
- **Description**: Automatically start the OmniContext daemon when opening a workspace

#### `omnicontext.contextInjection`
- **Type**: Boolean
- **Default**: `true`
- **Description**: Enable automatic context injection into AI chat requests

#### `omnicontext.tokenBudget`
- **Type**: Number
- **Default**: `8192`
- **Description**: Token budget for context injection (limits context size)

## Usage

### Sidebar Controls

Open the OmniContext sidebar from the Activity Bar (left side) to access:

**Pre-Fetch Cache Section:**
- View real-time cache statistics
- Enable/disable pre-fetch functionality
- Clear cache manually
- Monitor cache status (Active/Disabled/Offline)

**Context Control Section:**
- Toggle automatic context injection
- View daemon status
- Start/stop daemon manually

### Commands

Access via Command Palette (`Ctrl+Shift+P` or `Cmd+Shift+P`):

- `OmniContext: Index Workspace` - Manually trigger workspace indexing
- `OmniContext: Search Code` - Search codebase with semantic search
- `OmniContext: Show Status` - Display index statistics
- `OmniContext: Start Daemon` - Start the background daemon
- `OmniContext: Stop Daemon` - Stop the background daemon
- `OmniContext: Toggle Context Injection` - Enable/disable automatic context injection
- `OmniContext: Refresh Sidebar` - Refresh sidebar statistics

## Cache Hit Rate Expectations

### Good Performance (>60% hit rate)
- You're working in a focused area of the codebase
- Cache size and TTL are well-tuned for your workflow
- Daemon is running smoothly

### Fair Performance (30-60% hit rate)
- You're exploring different parts of the codebase
- Cache size might be too small for your project
- Consider increasing `cacheSize` or `cacheTtlSeconds`

### Poor Performance (<30% hit rate)
- You're jumping between many different files rapidly
- Cache size is too small or TTL is too short
- Daemon might be experiencing issues

**Optimization tips:**
1. Increase `cacheSize` to 200-500 for large projects
2. Increase `cacheTtlSeconds` to 600-1800 for stable codebases
3. Check daemon logs for errors: `OmniContext: Show Status`
4. Restart daemon if hit rate suddenly drops

## Troubleshooting

### Daemon Not Connecting

**Symptoms:**
- Sidebar shows "Daemon: Offline"
- Cache status shows "Offline"
- No context injection in AI chat

**Solutions:**
1. Check if daemon is running: `OmniContext: Show Status`
2. Manually start daemon: `OmniContext: Start Daemon`
3. Check daemon logs in Output panel (View → Output → OmniContext)
4. Verify binary path in settings: `omnicontext.binaryPath`
5. Restart VS Code

**Common causes:**
- Binary not found in PATH
- Port conflict (daemon already running)
- Insufficient permissions
- Antivirus blocking execution

### Cache Not Working

**Symptoms:**
- Hit rate always 0%
- All requests show [fresh search] indicator
- Cache size shows 0/100

**Solutions:**
1. Verify pre-fetch is enabled: `omnicontext.prefetch.enabled = true`
2. Check daemon connection (see above)
3. Clear cache and restart: `OmniContext: Stop Daemon` → `OmniContext: Start Daemon`
4. Check for errors in Output panel
5. Verify workspace is indexed: `OmniContext: Show Status`

**Common causes:**
- Pre-fetch disabled in settings
- Daemon not connected
- Cache TTL too short (entries expire immediately)
- Workspace not indexed yet

### High Memory Usage

**Symptoms:**
- VS Code using excessive memory (>500MB for extension)
- System slowdown
- Cache size growing unbounded

**Solutions:**
1. Reduce cache size: Set `omnicontext.prefetch.cacheSize` to 50-100
2. Reduce cache TTL: Set `omnicontext.prefetch.cacheTtlSeconds` to 180-300
3. Disable pre-fetch temporarily: Set `omnicontext.prefetch.enabled = false`
4. Restart daemon to clear memory: `OmniContext: Stop Daemon` → `OmniContext: Start Daemon`

**Expected memory usage:**
- Extension: 50-100MB
- Daemon: 100-200MB (depends on index size)
- Cache: ~1-5MB per 100 entries

### Performance Issues

**Symptoms:**
- Slow context injection (>2 seconds)
- UI freezing during searches
- High CPU usage

**Solutions:**
1. Increase debounce delay: Set `omnicontext.prefetch.debounceMs` to 500-1000
2. Reduce cache size to decrease memory pressure
3. Check if indexing is in progress: `OmniContext: Show Status`
4. Verify system resources (CPU, memory, disk)
5. Profile with VS Code Developer Tools: Help → Toggle Developer Tools

**Common causes:**
- Large workspace (>10k files) still indexing
- Insufficient system resources
- Disk I/O bottleneck
- Too many concurrent searches

### IPC Connection Errors

**Symptoms:**
- "Failed to connect to daemon" errors
- Reconnection attempts in logs
- Intermittent cache failures

**Solutions:**
1. Check named pipe/socket permissions (platform-specific)
2. Verify no firewall blocking local connections
3. Restart daemon: `OmniContext: Stop Daemon` → `OmniContext: Start Daemon`
4. Check for multiple daemon instances: Kill all and restart
5. Review IPC logs in Output panel

**Platform-specific:**
- **Windows**: Check named pipe `\\.\pipe\omnicontext-ipc`
- **macOS/Linux**: Check Unix socket `/tmp/omnicontext-ipc.sock`

## Architecture

### Event Flow

```
IDE Event (file open, cursor move, edit)
  |
  v
EventTracker (debouncing)
  |
  v
SymbolExtractor (get symbol at cursor)
  |
  v
IPC Client (send to daemon)
  |
  v
Daemon Pre-Fetch Cache (check cache)
  |
  v
  +-- Cache Hit --> Return cached context [cached]
  |
  +-- Cache Miss --> Search index --> Cache result --> Return context [fresh search]
```

### Components

- **EventTracker**: Monitors VS Code events with debouncing
- **SymbolExtractor**: Extracts symbols at cursor using language features
- **IPC Client**: Communicates with daemon via named pipe/socket
- **CacheStatsManager**: Retrieves and formats cache statistics
- **SidebarProvider**: Renders sidebar UI with cache controls

## Development

### Building from Source

```bash
cd editors/vscode
npm install
npm run compile
```

### Running in Development

1. Open `editors/vscode` in VS Code
2. Press F5 to launch Extension Development Host
3. Test features in the new window

### Running Tests

```bash
npm test
```

## Performance Metrics

Target performance (measured on typical developer machine):

- Event processing: <5ms
- IPC round-trip: <10ms
- Symbol extraction: <20ms
- Sidebar refresh: <100ms
- Memory overhead: <50MB

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for development guidelines.

## License

Apache 2.0 - See [LICENSE](../../LICENSE) for details.

## Support

- **Issues**: [GitHub Issues](https://github.com/steeltroops-ai/omnicontext/issues)
- **Documentation**: [Main README](../../README.md)
- **Installation**: [INSTALL.md](../../INSTALL.md)
