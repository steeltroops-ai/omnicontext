#!/usr/bin/env bash
# OmniContext Installer - macOS / Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash

set -euo pipefail

# ---------------------------------------------------------------------------
# constants
# ---------------------------------------------------------------------------
REPO_OWNER="steeltroops-ai"
REPO_NAME="omnicontext"
BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${HOME}/.omnicontext"
MODEL_PATH="${DATA_DIR}/models/jina-embeddings-v2-base-code/model.onnx"

# ---------------------------------------------------------------------------
# color helpers (degrade gracefully when no tty)
# ---------------------------------------------------------------------------
if [ -t 1 ]; then
    BOLD=$'\033[1m';  DIM=$'\033[2m';  RESET=$'\033[0m'
    RED=$'\033[31m';  GREEN=$'\033[32m'; YELLOW=$'\033[33m'
    BLUE=$'\033[34m'; CYAN=$'\033[36m';  WHITE=$'\033[97m'
else
    BOLD=""; DIM=""; RESET=""; RED=""; GREEN=""; YELLOW=""
    BLUE=""; CYAN=""; WHITE=""
fi

step()    { printf "${BOLD}${CYAN}  [%s]${RESET} %s\n" "$1" "$2"; }
ok()      { printf "${GREEN}  [v]${RESET} %s\n" "$*"; }
info()    { printf "${BLUE}  [»]${RESET} %s\n" "$*"; }
warn()    { printf "${YELLOW}  [!]${RESET} %s\n" "$*"; }
err()     { printf "${RED}  [x]${RESET} %s\n" "$*"; }
hr()      { printf "${DIM}%s${RESET}\n" "──────────────────────────────────────────────────────"; }
blank()   { echo ""; }

die() { blank; err "$1"; blank; exit 1; }

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
printf "${DIM}  Universal Code Context Engine - macOS / Linux Installer${RESET}\n"
hr
blank

# ---------------------------------------------------------------------------
# step 1 - resolve version
# ---------------------------------------------------------------------------
step "1/7" "Resolving latest version"

VERSION=""

# Primary: GitHub Releases API (ensures we get a published version with assets)
API="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases"
if RELEASES=$(curl -sSLf "$API" 2>/dev/null); then
    # Pick first release whose assets array is non-empty
    if command -v python3 >/dev/null 2>&1; then
        VERSION=$(python3 -c "
import json, sys
releases = json.load(sys.stdin)
for r in releases:
    if r.get('assets'):
        print(r['tag_name'])
        break
" <<< "$RELEASES" 2>/dev/null || echo "")
    else
        VERSION=$(echo "$RELEASES" | grep '"tag_name":' | head -n1 | sed -E 's/.*"([^"]+)".*/\1/' || echo "")
    fi
    if [ -n "$VERSION" ]; then
        ok "Latest release with assets  ${DIM}(${VERSION})${RESET}"
    fi
fi

# Fallback: parse Cargo.toml if API limit reached
if [ -z "$VERSION" ]; then
    warn "GitHub API limit reached or network error - falling back to source"
    CARGO_URL="https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/Cargo.toml"
    if CARGO_CONTENT=$(curl -sSLf "$CARGO_URL" 2>/dev/null); then
        if SOURCE_VER=$(echo "$CARGO_CONTENT" | grep -m1 '^version' | sed -E 's/.*"([^"]+)".*/\1/' 2>/dev/null); then
            if [ -n "$SOURCE_VER" ]; then
                VERSION="v${SOURCE_VER}"
                ok "Version resolved from source  ${DIM}(${VERSION})${RESET}"
            fi
        fi
    fi
    [ -n "$VERSION" ] || die "Could not resolve a published release version."
fi

CLEAN_VERSION="${VERSION#v}"

# ---------------------------------------------------------------------------
# step 2 - detect platform
# ---------------------------------------------------------------------------
blank
step "2/7" "Detecting platform"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)  OS_NAME="unknown-linux-gnu" ;;
    Darwin) OS_NAME="apple-darwin" ;;
    *)      die "Unsupported OS: $OS" ;;
esac

case "$ARCH" in
    x86_64|amd64) ARCH_NAME="x86_64" ;;
    arm64|aarch64) ARCH_NAME="aarch64" ;;
    *) die "Unsupported architecture: $ARCH" ;;
esac

ASSET_NAME="omnicontext-${CLEAN_VERSION}-${ARCH_NAME}-${OS_NAME}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${VERSION}/${ASSET_NAME}"

