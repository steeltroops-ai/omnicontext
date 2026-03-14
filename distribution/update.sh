#!/usr/bin/env bash
# OmniContext Updater - macOS / Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/update.sh | bash
#        bash update.sh [--force]

set -euo pipefail

# ---------------------------------------------------------------------------
# parse args
# ---------------------------------------------------------------------------
FORCE=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --force|-f) FORCE=true; shift ;;
        *) shift ;;
    esac
done

# ---------------------------------------------------------------------------
# color helpers
# ---------------------------------------------------------------------------
if [ -t 1 ]; then
    BOLD=$'\033[1m'; DIM=$'\033[2m'; RESET=$'\033[0m'
    RED=$'\033[31m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'
    BLUE=$'\033[34m'; CYAN=$'\033[36m'
else
    BOLD=""; DIM=""; RESET=""; RED=""; GREEN=""; YELLOW=""; BLUE=""; CYAN=""
fi

step()  { printf "${BOLD}${CYAN}  [%s]${RESET} %s\n" "$1" "$2"; }
ok()    { printf "${GREEN}  [v]${RESET} %s\n" "$*"; }
info()  { printf "${BLUE}  [»]${RESET} %s\n" "$*"; }
warn()  { printf "${YELLOW}  [!]${RESET} %s\n" "$*"; }
err()   { printf "${RED}  [x]${RESET} %s\n" "$*"; }
hr()    { printf "${DIM}%s${RESET}\n" "──────────────────────────────────────────────────────"; }
blank() { echo ""; }
die()   { blank; err "$1"; blank; exit 1; }

SECONDS=0
REPO_OWNER="steeltroops-ai"
REPO_NAME="omnicontext"
BIN_DIR="${HOME}/.local/bin"
BIN_PATH="${BIN_DIR}/omnicontext"

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
printf "${DIM}  Universal Code Context Engine - Updater${RESET}\n"
hr
blank

# ---------------------------------------------------------------------------
# step 1 - verify installation
# ---------------------------------------------------------------------------
step "1/4" "Checking installed version"

if [ ! -x "$BIN_PATH" ]; then
    err "OmniContext not found at ${BIN_PATH}"
    info "Run the installer first:"
    info "  curl -fsSL https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/distribution/install.sh | bash"
    exit 1
fi

CURRENT_RAW=$("$BIN_PATH" --version 2>/dev/null || echo "")
if echo "$CURRENT_RAW" | grep -qE '[0-9]+\.[0-9]+\.[0-9]+'; then
    CURRENT_VERSION=$(echo "$CURRENT_RAW" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n1)
    ok "Installed  ${DIM}${CURRENT_VERSION}${RESET}"
else
    warn "Could not parse installed version - proceeding anyway"
    CURRENT_VERSION="unknown"
fi

# ---------------------------------------------------------------------------
# step 2 - resolve latest version
# ---------------------------------------------------------------------------
blank
step "2/4" "Checking latest release"

LATEST_VERSION=""
LATEST_TAG=""

