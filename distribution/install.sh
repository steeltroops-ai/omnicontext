#!/usr/bin/env bash
# OmniContext Installer - macOS / Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
# Pinned version: OMNICONTEXT_VERSION=v1.2.3 ./install.sh
# Force reinstall:  FORCE=1 ./install.sh

set -euo pipefail

# ---------------------------------------------------------------------------
# Script-level config / overridable env vars
# ---------------------------------------------------------------------------
REPO_OWNER="steeltroops-ai"
REPO_NAME="omnicontext"
BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${HOME}/.omnicontext"
CARGO_BIN="${HOME}/.cargo/bin/omnicontext"
FORCE="${FORCE:-0}"
# Allow pinning: OMNICONTEXT_VERSION=v1.2.3 ./install.sh
PINNED_VERSION="${OMNICONTEXT_VERSION:-}"
SKIP_MODEL=0
SKIP_MCP=0
SKIP_ONNX=0
DRY_RUN=0
SELECTED_MODEL=""

# ---------------------------------------------------------------------------
# Usage / help  (printed before color detection so we use raw ANSI sequences)
# ---------------------------------------------------------------------------
_usage() {
    cat <<'OMNI_USAGE_EOF'
OmniContext Installer — macOS / Linux

USAGE
  curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash
  ./install.sh [OPTIONS]

OPTIONS
  -h, --help              Show this help message and exit
  -f, --force             Bypass up-to-date check and reinstall even if the
                          current version already matches the target
      --version <ver>     Pin a specific release version  (e.g. v1.2.3)
      --dir <path>        Override install directory for binaries
                          Default: ~/.local/bin
      --model <name>      Select the embedding model to download
                          Default: jina-embeddings-v2-base-code
      --no-model          Skip embedding model download entirely
      --no-mcp            Skip MCP client auto-configuration
      --no-onnx           Skip ONNX Runtime shared-library download
      --dry-run           Print every action that would be taken without
                          actually modifying the system; implies --no-model,
                          --no-mcp, and --no-onnx

ENVIRONMENT VARIABLES
  OMNICONTEXT_VERSION     Pin the release version without a flag
                          Example: OMNICONTEXT_VERSION=v1.2.3 ./install.sh
  FORCE                   Set to 1 to force reinstall  (same as --force)
  NO_COLOR                Set to any non-empty value to disable ANSI colour

EXAMPLES
  # Standard one-line install (always fetches the latest release)
  curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash

  # Pin a specific version
  OMNICONTEXT_VERSION=v1.2.3 bash install.sh

  # Install to a custom directory, skip model download
  ./install.sh --dir /usr/local/bin --no-model

  # Use a different embedding model
  ./install.sh --model all-minilm-l6-v2

  # Preview what the installer would do without touching the system
  ./install.sh --dry-run

  # Silent update inside CI (no tty, no model, no ONNX)
  ./install.sh --force --no-model --no-onnx --no-mcp

  # Reinstall current version, keep data, skip slow downloads
  ./install.sh --force --no-model --no-onnx

NOTES
  • Requires: bash 3.2+, curl, tar
  • Optional: python3 (for richer JSON merging of MCP configs)
  • Binaries are placed in BIN_DIR (~/.local/bin by default) and a PATH
    entry is appended to ~/.bashrc / ~/.zshrc / ~/.profile as appropriate
  • To uninstall:
      bash <(curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/uninstall.sh)
  • Homepage: https://github.com/steeltroops-ai/omnicontext
OMNI_USAGE_EOF
}

# ---------------------------------------------------------------------------
# Parse flags — safe for both direct execution AND piped-via-bash invocation.
# When piped (curl | bash) there are no positional args so the loop is a
# no-op.  When run directly the full flag set is parsed.
# ---------------------------------------------------------------------------
while [ $# -gt 0 ]; do
    case "$1" in
        -h|--help)
            _usage
            exit 0
            ;;
        -f|--force)
            FORCE=1
            shift
            ;;
        --version)
            PINNED_VERSION="$2"
            shift 2
            ;;
        --version=*)
            PINNED_VERSION="${1#--version=}"
            shift
            ;;
        --dir)
            BIN_DIR="$2"
            shift 2
            ;;
        --dir=*)
            BIN_DIR="${1#--dir=}"
            shift
            ;;
        --model)
            SELECTED_MODEL="$2"
            shift 2
            ;;
        --model=*)
            SELECTED_MODEL="${1#--model=}"
            shift
            ;;
        --no-model)
            SKIP_MODEL=1
            shift
            ;;
        --no-mcp)
            SKIP_MCP=1
            shift
            ;;
        --no-onnx)
            SKIP_ONNX=1
            shift
            ;;
        --dry-run)
            DRY_RUN=1
            SKIP_MODEL=1
            SKIP_MCP=1
            SKIP_ONNX=1
            shift
            ;;
        *)
            shift
            ;;
    esac
done

# ---------------------------------------------------------------------------
# color helpers (degrade gracefully when no tty)
# ---------------------------------------------------------------------------
if [ -t 1 ]; then
    BOLD=$'\033[1m';  DIM=$'\033[2m';   RESET=$'\033[0m'
    RED=$'\033[31m';  GREEN=$'\033[32m'; YELLOW=$'\033[33m'
    BLUE=$'\033[34m'; CYAN=$'\033[36m';  WHITE=$'\033[97m'