ok "Platform  ${DIM}${ARCH_NAME}-${OS_NAME}${RESET}"
info "Asset    ${DIM}${ASSET_NAME}${RESET}"

# ---------------------------------------------------------------------------
# step 3 - download
# ---------------------------------------------------------------------------
blank
step "3/7" "Downloading release archive"

TEMP_DIR="$(mktemp -d)"
# Guarantee cleanup even on error
trap 'rm -rf -- "$TEMP_DIR"' EXIT

info "URL  ${DIM}${DOWNLOAD_URL}${RESET}"

if ! curl -#Lf "$DOWNLOAD_URL" -o "${TEMP_DIR}/${ASSET_NAME}" 2>&1; then
    blank
    err "Download failed."
    info "Verify release: https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/tag/${VERSION}"
    die "Aborting."
fi

ARCHIVE_SIZE=$(du -sh "${TEMP_DIR}/${ASSET_NAME}" 2>/dev/null | cut -f1 || echo "?")
ok "Downloaded ${ARCHIVE_SIZE}"

# ---------------------------------------------------------------------------
# step 4 - stop running instances
# ---------------------------------------------------------------------------
blank
step "4/7" "Stopping active processes"

STOPPED=0
for proc in omnicontext-daemon omnicontext-mcp omnicontext; do
    if pkill -x "$proc" 2>/dev/null; then
        ok "  Stopped ${proc}"
        STOPPED=$((STOPPED+1))
    fi
done
if [ "$STOPPED" -eq 0 ]; then
    info "No active OmniContext processes found"
fi
sleep 0.4

# ---------------------------------------------------------------------------
# step 5 - extract and install
# ---------------------------------------------------------------------------
blank
step "5/7" "Extracting and installing binaries"

tar -xzf "${TEMP_DIR}/${ASSET_NAME}" -C "$TEMP_DIR"

# Locate binaries - handles flat, nested, or searched layout
locate_bin() {
    local name="$1"
    # flat
    [ -f "${TEMP_DIR}/${name}" ] && { echo "${TEMP_DIR}/${name}"; return; }
    # nested dir matching archive name
    local sub="${TEMP_DIR}/${ASSET_NAME%.tar.gz}"
    [ -f "${sub}/${name}" ] && { echo "${sub}/${name}"; return; }
    # recursive search
    find "$TEMP_DIR" -name "$name" -type f | head -n1
}

BIN_OMNI=$(locate_bin "omnicontext")
BIN_MCP=$(locate_bin "omnicontext-mcp")
BIN_DAEMON=$(locate_bin "omnicontext-daemon")

[ -n "$BIN_OMNI" ]  || die "omnicontext binary not found in archive."
[ -n "$BIN_MCP" ]   || die "omnicontext-mcp binary not found in archive."

mkdir -p "$BIN_DIR"

install -m 755 "$BIN_OMNI"  "${BIN_DIR}/omnicontext"
install -m 755 "$BIN_MCP"   "${BIN_DIR}/omnicontext-mcp"
[ -n "$BIN_DAEMON" ] && [ -f "$BIN_DAEMON" ] && install -m 755 "$BIN_DAEMON" "${BIN_DIR}/omnicontext-daemon"

ok "Installed to  ${DIM}${BIN_DIR}${RESET}"

BIN_CLI_SIZE=$(du -sh "${BIN_DIR}/omnicontext" 2>/dev/null | cut -f1 || echo "?")
BIN_MCP_SIZE=$(du -sh "${BIN_DIR}/omnicontext-mcp" 2>/dev/null | cut -f1 || echo "?")
info "omnicontext          ${DIM}${BIN_CLI_SIZE}${RESET}"
info "omnicontext-mcp      ${DIM}${BIN_MCP_SIZE}${RESET}"
[ -f "${BIN_DIR}/omnicontext-daemon" ] && \
    info "omnicontext-daemon   ${DIM}$(du -sh "${BIN_DIR}/omnicontext-daemon" | cut -f1)${RESET}"

# ---------------------------------------------------------------------------
# ONNX Runtime shared library -- required for the embedding model.
# The engine links ort dynamically; the library must be co-located with the binary.
# ---------------------------------------------------------------------------

get_latest_onnx_version() {
    local version=""
    local api="https://api.github.com/repos/microsoft/onnxruntime/releases/latest"
    if res=$(curl -sSLf "$api" 2>/dev/null); then
        version=$(echo "$res" | grep '"tag_name":' | head -n1 | sed -E 's/.*"v?([^"]+)".*/\1/' || echo "")
    fi
    if [ -z "$version" ]; then
        echo "1.24.3" # 2026 Fallback
    else
        echo "$version"
    fi
}

