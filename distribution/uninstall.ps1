#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Uninstall OmniContext completely
.DESCRIPTION
    Removes all OmniContext files, configurations, and PATH entries.
    Optionally preserves indexed data.
.PARAMETER KeepData
    Keep indexed repositories and models
.PARAMETER KeepConfig
    Keep MCP configuration
.EXAMPLE
    .\uninstall.ps1
.EXAMPLE
    .\uninstall.ps1 -KeepData -KeepConfig
#>

param(
    [switch]$KeepData,
    [switch]$KeepConfig
)

$ErrorActionPreference = "Stop"

function Write-Success { Write-Host "✓ $args" -ForegroundColor Green }
function Write-Info { Write-Host "ℹ $args" -ForegroundColor Cyan }
function Write-Warning { Write-Host "⚠ $args" -ForegroundColor Yellow }
function Write-Error { Write-Host "✗ $args" -ForegroundColor Red }

Write-Host "`n=== OmniContext Uninstaller ===" -ForegroundColor Magenta
Write-Host ""

# Confirm uninstallation
Write-Warning "This will remove OmniContext from your system"
if (-not $KeepData) {
    Write-Warning "All indexed data and models will be deleted (~600MB+)"
}
if (-not $KeepConfig) {
    Write-Warning "MCP configuration will be removed"
}

$confirmation = Read-Host "`nContinue? (yes/no)"
if ($confirmation -ne "yes") {
    Write-Info "Uninstallation cancelled"
    exit 0
}

Write-Host ""

# Stop running processes
Write-Info "Stopping running processes..."
$processes = Get-Process -Name "omnicontext", "omnicontext-mcp", "omnicontext-daemon" -ErrorAction SilentlyContinue
if ($processes) {
    $processes | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    Write-Success "Stopped $($processes.Count) process(es)"
} else {
    Write-Info "No running processes found"
}

# Remove binaries
$binDir = "$env:USERPROFILE\.omnicontext\bin"
if (Test-Path $binDir) {
    Write-Info "Removing binaries from $binDir..."
    Remove-Item -Path $binDir -Recurse -Force
    Write-Success "Binaries removed"
} else {
    Write-Info "Binary directory not found"
}

# Remove from PATH
Write-Info "Removing from PATH..."
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -like "*$binDir*") {
    $NewPath = ($UserPath -split ';' | Where-Object { $_ -ne $binDir }) -join ';'
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    Write-Success "Removed from PATH"
} else {
    Write-Info "Not in PATH"
}

# Remove data directory
if (-not $KeepData) {
    $dataDir = "$env:USERPROFILE\.omnicontext"
    if (Test-Path $dataDir) {
        Write-Info "Removing data directory..."
        $dataSize = (Get-ChildItem -Path $dataDir -Recurse | Measure-Object -Property Length -Sum).Sum / 1MB
        Remove-Item -Path $dataDir -Recurse -Force
        Write-Success "Removed data directory ($([math]::Round($dataSize, 2)) MB)"
    } else {
        Write-Info "Data directory not found"
    }
} else {
    Write-Info "Keeping data directory (--KeepData flag)"
}

# Remove MCP configuration
if (-not $KeepConfig) {
    $configPath = "$env:USERPROFILE\.kiro\settings\mcp.json"
    if (Test-Path $configPath) {
        Write-Info "Removing MCP configuration..."
        try {
            $config = Get-Content $configPath -Raw | ConvertFrom-Json -AsHashtable
            if ($config.mcpServers.omnicontext) {
                $config.mcpServers.Remove("omnicontext")
                $config | ConvertTo-Json -Depth 10 | Set-Content $configPath -Encoding UTF8
                Write-Success "Removed from MCP configuration"
            } else {
                Write-Info "Not in MCP configuration"
            }
        } catch {
            Write-Warning "Failed to update MCP configuration: $_"
        }
    } else {
        Write-Info "MCP configuration not found"
    }
} else {
    Write-Info "Keeping MCP configuration (--KeepConfig flag)"
}

# Summary
Write-Host "`n=== Uninstallation Complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "OmniContext has been removed from your system"
Write-Host ""

if ($KeepData) {
    Write-Info "Data preserved at: $env:USERPROFILE\.omnicontext"
}

if ($KeepConfig) {
    Write-Info "MCP configuration preserved"
}

Write-Host "`nTo reinstall OmniContext, visit:"
Write-Host "https://github.com/steeltroops-ai/omnicontext"
Write-Host ""