else
    BOLD=""; DIM=""; RESET=""; RED=""; GREEN=""; YELLOW=""
    BLUE=""; CYAN=""; WHITE=""
fi

step()  { printf "${BOLD}${CYAN}  [%s]${RESET} %s\n" "$1" "$2"; }
ok()    { printf "${GREEN}  ✔${RESET} %s\n" "$*"; }
info()  { printf "${BLUE}  »${RESET} %s\n" "$*"; }
warn()  { printf "${YELLOW}  ⚠${RESET} %s\n" "$*"; }
err()   { printf "${RED}  ✖${RESET} %s\n" "$*"; }
hr()    { printf "${DIM}%s${RESET}\n" "──────────────────────────────────────────────────────────────"; }
blank() { echo ""; }

die() { blank; err "$1"; blank; exit 1; }

# ---------------------------------------------------------------------------
# Cleanup / rollback state
# ---------------------------------------------------------------------------
TEMP_DIR=""
BACKUP_DIR=""
DID_BACKUP=0
NEED_BINARY_INSTALL=0
ONNX_PID=""

_cleanup() {
    local exit_code=$?
    # Kill any background ONNX download still running
    if [ -n "$ONNX_PID" ] && kill -0 "$ONNX_PID" 2>/dev/null; then
        kill "$ONNX_PID" 2>/dev/null || true
    fi
    # Clean temp dir
    [ -n "$TEMP_DIR" ] && rm -rf -- "$TEMP_DIR"
    # Rollback binaries on failure
    if [ $exit_code -ne 0 ] && [ "$DID_BACKUP" -eq 1 ] && [ -d "$BACKUP_DIR" ]; then
        blank
        warn "Install failed — restoring previous binaries from backup..."
        for bak in "$BACKUP_DIR"/*.bak; do
            [ -f "$bak" ] || continue
            orig="${BIN_DIR}/$(basename "${bak%.bak}")"
            cp -f "$bak" "$orig" 2>/dev/null && ok "  Restored $(basename "$orig")" || true
        done
        rm -rf "$BACKUP_DIR"
        warn "Rollback complete. Previous version is still in place."
    fi
    [ $exit_code -ne 0 ] && [ -d "$BACKUP_DIR" ] && rm -rf "$BACKUP_DIR" 2>/dev/null || true
    exit $exit_code
}
trap '_cleanup' EXIT

# ---------------------------------------------------------------------------
# Retry-enabled curl wrapper
# ---------------------------------------------------------------------------
curl_retry() {
    # Usage: curl_retry <url> <output_file> [desc]
    local url="$1"
    local out="$2"
    curl -#Lf \
         --retry 3 \
         --retry-delay 2 \
         --retry-connrefused \
         --connect-timeout 15 \
         --max-time 600 \
         "$url" -o "$out" 2>&1
}

# Silent version for API calls
curl_api() {
    curl -sSLf \
         --retry 3 \
         --retry-delay 2 \
         --connect-timeout 10 \
         --max-time 30 \
         "$@"
}

# ---------------------------------------------------------------------------
# Internet connectivity check
# ---------------------------------------------------------------------------
check_connectivity() {
    if ! curl_api -o /dev/null "https://api.github.com" 2>/dev/null; then
        blank
        err "No internet access detected."
        info "Offline install options:"
        info "  1. cargo install omnicontext"
        info "  2. Download manually from https://github.com/${REPO_OWNER}/${REPO_NAME}/releases"
        info "     and place binaries in ${BIN_DIR}/"
        die "Cannot continue without network access."
    fi
}

SECONDS=0

# ---------------------------------------------------------------------------
# banner
# ---------------------------------------------------------------------------
blank
printf "${BOLD}${CYAN}"
cat <<'EOF'
   ____                  _  ______            __            __
  / __ \____ ___  ____  (_)/ ____/___  ____  / /____  _  __/ /_
 / / / / __ `__ \/ __ \/ // /   / __ \/ __ \/ __/ _ \| |/_/ __/
/ /_/ / / / / / / / / / // /___/ /_/ / / / / /_/  __/_>  </ /_
\____/_/ /_/ /_/_/ /_/_/ \____/\____/_/ /_/\__/\___/_/|_|\__/
EOF
printf "${RESET}"
printf "${DIM}  Universal Code Context Engine — macOS / Linux Installer${RESET}\n"
hr
blank

# ---------------------------------------------------------------------------
# Environment warnings
# ---------------------------------------------------------------------------

# Root check
if [ "$(id -u)" -eq 0 ]; then
    warn "Running as root. A user-level install (without sudo) is recommended."
    warn "Binaries will still be installed to ${BIN_DIR} (user home)."
    blank
fi

# NixOS detection
if [ -f /etc/NIXOS ] || command -v nix-env >/dev/null 2>&1; then
    warn "NixOS / nix-env detected."
    warn "This installer writes to ${BIN_DIR}. For a fully declarative install:"
    warn "  nix profile install github:${REPO_OWNER}/${REPO_NAME}"
    warn "Continuing with standard install..."
    blank
fi

# ---------------------------------------------------------------------------
# step 1 - resolve version
# ---------------------------------------------------------------------------
step "1/8" "Resolving version"

TOTAL_STEPS=8
VERSION="${PINNED_VERSION:-}"

if [ -n "$VERSION" ]; then
    ok "Using pinned version  ${DIM}(${VERSION})${RESET}"
else
    # Connectivity check only needed when fetching version
    check_connectivity

    API="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases"
    if RELEASES=$(curl_api "$API" 2>/dev/null); then
        if command -v python3 >/dev/null 2>&1; then
            VERSION=$(python3 -c "
import json, sys
releases = json.load(sys.stdin)
for r in releases:
    if r.get('assets'):
        print(r['tag_name'])
        break
" <<< "$RELEASES" 2>/dev/null || true)
        else
            VERSION=$(printf '%s\n' "$RELEASES" | grep '"tag_name":' | head -n1 \
                      | sed -E 's/.*"([^"]+)".*/\1/' || true)
        fi
    fi

    if [ -n "$VERSION" ]; then
        ok "Latest release with assets  ${DIM}(${VERSION})${RESET}"
    else
        warn "GitHub API unavailable — falling back to Cargo.toml"
        CARGO_URL="https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/Cargo.toml"
        if CARGO_CONTENT=$(curl_api "$CARGO_URL" 2>/dev/null); then
            SOURCE_VER=$(printf '%s\n' "$CARGO_CONTENT" \
                | grep -m1 '^version' \
                | sed -E 's/.*"([^"]+)".*/\1/' 2>/dev/null || true)
            if [ -n "$SOURCE_VER" ]; then
                VERSION="v${SOURCE_VER}"
                ok "Version resolved from source  ${DIM}(${VERSION})${RESET}"
            fi
        fi
        [ -n "$VERSION" ] || die "Could not resolve a published release version."
    fi
fi

CLEAN_VERSION="${VERSION#v}"

# ---------------------------------------------------------------------------
# Update detection / already-up-to-date check
# ---------------------------------------------------------------------------
PREV_VERSION=""
IS_UPDATE=0

if command -v omnicontext >/dev/null 2>&1 || [ -x "${BIN_DIR}/omnicontext" ]; then
    PREV_VERSION=$(${BIN_DIR}/omnicontext --version 2>/dev/null \
                  || omnicontext --version 2>/dev/null \
                  || true)
    PREV_VERSION=$(printf '%s' "$PREV_VERSION" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n1 || true)
    if [ -n "$PREV_VERSION" ]; then
        IS_UPDATE=1
        if [ "$PREV_VERSION" = "$CLEAN_VERSION" ] && [ "$FORCE" -ne 1 ]; then
            blank
            ok "OmniContext ${BOLD}v${CLEAN_VERSION}${RESET} is already up-to-date."
            info "Use FORCE=1 ./install.sh or --force to reinstall."
            blank
            exit 0
        elif [ "$PREV_VERSION" != "$CLEAN_VERSION" ]; then
            info "Updating  ${DIM}v${PREV_VERSION}${RESET} → ${BOLD}v${CLEAN_VERSION}${RESET}"
        else
            info "Force-reinstalling v${CLEAN_VERSION}"
        fi
    fi
fi

# ---------------------------------------------------------------------------
# step 2 - detect platform
# ---------------------------------------------------------------------------
blank
step "2/8" "Detecting platform"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)  OS_NAME="unknown-linux-gnu" ;;
    Darwin) OS_NAME="apple-darwin" ;;
    *)      die "Unsupported OS: $OS" ;;
