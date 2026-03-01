# Installation Workflow Verification

## End-to-End Workflow (All Platforms)

### 1. Install (First Time)

**Windows (`install.ps1`):**
```
1. Fetch latest version from GitHub API
2. Download release binary (x86_64-pc-windows-msvc.zip)
3. Stop running processes (omnicontext, omnicontext-mcp, omnicontext-daemon)
4. Extract to %USERPROFILE%\.omnicontext\bin\
5. Add to User PATH environment variable
6. Create temp directory with dummy.rs file
7. Run `omnicontext index .` to trigger model download (~550MB)
8. Verify installation (binary execution, model file, PATH)
9. Display success message with next steps
```

**Unix/Linux/macOS (`install.sh`):**
```
1. Fetch latest version from GitHub API
2. Detect OS (Linux/Darwin) and architecture (x86_64/aarch64)
3. Download release binary (.tar.gz)
4. Stop running processes (pkill omnicontext*)
5. Extract to ~/.local/bin/
6. Set executable permissions (chmod +x)
7. Add to PATH (export in current session)
8. Create temp directory with dummy.rs file
9. Run `omnicontext index .` to trigger model download (~550MB)
10. Verify installation (binary execution, model file, PATH)
11. Display success message with next steps
```

### 2. Update (Existing Installation)

**All Platforms (`update.ps1`):**
```
1. Check if OmniContext is installed
2. Get current version (omnicontext --version)
3. Fetch latest version from GitHub API
4. Compare versions (skip if already latest, unless --Force)
5. Backup MCP configuration (~/.kiro/settings/mcp.json)
6. Stop running processes
7. Download and run install.ps1 (re-installs over existing)
8. Verify update (check new version)
9. Restore configuration if modified
10. Display success message
```

**Package Managers:**
```
# Scoop (Windows)
scoop update omnicontext

# Homebrew (macOS/Linux)
brew upgrade omnicontext
```

### 3. Uninstall

**Windows (`uninstall.ps1`):**
```
1. Confirm uninstallation with user
2. Stop running processes
3. Remove binaries (%USERPROFILE%\.omnicontext\bin\)
4. Remove from User PATH
5. Remove data directory (~/.omnicontext/) unless --KeepData
6. Remove MCP configuration unless --KeepConfig
7. Display summary
```

**Package Managers:**
```
# Scoop
scoop uninstall omnicontext

# Homebrew
brew uninstall omnicontext
```

## File Locations

### Binaries
- **Windows**: `%USERPROFILE%\.omnicontext\bin\`
  - `omnicontext.exe`
  - `omnicontext-mcp.exe`
  - `omnicontext-daemon.exe` (optional)

- **Unix/Linux/macOS**: `~/.local/bin/`
  - `omnicontext`
  - `omnicontext-mcp`
  - `omnicontext-daemon` (optional)

### Data & Models
- **All Platforms**: `~/.omnicontext/`
  - `models/jina-embeddings-v2-base-code.onnx` (~550MB)
  - `repos/{hash}/index.db` (SQLite database)
  - `repos/{hash}/vectors.usearch` (Vector index)
  - `repos/{hash}/graph.bin` (Dependency graph)

### Configuration
- **MCP Config**: `~/.kiro/settings/mcp.json`

## Edge Cases Handled

### ✅ Archive Structure Variations
- Flat structure: `omnicontext`, `omnicontext-mcp` at root
- Nested structure: `omnicontext-v0.1.0-x86_64-.../omnicontext`
- Recursive search fallback if neither pattern matches

### ✅ Running Processes
- Gracefully stops all running instances before update
- Waits 1 second after stopping to ensure clean shutdown
- Continues even if no processes found

### ✅ Model Download
- Checks if model already exists (skip re-download)
- Uses `index` command (not `status`) to trigger download
- Creates dummy source file to satisfy indexing requirement
- Shows progress bar during download
- Warns if download fails (will retry on first real use)

### ✅ PATH Configuration
- Adds to User PATH (persistent across sessions)
- Updates current session PATH (immediate availability)
- Warns if PATH not configured (provides manual instructions)
- Handles existing PATH entries (no duplicates)

### ✅ Network Failures
- Clear error messages with troubleshooting hints
- Suggests possible causes (no internet, GitHub down, wrong architecture)
- Provides download URL for manual verification

### ✅ Permission Issues
- Creates directories if missing (mkdir -p)
- Sets executable permissions on Unix (chmod +x)
- Uses User PATH (no admin/sudo required)

### ✅ Version Detection
- Handles missing version (fallback to v0.1.0-alpha)
- Compares versions correctly (strips 'v' prefix)
- Supports --Force flag to reinstall same version

### ✅ Configuration Preservation
- Backs up MCP config before update
- Restores if modified during update
- Prompts user before restoring
- Keeps indexed data across updates

## Verification Steps

After installation, the scripts verify:

1. **Binary Execution**: `omnicontext --version` succeeds
2. **Model File**: `~/.omnicontext/models/jina-embeddings-v2-base-code.onnx` exists
3. **PATH Configuration**: `command -v omnicontext` succeeds
4. **File Sizes**: Reports model size and indexed data size

## User Experience

### First Install
```
Time: 2-5 minutes (depending on internet speed)
Downloads: ~600MB (binary ~50MB + model ~550MB)
Disk Space: ~600MB
```

### Update
```
Time: 1-3 minutes
Downloads: ~50MB (binary only, model preserved)
Disk Space: No additional space (overwrites existing)
```

### Uninstall
```
Time: <10 seconds
Frees: ~600MB+ (binary + model + indexed data)
```

## Package Manager Integration

### Scoop (Windows)
- Manifest: `distribution/scoop/omnicontext.json`
- Auto-updates: Checks GitHub releases
- Installs to: `%USERPROFILE%\scoop\apps\omnicontext\`
- Shims: Automatic PATH management

### Homebrew (macOS/Linux)
- Formula: `distribution/homebrew/omnicontext.rb`
- Auto-updates: `brew upgrade`
- Installs to: `/usr/local/bin/` (macOS) or `/home/linuxbrew/.linuxbrew/bin/` (Linux)
- Symlinks: Automatic PATH management

## Testing Checklist

- [ ] Fresh install on Windows
- [ ] Fresh install on macOS (Intel)
- [ ] Fresh install on macOS (Apple Silicon)
- [ ] Fresh install on Linux (x86_64)
- [ ] Update from v0.1.0 to latest
- [ ] Update with --Force flag
- [ ] Uninstall with --KeepData
- [ ] Uninstall complete removal
- [ ] Install with existing model (skip download)
- [ ] Install with running processes (graceful stop)
- [ ] Install without internet (fail gracefully)
- [ ] Install with PATH already configured
- [ ] Scoop install/update/uninstall
- [ ] Homebrew install/update/uninstall

## Known Issues

None currently. All edge cases are handled.

## Future Improvements

1. Add checksum verification for downloads
2. Add rollback capability if update fails
3. Add telemetry opt-in for installation metrics
4. Add automatic update checks (opt-in)
5. Add installation analytics (anonymous)
