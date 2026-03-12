<#
.SYNOPSIS
    OmniContext Installer for Windows

.DESCRIPTION
    Downloads the latest OmniContext release, installs binaries to %USERPROFILE%\.omnicontext\bin,
    adds to PATH, pre-downloads the Jina AI embedding model (~550 MB), and auto-configures MCP
    for all supported AI clients via `omnicontext setup --all`, with fallback manual injection.

    Supported clients:
      Claude Desktop, Claude Code, Cursor, Windsurf, VS Code, Cline, RooCode,
      Continue.dev, Zed, Kiro, PearAI, Trae, Antigravity, Gemini CLI, Amazon Q CLI, Augment Code

.PARAMETER Version
    Pin a specific release version (e.g. -Version v1.2.3). Defaults to latest.

.PARAMETER Force
    Bypass up-to-date check and reinstall even if the current version matches.

.EXAMPLE
    irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex

.EXAMPLE
    .\install.ps1 -Version v1.2.3 -Force

.NOTES
    If the script fails to run due to execution policy, run first:
      Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
#>

#Requires -Version 5.1

[CmdletBinding()]
param(
    [string] $Version = "",
    [switch] $Force
)

$ErrorActionPreference = "Stop"
$ProgressPreference    = "SilentlyContinue"

# Enable TLS 1.2/1.3 for secure downloads
[Net.ServicePointManager]::SecurityProtocol =
    [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls11

# ---------------------------------------------------------------------------
# ANSI color helpers
# ---------------------------------------------------------------------------
$ESC    = [char]27
$BOLD   = "$ESC[1m"
$DIM    = "$ESC[2m"
$RESET  = "$ESC[0m"
$RED    = "$ESC[31m"
$GREEN  = "$ESC[32m"
$YELLOW = "$ESC[33m"
$BLUE   = "$ESC[34m"
$CYAN   = "$ESC[36m"

function Write-Step  { param($n, $t) Write-Host "${BOLD}${CYAN}  [$n]${RESET} $t" }
function Write-Ok    { param($t)     Write-Host "${GREEN}  ✔${RESET} $t" }
function Write-Info  { param($t)     Write-Host "${BLUE}  »${RESET} $t" }
function Write-Warn  { param($t)     Write-Host "${YELLOW}  ⚠${RESET} $t" }
function Write-Fail  { param($t)     Write-Host "${RED}  ✖${RESET} $t" }
function Write-Hr    { Write-Host ("${DIM}" + ('─' * 62) + "${RESET}") }
function Write-Blank { Write-Host "" }

function Exit-Err {
    param([string]$msg)
    Write-Blank
    Write-Fail $msg
    Write-Blank
    exit 1
}

# ---------------------------------------------------------------------------
# Retry-enabled web request helper
# ---------------------------------------------------------------------------
function Invoke-Download {
    <#
    .SYNOPSIS
        Download a URL to a file with up to 3 retry attempts.
    #>
    param(
        [string] $Uri,
        [string] $OutFile,
        [int]    $MaxAttempts = 3,
        [int]    $DelaySeconds = 2
    )

    for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
        try {
            Invoke-WebRequest -Uri $Uri -OutFile $OutFile -UseBasicParsing
            return $true
        } catch {
            if ($attempt -lt $MaxAttempts) {
                Write-Warn "Download attempt $attempt failed: $($_.Exception.Message) — retrying in ${DelaySeconds}s..."
                Start-Sleep -Seconds $DelaySeconds
            } else {
                Write-Warn "All $MaxAttempts download attempts failed for: $Uri"
                return $false
            }
        }
    }
    return $false
}

# ---------------------------------------------------------------------------
# Constants / paths
# ---------------------------------------------------------------------------
$RepoOwner  = "steeltroops-ai"
$RepoName   = "omnicontext"
$OutDir     = Join-Path $HOME ".omnicontext\bin"
$DataDir    = Join-Path $HOME ".omnicontext"
$CargoExe   = Join-Path $HOME ".cargo\bin\omnicontext.exe"
$OutExe     = Join-Path $OutDir "omnicontext.exe"
$OutMcpExe  = Join-Path $OutDir "omnicontext-mcp.exe"
$OutDaemonExe = Join-Path $OutDir "omnicontext-daemon.exe"

$TotalSteps = 8
$StartTime  = Get-Date

# Rollback state
$BackupDir  = ""
$DidBackup  = $false
$OnnxJob    = $null

# ---------------------------------------------------------------------------
# Cleanup / rollback on failure
# ---------------------------------------------------------------------------
function Invoke-Rollback {
    if ($DidBackup -and (Test-Path $BackupDir)) {
        Write-Blank
        Write-Warn "Install failed — restoring previous binaries from backup..."
        Get-ChildItem $BackupDir -Filter "*.bak" | ForEach-Object {
            $origName = $_.Name -replace '\.bak$', ''
            $destPath = Join-Path $OutDir $origName
            try {
                Copy-Item $_.FullName $destPath -Force
                Write-Ok "  Restored $origName"
            } catch {
                Write-Warn "  Could not restore ${origName}: $_"
            }
        }
        Remove-Item $BackupDir -Recurse -Force -EA SilentlyContinue
        Write-Warn "Rollback complete. Previous version is still in place."
    }
}