esac

case "$ARCH" in
    x86_64|amd64)  ARCH_NAME="x86_64" ;;
    arm64|aarch64) ARCH_NAME="aarch64" ;;
    *)             die "Unsupported architecture: $ARCH" ;;
esac

ASSET_NAME="omnicontext-${CLEAN_VERSION}-${ARCH_NAME}-${OS_NAME}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${VERSION}/${ASSET_NAME}"

ok "Platform  ${DIM}${ARCH_NAME}-${OS_NAME}${RESET}"
info "Asset    ${DIM}${ASSET_NAME}${RESET}"

# ---------------------------------------------------------------------------
# Cargo install path detection (skip download if cargo binary present)
# ---------------------------------------------------------------------------
USE_CARGO_BIN=0
if [ ! -f "${BIN_DIR}/omnicontext" ] && [ -x "$CARGO_BIN" ]; then
    blank
    info "Detected cargo-installed binary at ${DIM}${CARGO_BIN}${RESET}"
    info "Skipping binary download — using existing cargo install."
    info "${DIM}Tip: cargo install omnicontext  ← installs/updates from crates.io${RESET}"
    USE_CARGO_BIN=1
fi

# ---------------------------------------------------------------------------
# step 3 - download
# ---------------------------------------------------------------------------
blank
step "3/8" "Downloading release archive"

TEMP_DIR="$(mktemp -d)"

if [ "$DRY_RUN" -eq 1 ]; then
    info "[dry-run] Would download  ${DIM}${DOWNLOAD_URL}${RESET}"
    info "[dry-run] Would install binaries to  ${DIM}${BIN_DIR}${RESET}"
    NEED_BINARY_INSTALL=0
elif [ "$USE_CARGO_BIN" -eq 1 ]; then
    ok "Binary download skipped (cargo install path in use)"
    NEED_BINARY_INSTALL=0
