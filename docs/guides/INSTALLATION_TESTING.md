# Installation Testing Guide

This guide covers testing all OmniContext installation methods to ensure they work correctly.

## Scoop (Windows)

### Install
```powershell
scoop install omnicontext
```

**Expected behavior:**
1. Downloads omnicontext-v0.2.0-x86_64-pc-windows-msvc.zip
2. Extracts binaries to scoop apps directory
3. Runs post-install script:
   - Shows "Initializing OmniContext" message
   - Downloads ONNX Runtime (~50MB)
   - Downloads embedding model (~550MB)
   - Shows "OmniContext is ready to use!"
4. Adds to PATH automatically

**Verify:**
```powershell
# Check version
omnicontext --version
# Should show: omnicontext 0.2.0

# Check binaries exist
Get-Command omnicontext
Get-Command omnicontext-mcp
Get-Command omnicontext-daemon

# Check ONNX Runtime
Get-ChildItem (scoop prefix omnicontext) -Filter onnxruntime*.dll

# Check model downloaded
Test-Path $HOME\.omnicontext\models\jina-embeddings-v2-base-code.onnx
```

### Update
```powershell
scoop update omnicontext
```

**Expected behavior:**
1. Checks GitHub for new version
2. Downloads new version if available
3. Stops running processes
4. Replaces binaries
5. Runs post-install again (re-downloads model if needed)

**Verify:**
```powershell
omnicontext --version
# Should show latest version
```

### Uninstall
```powershell
scoop uninstall omnicontext
```

**Expected behavior:**
1. Runs pre-uninstall script:
   - Stops all omnicontext processes
   - Shows message about preserved user data
2. Removes binaries
3. Removes from PATH
4. Preserves `$HOME\.omnicontext` directory

**Verify:**
```powershell
# Binaries should be gone
Get-Command omnicontext -ErrorAction SilentlyContinue
# Should return nothing

# User data preserved
Test-Path $HOME\.omnicontext
# Should return True

# Manual cleanup (optional)
Remove-Item -Path $HOME\.omnicontext -Recurse -Force
```

## Homebrew (macOS/Linux)

### Install
```bash
brew install omnicontext
```

**Expected behavior:**
1. Downloads appropriate tarball for platform:
   - macOS ARM: omnicontext-v0.2.0-aarch64-apple-darwin.tar.gz
   - macOS Intel: omnicontext-v0.2.0-x86_64-apple-darwin.tar.gz
   - Linux: omnicontext-v0.2.0-x86_64-unknown-linux-gnu.tar.gz
2. Extracts and installs binaries to Homebrew cellar
3. Runs post-install hook:
   - Shows "Initializing OmniContext" message
   - Downloads ONNX Runtime (~50MB)
   - Downloads embedding model (~550MB)
   - Shows "OmniContext is ready to use!"
4. Links binaries to PATH

**Verify:**
```bash
# Check version
omnicontext --version
# Should show: omnicontext 0.2.0

# Check binaries exist
which omnicontext
which omnicontext-mcp
which omnicontext-daemon

# Check model downloaded
ls -lh ~/.omnicontext/models/jina-embeddings-v2-base-code.onnx
```

### Update
```bash
brew upgrade omnicontext
```

**Expected behavior:**
1. Checks GitHub for new version
2. Downloads new version if available
3. Replaces binaries
4. Runs post-install again

**Verify:**
```bash
omnicontext --version
# Should show latest version
```

### Uninstall
```bash
brew uninstall omnicontext
```

**Expected behavior:**
1. Removes binaries from Homebrew cellar
2. Removes symlinks from PATH
3. Preserves `~/.omnicontext` directory

**Verify:**
```bash
# Binaries should be gone
which omnicontext
# Should return nothing

# User data preserved
ls -la ~/.omnicontext

# Manual cleanup (optional)
rm -rf ~/.omnicontext
```

## PowerShell Installer (Windows)

### Install
```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex
```

**Expected behavior:**
1. Fetches latest version from GitHub API
2. Downloads omnicontext-v0.2.0-x86_64-pc-windows-msvc.zip
3. Stops running instances
4. Extracts to `$HOME\.omnicontext\bin`
5. Downloads ONNX Runtime 1.23.0:
   - Downloads onnxruntime-win-x64-1.23.0.zip
   - Extracts DLLs to `$HOME\.omnicontext\bin`
6. Adds to User PATH
7. Downloads embedding model (~550MB)
8. Shows installation summary

**Verify:**
```powershell
# Check version
omnicontext --version

# Check ONNX Runtime DLLs
Get-ChildItem $HOME\.omnicontext\bin\onnxruntime*.dll

# Check model
Test-Path $HOME\.omnicontext\models\jina-embeddings-v2-base-code.onnx

# Test indexing
cd $HOME
mkdir test-repo
cd test-repo
"fn main() {}" | Out-File test.rs
omnicontext index .
omnicontext status
```

