# OmniContext Distribution

This directory contains installation scripts and package manager manifests for distributing OmniContext.

## Quick Install

### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex
```

### Linux/macOS (Bash)
```bash
curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
```

### Scoop (Windows)
```powershell
scoop bucket add omnicontext https://github.com/steeltroops-ai/omnicontext
scoop install omnicontext
```

### Homebrew (macOS/Linux)
```bash
brew tap steeltroops-ai/omnicontext
brew install omnicontext
```

## What Gets Installed

All installation methods automatically:
1. Download OmniContext binaries (omnicontext, omnicontext-mcp, omnicontext-daemon)
2. Download ONNX Runtime 1.23.0 (required for AI embeddings)
3. Download Jina AI embedding model (~550MB)
4. Add binaries to PATH
5. Verify installation

## Files

### Installation Scripts

- **`install.ps1`** - Windows PowerShell installer
  - Downloads latest release from GitHub
  - Installs to `$HOME\.omnicontext\bin`
  - Automatically downloads ONNX Runtime 1.23.0
  - Downloads embedding model
  - Adds to User PATH

- **`install.sh`** - Linux/macOS Bash installer
  - Downloads latest release from GitHub
  - Installs to `$HOME/.local/bin`
  - Automatically downloads ONNX Runtime 1.23.0
  - Downloads embedding model
  - Adds to PATH (requires shell restart)

### Package Manager Manifests

- **`scoop/omnicontext.json`** - Scoop manifest for Windows
  - Auto-updates from GitHub releases
  - Includes post-install script for model download
  - Validates SHA256 checksums

- **`homebrew/omnicontext.rb`** - Homebrew formula for macOS/Linux
  - Auto-updates from GitHub releases
  - Includes post-install hook for model download
  - Validates SHA256 checksums

## Testing Locally

### Test PowerShell Installer
```powershell
# From project root
.\distribution\install.ps1
```

### Test Bash Installer
```bash
# From project root
bash distribution/install.sh
```

### Test Scoop Manifest
```powershell
# Validate JSON syntax
Get-Content distribution/scoop/omnicontext.json | ConvertFrom-Json

# Test installation (requires actual release)
scoop install distribution/scoop/omnicontext.json
```

### Test Homebrew Formula
```bash
# Validate Ruby syntax
ruby -c distribution/homebrew/omnicontext.rb

# Test installation (requires actual release)
brew install --build-from-source distribution/homebrew/omnicontext.rb
```

## Release Checklist

When creating a new release, update these files:

1. **Update version in `Cargo.toml`**
   ```toml
   [workspace.package]
   version = "0.2.0"
   ```

2. **Update `distribution/scoop/omnicontext.json`**
   - Update `version` field
   - Update SHA256 hash after building release

3. **Update `distribution/homebrew/omnicontext.rb`**
   - Update `version` field
   - Update SHA256 hashes for each platform after building release

4. **Build release binaries**
   ```bash
   # Build for all platforms
   cargo build --release --target x86_64-pc-windows-msvc
   cargo build --release --target x86_64-unknown-linux-gnu
   cargo build --release --target x86_64-apple-darwin
   cargo build --release --target aarch64-apple-darwin
   ```

5. **Generate SHA256 checksums**
   ```bash
   # Windows
   sha256sum omnicontext-v0.2.0-x86_64-pc-windows-msvc.zip > omnicontext-v0.2.0-x86_64-pc-windows-msvc.zip.sha256
   
   # Linux
   sha256sum omnicontext-v0.2.0-x86_64-unknown-linux-gnu.tar.gz > omnicontext-v0.2.0-x86_64-unknown-linux-gnu.tar.gz.sha256
   
   # macOS Intel
   sha256sum omnicontext-v0.2.0-x86_64-apple-darwin.tar.gz > omnicontext-v0.2.0-x86_64-apple-darwin.tar.gz.sha256
   
   # macOS ARM
   sha256sum omnicontext-v0.2.0-aarch64-apple-darwin.tar.gz > omnicontext-v0.2.0-aarch64-apple-darwin.tar.gz.sha256
   ```

6. **Create GitHub release**
   - Tag: `v0.2.0`
   - Upload all binaries and SHA256 files
   - Update release notes

7. **Test installation**
   - Test PowerShell installer on Windows
   - Test Bash installer on Linux/macOS
   - Test Scoop installation
   - Test Homebrew installation

## ONNX Runtime Auto-Download

All installation methods now automatically download ONNX Runtime 1.23.0, which is required for the AI embedding model to work.

### Windows
- Downloads from: `https://github.com/microsoft/onnxruntime/releases/download/v1.23.0/onnxruntime-win-x64-1.23.0.zip`
- Installs to: `$HOME\.omnicontext\bin\` (same directory as binaries)
- Files: `onnxruntime.dll`, `onnxruntime.lib`, `onnxruntime_providers_shared.dll`, etc.

### Linux
- Downloads from: `https://github.com/microsoft/onnxruntime/releases/download/v1.23.0/onnxruntime-linux-x64-1.23.0.tgz`
- Installs to: `$HOME/.local/lib/onnxruntime/`
- Requires: `export LD_LIBRARY_PATH="$HOME/.local/lib/onnxruntime:$LD_LIBRARY_PATH"`

### macOS
- Downloads from: 
  - Intel: `https://github.com/microsoft/onnxruntime/releases/download/v1.23.0/onnxruntime-osx-x64-1.23.0.tgz`
  - ARM: `https://github.com/microsoft/onnxruntime/releases/download/v1.23.0/onnxruntime-osx-arm64-1.23.0.tgz`
- Installs to: `$HOME/.local/lib/onnxruntime/`
- Requires: `export DYLD_LIBRARY_PATH="$HOME/.local/lib/onnxruntime:$DYLD_LIBRARY_PATH"`

## Troubleshooting

### ONNX Runtime Not Found
If you see "ONNX Runtime version mismatch" or "model not available" errors:

**Windows:**
```powershell
# Run the fix script
pwsh scripts/fix-onnx-runtime.ps1
```

**Linux/macOS:**
```bash
# Add to ~/.bashrc or ~/.zshrc
export LD_LIBRARY_PATH="$HOME/.local/lib/onnxruntime:$LD_LIBRARY_PATH"  # Linux
export DYLD_LIBRARY_PATH="$HOME/.local/lib/onnxruntime:$DYLD_LIBRARY_PATH"  # macOS
```

### Model Download Failed
If the embedding model fails to download during installation:
```bash
# Manually trigger download
omnicontext index .
```

### PATH Not Updated
If `omnicontext` command is not found after installation:

**Windows:**
```powershell
# Restart terminal or manually add to PATH
$env:PATH += ";$HOME\.omnicontext\bin"
```

**Linux/macOS:**
```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.local/bin:$PATH"
```

## Support

- Documentation: https://github.com/steeltroops-ai/omnicontext
- Issues: https://github.com/steeltroops-ai/omnicontext/issues
- Discussions: https://github.com/steeltroops-ai/omnicontext/discussions