ONNX_VERSION=$(get_latest_onnx_version)

install_onnx_runtime() {
    local dest_dir="$1"
    local version="$2"
    local onnx_tmp
    onnx_tmp="$(mktemp -d)"
    trap 'rm -rf "$onnx_tmp"' RETURN

    if [ "$(uname -s)" = "Darwin" ]; then
        if [ "$(uname -m)" = "arm64" ]; then
            local onnx_url="https://github.com/microsoft/onnxruntime/releases/download/v${version}/onnxruntime-osx-arm64-${version}.tgz"
            local lib_name="libonnxruntime.${version}.dylib"
            local lib_link="libonnxruntime.dylib"
        else
            local onnx_url="https://github.com/microsoft/onnxruntime/releases/download/v${version}/onnxruntime-osx-x86_64-${version}.tgz"
            local lib_name="libonnxruntime.${version}.dylib"
            local lib_link="libonnxruntime.dylib"
        fi
    else
        local onnx_url="https://github.com/microsoft/onnxruntime/releases/download/v${version}/onnxruntime-linux-x64-${version}.tgz"
        local lib_name="libonnxruntime.so.${version}"
        local lib_link="libonnxruntime.so"
    fi

    info "Fetching ONNX Runtime ${version} from github.com/microsoft..."
    info "URL  ${DIM}${onnx_url}${RESET}"

    if ! curl -#Lf "$onnx_url" -o "${onnx_tmp}/onnxruntime.tgz" 2>&1; then
        warn "ONNX Runtime download failed. Context injection may not work."
        return 1
    fi

    tar -xzf "${onnx_tmp}/onnxruntime.tgz" -C "$onnx_tmp"

    local lib_src
    lib_src=$(find "$onnx_tmp" -name "$lib_name" -type f | head -n1)
    if [ -z "$lib_src" ]; then
        lib_src=$(find "$onnx_tmp" -name "libonnxruntime*" -type f | head -n1)
    fi

    if [ -z "$lib_src" ]; then
        warn "ONNX Runtime library not found inside archive."
        return 1
    fi

    cp "$lib_src" "${dest_dir}/$(basename "$lib_src")"
    ln -sf "$(basename "$lib_src")" "${dest_dir}/${lib_link}" 2>/dev/null || true
    return 0
}

# Determine the expected library name for the current platform
if [ "$(uname -s)" = "Darwin" ]; then
    ONNX_LIB="${BIN_DIR}/libonnxruntime.dylib"
else
    ONNX_LIB="${BIN_DIR}/libonnxruntime.so"
fi

# Check if we already have the correct major version
HAS_CORRECT_ONNX=false
if [ -f "$ONNX_LIB" ]; then
    MAJOR="${ONNX_VERSION%%.*}"
    if ls "${BIN_DIR}"/libonnxruntime* | grep -q "${MAJOR}"; then
        HAS_CORRECT_ONNX=true
    fi
fi

if [ "$HAS_CORRECT_ONNX" = "true" ]; then
    ok "ONNX Runtime already present  ${DIM}${BIN_DIR}${RESET}"
else
    info "ONNX Runtime library missing or old -- fetching from Microsoft..."
    if install_onnx_runtime "$BIN_DIR" "$ONNX_VERSION"; then
        ok "ONNX Runtime installed (${ONNX_VERSION})"
    else
        warn "ONNX Runtime auto-install failed."
    fi
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

PATH_LINE='export PATH="$HOME/.local/bin:$PATH"'
if ! grep -qF "$PATH_LINE" "$SHELL_RC" 2>/dev/null; then
    {
        echo ""
        echo "# OmniContext"
        echo "$PATH_LINE"
    } >> "$SHELL_RC"
    ok "PATH entry added to  ${DIM}${SHELL_RC}${RESET}"
fi

# ---------------------------------------------------------------------------
# step 6 - embedding model setup
# ---------------------------------------------------------------------------
blank
step "6/7" "Embedding model setup"

HAS_SETUP=false
if "${BIN_DIR}/omnicontext" --help 2>&1 | grep -q "setup"; then
    HAS_SETUP=true
fi

