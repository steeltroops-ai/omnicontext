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
$ProgressPreference    = "SilentlyContinue"

# Enable TLS 1.2 for secure downloads (GitHub)
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

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
function ok     { param($t)    Write-Host "$GREEN  [v]$RESET $t" }
function info   { param($t)    Write-Host "$BLUE  [»]$RESET $t" }
function warn   { param($t)    Write-Host "$YELLOW  [!] $RESET $t" }
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

# Primary: GitHub Releases API, ensures binary assets exist
try {
    $releases = Invoke-RestMethod "https://api.github.com/repos/$RepoOwner/$RepoName/releases" -UseBasicParsing
    $release  = $releases | Where-Object { $_.assets.Count -gt 0 } | Select-Object -First 1
    if ($release) {
        $Version = $release.tag_name
        ok ("Latest release with assets  " + $DIM + "($Version)" + $RESET)
    }
} catch { }

# Fallback: parse Cargo.toml from main branch if API failed
if (-not $Version) {
    warn "GitHub API limit reached or network error - falling back to source"
    try {
        $raw = Invoke-RestMethod -Uri "https://raw.githubusercontent.com/$RepoOwner/$RepoName/main/Cargo.toml" -UseBasicParsing
        if ($raw -match '(?m)^version\s*=\s*"([^"]+)"') {
            $Version = "v$($Matches[1])"
            ok ("Version resolved from source  " + $DIM + "($Version)" + $RESET)
        }
    } catch {
        Exit-Err "Could not resolve version."
    }
    
    if (-not $Version) { Exit-Err "No published releases with binary assets found." }
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
    # Invoke-WebRequest is more reliable for following redirects and handling TLS than WebClient.DownloadFile
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempZip -UseBasicParsing
    
    if (Test-Path $TempZip) {
        $sizeMb = [math]::Round((Get-Item $TempZip).Length / 1MB, 1)
        ok "Downloaded  $DIM$sizeMb MB$RESET"
    } else {
        Exit-Err "Download succeeded but file not found at $TempZip"
    }
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
    # Suppress verbose expansion output
    $null = Expand-Archive -Path $TempZip -DestinationPath $StagingDir -Force
    Remove-Item $TempZip -Force -EA SilentlyContinue
    Copy-Item "$StagingDir\*" -Destination $OutDir -Recurse -Force
    Remove-Item $StagingDir -Recurse -Force -EA SilentlyContinue
} catch {
    Exit-Err "Extraction failed: $_"
}

if (-not (Test-Path $OutExe))    { Exit-Err "omnicontext.exe not found after extraction." }
if (-not (Test-Path $OutMcpExe)) { Exit-Err "omnicontext-mcp.exe not found after extraction." }

# ---------------------------------------------------------------------------
# ONNX Runtime DLL -- required on Windows for the embedding model.
# The engine binary links ort dynamically so the DLL must be co-located.
# We download it automatically from Microsoft's official GitHub releases.
# ---------------------------------------------------------------------------

function Get-LatestOnnxVersion {
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/microsoft/onnxruntime/releases/latest" -UseBasicParsing
        return $release.tag_name.TrimStart('v')
    } catch {
        return "1.24.3" # 2026 Fallback
    }
}

$OnnxVersion = Get-LatestOnnxVersion 
$dllPath     = Join-Path $OutDir "onnxruntime.dll"

