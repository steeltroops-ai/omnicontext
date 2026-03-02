# Testing the OmniContext VS Code Extension

## Quick Start

### 1. Build the Extension

```bash
cd editors/vscode
bun install
bun run compile
```

### 2. Run in Development Mode

**Option A: Using VS Code**
1. Open `editors/vscode` folder in VS Code
2. Press `F5` to launch Extension Development Host
3. A new VS Code window will open with the extension loaded

**Option B: Using Command Line**
```bash
code --extensionDevelopmentPath=./editors/vscode
```

### 3. Test the Extension

In the Extension Development Host window:

1. **Open the Sidebar**
   - Click the OmniContext icon in the Activity Bar (left side)
   - You should see the Control Center with 8 sections

2. **Check System Status**
   - System Status section shows initialization and connection health
   - Should show "Ready" and "Connected" if daemon is running

3. **Test Pre-Fetch Cache**
   - Open a code file
   - Move cursor around
   - Check cache statistics update in sidebar
   - Toggle pre-fetch on/off

4. **Test Repository Management**
   - Click "Re-index Repository" button
   - Watch progress notification
   - Check files/chunks indexed count updates

5. **Test Automation Toggles**
   - Toggle Auto-Index on Open
   - Toggle Auto-Start Daemon
   - Toggle Auto-Sync MCP
   - Settings should persist

6. **Test Quick Actions**
   - Click "Quick Search" button
   - Should open search input
   - Click "Sync to Claude" or "Sync to Kiro"

7. **Test Activity Log**
   - Perform actions (re-index, clear cache, etc.)
   - Check Activity Log section updates
   - Click on activity items to see details

8. **Test System Information**
   - Check version and platform display
   - Click "Copy Diagnostics" button
   - Paste clipboard to verify diagnostics copied
   - Click "Open Logs" to view output channel

## Running Tests

```bash
cd editors/vscode
bun run test
```

## Building VSIX Package

```bash
cd editors/vscode
bun run package
```

This creates `omnicontext-0.2.0.vsix` that can be installed in VS Code.

## Installing the VSIX

```bash
code --install-extension omnicontext-0.2.0.vsix
```

Or in VS Code:
1. Extensions view (Ctrl+Shift+X)
2. Click "..." menu
3. Select "Install from VSIX..."
4. Choose the .vsix file

## Debugging

### View Extension Logs
1. View → Output
2. Select "OmniContext" from dropdown

### View Developer Console
1. Help → Toggle Developer Tools
2. Check Console tab for errors

### Common Issues

**Extension not loading:**
- Check Output panel for errors
- Verify TypeScript compiled: `bun run compile`
- Check no syntax errors: `bun run lint`

**Sidebar not showing:**
- Check Activity Bar for OmniContext icon
- Try View → Open View → OmniContext

**Daemon not connecting:**
- Verify daemon is running: `omnicontext-daemon`
- Check daemon logs in Output panel
- Restart daemon: Command Palette → "OmniContext: Start Daemon"

## Performance Testing

Monitor extension performance:
1. Help → Toggle Developer Tools
2. Performance tab
3. Record while using extension
4. Check for slow operations

Target metrics:
- Event processing: <5ms
- IPC round-trip: <10ms
- Sidebar refresh: <100ms
- Memory usage: <50MB

## All Available Commands

Run these from Command Palette (Ctrl+Shift+P):

- `OmniContext: Index Workspace`
- `OmniContext: Search Code`
- `OmniContext: Show Status`
- `OmniContext: Start MCP Server`
- `OmniContext: Start Daemon`
- `OmniContext: Stop Daemon`
- `OmniContext: Toggle Context Injection`
- `OmniContext: Pre-flight Context`
- `OmniContext: Show Module Map`
- `OmniContext: Sync MCP to Claude/Config`
- `OmniContext: Refresh Sidebar`

## Verifying Icon Rendering

All icons should use VS Code codicons (no emojis):

**System Status:**
- codicon-pulse (pulsing icon)
- codicon-sync (refresh button)

**Pre-Fetch Cache:**
- codicon-zap (active status)
- codicon-warning (offline status)
- codicon-circle-slash (disabled status)

**Performance Metrics:**
- codicon-graph

**Repository Management:**
- codicon-folder

**Automation:**
- codicon-settings-gear

**Quick Actions:**
- codicon-rocket

**Activity Log:**
- codicon-list-unordered
- codicon-close (clear button)

**System Information:**
- codicon-info

If you see emoji characters instead of icons, the codicon CSS is not loading properly.