### Update
```powershell
# Just re-run the installer
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex
```

**Expected behavior:**
1. Stops running instances
2. Replaces binaries with new version
3. Updates ONNX Runtime if needed
4. Preserves existing model and data

### Uninstall
```powershell
# Remove from PATH
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
$newPath = ($userPath -split ';' | Where-Object { $_ -notlike "*omnicontext*" }) -join ';'
[Environment]::SetEnvironmentVariable("PATH", $newPath, "User")

# Remove binaries
Remove-Item -Path $HOME\.omnicontext\bin -Recurse -Force

# Optional: Remove all data
Remove-Item -Path $HOME\.omnicontext -Recurse -Force
```

## Bash Installer (Linux/macOS)

### Install
```bash
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
```

**Expected behavior:**
1. Detects OS and architecture
2. Fetches latest version from GitHub API
3. Downloads appropriate tarball
4. Stops running instances
5. Extracts to `~/.local/bin`
6. Downloads ONNX Runtime 1.23.0:
   - Linux: onnxruntime-linux-x64-1.23.0.tgz
   - macOS Intel: onnxruntime-osx-x64-1.23.0.tgz
   - macOS ARM: onnxruntime-osx-arm64-1.23.0.tgz
7. Installs to `~/.local/lib/onnxruntime`
8. Shows instructions for updating PATH and LD_LIBRARY_PATH
9. Downloads embedding model (~550MB)
10. Shows installation summary

**Verify:**
```bash
# Check version
omnicontext --version

# Check ONNX Runtime libraries
ls -la ~/.local/lib/onnxruntime/

# Check model
ls -lh ~/.omnicontext/models/jina-embeddings-v2-base-code.onnx

# Test indexing
cd ~
mkdir test-repo
cd test-repo
echo "fn main() {}" > test.rs
omnicontext index .
omnicontext status
```

### Update
```bash
# Just re-run the installer
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
```

**Expected behavior:**
1. Stops running instances
2. Replaces binaries with new version
3. Updates ONNX Runtime if needed
4. Preserves existing model and data

### Uninstall
```bash
# Remove binaries
rm -f ~/.local/bin/omnicontext*

# Remove ONNX Runtime
rm -rf ~/.local/lib/onnxruntime

# Optional: Remove all data
rm -rf ~/.omnicontext
```

## Common Issues

### ONNX Runtime Not Found

**Symptoms:**
- "ONNX Runtime version mismatch" error
- "model not available" error
- Search returns no results

**Solution (Windows):**
```powershell
pwsh scripts/fix-onnx-runtime.ps1
```

**Solution (Linux/macOS):**
```bash
# Add to ~/.bashrc or ~/.zshrc
export LD_LIBRARY_PATH="$HOME/.local/lib/onnxruntime:$LD_LIBRARY_PATH"  # Linux
export DYLD_LIBRARY_PATH="$HOME/.local/lib/onnxruntime:$DYLD_LIBRARY_PATH"  # macOS

# Reload shell
source ~/.bashrc  # or source ~/.zshrc
```

### Model Download Failed

**Symptoms:**
- Installation completes but model not found
- "Model not downloaded" warning

**Solution:**
```bash
# Manually trigger download
omnicontext index .
```

### PATH Not Updated

**Symptoms:**
- `omnicontext: command not found`
- Binaries installed but not accessible

**Solution (Windows):**
```powershell
# Restart terminal or manually add
$env:PATH += ";$HOME\.omnicontext\bin"
```

**Solution (Linux/macOS):**
```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.local/bin:$PATH"

# Reload shell
source ~/.bashrc  # or source ~/.zshrc
```

## Test Checklist

Before releasing a new version, test all installation methods:

- [ ] Scoop install works
- [ ] Scoop update works
- [ ] Scoop uninstall works
- [ ] Homebrew install works
- [ ] Homebrew upgrade works
- [ ] Homebrew uninstall works
- [ ] PowerShell installer works
- [ ] Bash installer works
- [ ] ONNX Runtime downloads correctly
- [ ] Embedding model downloads correctly
- [ ] All binaries are executable
- [ ] PATH is updated correctly
- [ ] `omnicontext --version` shows correct version
- [ ] `omnicontext index .` works
- [ ] `omnicontext search "test"` works
- [ ] `omnicontext status` shows healthy state
- [ ] MCP server starts: `omnicontext-mcp --repo .`

## Automated Testing

For CI/CD pipelines:

```bash
# Test installation
./distribution/install.sh

# Verify binaries
omnicontext --version
omnicontext-mcp --help

# Test basic functionality
mkdir test-repo
cd test-repo
echo "fn main() {}" > test.rs
omnicontext index .
omnicontext search "main"
omnicontext status

# Cleanup
cd ..
rm -rf test-repo
```