function Install-OnnxRuntime {
    param([string]$DestDir, [string]$Version)

    $onnxUrl  = "https://github.com/microsoft/onnxruntime/releases/download/v$Version/onnxruntime-win-x64-$Version.zip"
    $onnxZip  = Join-Path $env:TEMP "onnxruntime-$Version.zip"
    $onnxStag = Join-Path $env:TEMP "onnxruntime-$Version-staging"

    info "Fetching ONNX Runtime $Version from github.com/microsoft..."
    info "URL  $DIM$onnxUrl$RESET"

    try {
        Invoke-WebRequest -Uri $onnxUrl -OutFile $onnxZip -UseBasicParsing
    } catch {
        return $false, "Download failed: $_"
    }

    if (Test-Path $onnxStag) { Remove-Item $onnxStag -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $onnxStag | Out-Null

    try {
        $null = Expand-Archive -Path $onnxZip -DestinationPath $onnxStag -Force
        Remove-Item $onnxZip -Force -EA SilentlyContinue
    } catch {
        return $false, "Extraction failed: $_"
    }

    $dllSource = Get-ChildItem -Path $onnxStag -Recurse -Filter "onnxruntime.dll" | Select-Object -First 1
    if (-not $dllSource) {
        return $false, "onnxruntime.dll not found inside archive"
    }

    Copy-Item $dllSource.FullName -Destination (Join-Path $DestDir "onnxruntime.dll") -Force

    $providerDll = Get-ChildItem -Path $onnxStag -Recurse -Filter "onnxruntime_providers_shared.dll" | Select-Object -First 1
    if ($providerDll) {
        Copy-Item $providerDll.FullName -Destination (Join-Path $DestDir "onnxruntime_providers_shared.dll") -Force
    }

    Remove-Item $onnxStag -Recurse -Force -EA SilentlyContinue
    return $true, ""
}

$needsOnnxUpdate = $true
if (Test-Path $dllPath) {
    try {
        $existingVer = (Get-Item $dllPath).VersionInfo.ProductVersion
        if ($existingVer -like "$($OnnxVersion.Split('.')[0]).*") {
            $sizeMb = [math]::Round((Get-Item $dllPath).Length / 1MB, 1)
            ok "onnxruntime.dll  $DIM($sizeMb MB -- $existingVer)$RESET"
            $needsOnnxUpdate = $false
        } else {
            warn "Found old ONNX Runtime ($existingVer), upgrading to $OnnxVersion..."
        }
    } catch { }
}

if ($needsOnnxUpdate) {
    $onnxOk, $onnxErr = Install-OnnxRuntime -DestDir $OutDir -Version $OnnxVersion
    if ($onnxOk) {
        $sizeMb = [math]::Round((Get-Item $dllPath).Length / 1MB, 1)
        ok "onnxruntime.dll installed  $DIM($sizeMb MB)$RESET"
    } else {
        warn "ONNX Runtime auto-install failed: $onnxErr"
    }
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
# step 6 – embedding model setup
# ---------------------------------------------------------------------------
blank
step "6/7" "Embedding model setup"

# Priority: Use local build if running from source (Dev Mode)
$LocalExe = Join-Path $PSScriptRoot "..\target\release\omnicontext.exe"
if (Test-Path $LocalExe) { $OutExe = $LocalExe }

# Detect if the binary supports the new 'setup' command (Zero-Hardcoding)
$helpText = & $OutExe --help 2>&1 | Out-String
$hasSetup = $helpText -like "*setup*"

if ($hasSetup) {
    $status = $null
    try { $status = & $OutExe setup model-status --json | ConvertFrom-Json } catch { }

    if ($status -and $status.model_ready) {
        $sizeMb = [math]::Round($status.model_size_bytes / 1MB, 0)
        ok ("Model ready: " + $BOLD + $status.model_name + $RESET + $DIM + " ($sizeMb MB)" + $RESET)
    } else {
        $modelName = if ($status) { $status.model_name } else { "jina-embeddings-v2-base-code" }
        info ("Establishing model: " + $BOLD + $modelName + $RESET)
        info ("Source: HuggingFace (~550 MB)")
        
        try {
            Write-Host "  $DIM$($HR.Substring(0, 40))$RESET"
            # Trigger non-crashing download
            & $OutExe setup model-download
            Write-Host "  $DIM$($HR.Substring(0, 40))$RESET"
            
            $status = & $OutExe setup model-status --json | ConvertFrom-Json
            if ($status.model_ready) {
                $sizeMb = [math]::Round($status.model_size_bytes / 1MB, 0)
                ok "Model setup successful  $DIM($sizeMb MB)$RESET"
            }
        } catch {
            warn "Model initialization failed."
        }
    }
} else {
    # Fallback for older versions (v0.7.1)
    $DataDir   = Join-Path $HOME ".omnicontext"
    $ModelPath = Join-Path $DataDir "models\jina-embeddings-v2-base-code\model.onnx"
    
    if (Test-Path $ModelPath) {
        $sizeMb = [math]::Round((Get-Item $ModelPath).Length / 1MB, 0)
        ok ("Model ready (cached)  " + $DIM + "($sizeMb MB)" + $RESET)
    } else {
        info "Legacy binary detected - triggering automated download..."
        
        $InitTemp = Join-Path $env:TEMP "omnicontext_init_$PID"
        if (Test-Path $InitTemp) { Remove-Item $InitTemp -Recurse -Force }
        New-Item -ItemType Directory -Path $InitTemp -Force | Out-Null
        "// OmniContext Init" | Out-File "$InitTemp\main.rs" -Encoding UTF8
        
        Write-Host "  $DIM$($HR.Substring(0, 40))$RESET"
        Push-Location $InitTemp
        & $OutExe index .
        Pop-Location
        Write-Host "  $DIM$($HR.Substring(0, 40))$RESET"
        
        if (Test-Path $ModelPath) {
            $sizeMb = [math]::Round((Get-Item $ModelPath).Length / 1MB, 0)
            ok "Model ready  $DIM($sizeMb MB)$RESET"
        } else {
            warn "Model download may have been interrupted. Check 'omnicontext index .'"
        }
    }
}

# ---------------------------------------------------------------------------
# step 7 – MCP auto-configure
# ---------------------------------------------------------------------------
blank
step "7/7" "Auto-configuring MCP for AI clients"

# Build MCP entry.
# --repo MUST be an absolute project path. The install script does not know
# which repo the user will use, so it creates a disabled placeholder.
# The VS Code extension's auto-sync will overwrite this with the correct
# absolute path when the user opens a project.
# If the user invokes the MCP binary manually (e.g., from Claude Desktop),
# they should replace "REPLACE_WITH_YOUR_REPO_PATH" with their actual path.
$McpPlaceholderRepo = "REPLACE_WITH_YOUR_REPO_PATH"
$McpEntry = [ordered]@{
    command  = $OutMcpExe
    args     = @("--repo", $McpPlaceholderRepo)
    disabled = $true
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

        # Clean up any existing broken "omnicontext" entries that use --repo "."
        # These silently resolve to the AI launcher's install directory.
        if ($Powers) {
            $servers = $cfg["powers"]
            if ($servers) { $servers = $servers["mcpServers"] }
        } else {
            $servers = $cfg["mcpServers"]
        }

        if ($servers -and $servers["omnicontext"]) {
            $existing = $servers["omnicontext"]
            $existingArgs = $existing["args"]
            if ($existingArgs -is [array]) {
                $repoIdx = [array]::IndexOf($existingArgs, "--repo")
                $cwdIdx  = [array]::IndexOf($existingArgs, "--cwd")
                $hasEnv  = $existing["env"] -and $existing["env"]["OMNICONTEXT_REPO"]
                # If it has --repo "." but no --cwd and no OMNICONTEXT_REPO, it's broken
                if ($repoIdx -ge 0 -and ($repoIdx + 1) -lt $existingArgs.Count -and `
                    $existingArgs[$repoIdx + 1] -eq "." -and $cwdIdx -eq -1 -and -not $hasEnv) {
                    $servers.Remove("omnicontext")
                }
            }
        }

        # Only write placeholder if no existing omnicontext entry with real paths
        $hasExistingGood = $false
        if ($servers -and $servers["omnicontext"]) {
            $ea = $servers["omnicontext"]["args"]
            if ($ea -is [array]) {
                $ri = [array]::IndexOf($ea, "--repo")
                if ($ri -ge 0 -and ($ri + 1) -lt $ea.Count -and `
                    $ea[$ri + 1] -ne "." -and $ea[$ri + 1] -ne $McpPlaceholderRepo) {
                    $hasExistingGood = $true
                }
            }
        }

        if (-not $hasExistingGood) {
            if ($Powers) {
                if (-not $cfg["powers"])                          { $cfg["powers"] = @{} }
                if (-not $cfg["powers"]["mcpServers"])            { $cfg["powers"]["mcpServers"] = @{} }
                $cfg["powers"]["mcpServers"]["omnicontext"] = $McpEntry
            } else {
                if (-not $cfg["mcpServers"]) { $cfg["mcpServers"] = @{} }
                $cfg["mcpServers"]["omnicontext"] = $McpEntry
            }
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
    warn "No AI clients detected - install Claude/Cursor/etc and re-run to auto-configure"
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
    Write-Host "$BOLD  MCP$RESET  $DIM placeholder added for: $($configured -join ', ')$RESET"
    Write-Host "  Install the VS Code extension for automatic project detection."
    Write-Host "  Or set --repo to your project path in each client's config."
} else {
    Write-Host "$BOLD  MCP manual config$RESET"
    Write-Host "  command: $OutMcpExe"
    Write-Host '  args:    ["--repo", "C:\path\to\repo"]'
}
blank
Write-Host "  ${DIM}Update: re-run this script anytime${RESET}"
Write-Host "  ${DIM}Restart your terminal to apply PATH changes${RESET}"
blank
