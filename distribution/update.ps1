#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Update OmniContext to the latest version
.DESCRIPTION
    Downloads and installs the latest OmniContext release while preserving
    configuration and indexed data.
.PARAMETER Force
    Force update even if already on latest version
.EXAMPLE
    .\update.ps1
.EXAMPLE
    .\update.ps1 -Force
#>

param(
    [switch]$Force
)

$ErrorActionPreference = "Stop"

function Write-Success { Write-Host "✓ $args" -ForegroundColor Green }
function Write-Info { Write-Host "ℹ $args" -ForegroundColor Cyan }
function Write-Warning { Write-Host "⚠ $args" -ForegroundColor Yellow }
function Write-Error { Write-Host "✗ $args" -ForegroundColor Red }

Write-Host "`n=== OmniContext Updater ===" -ForegroundColor Magenta
Write-Host ""

# Check if OmniContext is installed
$binPath = "$env:USERPROFILE\.omnicontext\bin\omnicontext.exe"
if (-not (Test-Path $binPath)) {
    Write-Error "OmniContext not found at: $binPath"
    Write-Info "Please install OmniContext first:"
    Write-Info "  irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex"
    exit 1
}

# Get current version
Write-Info "Checking current version..."
try {
    $currentVersion = & $binPath --version 2>&1 | Select-String -Pattern "omnicontext\s+(\S+)" | ForEach-Object { $_.Matches.Groups[1].Value }
    Write-Success "Current version: $currentVersion"
} catch {
    Write-Warning "Could not determine current version"
    $currentVersion = "unknown"
}

# Get latest version
Write-Info "Checking for updates..."
try {
    $latestRelease = Invoke-RestMethod -Uri "https://api.github.com/repos/steeltroops-ai/omnicontext/releases/latest" -UseBasicParsing
    $latestVersion = $latestRelease.tag_name
    Write-Success "Latest version: $latestVersion"
} catch {
    Write-Error "Failed to check for updates: $_"
    exit 1
}

# Compare versions
if ($currentVersion -eq $latestVersion.TrimStart('v') -and -not $Force) {
    Write-Success "Already on latest version!"
    Write-Info "Use -Force to reinstall anyway"
    exit 0
}

if ($Force) {
    Write-Info "Forcing update (--Force flag)"
} else {
    Write-Info "Update available: $currentVersion → $latestVersion"
}

# Backup configuration
Write-Info "Backing up configuration..."
$configPath = "$env:USERPROFILE\.kiro\settings\mcp.json"
$configBackup = $null

if (Test-Path $configPath) {
    try {
        $configBackup = Get-Content $configPath -Raw
        Write-Success "Configuration backed up"
    } catch {
        Write-Warning "Failed to backup configuration: $_"
    }
}

# Stop running processes
Write-Info "Stopping running processes..."
$processes = Get-Process -Name "omnicontext", "omnicontext-mcp", "omnicontext-daemon" -ErrorAction SilentlyContinue
if ($processes) {
    $processes | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    Write-Success "Stopped $($processes.Count) process(es)"
}

# Download and install update
Write-Info "Downloading update..."
$installScript = "https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1"

try {
    $scriptContent = Invoke-RestMethod -Uri $installScript -UseBasicParsing
    Invoke-Expression $scriptContent
} catch {
    Write-Error "Update failed: $_"
    
    # Restore configuration if backup exists
    if ($configBackup) {
        Write-Info "Restoring configuration..."
        $configBackup | Set-Content $configPath -Encoding UTF8
    }
    
    exit 1
}

# Verify update
Write-Info "Verifying update..."
try {
    $newVersion = & $binPath --version 2>&1 | Select-String -Pattern "omnicontext\s+(\S+)" | ForEach-Object { $_.Matches.Groups[1].Value }
    
    if ($newVersion -eq $latestVersion.TrimStart('v')) {
        Write-Success "Update successful: $currentVersion → $newVersion"
    } else {
        Write-Warning "Update may have failed (version: $newVersion, expected: $latestVersion)"
    }
} catch {
    Write-Warning "Could not verify update: $_"
}

# Restore configuration if needed
if ($configBackup) {
    Write-Info "Checking configuration..."
    try {
        $currentConfig = Get-Content $configPath -Raw
        if ($currentConfig -ne $configBackup) {
            Write-Info "Configuration was modified during update"
            $restore = Read-Host "Restore previous configuration? (yes/no)"
            if ($restore -eq "yes") {
                $configBackup | Set-Content $configPath -Encoding UTF8
                Write-Success "Configuration restored"
            }
        }
    } catch {
        Write-Warning "Could not check configuration: $_"
    }
}

Write-Host "`n=== Update Complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "OmniContext has been updated to $latestVersion"
Write-Host ""
Write-Host "Next steps:"
Write-Host "1. Restart your IDE to reload the MCP server"
Write-Host "2. Verify functionality with: omnicontext status"
Write-Host "3. Re-index if needed: omnicontext index ."
Write-Host ""

