#!/usr/bin/env bash

# OmniContext Installer
#
# This script determines the OS and architecture, downloads the correct
# OmniContext release binary, moves it to a directory in your PATH,
# and initializes the system by pre-downloading the Jina AI code embedding model.

set -euo pipefail

REPO_OWNER="steeltroops-ai"
REPO_NAME="omnicontext"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

info() { echo -e "${CYAN}ℹ${NC} $*"; }
success() { echo -e "${GREEN}✓${NC} $*"; }
warning() { echo -e "${YELLOW}⚠${NC} $*"; }
error() { echo -e "${RED}✗${NC} $*"; }

echo "========================================="
echo " 🚀 Installing OmniContext"
echo "========================================="

# Fetch version from source code (Cargo.toml)
info "Fetching latest version from source..."
CARGO_URL="https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/Cargo.toml"

if CARGO_CONTENT=$(curl -sSL -f "$CARGO_URL"); then
    if SOURCE_VERSION=$(echo "$CARGO_CONTENT" | grep -m1 'version\s*=' | sed -E 's/.*version\s*=\s*"([^"]+)".*/\1/'); then
        if [ -n "$SOURCE_VERSION" ]; then
            VERSION="v${SOURCE_VERSION}"
            success "Latest version from source: $VERSION"
        else
            warning "Could not parse version from Cargo.toml"
            VERSION=""
        fi
    else
        warning "Could not parse version from Cargo.toml"
        VERSION=""
    fi
else
    warning "Could not fetch Cargo.toml from GitHub"
    VERSION=""
fi

# Fallback: Check GitHub releases if source version fetch failed
if [ -z "$VERSION" ]; then
    info "Trying GitHub releases..."
    RELEASES_JSON=$(curl -sSL "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases" || echo "[]")

    # Check if any releases exist
    RELEASE_COUNT=$(echo "$RELEASES_JSON" | grep -c '"tag_name"' || echo "0")

    if [ "$RELEASE_COUNT" -eq 0 ]; then
        error "No pre-built releases available yet"
        echo ""
        warning "OmniContext doesn't have pre-built releases yet."
        info "You'll need to build from source."
        echo ""
        echo "To build from source:"
        echo "  1. Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "  2. Clone the repo: git clone https://github.com/steeltroops-ai/omnicontext.git"
        echo "  3. Build: cd omnicontext && cargo build --release"
        echo "  4. Binaries will be in: target/release/"
        echo ""
        echo "For detailed instructions, see:"
        echo "  https://github.com/steeltroops-ai/omnicontext/blob/main/CONTRIBUTING.md"
        echo ""
        exit 1
    fi

    # Get the latest release tag
    LATEST_RELEASE=$(echo "$RELEASES_JSON" | grep '"tag_name":' | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/' || echo "")

    if [ -z "$LATEST_RELEASE" ]; then
        error "Could not determine version. Please build from source."
        echo "See: https://github.com/steeltroops-ai/omnicontext/blob/main/CONTRIBUTING.md"
        exit 1
    else
        VERSION="$LATEST_RELEASE"
        success "Using release version: $VERSION"
    fi
fi

# Determine OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        OS_NAME="unknown-linux-gnu"
        ;;
    Darwin)
        OS_NAME="apple-darwin"
        ;;
    *)
        error "Operating system $OS is not supported"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        ARCH_NAME="x86_64"
        ;;
    arm64|aarch64)
        ARCH_NAME="aarch64"
        ;;
    *)
        error "CPU architecture $ARCH is not supported"
        exit 1
        ;;
esac

success "Platform: $ARCH_NAME-$OS_NAME"

ASSET_NAME="omnicontext-${VERSION}-${ARCH_NAME}-${OS_NAME}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${VERSION}/${ASSET_NAME}"

# Download and extract
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TEMP_DIR"' EXIT

info "Downloading $ASSET_NAME..."
info "URL: $DOWNLOAD_URL"