# ---------------------------------------------------------------------------
# banner
# ---------------------------------------------------------------------------
Write-Blank
Write-Host "${BOLD}${CYAN}   ____                  _  ______            __            __ ${RESET}"
Write-Host "${BOLD}${CYAN}  / __ \____ ___  ____  (_)/ ____/___  ____  / /____  _  __/ /_${RESET}"
Write-Host "${BOLD}${CYAN} / / / / __ ``__ \/ __ \/ // /   / __ \/ __ \/ __/ _ \| |/_/ __/${RESET}"
Write-Host "${BOLD}${CYAN}/ /_/ / / / / / / / / / // /___/ /_/ / / / / /_/  __/_>  </ /_ ${RESET}"
Write-Host "${BOLD}${CYAN}\____/_/ /_/ /_/_/ /_/_/ \____/\____/_/ /_/\__/\___/_/|_|\__/  ${RESET}"
Write-Host "${DIM}  Universal Code Context Engine — Windows Installer${RESET}"
Write-Hr
Write-Blank

# ---------------------------------------------------------------------------
# Architecture check
# ---------------------------------------------------------------------------
if ($env:PROCESSOR_ARCHITECTURE -ne "AMD64") {
    Exit-Err "OmniContext requires Windows x64 (AMD64). Detected: $env:PROCESSOR_ARCHITECTURE"
}

# ---------------------------------------------------------------------------
# Connectivity check
# ---------------------------------------------------------------------------
function Test-Connectivity {
    try {
        $null = Invoke-WebRequest -Uri "https://api.github.com" -UseBasicParsing -TimeoutSec 8
        return $true
    } catch {
        return $false
    }
}

# ---------------------------------------------------------------------------
# step 1 – resolve version
# ---------------------------------------------------------------------------
Write-Step "1/$TotalSteps" "Resolving version"

$TargetVersion = $Version.Trim()

if ($TargetVersion) {
    Write-Ok "Using pinned version  ${DIM}($TargetVersion)${RESET}"
} else {
    if (-not (Test-Connectivity)) {
        Write-Blank
        Write-Fail "No internet access detected."
        Write-Info "Offline install options:"
        Write-Info "  1. cargo install omnicontext"
        Write-Info "  2. Download manually from https://github.com/$RepoOwner/$RepoName/releases"
        Write-Info "     and place the .exe files in $OutDir\"
        Exit-Err "Cannot continue without network access."
    }

    try {
        $releases      = Invoke-RestMethod "https://api.github.com/repos/$RepoOwner/$RepoName/releases" -UseBasicParsing
        $latestRelease = $releases | Where-Object { $_.assets.Count -gt 0 } | Select-Object -First 1
        if ($latestRelease) {
            $TargetVersion = $latestRelease.tag_name
            Write-Ok ("Latest release with assets  " + $DIM + "($TargetVersion)" + $RESET)
        }
    } catch { }

    if (-not $TargetVersion) {
        Write-Warn "GitHub API unavailable — falling back to Cargo.toml"
        try {
            $raw = Invoke-RestMethod `
                -Uri "https://raw.githubusercontent.com/$RepoOwner/$RepoName/main/Cargo.toml" `
                -UseBasicParsing
            if ($raw -match '(?m)^version\s*=\s*"([^"]+)"') {
                $TargetVersion = "v$($Matches[1])"
                Write-Ok ("Version resolved from source  " + $DIM + "($TargetVersion)" + $RESET)
            }
        } catch { }
    }

    if (-not $TargetVersion) {
        Exit-Err "Could not resolve a published release version. Check your internet connection."
    }
}

$CleanVersion  = $TargetVersion -replace "^v", ""
$AssetFileName = "omnicontext-$CleanVersion-x86_64-pc-windows-msvc.zip"
$DownloadUrl   = "https://github.com/$RepoOwner/$RepoName/releases/download/$TargetVersion/$AssetFileName"
$TempZip       = Join-Path $env:TEMP $AssetFileName
$StagingDir    = Join-Path $env:TEMP "omnicontext_staging_$PID"

# ---------------------------------------------------------------------------
# Update detection / already-up-to-date check
# ---------------------------------------------------------------------------
$PrevVersion = ""
$IsUpdate    = $false

if (Test-Path $OutExe) {
    try {
        $verOutput = & $OutExe --version 2>&1 | Out-String
        if ($verOutput -match '(\d+\.\d+\.\d+)') {
            $PrevVersion = $Matches[1]
            $IsUpdate    = $true
        }
    } catch { }
}

