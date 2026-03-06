#!/usr/bin/env bash
# OmniContext Installer — macOS / Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash

set -euo pipefail

# ---------------------------------------------------------------------------
# constants
# ---------------------------------------------------------------------------
REPO_OWNER="steeltroops-ai"
REPO_NAME="omnicontext"
BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${HOME}/.omnicontext"
MODEL_PATH="${DATA_DIR}/models/jina-embeddings-v2-base-code.onnx"

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
ok()      { printf "${GREEN}  [+]${RESET} %s\n" "$*"; }
info()    { printf "${BLUE}  [-]${RESET} %s\n" "$*"; }
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
printf "${DIM}  Universal Code Context Engine — macOS / Linux Installer${RESET}\n"
hr
blank

# ---------------------------------------------------------------------------
# step 1 — resolve version
# ---------------------------------------------------------------------------
step "1/7" "Resolving latest version"

VERSION=""

# Primary: parse Cargo.toml
CARGO_URL="https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/Cargo.toml"
if CARGO_CONTENT=$(curl -sSLf "$CARGO_URL" 2>/dev/null); then
    if SOURCE_VER=$(echo "$CARGO_CONTENT" | grep -m1 '^version' | sed -E 's/.*"([^"]+)".*/\1/' 2>/dev/null); then
        if [ -n "$SOURCE_VER" ]; then
            VERSION="v${SOURCE_VER}"
            ok "Version resolved from source  ${DIM}(${VERSION})${RESET}"
        fi
    fi
fi

# Fallback: GitHub Releases API (skip empty-asset releases)
if [ -z "$VERSION" ]; then
    warn "Cargo.toml fetch failed — querying GitHub Releases API"
    API="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases"
    if RELEASES=$(curl -sSLf "$API" 2>/dev/null); then
        # Pick first release whose assets array is non-empty
        # Use python3 if available for reliable JSON parsing
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
    fi
    [ -n "$VERSION" ] || die "Could not resolve a published release version."
    ok "Latest release with assets  ${DIM}(${VERSION})${RESET}"
fi

CLEAN_VERSION="${VERSION#v}"

# ---------------------------------------------------------------------------
# step 2 — detect platform
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
# step 3 — download
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
# step 4 — stop running instances
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
# step 5 — extract and install
# ---------------------------------------------------------------------------
blank
step "5/7" "Extracting and installing binaries"

tar -xzf "${TEMP_DIR}/${ASSET_NAME}" -C "$TEMP_DIR"

# Locate binaries — handles flat, nested, or searched layout
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
# step 6 — download embedding model
# ---------------------------------------------------------------------------
blank
step "6/7" "Embedding model  ${DIM}(jina-embeddings-v2-base-code, ~550 MB)${RESET}"

if [ -f "$MODEL_PATH" ]; then
    MODEL_SIZE=$(du -sh "$MODEL_PATH" 2>/dev/null | cut -f1 || echo "?")
    ok "Model already cached  ${DIM}${MODEL_SIZE}${RESET}"
else
    info "Triggering download via  ${DIM}omnicontext index${RESET}"
    info "This may take several minutes on a slow connection..."
    blank
    INIT_TMP="$(mktemp -d)"
    echo "fn main() {}" > "${INIT_TMP}/dummy.rs"
    (cd "$INIT_TMP" && "${BIN_DIR}/omnicontext" index . 2>&1) || true
    rm -rf "$INIT_TMP"

    if [ -f "$MODEL_PATH" ]; then
        MODEL_SIZE=$(du -sh "$MODEL_PATH" 2>/dev/null | cut -f1 || echo "?")
        ok "Model downloaded  ${DIM}${MODEL_SIZE}${RESET}"
    else
        warn "Model not found — will auto-download on first  ${DIM}omnicontext index${RESET}"
    fi
fi

# ---------------------------------------------------------------------------
# step 7 — MCP auto-configure
# ---------------------------------------------------------------------------
blank
step "7/7" "Auto-configuring MCP for AI clients"

MCP_BIN="${BIN_DIR}/omnicontext-mcp"
MCP_ENTRY="{\"command\":\"${MCP_BIN}\",\"args\":[\"--repo\",\".\"],\"disabled\":false}"

_write_mcp_config() {
    local config_path="$1"
    local use_powers="$2"
    local config_dir
    config_dir="$(dirname "$config_path")"
    [ -d "$config_dir" ] || return 1

    if command -v python3 >/dev/null 2>&1; then
        python3 - "$config_path" "$use_powers" "$MCP_BIN" <<'PYEOF' 2>/dev/null && return 0
import json, sys, os

path, use_powers, mcp_bin = sys.argv[1], sys.argv[2] == "true", sys.argv[3]
entry = {"command": mcp_bin, "args": ["--repo", "."], "disabled": False}

cfg = {}
if os.path.exists(path):
    try:
        with open(path) as f: cfg = json.load(f)
    except: cfg = {}

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
    warn "No AI clients detected — install Claude/Cursor/etc and re-run to auto-configure"
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
    printf "${BOLD}${GREEN}  OmniContext ${CLEAN_VERSION} installed${RESET}  ${DIM}(${ELAPSED}s — restart shell to apply PATH)${RESET}\n"
fi
hr
blank

printf "${BOLD}  Quick Start${RESET}\n"
printf "  cd /path/to/your/repo\n"
printf "  omnicontext index .\n"
printf "  omnicontext search \"error handling\"\n"
blank

if [ -n "$MCP_CONFIGURED" ]; then
    printf "${BOLD}  MCP${RESET}  ${DIM}auto-configured for: ${MCP_CONFIGURED}${RESET}\n"
    printf "  Default --repo is '.' (cwd). Edit config files for project-specific paths.\n"
else
    printf "${BOLD}  MCP manual config${RESET}\n"
    printf "  command: %s\n" "$MCP_BIN"
    printf '  args:    ["--repo", "/path/to/repo"]\n'
fi

blank
printf "  ${DIM}Update:    re-run this script anytime${RESET}\n"
printf "  ${DIM}Docs:      https://github.com/${REPO_OWNER}/${REPO_NAME}${RESET}\n"
blank