if ! curl -sSL -f "$DOWNLOAD_URL" -o "${TEMP_DIR}/${ASSET_NAME}"; then
    error "Failed to download release"
    info "URL: $DOWNLOAD_URL"
    info ""
    info "Possible causes:"
    info "- Release $VERSION doesn't exist for $ARCH_NAME-$OS_NAME"
    info "- No internet connection"
    info "- GitHub is down"
    exit 1
fi

success "Download complete"

# Stop running instances
info "Checking for running instances..."
pkill -x omnicontext-mcp 2>/dev/null && success "Stopped omnicontext-mcp" || true
pkill -x omnicontext 2>/dev/null && success "Stopped omnicontext" || true
pkill -x omnicontext-daemon 2>/dev/null && success "Stopped omnicontext-daemon" || true

# Extract
info "Extracting..."
tar -xzf "${TEMP_DIR}/${ASSET_NAME}" -C "$TEMP_DIR"

# Find binaries (handle both flat and nested structures)
OMNICONTEXT_BIN=""
OMNICONTEXT_MCP_BIN=""
OMNICONTEXT_DAEMON_BIN=""

# Check for flat structure first
if [ -f "${TEMP_DIR}/omnicontext" ]; then
    OMNICONTEXT_BIN="${TEMP_DIR}/omnicontext"
    OMNICONTEXT_MCP_BIN="${TEMP_DIR}/omnicontext-mcp"
    OMNICONTEXT_DAEMON_BIN="${TEMP_DIR}/omnicontext-daemon"
else
    # Check for nested structure
    SUBDIR="${ASSET_NAME%.tar.gz}"
    if [ -d "${TEMP_DIR}/${SUBDIR}" ]; then
        OMNICONTEXT_BIN="${TEMP_DIR}/${SUBDIR}/omnicontext"
        OMNICONTEXT_MCP_BIN="${TEMP_DIR}/${SUBDIR}/omnicontext-mcp"
        OMNICONTEXT_DAEMON_BIN="${TEMP_DIR}/${SUBDIR}/omnicontext-daemon"
    else
        # Search recursively
        OMNICONTEXT_BIN=$(find "$TEMP_DIR" -name "omnicontext" -type f | head -n 1)
        OMNICONTEXT_MCP_BIN=$(find "$TEMP_DIR" -name "omnicontext-mcp" -type f | head -n 1)
        OMNICONTEXT_DAEMON_BIN=$(find "$TEMP_DIR" -name "omnicontext-daemon" -type f | head -n 1)
    fi
fi

# Verify binaries found
if [ -z "$OMNICONTEXT_BIN" ] || [ ! -f "$OMNICONTEXT_BIN" ]; then
    error "Could not locate omnicontext binary in archive"
    find "$TEMP_DIR" -type f
    exit 1
fi

if [ -z "$OMNICONTEXT_MCP_BIN" ] || [ ! -f "$OMNICONTEXT_MCP_BIN" ]; then
    error "Could not locate omnicontext-mcp binary in archive"
    exit 1
fi

success "Found binaries"

# Install to ~/.local/bin
BIN_DIR="${HOME}/.local/bin"
mkdir -p "$BIN_DIR"

info "Installing to $BIN_DIR..."
mv "$OMNICONTEXT_BIN" "$BIN_DIR/"
mv "$OMNICONTEXT_MCP_BIN" "$BIN_DIR/"
[ -f "$OMNICONTEXT_DAEMON_BIN" ] && mv "$OMNICONTEXT_DAEMON_BIN" "$BIN_DIR/" || true

chmod +x "${BIN_DIR}/omnicontext" "${BIN_DIR}/omnicontext-mcp"
[ -f "${BIN_DIR}/omnicontext-daemon" ] && chmod +x "${BIN_DIR}/omnicontext-daemon" || true

success "Binaries installed"

# Ensure ~/.local/bin is in PATH
export PATH="${BIN_DIR}:${PATH}"

if ! command -v omnicontext >/dev/null 2>&1; then
    warning "${BIN_DIR} is not in your PATH"
    info "Add this to your shell configuration (~/.bashrc or ~/.zshrc):"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
fi