else
    info "URL  ${DIM}${DOWNLOAD_URL}${RESET}"

    # Connectivity guard (may have been skipped with pinned version)
    if ! curl_api -o /dev/null "https://api.github.com" 2>/dev/null; then
        blank
        err "No internet access. Cannot download release archive."
        info "Offline options:"
        info "  cargo install omnicontext"
        info "  OR download ${ASSET_NAME} manually and place in ${BIN_DIR}/"
        die "Aborting."
    fi

    if ! curl_retry "$DOWNLOAD_URL" "${TEMP_DIR}/${ASSET_NAME}"; then
        blank
        err "Download failed."
        info "Verify the release exists: https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/tag/${VERSION}"
        info "Alternative: cargo install omnicontext"
        die "Aborting."
    fi

    # Integrity: minimum 1 MB
    ARCHIVE_BYTES=$(wc -c < "${TEMP_DIR}/${ASSET_NAME}" 2>/dev/null | tr -d ' ' || echo 0)
    if [ "$ARCHIVE_BYTES" -lt 1048576 ]; then
        die "Downloaded archive is suspiciously small (${ARCHIVE_BYTES} bytes). Partial download or wrong URL."
    fi

    # Verify archive is valid before extracting
    if ! tar -tzf "${TEMP_DIR}/${ASSET_NAME}" >/dev/null 2>&1; then
        die "Archive integrity check failed — the file may be corrupted. Try again."
    fi

    ARCHIVE_SIZE=$(du -sh "${TEMP_DIR}/${ASSET_NAME}" 2>/dev/null | cut -f1 || echo "?")
    ok "Downloaded and verified  ${DIM}${ARCHIVE_SIZE}${RESET}"
    NEED_BINARY_INSTALL=1
fi

# ---------------------------------------------------------------------------
# step 4 - stop running instances
# ---------------------------------------------------------------------------
blank
step "4/8" "Stopping active processes"

if [ "$DRY_RUN" -eq 1 ]; then
    info "[dry-run] Would stop any running omnicontext processes"
else
STOPPED=0
for proc in omnicontext-daemon omnicontext-mcp omnicontext; do
    if pkill -x "$proc" 2>/dev/null; then
        ok "  Stopped ${proc}"
        STOPPED=$((STOPPED + 1))
    fi
done
if [ "$STOPPED" -eq 0 ]; then
    info "No active OmniContext processes found"
else
    sleep 0.5
fi
fi  # end dry-run guard for step 4

# ---------------------------------------------------------------------------
# step 5 - backup existing, extract and install
# ---------------------------------------------------------------------------
blank
step "5/8" "Installing binaries"

if [ "$DRY_RUN" -eq 1 ]; then
    info "[dry-run] Would extract ${ASSET_NAME} and install binaries to ${DIM}${BIN_DIR}${RESET}"
else

mkdir -p "$BIN_DIR"

# Backup existing binaries before overwriting
BACKUP_DIR="${DATA_DIR}/backup_${CLEAN_VERSION}"
mkdir -p "$BACKUP_DIR"
for bin_name in omnicontext omnicontext-mcp omnicontext-daemon; do
    existing="${BIN_DIR}/${bin_name}"
    if [ -f "$existing" ]; then
        cp -f "$existing" "${BACKUP_DIR}/${bin_name}.bak" 2>/dev/null && DID_BACKUP=1 || true
    fi
done
if [ "$DID_BACKUP" -eq 1 ]; then
    info "Backed up existing binaries to  ${DIM}${BACKUP_DIR}${RESET}"
fi

if [ "$NEED_BINARY_INSTALL" -eq 1 ]; then
    tar -xzf "${TEMP_DIR}/${ASSET_NAME}" -C "$TEMP_DIR"

    # Locate binaries — handles flat, nested, or deep layouts
    locate_bin() {
        local name="$1"
        [ -f "${TEMP_DIR}/${name}" ]                        && { printf '%s' "${TEMP_DIR}/${name}"; return; }
        local sub="${TEMP_DIR}/${ASSET_NAME%.tar.gz}"
        [ -f "${sub}/${name}" ]                             && { printf '%s' "${sub}/${name}"; return; }
        find "$TEMP_DIR" -name "$name" -type f | head -n1
    }

    BIN_OMNI=$(locate_bin "omnicontext")
    BIN_MCP=$(locate_bin "omnicontext-mcp")
    BIN_DAEMON=$(locate_bin "omnicontext-daemon")

    if [ -z "$BIN_OMNI" ]; then
        blank
        err "omnicontext binary not found in archive."
        info "Fallback option: cargo install omnicontext"
        die "Extraction failed."
    fi
    [ -n "$BIN_MCP" ] || die "omnicontext-mcp binary not found in archive."

    install -m 755 "$BIN_OMNI" "${BIN_DIR}/omnicontext"
    install -m 755 "$BIN_MCP"  "${BIN_DIR}/omnicontext-mcp"
    [ -n "$BIN_DAEMON" ] && [ -f "$BIN_DAEMON" ] \
        && install -m 755 "$BIN_DAEMON" "${BIN_DIR}/omnicontext-daemon"

    ok "Installed to  ${DIM}${BIN_DIR}${RESET}"
    info "omnicontext      ${DIM}$(du -sh "${BIN_DIR}/omnicontext" 2>/dev/null | cut -f1)${RESET}"
    info "omnicontext-mcp  ${DIM}$(du -sh "${BIN_DIR}/omnicontext-mcp" 2>/dev/null | cut -f1)${RESET}"
    [ -f "${BIN_DIR}/omnicontext-daemon" ] && \
        info "omnicontext-daemon  ${DIM}$(du -sh "${BIN_DIR}/omnicontext-daemon" 2>/dev/null | cut -f1)${RESET}"
else
    ok "Using cargo-installed binary — no binary extraction needed"
    BIN_DIR="$(dirname "$CARGO_BIN")"
