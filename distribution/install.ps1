<#
.SYNOPSIS
    OmniContext Installer for Windows

.DESCRIPTION
    Downloads the latest OmniContext release, installs binaries to %USERPROFILE%\.omnicontext\bin,
    adds to PATH, pre-downloads the Jina AI embedding model (~550MB), and auto-configures
    MCP for Claude, Cursor, Windsurf, Kiro, Cline, RooCode, Continue, Trae, Antigravity, and Claude Code.

.EXAMPLE
    irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex
#>

#Requires -Version 5.1
$ErrorActionPreference = "Stop"
$ProgressPreference    = "SilentlyContinue"   # suppress Invoke-WebRequest default progress bar

# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------
$ESC = [char]27
function c($code) { "$ESC[${code}m" }

$BOLD   = c "1"
$DIM    = c "2"
$RESET  = c "0"
$RED    = c "31"
$GREEN  = c "32"
$YELLOW = c "33"
$BLUE   = c "34"
$CYAN   = c "36"
$WHITE  = c "97"

function step   { param($n,$t) Write-Host "$BOLD$CYAN  [$n]$RESET $t" }
function ok     { param($t)    Write-Host "$GREEN  [+]$RESET $t" }
function info   { param($t)    Write-Host "$BLUE  [-]$RESET $t" }
function warn   { param($t)    Write-Host "$YELLOW  [!]$RESET $t" }
function fail   { param($t)    Write-Host "$RED  [x]$RESET $t" }
$HR      = $DIM + ('-' * 54) + $RESET
function hr     { Write-Host $HR }
function blank  { Write-Host '' }

function Exit-Err {
    param($msg)
    blank
    fail $msg
    blank
    exit 1
}

$StartTime = Get-Date

# ---------------------------------------------------------------------------
# banner
# ---------------------------------------------------------------------------
blank
Write-Host "$BOLD$CYAN   ____                  _  ______            __            __ $RESET"
Write-Host "$BOLD$CYAN  / __ \____ ___  ____  (_)/ ____/___  ____  / /____  _  __/ /_$RESET"
Write-Host "$BOLD$CYAN / / / / __ ``__ \/ __ \/ // /   / __ \/ __ \/ __/ _ \| |/_/ __/$RESET"
Write-Host "$BOLD$CYAN/ /_/ / / / / / / / / / // /___/ /_/ / / / / /_/  __/_>  </ /_ $RESET"
Write-Host "$BOLD$CYAN\____/_/ /_/ /_/_/ /_/_/ \____/\____/_/ /_/\__/\___/_/|_|\__/  $RESET"
Write-Host "${DIM}  Universal Code Context Engine -- Windows Installer${RESET}"
hr
blank

# ---------------------------------------------------------------------------
# arch check
# ---------------------------------------------------------------------------
if ($env:PROCESSOR_ARCHITECTURE -ne "AMD64") {
    Exit-Err "OmniContext requires Windows x64 (AMD64). Detected: $env:PROCESSOR_ARCHITECTURE"
}

# ---------------------------------------------------------------------------
# step 1 – resolve version
# ---------------------------------------------------------------------------
step "1/7" "Resolving latest version"

$RepoOwner = "steeltroops-ai"
$RepoName  = "omnicontext"
$Version   = $null

# Primary: parse Cargo.toml from main branch
try {
    $raw = Invoke-RestMethod -Uri "https://raw.githubusercontent.com/$RepoOwner/$RepoName/main/Cargo.toml" -UseBasicParsing
    if ($raw -match '(?m)^version\s*=\s*"([^"]+)"') {
        $Version = "v$($Matches[1])"
        ok ("Version resolved from source  " + $DIM + "($Version)" + $RESET)
    }
} catch { }

# Fallback: GitHub Releases API, skip releases with no assets
if (-not $Version) {
    warn "Cargo.toml fetch failed — querying GitHub Releases"
    try {
        $releases = Invoke-RestMethod "https://api.github.com/repos/$RepoOwner/$RepoName/releases" -UseBasicParsing
        $release  = $releases | Where-Object { $_.assets.Count -gt 0 } | Select-Object -First 1
        if (-not $release) { Exit-Err "No published releases with binary assets found." }
        $Version = $release.tag_name
        ok ("Latest release with assets  " + $DIM + "($Version)" + $RESET)
    } catch {
        Exit-Err "Could not resolve version: $_"
    }
}

$CleanVersion  = $Version -replace "^v", ""
$OutDir        = Join-Path $HOME ".omnicontext\bin"
$OutExe        = Join-Path $OutDir "omnicontext.exe"
$OutMcpExe     = Join-Path $OutDir "omnicontext-mcp.exe"
$OutDaemonExe  = Join-Path $OutDir "omnicontext-daemon.exe"
$AssetFileName = "omnicontext-$CleanVersion-x86_64-pc-windows-msvc.zip"
$DownloadUrl   = "https://github.com/$RepoOwner/$RepoName/releases/download/$Version/$AssetFileName"
$TempZip       = Join-Path $env:TEMP $AssetFileName
$StagingDir    = Join-Path $env:TEMP "omnicontext_staging"

