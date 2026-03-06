#!/usr/bin/env bash
# OmniContext Uninstaller - macOS / Linux
# Usage: bash uninstall.sh [--keep-data] [--keep-config] [--silent]
#        curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/uninstall.sh | bash

set -euo pipefail

# ---------------------------------------------------------------------------
# parse args
# ---------------------------------------------------------------------------
KEEP_DATA=false
KEEP_CONFIG=false
SILENT=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --keep-data)   KEEP_DATA=true;   shift ;;
        --keep-config) KEEP_CONFIG=true; shift ;;
        --silent|-y)   SILENT=true;      shift ;;
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
ok()    { printf "${GREEN}  [+]${RESET} %s\n" "$*"; }
info()  { printf "${BLUE}  [-]${RESET} %s\n" "$*"; }
warn()  { printf "${YELLOW}  [!]${RESET} %s\n" "$*"; }
err()   { printf "${RED}  [x]${RESET} %s\n" "$*"; }
hr()    { printf "${DIM}%s${RESET}\n" "──────────────────────────────────────────────────────"; }
blank() { echo ""; }
die()   { blank; err "$1"; blank; exit 1; }

SECONDS=0
BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${HOME}/.omnicontext"

# ---------------------------------------------------------------------------
# banner
# ---------------------------------------------------------------------------
blank
printf "${BOLD}${RED}"
cat <<'EOF'
   ____                  _  ______            __            __ 
  / __ \____ ___  ____  (_)/ ____/___  ____  / /____  _  __/ /_
 / / / / __ `__ \/ __ \/ // /   / __ \/ __ \/ __/ _ \| |/_/ __/
/ /_/ / / / / / / / / / // /___/ /_/ / / / / /_/  __/_>  </ /_ 
\____/_/ /_/ /_/_/ /_/_/ \____/\____/_/ /_/\__/\___/_/|_|\__/  
EOF
printf "${RESET}"
printf "${DIM}  Universal Code Context Engine - Uninstaller${RESET}\n"
hr
blank

# ---------------------------------------------------------------------------
# confirm
# ---------------------------------------------------------------------------
warn "This will remove OmniContext from your system."
[ "$KEEP_DATA"   = false ] && warn "Indexed data and AI models (~600 MB+) will be deleted."
[ "$KEEP_CONFIG" = false ] && warn "MCP entries will be removed from all AI client configs."
blank

if [ "$SILENT" = false ]; then
    read -r -p "  Proceed with uninstallation? [y/N] " response
    if [[ ! "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
        info "Uninstallation cancelled."
        blank
        exit 0
    fi
fi

blank

# ---------------------------------------------------------------------------
# step 1 - stop processes
# ---------------------------------------------------------------------------
step "1/4" "Stopping active processes"

STOPPED=0
for proc in omnicontext-daemon omnicontext-mcp omnicontext; do
    if pkill -x "$proc" 2>/dev/null; then
        ok "  Stopped ${proc}"
        STOPPED=$((STOPPED + 1))
    fi
done
[ "$STOPPED" -eq 0 ] && info "No active OmniContext processes found"
sleep 0.3

# ---------------------------------------------------------------------------
# step 2 - remove binaries
# ---------------------------------------------------------------------------
blank
step "2/4" "Removing binaries"

REMOVED_BINS=0
for bin in omnicontext omnicontext-mcp omnicontext-daemon; do
    target="${BIN_DIR}/${bin}"
    if [ -f "$target" ]; then
        rm -f "$target"
        info "Removed  ${DIM}${target}${RESET}"
        REMOVED_BINS=$((REMOVED_BINS + 1))
    fi
done

if [ "$REMOVED_BINS" -gt 0 ]; then
    ok "${REMOVED_BINS} binary/binaries removed from  ${DIM}${BIN_DIR}${RESET}"
else
    info "No binaries found in  ${DIM}${BIN_DIR}${RESET}"
fi

# Remove PATH line from shell RC files
PATH_LINE='export PATH="$HOME/.local/bin:$PATH"'
for rc in "${HOME}/.bashrc" "${HOME}/.zshrc" "${HOME}/.profile"; do
    if [ -f "$rc" ] && grep -qF "$PATH_LINE" "$rc"; then
        # Use portable in-place removal
        TMP="${rc}.omni_bak"
        grep -vF "$PATH_LINE" "$rc" > "$TMP" && mv "$TMP" "$rc"
        # Also remove "# OmniContext" comment line above it if present
        if grep -q "^# OmniContext$" "$rc" 2>/dev/null; then
            grep -v "^# OmniContext$" "$rc" > "$TMP" && mv "$TMP" "$rc"
        fi
        ok "Removed PATH entry from  ${DIM}$(basename "$rc")${RESET}"
    fi
done

# ---------------------------------------------------------------------------
# step 3 - remove data
# ---------------------------------------------------------------------------
blank
step "3/4" "Removing data and models"

if [ "$KEEP_DATA" = false ]; then
    if [ -d "$DATA_DIR" ]; then
        DATA_SIZE=$(du -sh "$DATA_DIR" 2>/dev/null | cut -f1 || echo "?")
        rm -rf "$DATA_DIR"
        ok "Data directory removed  ${DIM}${DATA_DIR}  (${DATA_SIZE})${RESET}"
    else
        info "Data directory not found  ${DIM}${DATA_DIR}${RESET}"
    fi
else
    ok "Data preserved  ${DIM}(--keep-data)${RESET}"
fi

# ---------------------------------------------------------------------------
# step 4 - remove MCP entries from all known clients
# ---------------------------------------------------------------------------
blank
step "4/4" "Unlinking MCP configurations"

if [ "$KEEP_CONFIG" = false ]; then

    _remove_mcp_entry() {
        local config_path="$1"
        local use_powers="$2"

        [ -f "$config_path" ] || return 0

        if command -v python3 >/dev/null 2>&1; then
            python3 - "$config_path" "$use_powers" <<'PYEOF' 2>/dev/null && return 0
import json, sys, os

path, use_powers = sys.argv[1], sys.argv[2] == "true"
modified = False

try:
    with open(path) as f: cfg = json.load(f)
except: sys.exit(0)

try:
    if use_powers:
        servers = cfg.get("powers", {}).get("mcpServers", {})
        if "omnicontext" in servers:
            del cfg["powers"]["mcpServers"]["omnicontext"]
            modified = True
    else:
        servers = cfg.get("mcpServers", {})
        if "omnicontext" in servers:
            del cfg["mcpServers"]["omnicontext"]
            modified = True
except: pass

if modified:
    with open(path, "w") as f:
        json.dump(cfg, f, indent=2)
    print("removed")
PYEOF
        fi
    }

    if [ "$(uname -s)" = "Darwin" ]; then
        CLAUDE_CFG="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
    else
        CLAUDE_CFG="$HOME/.config/claude/claude_desktop_config.json"
    fi

    _unlink() {
        local name="$1"
        local path="$2"
        local powers="$3"
        if [ -f "$path" ]; then
            result=$(_remove_mcp_entry "$path" "$powers" || echo "")
            if echo "$result" | grep -q "removed"; then
                ok "  Unlinked  ${DIM}${name}${RESET}"
            else
                info "  Not configured  ${DIM}${name}${RESET}"
            fi
        fi
    }

    _unlink "Claude Desktop"  "$CLAUDE_CFG" "false"
    _unlink "Claude Code CLI" "${HOME}/.claude.json" "false"
    _unlink "Cursor"          "${HOME}/.cursor/mcp.json" "false"
    _unlink "Continue.dev"    "${HOME}/.continue/config.json" "false"
    _unlink "Kiro"            "${HOME}/.kiro/settings/mcp.json" "true"
    _unlink "Windsurf"        "${HOME}/.windsurf/mcp_config.json" "false"
    _unlink "Cline"           "${HOME}/.cline/mcp_settings.json" "false"
    _unlink "RooCode"         "${HOME}/.roo-cline/mcp_settings.json" "false"
    _unlink "Trae"            "${HOME}/.trae/mcp.json" "false"
    _unlink "Antigravity"     "${HOME}/.gemini/antigravity/mcp_config.json" "false"

else
    ok "MCP configurations preserved  ${DIM}(--keep-config)${RESET}"
fi

# ---------------------------------------------------------------------------
# summary
# ---------------------------------------------------------------------------
ELAPSED=$SECONDS
blank
hr
printf "${BOLD}${GREEN}  OmniContext removed${RESET}  ${DIM}(${ELAPSED}s)${RESET}\n"
hr
blank

[ "$KEEP_DATA"   = true ] && info "Data preserved at  ${DIM}${DATA_DIR}${RESET}"
[ "$KEEP_CONFIG" = true ] && info "MCP configurations untouched"

blank
printf "  ${DIM}To reinstall:${RESET}\n"
printf "  curl -fsSL https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.sh | bash\n"
blank