fi

fi  # end dry-run guard for step 5

# ---------------------------------------------------------------------------
# ONNX Runtime shared library
# Attempt parallel download alongside model setup (launched below).
# ---------------------------------------------------------------------------

get_latest_onnx_version() {
    local api="https://api.github.com/repos/microsoft/onnxruntime/releases/latest"
    local ver=""
    if res=$(curl_api "$api" 2>/dev/null); then
        ver=$(printf '%s\n' "$res" \
              | grep '"tag_name":' | head -n1 \
              | sed -E 's/.*"v?([^"]+)".*/\1/' || true)
    fi
    printf '%s' "${ver:-1.24.3}"
}

install_onnx_runtime() {
    local dest_dir="$1"
    local version="$2"
    local onnx_tmp
    onnx_tmp="$(mktemp -d)"

    local onnx_url lib_name lib_link
    if [ "$(uname -s)" = "Darwin" ]; then
        if [ "$(uname -m)" = "arm64" ]; then
            onnx_url="https://github.com/microsoft/onnxruntime/releases/download/v${version}/onnxruntime-osx-arm64-${version}.tgz"
        else
            onnx_url="https://github.com/microsoft/onnxruntime/releases/download/v${version}/onnxruntime-osx-x86_64-${version}.tgz"
        fi
        lib_name="libonnxruntime.${version}.dylib"
        lib_link="libonnxruntime.dylib"
    else
        onnx_url="https://github.com/microsoft/onnxruntime/releases/download/v${version}/onnxruntime-linux-x64-${version}.tgz"
        lib_name="libonnxruntime.so.${version}"
        lib_link="libonnxruntime.so"
    fi

    info "ONNX Runtime ${version}  ${DIM}${onnx_url}${RESET}"

    if ! curl_retry "$onnx_url" "${onnx_tmp}/onnxruntime.tgz"; then
        rm -rf "$onnx_tmp"
        return 1
    fi

    tar -xzf "${onnx_tmp}/onnxruntime.tgz" -C "$onnx_tmp"

    local lib_src
    lib_src=$(find "$onnx_tmp" -name "$lib_name" -type f | head -n1)
    [ -z "$lib_src" ] && lib_src=$(find "$onnx_tmp" -name "libonnxruntime*" -type f | head -n1)

    if [ -z "$lib_src" ]; then
        rm -rf "$onnx_tmp"
        return 1
    fi

    cp "$lib_src" "${dest_dir}/$(basename "$lib_src")"
    ln -sf "$(basename "$lib_src")" "${dest_dir}/${lib_link}" 2>/dev/null || true
    rm -rf "$onnx_tmp"
    return 0
}

# Determine expected symlink path
if [ "$(uname -s)" = "Darwin" ]; then
    ONNX_LIB="${BIN_DIR}/libonnxruntime.dylib"
else
    ONNX_LIB="${BIN_DIR}/libonnxruntime.so"
fi

NEED_ONNX=1
ONNX_VERSION=$(get_latest_onnx_version)

if [ "$SKIP_ONNX" -eq 1 ] || [ "$DRY_RUN" -eq 1 ]; then
    NEED_ONNX=0
elif [ -f "$ONNX_LIB" ]; then
    MAJOR="${ONNX_VERSION%%.*}"
    if ls "${BIN_DIR}"/libonnxruntime* 2>/dev/null | grep -q "${MAJOR}"; then
        ok "ONNX Runtime already present  ${DIM}v${ONNX_VERSION}${RESET}"
        NEED_ONNX=0
    fi
fi

# Launch ONNX download in background so it runs concurrently with model setup
if [ "$NEED_ONNX" -eq 1 ] && [ "$SKIP_ONNX" -eq 0 ] && [ "$DRY_RUN" -eq 0 ]; then
    info "Starting parallel ONNX Runtime download in background..."
    # Capture color variables NOW (before the subshell) so they are available
    # even when stdout is not a tty inside the background process.
    _BG_GREEN="$GREEN"
    _BG_YELLOW="$YELLOW"
    _BG_RESET="$RESET"
    (install_onnx_runtime "$BIN_DIR" "$ONNX_VERSION" \
        && printf '\n%s✔%s ONNX Runtime %s installed (parallel download complete)\n' \
            "${_BG_GREEN}" "${_BG_RESET}" "$ONNX_VERSION" \
        || printf '\n%s⚠%s ONNX Runtime download failed (context injection may not work)\n' \
            "${_BG_YELLOW}" "${_BG_RESET}") &
    ONNX_PID=$!
elif [ "$SKIP_ONNX" -eq 1 ]; then
    info "ONNX Runtime download skipped (--no-onnx)"
fi

# ---------------------------------------------------------------------------
# PATH bootstrap (non-login shells)
# ---------------------------------------------------------------------------
export PATH="${BIN_DIR}:${PATH}"

SHELL_RC=""
case "${SHELL:-}" in
    */zsh)  SHELL_RC="${HOME}/.zshrc" ;;
    */bash) SHELL_RC="${HOME}/.bashrc" ;;
    *)      SHELL_RC="${HOME}/.profile" ;;
esac

PATH_LINE="export PATH=\"\$HOME/.local/bin:\$PATH\""
# Only add if not already present AND bin_dir is the standard ~/.local/bin
if [ "$BIN_DIR" = "${HOME}/.local/bin" ] \
   && ! grep -qF "$PATH_LINE" "$SHELL_RC" 2>/dev/null; then
    {
        echo ""
        echo "# OmniContext"
        echo "$PATH_LINE"
    } >> "$SHELL_RC"
    ok "PATH entry added to  ${DIM}${SHELL_RC}${RESET}"