# Primary: GitHub Releases (ensures we get a published version with assets)
RELEASES=$(curl -sSLf "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases" 2>/dev/null || echo "[]")
if command -v python3 >/dev/null 2>&1; then
    LATEST_TAG=$(python3 -c "
import json, sys
releases = json.loads(sys.stdin.read())
for r in releases:
    if r.get('assets'):
        print(r['tag_name'])
        break
" <<< "$RELEASES" 2>/dev/null || echo "")
else
    LATEST_TAG=$(echo "$RELEASES" | grep '"tag_name":' | head -n1 | sed -E 's/.*"([^"]+)".*/\1/' || echo "")
fi

if [ -n "$LATEST_TAG" ]; then
    LATEST_VERSION="${LATEST_TAG#v}"
    ok "Latest release with assets  ${DIM}${LATEST_TAG}${RESET}"
fi

# Fallback: Cargo.toml
if [ -z "$LATEST_VERSION" ]; then
    warn "GitHub API limit reached or network error - falling back to source"
    CARGO_URL="https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/Cargo.toml"
    if CARGO=$(curl -sSLf "$CARGO_URL" 2>/dev/null); then
        VER=$(echo "$CARGO" | grep -m1 '^version' | sed -E 's/.*"([^"]+)".*/\1/' 2>/dev/null || echo "")
        if [ -n "$VER" ]; then
            LATEST_VERSION="$VER"
            LATEST_TAG="v${VER}"
            ok "Latest from source  ${DIM}${LATEST_TAG}${RESET}"
        fi
    fi
    [ -n "$LATEST_VERSION" ] || die "Could not resolve latest version."
fi

# ---------------------------------------------------------------------------
# Version Resolution (Dynamic)
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

# Compare
if [ "$CURRENT_VERSION" = "$LATEST_VERSION" ] && [ "$FORCE" = false ]; then
    blank
    ok "Already on latest version  ${DIM}(${LATEST_VERSION})${RESET}"
    info "Use --force to reinstall"
    blank
    exit 0
fi

if [ "$FORCE" = true ]; then
    warn "Forcing reinstall  ${DIM}(--force)${RESET}"
else
    ok "Update available  ${DIM}${CURRENT_VERSION}  ->  ${LATEST_VERSION}${RESET}"
fi


# ---------------------------------------------------------------------------
# step 2.5 - verify model status
# ---------------------------------------------------------------------------
blank
step "2.5/4" "Verifying embedding model"

# Check if binary supports setup command
if "$BIN_PATH" --help 2>&1 | grep -q "setup"; then
    if status_json=$("$BIN_PATH" setup model-status --json 2>/dev/null); then
        MODEL_READY=$(echo "$status_json" | grep -o '"model_ready": [a-z]*' | cut -d' ' -f2)
        MODEL_NAME=$(echo "$status_json" | grep -o '"model_name": "[^"]*"' | cut -d'"' -f4)
        MODEL_BYTES=$(echo "$status_json" | grep -o '"model_size_bytes": [0-9]*' | cut -d' ' -f2)
        
        if [ "$MODEL_READY" = "true" ]; then
            SIZE_MB=$((MODEL_BYTES / 1024 / 1024))
            ok "Model ready: ${BOLD}${MODEL_NAME}${RESET} ${DIM}(${SIZE_MB} MB)${RESET}"
        else
            warn "Model not ready - will be initialized during update"
        fi
    else
        warn "Could not verify model status via binary"
    fi
else
    # Legacy check — accept both CodeRankEmbed and the old jina path
    if [ -f "$HOME/.omnicontext/models/CodeRankEmbed/model.onnx" ] || \
       [ -f "$HOME/.omnicontext/models/jina-embeddings-v2-base-code/model.onnx" ]; then
        ok "Model already cached"
    else
        warn "Model not found - will be re-downloaded during installation"
    fi
fi

# ---------------------------------------------------------------------------
# step 3 - backup MCP configs
# ---------------------------------------------------------------------------
blank
step "3/4" "Backing up MCP configurations"

BACKUP_DIR="/tmp/omnicontext_mcp_backup_$$"
mkdir -p "$BACKUP_DIR"

if [ "$(uname -s)" = "Darwin" ]; then
    CLAUDE_CFG="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
else
    CLAUDE_CFG="$HOME/.config/claude/claude_desktop_config.json"
fi

MCP_PATHS=(
    "$CLAUDE_CFG"
    "${HOME}/.claude.json"
    "${HOME}/.cursor/mcp.json"
    "${HOME}/.continue/config.json"
    "${HOME}/.kiro/settings/mcp.json"
    "${HOME}/.codeium/windsurf/mcp_config.json"
    "${HOME}/.config/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json"
    "${HOME}/.config/Code/User/globalStorage/rooveterinaryinc.roo-cline/settings/mcp_settings.json"
    "${HOME}/.gemini/settings.json"
    "${HOME}/.aws/amazonq/mcp.json"
    "${HOME}/.config/Trae/mcp_config.json"
    "${HOME}/.config/Antigravity/User/mcp.json"
    "${HOME}/.config/zed/settings.json"
    "${HOME}/.config/PearAI/User/mcp.json"
)

BACKED_UP=0
for cfg in "${MCP_PATHS[@]}"; do
    if [ -f "$cfg" ]; then
        cp "$cfg" "${BACKUP_DIR}/$(basename "$cfg").${$}.bak"
        info "Backed up  ${DIM}$(basename "$cfg")${RESET}"
        BACKED_UP=$((BACKED_UP + 1))
    fi
done

if [ "$BACKED_UP" -eq 0 ]; then
    info "No existing MCP config files found"
else
    ok "${BACKED_UP} config(s) backed up to  ${DIM}${BACKUP_DIR}${RESET}"
fi

# ---------------------------------------------------------------------------
# step 4 - run installer
# ---------------------------------------------------------------------------
blank
step "4/4" "Running installer"
blank

INSTALL_SUCCESS=false
if [ -f "./install.sh" ]; then
    info "Running local installer  ${DIM}(./install.sh)${RESET}"
    bash ./install.sh && INSTALL_SUCCESS=true
else
    INSTALL_URL="https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/distribution/install.sh"
    bash <(curl -fsSL "$INSTALL_URL") && INSTALL_SUCCESS=true
fi

if [ "$INSTALL_SUCCESS" = false ]; then
    blank
    err "Installer failed."
    if [ "$BACKED_UP" -gt 0 ]; then
        warn "Restoring MCP configs from backup..."
        for cfg in "${MCP_PATHS[@]}"; do
            bak="${BACKUP_DIR}/$(basename "$cfg").${$}.bak"
            if [ -f "$bak" ]; then
                cp "$bak" "$cfg"
                ok "  Restored $(basename "$cfg")"
            fi
        done
    fi
    die "Update aborted."
fi

# ---------------------------------------------------------------------------
# verify
# ---------------------------------------------------------------------------
NEW_RAW=$("$BIN_PATH" --version 2>/dev/null || echo "")
NEW_VERSION=$(echo "$NEW_RAW" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n1 || echo "?")
ELAPSED=$SECONDS

blank
hr
if [ "$NEW_VERSION" = "$LATEST_VERSION" ]; then
    printf "${BOLD}${GREEN}  Updated  ${CURRENT_VERSION}  ->  ${NEW_VERSION}${RESET}  ${DIM}(${ELAPSED}s)${RESET}\n"
else
    printf "${BOLD}${YELLOW}  Installer ran - version ${NEW_VERSION} (expected ${LATEST_VERSION})${RESET}\n"
fi
hr
blank
info "Restart your IDE to reload the MCP server"
info "Verify:  ${DIM}omnicontext --version${RESET}"
blank