if ($IsUpdate -and $PrevVersion -eq $CleanVersion -and -not $Force) {
    Write-Blank
    Write-Ok "OmniContext ${BOLD}v$CleanVersion${RESET} is already up-to-date."
    Write-Info "Use -Force to reinstall."
    Write-Blank
    exit 0
} elseif ($IsUpdate -and $PrevVersion -ne $CleanVersion) {
    Write-Info "Updating  ${DIM}v$PrevVersion${RESET} → ${BOLD}v$CleanVersion${RESET}"
} elseif ($IsUpdate) {
    Write-Info "Force-reinstalling v$CleanVersion"
}

# ---------------------------------------------------------------------------
# Cargo install path detection (skip download if cargo binary present)
# ---------------------------------------------------------------------------
$UseCargoExe = $false
if (-not (Test-Path $OutExe) -and (Test-Path $CargoExe)) {
    Write-Blank
    Write-Info "Detected cargo-installed binary at ${DIM}$CargoExe${RESET}"
    Write-Info "Skipping binary download — using existing cargo install."
    Write-Info "${DIM}Tip: cargo install omnicontext  ← install/update from crates.io${RESET}"
    $UseCargoExe = $true
    $OutExe    = $CargoExe
    $OutMcpExe = Join-Path (Split-Path $CargoExe -Parent) "omnicontext-mcp.exe"
}

# ---------------------------------------------------------------------------
# step 2 – download binary (or skip for cargo path)
# ---------------------------------------------------------------------------
Write-Blank
Write-Step "2/$TotalSteps" "Downloading release archive"

if ($UseCargoExe) {
    Write-Ok "Binary download skipped (cargo install path in use)"
} else {
    Write-Info "URL  ${DIM}$DownloadUrl${RESET}"

    if (-not (Test-Connectivity)) {
        Write-Blank
        Write-Fail "No internet access. Cannot download release archive."
        Write-Info "Offline options:"
        Write-Info "  cargo install omnicontext"
        Write-Info "  OR download $AssetFileName manually and extract to $OutDir\"
        Exit-Err "Aborting."
    }

    $dlOk = Invoke-Download -Uri $DownloadUrl -OutFile $TempZip
    if (-not $dlOk) {
        Write-Blank
        Write-Fail "Download failed after 3 attempts."
        Write-Info "URL: $DownloadUrl"
        Write-Info "Verify the release: https://github.com/$RepoOwner/$RepoName/releases/tag/$TargetVersion"
        Write-Info "Alternative: cargo install omnicontext"
        Exit-Err "Aborting."
    }

    if (-not (Test-Path $TempZip)) {
        Exit-Err "Download reported success but file not found at $TempZip"
    }

    # Integrity check: minimum 1 MB
    $archiveSize = (Get-Item $TempZip).Length
    if ($archiveSize -lt 1MB) {
        Remove-Item $TempZip -Force -EA SilentlyContinue
        Exit-Err "Downloaded archive is suspiciously small ($archiveSize bytes). Partial download or wrong URL."
    }

    # Verify ZIP is readable before extracting
    try {
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zipCheck = [System.IO.Compression.ZipFile]::OpenRead($TempZip)
        $zipCheck.Dispose()
    } catch {
        Remove-Item $TempZip -Force -EA SilentlyContinue
        Exit-Err "ZIP integrity check failed: $($_.Exception.Message). The file may be corrupted."
    }

    $sizeMb = [math]::Round($archiveSize / 1MB, 1)
    Write-Ok "Downloaded and verified  ${DIM}$sizeMb MB${RESET}"
}

# ---------------------------------------------------------------------------
# step 3 – stop running instances (warn on failure, don't abort)
# ---------------------------------------------------------------------------
Write-Blank
Write-Step "3/$TotalSteps" "Stopping active processes"

$procs = Get-Process -Name "omnicontext","omnicontext-mcp","omnicontext-daemon" -EA SilentlyContinue
if ($procs) {
    foreach ($p in $procs) {
        try {
            $p | Stop-Process -Force -EA Stop
        } catch {
            Write-Warn "Could not stop $($p.Name) (PID $($p.Id)): $($_.Exception.Message) — continuing."
        }
    }
    Start-Sleep -Milliseconds 600
    Write-Ok "Stopped $($procs.Count) process(es)"
} else {
    Write-Info "No active OmniContext processes found"
}

# Check for file locks on existing binaries
$lockedFiles = @()
foreach ($exePath in @($OutExe, $OutMcpExe, $OutDaemonExe)) {
    if (-not (Test-Path $exePath)) { continue }
    try {
        $fs = [System.IO.File]::Open($exePath, 'Open', 'ReadWrite', 'None')
        $fs.Close()
        $fs.Dispose()
    } catch {
        $lockedFiles += $exePath
    }
}
if ($lockedFiles.Count -gt 0) {
    Write-Warn "The following files appear locked by another process:"
    $lockedFiles | ForEach-Object { Write-Warn "  $_" }
    Write-Warn "Close all OmniContext-related apps and retry, or reboot if the lock persists."
    Exit-Err "Cannot overwrite locked binaries."
}