# ---------------------------------------------------------------------------
# step 2 – download binary
# ---------------------------------------------------------------------------
blank
step "2/7" "Downloading  $DIM$AssetFileName$RESET"
info "URL  $DIM$DownloadUrl$RESET"

try {
    # Use WebClient for a real progress bar
    $wc = New-Object System.Net.WebClient
    $wc.DownloadFile($DownloadUrl, $TempZip)
    $sizeMb = [math]::Round((Get-Item $TempZip).Length / 1MB, 1)
    ok "Downloaded $sizeMb MB"
} catch {
    blank
    fail "Download failed: $_"
    info "URL: $DownloadUrl"
    info "Verify the release exists: https://github.com/$RepoOwner/$RepoName/releases"
    Exit-Err "Aborting."
}

# ---------------------------------------------------------------------------
# step 3 – stop running instances
# ---------------------------------------------------------------------------
blank
step "3/7" "Stopping active processes"

$procs = Get-Process -Name "omnicontext","omnicontext-mcp","omnicontext-daemon" -EA SilentlyContinue
if ($procs) {
    $procs | Stop-Process -Force -EA SilentlyContinue
    Start-Sleep -Milliseconds 600
    ok "Stopped $($procs.Count) process(es)"
} else {
    info "No active OmniContext processes found"
}

# ---------------------------------------------------------------------------
# step 4 – extract and install
# ---------------------------------------------------------------------------
blank
step "4/7" "Extracting and installing binaries"

# Clean staging
if (Test-Path $StagingDir) { Remove-Item $StagingDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $StagingDir | Out-Null
New-Item -ItemType Directory -Force -Path $OutDir      | Out-Null

try {
    Expand-Archive -Path $TempZip -DestinationPath $StagingDir -Force
    Remove-Item $TempZip -Force -EA SilentlyContinue
    Copy-Item "$StagingDir\*" -Destination $OutDir -Recurse -Force
    Remove-Item $StagingDir -Recurse -Force -EA SilentlyContinue
} catch {
    Exit-Err "Extraction failed: $_"
}

if (-not (Test-Path $OutExe))    { Exit-Err "omnicontext.exe not found after extraction." }
if (-not (Test-Path $OutMcpExe)) { Exit-Err "omnicontext-mcp.exe not found after extraction." }

ok "Binaries installed  $DIM$OutDir$RESET"
info "omnicontext.exe        $DIM$([math]::Round((Get-Item $OutExe).Length/1KB)) KB$RESET"
info "omnicontext-mcp.exe    $DIM$([math]::Round((Get-Item $OutMcpExe).Length/1KB)) KB$RESET"
if (Test-Path $OutDaemonExe) {
    info "omnicontext-daemon.exe $DIM$([math]::Round((Get-Item $OutDaemonExe).Length/1KB)) KB$RESET"
}

# ---------------------------------------------------------------------------
# step 5 – PATH
# ---------------------------------------------------------------------------
blank
step "5/7" "Configuring PATH"

$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$OutDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$UserPath;$OutDir", "User")
    $env:PATH = "$($env:PATH);$OutDir"
    ok "Added to User PATH  $DIM$OutDir$RESET"
} else {
    ok "Already in PATH  $DIM$OutDir$RESET"
}

# ---------------------------------------------------------------------------
# step 6 – download embedding model
# ---------------------------------------------------------------------------
blank
step "6/7" ("Downloading Jina AI embedding model  " + $DIM + "(~550 MB, one-time)" + $RESET)

$ModelPath = Join-Path $HOME ".omnicontext\models\jina-embeddings-v2-base-code.onnx"

if (Test-Path $ModelPath) {
    $modelSizeMb = [math]::Round((Get-Item $ModelPath).Length / 1MB, 0)
    ok "Model already cached  $DIM${modelSizeMb} MB$RESET"
} else {
    info "Triggering model download via  $DIM\`omnicontext index\`$RESET"
    info "This may take several minutes on first run..."
    blank
    try {
        $InitTemp = Join-Path $env:TEMP "omnicontext_init_$$"
        New-Item -ItemType Directory -Path $InitTemp -Force | Out-Null
        "fn main() {}" | Out-File "$InitTemp\dummy.rs" -Encoding UTF8
        Push-Location $InitTemp
        & $OutExe index . 2>&1
        Pop-Location
        Remove-Item $InitTemp -Recurse -Force -EA SilentlyContinue
    } catch {
        warn "Model download may have been interrupted."
        info "Trigger manually later: $DIM omnicontext index .$RESET"
    }

    if (Test-Path $ModelPath) {
        $modelSizeMb = [math]::Round((Get-Item $ModelPath).Length / 1MB, 0)
        ok "Model downloaded  $DIM${modelSizeMb} MB$RESET"
    } else {
        warn "Model not found — will auto-download on first use"
    }
}

