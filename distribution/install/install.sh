#!/usr/bin/env bash

# OmniContext Installer
#
# This script determines the OS and architecture, downloads the correct
# OmniContext release binary, moves it to a directory in your PATH,
# and initializes the system by pre-downloading the Jina AI code embedding model.

set -euo pipefail

REPO_OWNER="steeltroops-ai"
REPO_NAME="omnicontext"

LATEST_RELEASE=$(curl -sSL "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" | grep '"tag_name":' | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/')
if [ -z "$LATEST_RELEASE" ]; then
    echo "Warning: Failed to fetch latest version from GitHub. Falling back to explicit alpha version."
    VERSION="v0.1.0-alpha"
else
    VERSION="$LATEST_RELEASE"
fi

echo "========================================="
echo " 🚀 Installing OmniContext"
echo "========================================="

# 1. Determine OS and architecture
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
        echo "Error: Operating system $OS is not supported by this script."
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
        echo "Error: CPU architecture $ARCH is not supported."
        exit 1
        ;;
esac

ASSET_NAME="omnicontext-${VERSION}-${ARCH_NAME}-${OS_NAME}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${VERSION}/${ASSET_NAME}"

# 2. Download and Extract
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TEMP_DIR"' EXIT

echo "Downloading $ASSET_NAME..."
if ! curl -sSL -f "$DOWNLOAD_URL" -o "${TEMP_DIR}/${ASSET_NAME}"; then
    echo "Error: Failed to download release from $DOWNLOAD_URL"
    echo "This version may not exist for your architecture ($ARCH_NAME-$OS_NAME)."
    exit 1
fi

echo "Checking for running instances for seamless update..."
pkill -x omnicontext-mcp || true
pkill -x omnicontext || true

echo "Extracting..."
tar -xzf "${TEMP_DIR}/${ASSET_NAME}" -C "$TEMP_DIR"

# 3. Move binaries to PATH
BIN_DIR="${HOME}/.local/bin"
mkdir -p "$BIN_DIR"

# Assuming the tarball contains omnicontext and omnicontext-mcp executables
if [ -f "${TEMP_DIR}/omnicontext" ] && [ -f "${TEMP_DIR}/omnicontext-mcp" ]; then
    mv "${TEMP_DIR}/omnicontext" "${TEMP_DIR}/omnicontext-mcp" "$BIN_DIR/"
else
    # Sometime tarballs unpack into a versioned subdirectory
    # e.g., omnicontext-v0.1.0-alpha-x86_64-unknown-linux-gnu/omnicontext
    SUBDIR="${ASSET_NAME%.tar.gz}"
    if [ -d "${TEMP_DIR}/${SUBDIR}" ]; then
        mv "${TEMP_DIR}/${SUBDIR}/omnicontext" "${TEMP_DIR}/${SUBDIR}/omnicontext-mcp" "$BIN_DIR/"
    else
        echo "Error: Could not locate binaries in the extracted archive."
        find "$TEMP_DIR"
        exit 1
    fi
fi

chmod +x "${BIN_DIR}/omnicontext" "${BIN_DIR}/omnicontext-mcp"

# Ensure ~/.local/bin is in PATH for this session
export PATH="${BIN_DIR}:${PATH}"

if ! command -v omnicontext >/dev/null 2>&1; then
    echo ""
    echo "Warning: ${BIN_DIR} is not in your PATH."
    echo "Please add it to your shell configuration (e.g., ~/.bashrc or ~/.zshrc):"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
fi

# 4. Initialize the system
echo ""
echo "Initializing OmniContext & downloading Jina AI embedding model..."
echo "This requires a robust internet connection. Please wait while the model downloads."

INIT_TEMP="$(mktemp -d)"
if ! (
    cd "$INIT_TEMP"
    # Runs status to force engine initialization and trigger model download with progress bar
    omnicontext status
); then
    echo "Warning: Model download may have been interrupted or failed."
fi
rm -rf "$INIT_TEMP"

echo ""
echo "========================================="
echo " ✅ OmniContext installation complete!"
echo "========================================="
echo ""
echo "To keep OmniContext updated locally, just re-run this install command anytime!"
echo ""
echo "Where to start indexing:"
echo "  Navigate to your code folder:  cd /path/to/your/repo"
echo "  Create the search index:       omnicontext index ."
echo "  Test searching your code:      omnicontext search \"auth\""
echo ""
echo "To connect your MCP (Claude, AI Agents), use this configuration:"
echo "  Command:  omnicontext-mcp"
echo "  Args:     [\"--repo\", \"/path/to/your/repo\"]"
echo ""
echo "Note: If ${BIN_DIR} was not previously in your PATH, please restart your terminal."
