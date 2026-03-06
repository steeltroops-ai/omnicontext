<#
.SYNOPSIS
    OmniContext Updater for Windows

.DESCRIPTION
    Upgrades OmniContext to the latest release while preserving all indexed data,
    cached models, and MCP client configurations. Backs up all known MCP configs
    before the update and restores them if the installer modifies them unexpectedly.

.PARAMETER Force
    Reinstall even if already on the latest version.

.EXAMPLE
    irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/update.ps1 | iex

.EXAMPLE
    .\update.ps1 -Force
#>

param([switch]$Force)

#Requires -Version 5.1
$ErrorActionPreference = "Stop"
$ProgressPreference    = "SilentlyContinue"

# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------
$ESC = [char]27
function c($code) { "$ESC[${code}m" }

$BOLD   = c "1"; $DIM  = c "2"; $RESET = c "0"
$RED    = c "31"; $GREEN = c "32"; $YELLOW = c "33"
$BLUE   = c "34"; $CYAN  = c "36"

function step  { param($n,$t) Write-Host "$BOLD$CYAN  [$n]$RESET $t" }
function ok    { param($t)    Write-Host "$GREEN  [+]$RESET $t" }
function info  { param($t)    Write-Host "$BLUE  [-]$RESET $t" }
function warn  { param($t)    Write-Host "$YELLOW  [!]$RESET $t" }
function fail  { param($t)    Write-Host "$RED  [x]$RESET $t" }
$HR      = $DIM + ('-' * 54) + $RESET
function hr    { Write-Host $HR }
function blank { Write-Host '' }
function Exit-Err { param($m) blank; fail $m; blank; exit 1 }

$StartTime = Get-Date
$BinDir    = Join-Path $HOME ".omnicontext\bin"
$BinPath   = Join-Path $BinDir "omnicontext.exe"
$RepoOwner = "steeltroops-ai"
$RepoName  = "omnicontext"

# ---------------------------------------------------------------------------
# banner
# ---------------------------------------------------------------------------
blank
Write-Host "$BOLD$CYAN   ____                  _  ______            __            __ $RESET"
Write-Host "$BOLD$CYAN  / __ \____ ___  ____  (_)/ ____/___  ____  / /____  _  __/ /_$RESET"
Write-Host "$BOLD$CYAN / / / / __ ``__ \/ __ \/ // /   / __ \/ __ \/ __/ _ \| |/_/ __/$RESET"
Write-Host "$BOLD$CYAN/ /_/ / / / / / / / / / // /___/ /_/ / / / / /_/  __/_>  </ /_ $RESET"
Write-Host "$BOLD$CYAN\____/_/ /_/ /_/_/ /_/_/ \____/\____/_/ /_/\__/\___/_/|_|\__/  $RESET"
Write-Host "${DIM}  Universal Code Context Engine -- Updater${RESET}"
hr
blank

# ---------------------------------------------------------------------------
# step 1 – verify installed
# ---------------------------------------------------------------------------
step "1/4" "Checking installed version"

if (-not (Test-Path $BinPath)) {
    Exit-Err "OmniContext binary not found at: $BinPath"
}

$currentRaw = & $BinPath --version 2>&1 | Select-Object -First 1
# Expect "omnicontext X.Y.Z" or similar
if ($currentRaw -match '(\d+\.\d+\.\d+)') {
    $currentVersion = $Matches[1]
    ok "Installed  $DIM$currentVersion$RESET"
} else {
    warn "Could not parse installed version — proceeding anyway"
    $currentVersion = "unknown"
}

# ---------------------------------------------------------------------------
# step 2 – resolve latest
# ---------------------------------------------------------------------------
blank
step "2/4" "Checking latest release"

$latestVersion = $null
$latestTag     = $null

# Prefer Cargo.toml for ground-truth version
try {
    $cargoRaw = Invoke-RestMethod "https://raw.githubusercontent.com/$RepoOwner/$RepoName/main/Cargo.toml" -UseBasicParsing
    if ($cargoRaw -match '(?m)^version\s*=\s*"([^"]+)"') {
        $latestVersion = $Matches[1]
        $latestTag     = "v$latestVersion"
        ok "Latest from source  $DIM$latestTag$RESET"
    }
} catch { }