else
    ok "PATH already configured  ${DIM}${SHELL_RC}${RESET}"
fi

# ---------------------------------------------------------------------------
# step 6 - embedding model setup (runs while ONNX downloads in parallel)
# ---------------------------------------------------------------------------
blank
step "6/8" "Embedding model setup"

OMNI_BIN="${BIN_DIR}/omnicontext"
[ "$USE_CARGO_BIN" -eq 1 ] && OMNI_BIN="$CARGO_BIN"

if [ "$SKIP_MODEL" -eq 1 ]; then
    info "Embedding model download skipped (--no-model)"
elif [ "$DRY_RUN" -eq 1 ]; then
    info "[dry-run] Would download embedding model to ${DIM}${DATA_DIR}/models/${RESET}"
else
    HAS_SETUP=false
    if "$OMNI_BIN" --help 2>&1 | grep -q "setup"; then
        HAS_SETUP=true
    fi

    if [ "$HAS_SETUP" = "true" ]; then
        MODEL_READY="false"
        MODEL_NAME="${SELECTED_MODEL:-jina-embeddings-v2-base-code}"
        MODEL_SIZE="?"

        if status_json=$("$OMNI_BIN" setup model-status --json 2>/dev/null); then
            MODEL_READY=$(printf '%s' "$status_json" \
                | grep -o '"model_ready": [a-z]*' | cut -d' ' -f2 || true)
            # Use --model override if provided, otherwise take name from status
            if [ -z "$SELECTED_MODEL" ]; then
                MODEL_NAME=$(printf '%s' "$status_json" \
                    | grep -o '"model_name": "[^"]*"' | cut -d'"' -f4 || true)
            fi
            MODEL_BYTES=$(printf '%s' "$status_json" \
                | grep -o '"model_size_bytes": [0-9]*' | cut -d' ' -f2 || true)
            if [ "${MODEL_BYTES:-0}" -gt 0 ] 2>/dev/null; then
                MODEL_SIZE="$((MODEL_BYTES / 1024 / 1024)) MB"
            fi
        fi

        if [ "$MODEL_READY" = "true" ]; then
            ok "Model ready: ${BOLD}${MODEL_NAME}${RESET}  ${DIM}(${MODEL_SIZE})${RESET}"
        else
            info "Downloading model: ${BOLD}${MODEL_NAME}${RESET}  ${DIM}(~550 MB, HuggingFace)${RESET}"
            blank
            printf "  ${DIM}────────────────────────────────────────${RESET}\n"
            # Build model-download command — pass --model if the user requested one
            MODEL_DL_CMD=("$OMNI_BIN" setup model-download)
            [ -n "$SELECTED_MODEL" ] && MODEL_DL_CMD+=("--model" "$SELECTED_MODEL")
            # Model download failure is non-fatal — user can run later
            if ! "${MODEL_DL_CMD[@]}" 2>&1; then
                warn "Model download interrupted or failed."
                warn "Run later: ${DIM}omnicontext setup model-download${RESET}"
            fi
            printf "  ${DIM}────────────────────────────────────────${RESET}\n"

            if status_json=$("$OMNI_BIN" setup model-status --json 2>/dev/null); then
                MODEL_READY=$(printf '%s' "$status_json" \
                    | grep -o '"model_ready": [a-z]*' | cut -d' ' -f2 || true)
                if [ "$MODEL_READY" = "true" ]; then
                    ok "Model setup successful"
                else
                    warn "Model download incomplete. Run: ${DIM}omnicontext setup model-download${RESET}"
                fi
            fi
        fi
    else
        # Legacy fallback
        if [ -f "${DATA_DIR}/models/jina-embeddings-v2-base-code/model.onnx" ]; then
            ok "Model ready (cached)"
        else
            warn "Legacy binary detected — model will be initialized on first index."
            info "Run: ${DIM}omnicontext index .${RESET}  to trigger model download."
        fi
    fi
fi

# ---------------------------------------------------------------------------
# Wait for background ONNX download to finish
# ---------------------------------------------------------------------------
if [ -n "$ONNX_PID" ] && kill -0 "$ONNX_PID" 2>/dev/null; then
    info "Waiting for background ONNX Runtime download..."
    wait "$ONNX_PID" || warn "ONNX Runtime background download failed (can be installed later)."
    ONNX_PID=""
fi

# ---------------------------------------------------------------------------
# step 7 - MCP auto-configure via setup --all
# ---------------------------------------------------------------------------
#
# Supported clients (handled by omnicontext setup --all):
#   - Claude Desktop        (~/.config/claude/claude_desktop_config.json)
#   - Claude Code           (~/.claude.json)
#   - Cursor                (~/.cursor/mcp.json)
#   - Windsurf              (~/.codeium/windsurf/mcp_config.json)
#   - VS Code               (global mcp.json, key="servers")
#   - Cline                 (~/.cline/mcp_settings.json)
#   - RooCode               (~/.roo-cline/mcp_settings.json)
#   - Continue.dev          (~/.continue/config.json)
#   - Zed                   (~/.config/zed/settings.json)
#   - Kiro                  (~/.kiro/settings/mcp.json  powers.mcpServers)
#   - PearAI                (~/.pearai/mcp_config.json)
#   - Trae                  (~/.trae/mcp.json)
#   - Antigravity           (~/.config/Antigravity/User/mcp.json)
#   - Gemini CLI            (~/.gemini/settings.json)
#   - Amazon Q CLI          (~/.aws/amazonq/mcp.json)
#   - Augment Code          (~/.augment/mcp_config.json)
#
# The orchestrator uses --repo . as the universal entry point; each IDE's
# extension/plugin resolves the actual project root at launch time.
# ---------------------------------------------------------------------------
blank
step "7/8" "Auto-configuring MCP for AI clients"