# Download embedding model
echo ""
info "Downloading Jina AI embedding model (~550MB)..."
info "This requires a good internet connection and may take several minutes."
info "The model enables semantic code search and AI agent capabilities."
echo ""

MODEL_PATH="${HOME}/.omnicontext/models/jina-embeddings-v2-base-code.onnx"

if [ -f "$MODEL_PATH" ]; then
    success "Model already downloaded"
else
    # Create temporary directory with dummy file
    INIT_TEMP="$(mktemp -d)"
    
    # Create dummy source file
    echo "// Dummy file for model download" > "${INIT_TEMP}/dummy.rs"
    echo "fn main() {}" >> "${INIT_TEMP}/dummy.rs"
    
    # Run index command to trigger model download
    info "Triggering model download..."
    (
        cd "$INIT_TEMP"
        "${BIN_DIR}/omnicontext" index . || warning "Model download may have failed"
    )
    
    if [ -f "$MODEL_PATH" ]; then
        success "Model download complete"
    else
        warning "Model not downloaded (will download on first use)"
    fi
    
    # Cleanup
    rm -rf "$INIT_TEMP"
fi

# Installation verification
echo ""
info "Running installation verification..."

VERIFICATION_PASSED=0
VERIFICATION_FAILED=0

# Test 1: Binary execution
if VERSION_OUTPUT=$("${BIN_DIR}/omnicontext" --version 2>&1); then
    success "Binary works: $VERSION_OUTPUT"
    ((VERIFICATION_PASSED++))
else
    error "Binary execution failed"
    ((VERIFICATION_FAILED++))
fi

# Test 2: Model file
if [ -f "$MODEL_PATH" ]; then
    MODEL_SIZE=$(du -h "$MODEL_PATH" | cut -f1)
    success "Model file: $MODEL_SIZE"
    ((VERIFICATION_PASSED++))
else
    warning "Model not downloaded (will download on first use)"
    ((VERIFICATION_FAILED++))
fi

# Test 3: PATH
if command -v omnicontext >/dev/null 2>&1; then
    success "PATH configured correctly"
    ((VERIFICATION_PASSED++))
else
    warning "PATH not updated (restart terminal or add to shell config)"
    ((VERIFICATION_FAILED++))
fi

# Summary
echo ""
echo "========================================="
echo " ✅ Installation Complete!"
echo "========================================="
echo ""
echo "Verification: $VERIFICATION_PASSED passed, $VERIFICATION_FAILED warnings"
echo ""
echo "Installation Details:"
echo "  Binaries: $BIN_DIR"
echo "  Model: $MODEL_PATH"
echo "  Version: $VERSION"
echo ""
echo -e "${CYAN}Quick Start:${NC}"
echo "  1. Open a NEW terminal (to load PATH changes)"
echo "  2. Navigate to your code: cd /path/to/your/repo"
echo "  3. Index your code: omnicontext index ."
echo "  4. Search your code: omnicontext search \"authentication\""
echo ""
echo -e "${CYAN}MCP Configuration (for AI agents):${NC}"
echo "  Add to your MCP config (~/.kiro/settings/mcp.json):"
echo ""
echo '  {'
echo '    "mcpServers": {'
echo '      "omnicontext": {'
echo "        \"command\": \"${BIN_DIR}/omnicontext-mcp\","
echo '        "args": ["--repo", "/path/to/your/repo"],'
echo '        "disabled": false'
echo '      }'
echo '    }'
echo '  }'
echo ""
echo -e "${CYAN}Update OmniContext:${NC}"
echo "  Just re-run this installation script anytime!"
echo ""
echo "Documentation: https://github.com/steeltroops-ai/omnicontext"
echo "Issues: https://github.com/steeltroops-ai/omnicontext/issues"
echo ""

if [ $VERIFICATION_FAILED -gt 0 ]; then
    warning "Installation completed with $VERIFICATION_FAILED warnings"
    if ! command -v omnicontext >/dev/null 2>&1; then
        info "Add ${BIN_DIR} to your PATH by adding this to ~/.bashrc or ~/.zshrc:"
        echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi
fi