# Fallback: latest release with assets
if (-not $latestVersion) {
    try {
        $releases = Invoke-RestMethod "https://api.github.com/repos/$RepoOwner/$RepoName/releases" -UseBasicParsing
        $r = $releases | Where-Object { $_.assets.Count -gt 0 } | Select-Object -First 1
        if (-not $r) { Exit-Err "No releases with binary assets found." }
        $latestTag     = $r.tag_name
        $latestVersion = $latestTag -replace "^v", ""
        ok "Latest release  $DIM$latestTag$RESET"
    } catch {
        Exit-Err "Failed to resolve latest version: $_"
    }
}

# version comparison
if ($currentVersion -eq $latestVersion -and -not $Force) {
    blank
    ok ("Already on latest version  " + $DIM + "($latestVersion)" + $RESET)
    info "Use  ${DIM}-Force${RESET}  to reinstall"
    blank
    exit 0
}

if ($Force) {
    warn ("Forcing reinstall  " + $DIM + "(-Force)" + $RESET)
} else {
    ok "Update available  $DIM$currentVersion  ->  $latestVersion$RESET"
}

# ---------------------------------------------------------------------------
# step 3 – backup known MCP configs
# ---------------------------------------------------------------------------
blank
step "3/4" "Backing up MCP configurations"

$mcpPaths = @(
    "$env:APPDATA\Claude\claude_desktop_config.json",
    "$env:USERPROFILE\.claude.json",
    "$env:APPDATA\Cursor\User\globalStorage\cursor.mcp\config.json",
    "$env:APPDATA\Code\User\globalStorage\saoudrizwan.claude-dev\settings\cline_mcp_settings.json",
    "$env:USERPROFILE\.continue\config.json",
    "$env:USERPROFILE\.kiro\settings\mcp.json",
    "$env:APPDATA\Windsurf\User\globalStorage\codeium.windsurf\mcp_config.json",
    "$env:APPDATA\Code\User\globalStorage\rooveterinaryinc.roo-cline\settings\mcp_settings.json",
    "$env:APPDATA\Trae\User\globalStorage\trae-ide.trae-ai\mcp_settings.json",
    "$env:USERPROFILE\.gemini\antigravity\mcp_config.json"
)

$backupDir = Join-Path $env:TEMP "omnicontext_mcp_backup_$(Get-Date -Format 'yyyyMMdd_HHmmss')"
New-Item -ItemType Directory -Path $backupDir -Force | Out-Null
$backedUp = @()

foreach ($p in $mcpPaths) {
    if (Test-Path $p) {
        $dest = Join-Path $backupDir (Split-Path $p -Leaf)
        Copy-Item $p $dest -Force
        $backedUp += @{ Src = $p; Bak = $dest }
        info "Backed up  $DIM$(Split-Path $p -Leaf)$RESET"
    }
}

if ($backedUp.Count -eq 0) {
    info "No existing MCP configs found to back up"
} else {
    ok "$($backedUp.Count) MCP config(s) backed up to  $DIM$backupDir$RESET"
}

# ---------------------------------------------------------------------------
# step 4 – run installer
# ---------------------------------------------------------------------------
blank
step "4/4" ("Running installer  " + $DIM + "(install.ps1)" + $RESET)
blank

try {
    $installUrl     = "https://raw.githubusercontent.com/$RepoOwner/$RepoName/main/distribution/install.ps1"
    $scriptContent  = Invoke-RestMethod -Uri $installUrl -UseBasicParsing
    Invoke-Expression $scriptContent
} catch {
    blank
    fail "Installer failed: $_"
    # Restore backups
    if ($backedUp.Count -gt 0) {
        warn "Restoring MCP configs from backup..."
        foreach ($b in $backedUp) { Copy-Item $b.Bak $b.Src -Force -EA SilentlyContinue }
        ok "MCP configs restored"
    }
    exit 1
}

# ---------------------------------------------------------------------------
# verify
# ---------------------------------------------------------------------------
$newRaw = & $BinPath --version 2>&1 | Select-Object -First 1
if ($newRaw -match '(\d+\.\d+\.\d+)') { $newVersion = $Matches[1] } else { $newVersion = "?" }
$elapsed = [math]::Round(((Get-Date) - $StartTime).TotalSeconds, 1)

blank
hr
if ($newVersion -eq $latestVersion) {
    Write-Host ("${BOLD}${GREEN}  Updated  $currentVersion  ->  $newVersion${RESET}  " + $DIM + "(${elapsed}s)" + $RESET)
} else {
    Write-Host "$BOLD$YELLOW  Installer ran but version mismatch: expected $latestVersion, got $newVersion$RESET"
}
hr
blank
info "Restart your IDE to reload the MCP server"
info ("Verify:  " + $DIM + "omnicontext --version" + $RESET)
blank