if [ "$HAS_SETUP" = "true" ]; then
    # Stage 1: Query Current Status
    MODEL_READY="false"
    MODEL_NAME="jina-embeddings-v2-base-code"
    MODEL_SIZE="?"

    if status_json=$("${BIN_DIR}/omnicontext" setup model-status --json 2>/dev/null); then
        MODEL_READY=$(echo "$status_json" | grep -o '"model_ready": [a-z]*' | cut -d' ' -f2)
        MODEL_NAME=$(echo "$status_json" | grep -o '"model_name": "[^"]*"' | cut -d'"' -f4)
        MODEL_BYTES=$(echo "$status_json" | grep -o '"model_size_bytes": [0-9]*' | cut -d' ' -f2)
        if [ "$MODEL_BYTES" -gt 0 ] 2>/dev/null; then
            MODEL_SIZE="$((MODEL_BYTES / 1024 / 1024)) MB"
        fi
    fi

    if [ "$MODEL_READY" = "true" ]; then
        ok "Model ready: ${BOLD}${MODEL_NAME}${RESET} ${DIM}(${MODEL_SIZE})${RESET}"
    else
        info "Establishing model: ${BOLD}${MODEL_NAME}${RESET}"
        info "Source: HuggingFace (~550 MB)"
        blank
        
        printf "  ${DIM}────────────────────────────────────────${RESET}\n"
        "${BIN_DIR}/omnicontext" setup model-download || true
        printf "  ${DIM}────────────────────────────────────────${RESET}\n"

        # Final check
        if status_json=$("${BIN_DIR}/omnicontext" setup model-status --json 2>/dev/null); then
            MODEL_READY=$(echo "$status_json" | grep -o '"model_ready": [a-z]*' | cut -d' ' -f2)
            if [ "$MODEL_READY" = "true" ]; then
                ok "Model setup successful"
            else
                warn "Model setup finished but verification is pending."
                info "Run: ${DIM}omnicontext setup model-download${RESET}"
            fi
        fi
    fi
else
    # Fallback for older versions (v0.7.1)
    if [ -f "$HOME/.omnicontext/models/jina-embeddings-v2-base-code/model.onnx" ]; then
        ok "Model ready (cached)"
    else
        warn "Model download trigger deferred (legacy binary detected)."
        info "Use 'omnicontext index .' to initialize the model after setup."
    fi
fi

# ---------------------------------------------------------------------------
# step 7 - MCP auto-configure
# ---------------------------------------------------------------------------
blank
step "7/7" "Auto-configuring MCP for AI clients"

MCP_BIN="${BIN_DIR}/omnicontext-mcp"
# Install creates disabled placeholder entries. The VS Code extension auto-sync
# will overwrite these with correct absolute paths when the user opens a project.
# Using --repo "." is DANGEROUS: AI agent launchers spawn MCP from their own
# install directory, causing "." to resolve to the wrong path silently.
MCP_PLACEHOLDER="REPLACE_WITH_YOUR_REPO_PATH"
MCP_ENTRY="{\"command\":\"${MCP_BIN}\",\"args\":[\"--repo\",\"${MCP_PLACEHOLDER}\"],\"disabled\":true}"

_write_mcp_config() {
    local config_path="$1"
    local use_powers="$2"
    local config_dir
    config_dir="$(dirname "$config_path")"
    [ -d "$config_dir" ] || return 1

    if command -v python3 >/dev/null 2>&1; then
        python3 - "$config_path" "$use_powers" "$MCP_BIN" "$MCP_PLACEHOLDER" <<'PYEOF' 2>/dev/null && return 0
import json, sys, os

path, use_powers, mcp_bin, placeholder = sys.argv[1], sys.argv[2] == "true", sys.argv[3], sys.argv[4]
entry = {"command": mcp_bin, "args": ["--repo", placeholder], "disabled": True}

cfg = {}
if os.path.exists(path):
    try:
        with open(path) as f: cfg = json.load(f)
    except: cfg = {}

# Determine which servers dict to work with
if use_powers:
    servers = cfg.get("powers", {}).get("mcpServers", {})
else:
    servers = cfg.get("mcpServers", {})

# Clean up legacy broken entries with --repo "." and no --cwd / env
existing = servers.get("omnicontext", {})
existing_args = existing.get("args", [])
if "--repo" in existing_args:
    repo_idx = existing_args.index("--repo")
    has_cwd = "--cwd" in existing_args
    has_env = existing.get("env", {}).get("OMNICONTEXT_REPO", "")
    if repo_idx + 1 < len(existing_args) and existing_args[repo_idx + 1] == "." and not has_cwd and not has_env:
        del servers["omnicontext"]

# Only write placeholder if no existing entry with real absolute paths
has_good = False
if "omnicontext" in servers:
    ea = servers["omnicontext"].get("args", [])
    if "--repo" in ea:
        ri = ea.index("--repo")
        if ri + 1 < len(ea) and ea[ri + 1] not in (".", placeholder):
            has_good = True

if not has_good:
    if use_powers:
        cfg.setdefault("powers", {}).setdefault("mcpServers", {})["omnicontext"] = entry
    else:
        cfg.setdefault("mcpServers", {})["omnicontext"] = entry

os.makedirs(os.path.dirname(path), exist_ok=True)
with open(path, "w") as f:
    json.dump(cfg, f, indent=2)
PYEOF
    fi

    # Fallback: basic JSON write (no merging)
    mkdir -p "$config_dir"
    if [ "$use_powers" = "true" ]; then
        printf '{"powers":{"mcpServers":{"omnicontext":%s}}}\n' "$MCP_ENTRY" > "$config_path"
    else
        printf '{"mcpServers":{"omnicontext":%s}}\n' "$MCP_ENTRY" > "$config_path"
    fi
    return 0
}