# ---------------------------------------------------------------------------
# step 7 – MCP auto-configure
# ---------------------------------------------------------------------------
blank
step "7/7" "Auto-configuring MCP for AI clients"

$McpEntry = [ordered]@{
    command  = $OutMcpExe
    args     = @("--repo", ".")
    disabled = $false
}

function Set-McpConfig {
    param($Name, $Path, [bool]$Powers)
    $dir = Split-Path $Path -Parent
    if (-not (Test-Path $dir)) { return $false }
    try {
        $cfg = @{}
        if (Test-Path $Path) {
            $raw = Get-Content $Path -Raw -EA SilentlyContinue
            if ($raw) { $cfg = $raw | ConvertFrom-Json -AsHashtable -EA SilentlyContinue }
            if (-not $cfg) { $cfg = @{} }
        }
        if ($Powers) {
            if (-not $cfg["powers"])                          { $cfg["powers"] = @{} }
            if (-not $cfg["powers"]["mcpServers"])            { $cfg["powers"]["mcpServers"] = @{} }
            $cfg["powers"]["mcpServers"]["omnicontext"] = $McpEntry
        } else {
            if (-not $cfg["mcpServers"]) { $cfg["mcpServers"] = @{} }
            $cfg["mcpServers"]["omnicontext"] = $McpEntry
        }
        $cfg | ConvertTo-Json -Depth 10 | Set-Content $Path -Encoding UTF8
        return $true
    } catch { return $false }
}

$clients = @(
    @{ Name = "Claude Desktop"; Path = "$env:APPDATA\Claude\claude_desktop_config.json";                                    Powers = $false },
    @{ Name = "Claude Code CLI";Path = "$env:USERPROFILE\.claude.json";                                                      Powers = $false },
    @{ Name = "Cursor";         Path = "$env:APPDATA\Cursor\User\globalStorage\cursor.mcp\config.json";                     Powers = $false },
    @{ Name = "VS Code (Cline)";Path = "$env:APPDATA\Code\User\globalStorage\saoudrizwan.claude-dev\settings\cline_mcp_settings.json"; Powers = $false },
    @{ Name = "RooCode";        Path = "$env:APPDATA\Code\User\globalStorage\rooveterinaryinc.roo-cline\settings\mcp_settings.json"; Powers = $false },
    @{ Name = "Continue.dev";   Path = "$env:USERPROFILE\.continue\config.json";                                            Powers = $false },
    @{ Name = "Kiro";           Path = "$env:USERPROFILE\.kiro\settings\mcp.json";                                          Powers = $true  },
    @{ Name = "Windsurf";       Path = "$env:APPDATA\Windsurf\User\globalStorage\codeium.windsurf\mcp_config.json";         Powers = $false },
    @{ Name = "Trae";           Path = "$env:APPDATA\Trae\User\globalStorage\trae-ide.trae-ai\mcp_settings.json";           Powers = $false },
    @{ Name = "Antigravity";    Path = "$env:USERPROFILE\.gemini\antigravity\mcp_config.json";                              Powers = $false }
)

$configured = @()
foreach ($c in $clients) {
    if (Set-McpConfig -Name $c.Name -Path $c.Path -Powers $c.Powers) {
        $configured += $c.Name
        ok "  $($c.Name)  $DIM$($c.Path)$RESET"
    }
}

if ($configured.Count -eq 0) {
    warn "No AI clients detected — install Claude/Cursor/etc and re-run to auto-configure"
} else {
    blank
    ok "$($configured.Count) client(s) configured"
}

# ---------------------------------------------------------------------------
# summary
# ---------------------------------------------------------------------------
$elapsed = [math]::Round(((Get-Date) - $StartTime).TotalSeconds, 1)
blank
hr
Write-Host ("${BOLD}${GREEN}  OmniContext ${CleanVersion} installed${RESET}  " + $DIM + "(${elapsed}s)" + $RESET)
hr
blank
Write-Host "$BOLD  Quick Start$RESET"
Write-Host "  cd C:\path\to\your\repo"
Write-Host "  omnicontext index ."
Write-Host '  omnicontext search "error handling"'
blank
if ($configured.Count -gt 0) {
    Write-Host "$BOLD  MCP$RESET  $DIM auto-configured for: $($configured -join ', ')$RESET"
    Write-Host "  Default --repo is '.' (cwd). Edit config files for project-specific paths."
} else {
    Write-Host "$BOLD  MCP manual config$RESET"
    Write-Host "  command: $OutMcpExe"
    Write-Host '  args:    ["--repo", "C:\path\to\repo"]'
}
blank
Write-Host "  ${DIM}Update: re-run this script anytime${RESET}"
Write-Host "  ${DIM}Restart your terminal to apply PATH changes${RESET}"
blank
