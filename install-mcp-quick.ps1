#!/usr/bin/env pwsh
# Quick installation without initial indexing

$ErrorActionPreference = "Stop"

function Write-Success { Write-Host "✓ $args" -ForegroundColor Green }
function Write-Info { Write-Host "ℹ $args" -ForegroundColor Cyan }

Write-Host "`n=== OmniContext MCP Quick Install ===" -ForegroundColor Magenta

$binaryPath = Join-Path $PWD "target\release\omnicontext-mcp.exe"
$ConfigPath = "$env:USERPROFILE\.kiro\settings\mcp.json"
$repoPath = $PWD

# Verify binary
if (-not (Test-Path $binaryPath)) {
    Write-Host "✗ Binary not found. Run: cargo build -p omni-mcp --release" -ForegroundColor Red
    exit 1
}

Write-Success "Binary found: $binaryPath"

# Update config
$config = Get-Content $ConfigPath -Raw | ConvertFrom-Json
$config.powers.mcpServers.omnicontext = @{
    command = $binaryPath
    args = @("--repo", $repoPath)
    disabled = $false
    autoApprove = @("search_code", "get_symbol", "get_file_summary", "get_status")
}

$config | ConvertTo-Json -Depth 10 | Set-Content $ConfigPath -Encoding UTF8
Write-Success "Configuration updated"

Write-Host "`n✓ Installation complete!" -ForegroundColor Green
Write-Host "Restart your IDE to load the new configuration`n"