MCP_CONFIGURED_COUNT=0
SETUP_ALL_USED=0

if [ "$SKIP_MCP" -eq 1 ]; then
    info "MCP configuration skipped (--no-mcp)"
elif [ "$DRY_RUN" -eq 1 ]; then
    info "[dry-run] Would configure MCP entries for all detected AI clients"
else

# Primary: use setup --all (binary orchestrates all 15+ IDEs correctly)
if "$OMNI_BIN" --help 2>&1 | grep -qE 'setup.*--all|setup --all'; then
    info "Using ${BOLD}omnicontext setup --all${RESET} (orchestrator handles all clients)"
    blank
    printf "  ${DIM}────────────────────────────────────────${RESET}\n"
    if "$OMNI_BIN" setup --all 2>&1 | while IFS= read -r line; do
           printf "  %s\n" "$line"
           case "$line" in *configured*) MCP_CONFIGURED_COUNT=$((MCP_CONFIGURED_COUNT + 1)) ;; esac
        done; then
        printf "  ${DIM}────────────────────────────────────────${RESET}\n"
        ok "MCP configuration complete via orchestrator"
        SETUP_ALL_USED=1
    else
        printf "  ${DIM}────────────────────────────────────────${RESET}\n"
        warn "setup --all returned non-zero; falling back to manual configuration"
    fi
fi

# Fallback: manual JSON injection if setup --all not available or failed
if [ "$SETUP_ALL_USED" -eq 0 ]; then
    info "Falling back to manual JSON injection..."
    blank

    MCP_BIN="${BIN_DIR}/omnicontext-mcp"
    [ "$USE_CARGO_BIN" -eq 1 ] && MCP_BIN="${HOME}/.cargo/bin/omnicontext-mcp"

    # --repo . is the universal entry; IDE plugins resolve the real project root
    MCP_ENTRY="{\"command\":\"${MCP_BIN}\",\"args\":[\"--repo\",\".\"]}"

    _write_mcp_config() {
        local config_path="$1"
        local top_key="$2"    # "mcpServers" or "powers"
        local server_key="$3" # "mcpServers" (under powers) or "mcpServers" (direct)
        local config_dir
        config_dir="$(dirname "$config_path")"
        [ -d "$config_dir" ] || return 1

        if command -v python3 >/dev/null 2>&1; then
            python3 - "$config_path" "$top_key" "$server_key" "$MCP_BIN" <<'PYEOF' 2>/dev/null && return 0
import json, sys, os

path, top_key, server_key, mcp_bin = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
entry = {"command": mcp_bin, "args": ["--repo", "."]}

cfg = {}
if os.path.exists(path):
    try:
        with open(path) as f:
            cfg = json.load(f)
    except Exception:
        cfg = {}

if top_key == "powers":
    cfg.setdefault("powers", {}).setdefault("mcpServers", {})["omnicontext"] = entry
elif top_key == "servers":
    cfg.setdefault("servers", {})["omnicontext"] = entry
else:
    cfg.setdefault("mcpServers", {})["omnicontext"] = entry

os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
with open(path, "w") as f:
    json.dump(cfg, f, indent=2)
PYEOF
        fi

        # Bare-minimum fallback (no merging)
        mkdir -p "$config_dir"
        if [ "$top_key" = "powers" ]; then
            printf '{"powers":{"mcpServers":{"omnicontext":%s}}}\n' "$MCP_ENTRY" > "$config_path"
        elif [ "$top_key" = "servers" ]; then
            printf '{"servers":{"omnicontext":%s}}\n' "$MCP_ENTRY" > "$config_path"
        else
            printf '{"mcpServers":{"omnicontext":%s}}\n' "$MCP_ENTRY" > "$config_path"
        fi
        return 0
    }

    if [ "$(uname -s)" = "Darwin" ]; then
        CLAUDE_CFG="${HOME}/Library/Application Support/Claude/claude_desktop_config.json"
    else
        CLAUDE_CFG="${HOME}/.config/claude/claude_desktop_config.json"
    fi

    # "Client Name|config_path|top_key"
    # Plain array — compatible with bash 3.2 (macOS default); no declare -A needed.
    # VS Code: macOS keeps mcp.json under ~/.vscode/; Linux uses the XDG path.
    if [ "$(uname -s)" = "Darwin" ]; then
        _vscode_mcp="${HOME}/.vscode/mcp.json"
    else
        _vscode_mcp="${HOME}/.config/Code/User/mcp.json"
    fi

    MCP_FALLBACK_CLIENTS=(
        "Claude Desktop|${CLAUDE_CFG}|mcpServers"
        "Claude Code|${HOME}/.claude.json|mcpServers"
        "Cursor|${HOME}/.cursor/mcp.json|mcpServers"
        "Windsurf|${HOME}/.codeium/windsurf/mcp_config.json|mcpServers"
        "VS Code|${_vscode_mcp}|servers"
        "Cline|${HOME}/.cline/mcp_settings.json|mcpServers"
        "RooCode|${HOME}/.roo-cline/mcp_settings.json|mcpServers"
        "Continue.dev|${HOME}/.continue/config.json|mcpServers"
        "Zed|${HOME}/.config/zed/settings.json|context_servers"
        "Kiro|${HOME}/.kiro/settings/mcp.json|powers"
        "PearAI|${HOME}/.pearai/mcp_config.json|mcpServers"
        "Trae|${HOME}/.trae/mcp.json|mcpServers"
        "Antigravity|${HOME}/.config/Antigravity/User/mcp.json|servers"
        "Gemini CLI|${HOME}/.gemini/settings.json|mcpServers"
        "Amazon Q CLI|${HOME}/.aws/amazonq/mcp.json|mcpServers"
        "Augment Code|${HOME}/.augment/mcp_config.json|mcpServers"
    )

    MCP_CONFIGURED=""
    for entry in "${MCP_FALLBACK_CLIENTS[@]}"; do
        IFS='|' read -r client config_path top_key <<< "$entry"
        if _write_mcp_config "$config_path" "$top_key" "mcpServers"; then
            MCP_CONFIGURED="${MCP_CONFIGURED:+${MCP_CONFIGURED}, }${client}"
            ok "  ${client}  ${DIM}${config_path}${RESET}"
        fi
    done

    if [ -z "$MCP_CONFIGURED" ]; then
        warn "No AI client config dirs detected."
        warn "Install Claude Desktop / Cursor / Windsurf / VS Code / etc. and re-run."
    else
        blank
        ok "$(printf '%s\n' "$MCP_CONFIGURED" | tr ',' '\n' | wc -l | tr -d ' ') client(s) configured (fallback)"
    fi