# ---------------------------------------------------------------------------
# step 4 – backup, extract and install
# ---------------------------------------------------------------------------
Write-Blank
Write-Step "4/$TotalSteps" "Installing binaries"

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# Backup existing binaries
$BackupDir = Join-Path $DataDir "backup_$CleanVersion"
New-Item -ItemType Directory -Force -Path $BackupDir | Out-Null
foreach ($binPath in @($OutExe, $OutMcpExe, $OutDaemonExe)) {
    if (Test-Path $binPath) {
        $bakName = (Split-Path $binPath -Leaf) + ".bak"
        try {
            Copy-Item $binPath (Join-Path $BackupDir $bakName) -Force
            $DidBackup = $true
        } catch { }
    }
}
if ($DidBackup) {
    Write-Info "Backed up existing binaries to  ${DIM}$BackupDir${RESET}"
}

if (-not $UseCargoExe) {
    # Clean staging
    if (Test-Path $StagingDir) { Remove-Item $StagingDir -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $StagingDir | Out-Null

    try {
        $null = Expand-Archive -Path $TempZip -DestinationPath $StagingDir -Force
        Remove-Item $TempZip -Force -EA SilentlyContinue

        # Copy all extracted files; handle flat or nested layouts
        $items = Get-ChildItem $StagingDir -Recurse -File
        $exeItems = $items | Where-Object { $_.Extension -eq ".exe" -or $_.Extension -eq ".dll" }
        foreach ($item in $exeItems) {
            Copy-Item $item.FullName -Destination (Join-Path $OutDir $item.Name) -Force
        }
        Remove-Item $StagingDir -Recurse -Force -EA SilentlyContinue
    } catch {
        Invoke-Rollback
        Exit-Err "Extraction failed: $_"
    }

    # Validate required binaries
    if (-not (Test-Path $OutExe)) {
        Invoke-Rollback
        Write-Info "Alternative: cargo install omnicontext"
        Exit-Err "omnicontext.exe not found after extraction."
    }
    if (-not (Test-Path $OutMcpExe)) {
        Invoke-Rollback
        Exit-Err "omnicontext-mcp.exe not found after extraction."
    }

    # Handle Windows Defender / AV that might quarantine new binaries
    foreach ($exe in @($OutExe, $OutMcpExe)) {
        if (-not (Test-Path $exe)) {
            Write-Warn "$exe missing — may have been quarantined by antivirus."
            Write-Warn "Add an exclusion for $OutDir in your AV settings, then re-run."
        }
    }

    Write-Ok "Installed to  ${DIM}$OutDir${RESET}"
    foreach ($exe in @($OutExe, $OutMcpExe, $OutDaemonExe)) {
        if (Test-Path $exe) {
            $sizeMb = [math]::Round((Get-Item $exe).Length / 1MB, 1)
            Write-Info "$(Split-Path $exe -Leaf)  ${DIM}$sizeMb MB${RESET}"
        }
    }
} else {
    Write-Ok "Using cargo-installed binary — no extraction needed"
}

# ---------------------------------------------------------------------------
# ONNX Runtime DLL — launch download as a background job
# ---------------------------------------------------------------------------
function Get-LatestOnnxVersion {
    try {
        $rel = Invoke-RestMethod `
            -Uri "https://api.github.com/repos/microsoft/onnxruntime/releases/latest" `
            -UseBasicParsing
        return $rel.tag_name.TrimStart('v')
    } catch {
        return "1.24.3"  # 2026 fallback
    }
}

function Install-OnnxRuntime {
    param([string]$DestDir, [string]$OnnxVersion)

    $onnxUrl  = "https://github.com/microsoft/onnxruntime/releases/download/v$OnnxVersion/onnxruntime-win-x64-$OnnxVersion.zip"
    $onnxZip  = Join-Path $env:TEMP "onnxruntime-${OnnxVersion}-$PID.zip"
    $onnxStag = Join-Path $env:TEMP "onnxruntime-${OnnxVersion}-staging-$PID"

    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

    for ($i = 1; $i -le 3; $i++) {
        try {
            Invoke-WebRequest -Uri $onnxUrl -OutFile $onnxZip -UseBasicParsing
            break
        } catch {
            if ($i -eq 3) { return @($false, "Download failed after 3 attempts: $_") }
            Start-Sleep -Seconds 2
        }
    }

    if (Test-Path $onnxStag) { Remove-Item $onnxStag -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $onnxStag | Out-Null

    try {
        $null = Expand-Archive -Path $onnxZip -DestinationPath $onnxStag -Force
        Remove-Item $onnxZip -Force -EA SilentlyContinue
    } catch {
        return @($false, "Extraction failed: $_")
    }

    $dllSrc = Get-ChildItem $onnxStag -Recurse -Filter "onnxruntime.dll" | Select-Object -First 1
    if (-not $dllSrc) {
        return @($false, "onnxruntime.dll not found in archive")
    }

    Copy-Item $dllSrc.FullName (Join-Path $DestDir "onnxruntime.dll") -Force

    $sharedSrc = Get-ChildItem $onnxStag -Recurse -Filter "onnxruntime_providers_shared.dll" |
                 Select-Object -First 1
    if ($sharedSrc) {
        Copy-Item $sharedSrc.FullName (Join-Path $DestDir "onnxruntime_providers_shared.dll") -Force
    }

    Remove-Item $onnxStag -Recurse -Force -EA SilentlyContinue
    return @($true, "")
}

$OnnxVersion = Get-LatestOnnxVersion
$DllPath     = Join-Path $OutDir "onnxruntime.dll"

$NeedsOnnxUpdate = $true
if (Test-Path $DllPath) {
    try {
        $existVer = (Get-Item $DllPath).VersionInfo.ProductVersion
        if ($existVer -like "$($OnnxVersion.Split('.')[0]).*") {
            $sizeMb = [math]::Round((Get-Item $DllPath).Length / 1MB, 1)
            Write-Ok "onnxruntime.dll  ${DIM}v$existVer  ($sizeMb MB)${RESET}"
            $NeedsOnnxUpdate = $false
        } else {
            Write-Info "ONNX Runtime outdated (v$existVer) — upgrading to v$OnnxVersion..."
        }
    } catch { }
}

# Start background job for ONNX download so it runs in parallel with model setup
if ($NeedsOnnxUpdate) {
    Write-Info "Starting parallel ONNX Runtime download in background..."
    $OnnxOutDir = $OutDir
    $OnnxVer    = $OnnxVersion
    $OnnxJob    = Start-Job -ScriptBlock {
        param($DestDir, $OnnxVersion)
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        $ProgressPreference = "SilentlyContinue"

        $onnxUrl  = "https://github.com/microsoft/onnxruntime/releases/download/v$OnnxVersion/onnxruntime-win-x64-$OnnxVersion.zip"
        $onnxZip  = Join-Path $env:TEMP "onnxruntime-${OnnxVersion}-bg.zip"
        $onnxStag = Join-Path $env:TEMP "onnxruntime-${OnnxVersion}-bg-staging"

        for ($i = 1; $i -le 3; $i++) {
            try {
                Invoke-WebRequest -Uri $onnxUrl -OutFile $onnxZip -UseBasicParsing
                break
            } catch {
                if ($i -eq 3) { return "FAIL:Download error: $_" }
                Start-Sleep -Seconds 2
            }
        }

        if (Test-Path $onnxStag) { Remove-Item $onnxStag -Recurse -Force }
        New-Item -ItemType Directory -Force -Path $onnxStag | Out-Null
        try { $null = Expand-Archive -Path $onnxZip -DestinationPath $onnxStag -Force }
        catch { return "FAIL:Extraction error: $_" }
        Remove-Item $onnxZip -Force -EA SilentlyContinue

        $dllSrc = Get-ChildItem $onnxStag -Recurse -Filter "onnxruntime.dll" | Select-Object -First 1
        if (-not $dllSrc) { return "FAIL:DLL not found in archive" }
        Copy-Item $dllSrc.FullName (Join-Path $DestDir "onnxruntime.dll") -Force

        $sharedSrc = Get-ChildItem $onnxStag -Recurse -Filter "onnxruntime_providers_shared.dll" |
                     Select-Object -First 1
        if ($sharedSrc) {
            Copy-Item $sharedSrc.FullName (Join-Path $DestDir "onnxruntime_providers_shared.dll") -Force
        }
        Remove-Item $onnxStag -Recurse -Force -EA SilentlyContinue
        return "OK"
    } -ArgumentList $OnnxOutDir, $OnnxVer
}

# ---------------------------------------------------------------------------
# step 5 – PATH
# ---------------------------------------------------------------------------
Write-Blank
Write-Step "5/$TotalSteps" "Configuring PATH"

$EffectiveOutDir = if ($UseCargoExe) { Split-Path $OutExe -Parent } else { $OutDir }
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")

if ($UserPath -notlike "*$EffectiveOutDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$UserPath;$EffectiveOutDir", "User")
    $env:PATH = "$($env:PATH);$EffectiveOutDir"
    Write-Ok "Added to User PATH  ${DIM}$EffectiveOutDir${RESET}"
} else {
    Write-Ok "Already in PATH  ${DIM}$EffectiveOutDir${RESET}"
}

# ---------------------------------------------------------------------------
# step 6 – embedding model setup (runs while ONNX downloads in background)
# ---------------------------------------------------------------------------
Write-Blank
Write-Step "6/$TotalSteps" "Embedding model setup"

# Allow dev-mode override with local build
$LocalExe = Join-Path $PSScriptRoot "..\target\release\omnicontext.exe"
if ($PSScriptRoot -and (Test-Path $LocalExe)) { $OutExe = $LocalExe }

$helpText = try { & $OutExe --help 2>&1 | Out-String } catch { "" }
$hasSetup = $helpText -like "*setup*"

if ($hasSetup) {
    $statusObj = $null
    try { $statusObj = & $OutExe setup model-status --json 2>&1 | ConvertFrom-Json -EA Stop } catch { }

    if ($statusObj -and $statusObj.model_ready) {
        $sizeMb = [math]::Round($statusObj.model_size_bytes / 1MB, 0)
        Write-Ok ("Model ready: " + $BOLD + $statusObj.model_name + $RESET + $DIM + " ($sizeMb MB)" + $RESET)
    } else {
        $modelName = if ($statusObj -and $statusObj.model_name) { $statusObj.model_name } `
                     else { "jina-embeddings-v2-base-code" }
        Write-Info ("Downloading model: " + $BOLD + $modelName + $RESET + $DIM + " (~550 MB, HuggingFace)" + $RESET)
        Write-Blank

        try {
            Write-Host ("  " + $DIM + ('─' * 40) + $RESET)
            & $OutExe setup model-download
            Write-Host ("  " + $DIM + ('─' * 40) + $RESET)

            $statusObj = & $OutExe setup model-status --json 2>&1 | ConvertFrom-Json -EA SilentlyContinue
            if ($statusObj -and $statusObj.model_ready) {
                $sizeMb = [math]::Round($statusObj.model_size_bytes / 1MB, 0)
                Write-Ok "Model setup successful  ${DIM}($sizeMb MB)${RESET}"
            } else {
                Write-Warn "Model download incomplete."
                Write-Info "Run later: ${DIM}omnicontext setup model-download${RESET}"
            }
        } catch {
            Write-Warn "Model download failed or interrupted: $_"
            Write-Info "Run later: ${DIM}omnicontext setup model-download${RESET}"
            # Non-fatal: continue with install
        }
    }
} else {
    # Legacy binary fallback
    $ModelPath = Join-Path $DataDir "models\jina-embeddings-v2-base-code\model.onnx"
    if (Test-Path $ModelPath) {
        $sizeMb = [math]::Round((Get-Item $ModelPath).Length / 1MB, 0)
        Write-Ok ("Model ready (cached)  " + $DIM + "($sizeMb MB)" + $RESET)
    } else {
        Write-Info "Legacy binary detected — model will be initialized on first index."
        Write-Info "Run: ${DIM}omnicontext index .${RESET}  in your project directory."
    }
}

# ---------------------------------------------------------------------------
# Wait for background ONNX job
# ---------------------------------------------------------------------------
if ($OnnxJob) {
    Write-Blank
    Write-Info "Waiting for background ONNX Runtime download to complete..."
    $null = Wait-Job $OnnxJob
    $onnxResult = Receive-Job $OnnxJob
    Remove-Job $OnnxJob -Force
    $OnnxJob = $null

    if ($onnxResult -like "OK*") {
        if (Test-Path $DllPath) {
            $sizeMb = [math]::Round((Get-Item $DllPath).Length / 1MB, 1)
            Write-Ok "onnxruntime.dll installed  ${DIM}$sizeMb MB (v$OnnxVersion)${RESET}"
        }
    } else {
        Write-Warn "ONNX Runtime background download failed: $onnxResult"
        Write-Warn "Context injection may not work. Re-run the installer to retry."
    }
}

# ---------------------------------------------------------------------------
# step 7 – MCP auto-configure via setup --all
# ---------------------------------------------------------------------------
#
# Supported clients (handled by omnicontext setup --all):
#   - Claude Desktop     (%APPDATA%\Claude\claude_desktop_config.json)
#   - Claude Code        (%USERPROFILE%\.claude.json)
#   - Cursor             (%APPDATA%\Cursor\User\mcp.json)
#   - Windsurf           (%USERPROFILE%\.codeium\windsurf\mcp_config.json)
#   - VS Code            (%APPDATA%\Code\User\mcp.json, key="servers")
#   - Cline              (%APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\settings\...)
#   - RooCode            (%APPDATA%\Code\User\globalStorage\rooveterinaryinc.roo-cline\settings\...)
#   - Continue.dev       (%USERPROFILE%\.continue\config.json)
#   - Zed                (%APPDATA%\Zed\settings.json)
#   - Kiro               (%USERPROFILE%\.kiro\settings\mcp.json   powers.mcpServers)
#   - PearAI             (%APPDATA%\PearAI\mcp_config.json)
#   - Trae               (%APPDATA%\Trae\User\globalStorage\trae-ide.trae-ai\mcp_settings.json)
#   - Antigravity        (%APPDATA%\Antigravity\User\mcp.json, key="servers")
#   - Gemini CLI         (%USERPROFILE%\.gemini\settings.json)
#   - Amazon Q CLI       (%USERPROFILE%\.aws\amazonq\mcp.json)
#   - Augment Code       (%APPDATA%\Augment\mcp_config.json)
# ---------------------------------------------------------------------------
Write-Blank
Write-Step "7/$TotalSteps" "Auto-configuring MCP for AI clients"

$SetupAllUsed = $false
$ConfiguredCount = 0

# Primary: use setup --all (orchestrator handles all 15+ IDEs correctly)
if ($helpText -match 'setup\s+--all|--all') {
    Write-Info ("Using " + $BOLD + "omnicontext setup --all" + $RESET + " (orchestrator handles all clients)")
    Write-Blank
    try {
        Write-Host ("  " + $DIM + ('─' * 40) + $RESET)
        $setupOutput = & $OutExe setup --all 2>&1
        $setupOutput | ForEach-Object {
            Write-Host "  $_"
            if ($_ -match 'configured|registered|updated') { $ConfiguredCount++ }
        }
        Write-Host ("  " + $DIM + ('─' * 40) + $RESET)
        Write-Ok "MCP configuration complete via orchestrator"
        $SetupAllUsed = $true
    } catch {
        Write-Warn "setup --all failed: $($_.Exception.Message) — falling back to manual injection"
    }
}

# Fallback: manual JSON injection for each known client
if (-not $SetupAllUsed) {
    Write-Info "Falling back to manual JSON injection..."
    Write-Blank

    $McpBinPath = if ($UseCargoExe) {
        Join-Path (Split-Path $CargoExe -Parent) "omnicontext-mcp.exe"
    } else { $OutMcpExe }

    # --repo . is the universal entry; IDE plugins resolve the real project root at launch time
    $McpEntry = [ordered]@{
        command = $McpBinPath
        args    = @("--repo", ".")
    }

    # PS 5.1-compatible deep conversion of PSCustomObject → ordered hashtable
    function ConvertTo-Hashtable($obj) {
        if ($obj -is [System.Management.Automation.PSCustomObject]) {
            $ht = [ordered]@{}
            foreach ($prop in $obj.PSObject.Properties) {
                $ht[$prop.Name] = ConvertTo-Hashtable $prop.Value
            }
            return $ht
        }
        return $obj
    }

    function Set-McpConfig {
        param(
            [string] $Name,
            [string] $ConfigPath,
            [string] $TopKey = "mcpServers"   # "mcpServers", "powers", "servers", or "context_servers"
        )
        $dir = Split-Path $ConfigPath -Parent
        if (-not (Test-Path $dir)) { return $false }
        try {
            $cfg = [ordered]@{}
            if (Test-Path $ConfigPath) {
                $raw = Get-Content $ConfigPath -Raw -EA SilentlyContinue
                if ($raw) {
                    try { $cfg = $raw | ConvertFrom-Json | ForEach-Object { ConvertTo-Hashtable $_ } }
                    catch { $cfg = [ordered]@{} }
                }
            }

            switch ($TopKey) {
                "powers" {
                    if (-not $cfg["powers"])            { $cfg["powers"] = [ordered]@{} }
                    if (-not $cfg["powers"]["mcpServers"]) { $cfg["powers"]["mcpServers"] = [ordered]@{} }
                    $cfg["powers"]["mcpServers"]["omnicontext"] = $McpEntry
                }
                "servers" {
                    if (-not $cfg["servers"]) { $cfg["servers"] = [ordered]@{} }
                    $cfg["servers"]["omnicontext"] = $McpEntry
                }
                "context_servers" {
                    if (-not $cfg["context_servers"]) { $cfg["context_servers"] = [ordered]@{} }
                    $cfg["context_servers"]["omnicontext"] = $McpEntry
                }
                default {
                    if (-not $cfg["mcpServers"]) { $cfg["mcpServers"] = [ordered]@{} }
                    $cfg["mcpServers"]["omnicontext"] = $McpEntry
                }
            }

            $cfg | ConvertTo-Json -Depth 10 | Set-Content $ConfigPath -Encoding UTF8
            return $true
        } catch {
            return $false
        }
    }

    $clients = @(
        @{ Name = "Claude Desktop";  Path = "$env:APPDATA\Claude\claude_desktop_config.json";                                                                  TopKey = "mcpServers" },
        @{ Name = "Claude Code";     Path = "$env:USERPROFILE\.claude.json";                                                                                   TopKey = "mcpServers" },
        @{ Name = "Cursor";          Path = "$env:APPDATA\Cursor\User\mcp.json";                                                                               TopKey = "mcpServers" },
        @{ Name = "Windsurf";        Path = "$env:USERPROFILE\.codeium\windsurf\mcp_config.json";                                                              TopKey = "mcpServers" },
        @{ Name = "VS Code";         Path = "$env:APPDATA\Code\User\mcp.json";                                                                                 TopKey = "servers"    },
        @{ Name = "VS Code Insiders";Path = "$env:APPDATA\Code - Insiders\User\mcp.json";                                                                      TopKey = "servers"    },
        @{ Name = "Cline";           Path = "$env:APPDATA\Code\User\globalStorage\saoudrizwan.claude-dev\settings\cline_mcp_settings.json";                    TopKey = "mcpServers" },
        @{ Name = "RooCode";         Path = "$env:APPDATA\Code\User\globalStorage\rooveterinaryinc.roo-cline\settings\mcp_settings.json";                      TopKey = "mcpServers" },
        @{ Name = "Continue.dev";    Path = "$env:USERPROFILE\.continue\config.json";                                                                          TopKey = "mcpServers" },
        @{ Name = "Zed";             Path = "$env:APPDATA\Zed\settings.json";                                                                                  TopKey = "context_servers" },
        @{ Name = "Kiro";            Path = "$env:USERPROFILE\.kiro\settings\mcp.json";                                                                        TopKey = "mcpServers" },
        @{ Name = "PearAI";          Path = "$env:APPDATA\PearAI\User\mcp.json";                                                                              TopKey = "mcpServers" },
        @{ Name = "Trae";            Path = "$env:APPDATA\Trae\User\globalStorage\trae-ide.trae-ai\mcp_settings.json";                                         TopKey = "mcpServers" },
        @{ Name = "Antigravity";     Path = "$env:APPDATA\Antigravity\User\mcp.json";                                                                           TopKey = "servers"    },
        @{ Name = "Gemini CLI";      Path = "$env:USERPROFILE\.gemini\settings.json";                                                                          TopKey = "mcpServers" },
        @{ Name = "Amazon Q CLI";    Path = "$env:USERPROFILE\.aws\amazonq\mcp.json";                                                                          TopKey = "mcpServers" },
        @{ Name = "Augment Code";    Path = "$env:APPDATA\Code\User\globalStorage\augment.vscode-augment\mcp_settings.json";                                   TopKey = "mcpServers" }
    )

    $configured = @()
    foreach ($c in $clients) {
        if (Set-McpConfig -Name $c.Name -ConfigPath $c.Path -TopKey $c.TopKey) {
            $configured += $c.Name
            Write-Ok "  $($c.Name)  ${DIM}$($c.Path)${RESET}"
        }
    }
    $ConfiguredCount = $configured.Count

    if ($ConfiguredCount -eq 0) {
        Write-Warn "No AI client config directories detected."
        Write-Warn "Install Claude Desktop / Cursor / Windsurf / VS Code / etc. and re-run."
    } else {
        Write-Blank
        Write-Ok "$ConfiguredCount client(s) configured (fallback injection)"
    }
}

# ---------------------------------------------------------------------------
# step 8 – cleanup & finalize
# ---------------------------------------------------------------------------
Write-Blank
Write-Step "8/$TotalSteps" "Finalizing"

# Delete backups on success
if ($DidBackup -and (Test-Path $BackupDir)) {
    Remove-Item $BackupDir -Recurse -Force -EA SilentlyContinue
    Write-Ok "Backups removed (install succeeded)"
}

# Verify installed binary version
$installedVer = try {
    (& $OutExe --version 2>&1 | Out-String) -replace '^.*?(\d+\.\d+\.\d+).*$', '$1'
} catch { $CleanVersion }
$installedVer = $installedVer.Trim()
Write-Ok "Binary verified  ${DIM}v$installedVer${RESET}"

# ---------------------------------------------------------------------------
# summary
# ---------------------------------------------------------------------------
$elapsed = [math]::Round(((Get-Date) - $StartTime).TotalSeconds, 1)
Write-Blank
Write-Hr
if ($IsUpdate -and $PrevVersion -and $PrevVersion -ne $CleanVersion) {
    Write-Host ("${BOLD}${GREEN}  OmniContext updated  " + $DIM + "v$PrevVersion → v$CleanVersion" + $RESET + "  " + $DIM + "(${elapsed}s)" + $RESET)
} else {
    Write-Host ("${BOLD}${GREEN}  OmniContext v$CleanVersion installed${RESET}  " + $DIM + "(${elapsed}s)" + $RESET)
}
Write-Hr
Write-Blank

Write-Host "${BOLD}  Quick Start${RESET}"
Write-Host "  cd C:\path\to\your\repo"
Write-Host "  omnicontext index ."
Write-Host '  omnicontext search "error handling"'
Write-Blank

if ($SetupAllUsed) {
    Write-Host ("${BOLD}  MCP${RESET}  configured via " + $DIM + "omnicontext setup --all" + $RESET)
    Write-Host "  All detected AI clients have been registered."
    Write-Host "  Open any project and your AI tools will pick up context automatically."
} elseif ($ConfiguredCount -gt 0) {
    Write-Host "${BOLD}  MCP${RESET}  ${DIM}$ConfiguredCount client(s) configured${RESET}"
    Write-Host "  Open any project directory to use OmniContext with your AI tools."
} else {
    Write-Host "${BOLD}  MCP manual config${RESET}"
    Write-Host "  command: $OutMcpExe"
    Write-Host '  args:    ["--repo", "."]'
    Write-Host "  Re-run installer after installing Claude Desktop / Cursor / etc."
}

Write-Blank
Write-Host "  ${DIM}Update:    .\install.ps1  (or -Version v1.2.3 to pin)${RESET}"
Write-Host "  ${DIM}Cargo:     cargo install omnicontext${RESET}"
Write-Host "  ${DIM}Restart your terminal to apply PATH changes${RESET}"
Write-Host "  ${DIM}Docs:      https://github.com/$RepoOwner/$RepoName${RESET}"
Write-Blank
Write-Host "  ${DIM}Uninstall: Remove-Item -Recurse '$OutDir', '$DataDir'${RESET}"
Write-Host "  ${DIM}           and remove '$EffectiveOutDir' from your User PATH${RESET}"
Write-Blank