if [ "$(uname -s)" = "Darwin" ]; then
    CLAUDE_CFG="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
else
    CLAUDE_CFG="$HOME/.config/claude/claude_desktop_config.json"
fi

declare -A MCP_TARGETS=(
    ["Claude Desktop"]="${CLAUDE_CFG}:false"
    ["Claude Code CLI"]="${HOME}/.claude.json:false"
    ["Cursor"]="${HOME}/.cursor/mcp.json:false"
    ["Continue.dev"]="${HOME}/.continue/config.json:false"
    ["Kiro"]="${HOME}/.kiro/settings/mcp.json:true"
    ["Windsurf"]="${HOME}/.windsurf/mcp_config.json:false"
    ["Cline"]="${HOME}/.cline/mcp_settings.json:false"
    ["RooCode"]="${HOME}/.roo-cline/mcp_settings.json:false"
    ["Trae"]="${HOME}/.trae/mcp.json:false"
    ["Antigravity"]="${HOME}/.gemini/antigravity/mcp_config.json:false"
)

MCP_CONFIGURED=""
for client in "${!MCP_TARGETS[@]}"; do
    spec="${MCP_TARGETS[$client]}"
    config_path="${spec%%:*}"
    use_powers="${spec##*:}"
    if _write_mcp_config "$config_path" "$use_powers"; then
        MCP_CONFIGURED="${MCP_CONFIGURED:+${MCP_CONFIGURED}, }${client}"
        ok "  ${client}  ${DIM}${config_path}${RESET}"
    fi
done

if [ -z "$MCP_CONFIGURED" ]; then
    warn "No AI clients detected - install Claude/Cursor/etc and re-run to auto-configure"
else
    blank
    ok "$(echo "$MCP_CONFIGURED" | tr ',' '\n' | wc -l | tr -d ' ') client(s) configured"
fi

# ---------------------------------------------------------------------------
# verification
# ---------------------------------------------------------------------------
blank
hr
ELAPSED=$SECONDS
if command -v omnicontext >/dev/null 2>&1; then
    INSTALLED_VER=$(omnicontext --version 2>/dev/null || echo "?")
    printf "${BOLD}${GREEN}  OmniContext ${CLEAN_VERSION} installed${RESET}  ${DIM}(${ELAPSED}s)${RESET}\n"
else
    printf "${BOLD}${GREEN}  OmniContext ${CLEAN_VERSION} installed${RESET}  ${DIM}(${ELAPSED}s - restart shell to apply PATH)${RESET}\n"
fi
hr
blank

printf "${BOLD}  Quick Start${RESET}\n"
printf "  cd /path/to/your/repo\n"
printf "  omnicontext index .\n"
printf "  omnicontext search \"error handling\"\n"
blank

if [ -n "$MCP_CONFIGURED" ]; then
    printf "${BOLD}  MCP${RESET}  ${DIM}placeholder added for: ${MCP_CONFIGURED}${RESET}\n"
    printf "  Install the VS Code extension for automatic project detection.\n"
    printf "  Or set --repo to your project path in each client's config.\n"
else
    printf "${BOLD}  MCP manual config${RESET}\n"
    printf "  command: %s\n" "$MCP_BIN"
    printf '  args:    ["--repo", "/path/to/repo"]\n'
fi

blank
printf "  ${DIM}Update:    re-run this script anytime${RESET}\n"
printf "  ${DIM}Docs:      https://github.com/${REPO_OWNER}/${REPO_NAME}${RESET}\n"
blank