fi

fi  # end SKIP_MCP / DRY_RUN guard

# ---------------------------------------------------------------------------
# step 8 - cleanup & success
# ---------------------------------------------------------------------------
blank
step "8/8" "Finalizing"

if [ "$DRY_RUN" -eq 1 ]; then
    blank
    hr
    printf "${BOLD}${YELLOW}  [dry-run] No changes made.${RESET}\n"
    printf "  The steps above show exactly what the installer would do.\n"
    hr
    blank
    printf "  To run for real, re-run without ${BOLD}--dry-run${RESET}.\n"
    blank
    exit 0
fi

# Delete backups on success
if [ "$DID_BACKUP" -eq 1 ] && [ -d "$BACKUP_DIR" ]; then
    rm -rf "$BACKUP_DIR"
    ok "Backups removed (install succeeded)"
fi

# Version report
INSTALLED_VER=$(${OMNI_BIN} --version 2>/dev/null \
    | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n1 || echo "$CLEAN_VERSION")
ok "Binary verified  ${DIM}v${INSTALLED_VER}${RESET}"

ELAPSED=$SECONDS
blank
hr
if [ "$IS_UPDATE" -eq 1 ] && [ -n "$PREV_VERSION" ] && [ "$PREV_VERSION" != "$CLEAN_VERSION" ]; then
    printf "${BOLD}${GREEN}  OmniContext updated  ${DIM}v%s → v%s${RESET}  ${DIM}(%ss)${RESET}\n" \
        "$PREV_VERSION" "$CLEAN_VERSION" "$ELAPSED"
else
    printf "${BOLD}${GREEN}  OmniContext v%s installed${RESET}  ${DIM}(%ss)${RESET}\n" \
        "$CLEAN_VERSION" "$ELAPSED"
fi
hr
blank

printf "${BOLD}  Quick Start${RESET}\n"
printf "  cd /path/to/your/repo\n"
printf "  omnicontext index .\n"
printf "  omnicontext search \"error handling\"\n"
blank

if [ "$SETUP_ALL_USED" -eq 1 ]; then
    printf "${BOLD}  MCP${RESET}  configured via ${DIM}omnicontext setup --all${RESET}\n"
    printf "  All detected AI clients have been registered.\n"
    printf "  Open any project and your AI tools will pick up context automatically.\n"
else
    MCP_BIN_DISPLAY="${BIN_DIR}/omnicontext-mcp"
    printf "${BOLD}  MCP${RESET}\n"
    printf "  command: %s\n" "$MCP_BIN_DISPLAY"
    printf '  args:    ["--repo", "."]\n'
    printf "  Re-run installer after installing Claude / Cursor / Windsurf / VS Code.\n"
fi

blank
printf "  ${DIM}Update:     OMNICONTEXT_VERSION=<ver> ./install.sh  or re-run anytime${RESET}\n"
printf "  ${DIM}Options:    --no-model  --no-onnx  --no-mcp  --dir <path>  --dry-run${RESET}\n"
printf "  ${DIM}Cargo:      cargo install omnicontext${RESET}\n"
printf "  ${DIM}Docs:       https://github.com/${REPO_OWNER}/${REPO_NAME}${RESET}\n"
blank
printf "  ${DIM}Uninstall:  bash <(curl -fsSL https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/distribution/uninstall.sh)${RESET}\n"
printf "  ${DIM}Manual:     rm -rf ${BIN_DIR}/omnicontext* ${DATA_DIR}${RESET}\n"
printf "  ${DIM}            and remove the PATH line from ${SHELL_RC}${RESET}\n"
blank
