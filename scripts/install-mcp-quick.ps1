#!/usr/bin/env pwsh
# Quick MCP configuration for local development (no initial indexing).
# Detects all installed AI clients and configures OmniContext MCP.

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "=== OmniContext MCP Quick Install ===" -ForegroundColor Magenta

$binaryPath = Join-Path $PWD "target\release\omnicontext-mcp.exe"
$repoPath = $PWD

# Verify binary
if (-not (Test-Path $binaryPath)) {
    Write-Host "[x] Binary not found. Run: cargo build -p omni-mcp --release" -ForegroundColor Red
    exit 1
}

Write-Host "[OK] Binary found: $binaryPath" -ForegroundColor Green

# Define all known AI client targets
$McpTargets = @(
    @{ Name = "Claude Desktop"; Path = (Join-Path $env:APPDATA "Claude\claude_desktop_config.json"); Namespace = $false },
    @{ Name = "Cursor"; Path = (Join-Path $env:APPDATA "Cursor\User\globalStorage\cursor.mcp\config.json"); Namespace = $false },
    @{ Name = "Continue.dev"; Path = (Join-Path $env:USERPROFILE ".continue\config.json"); Namespace = $false },
    @{ Name = "Kiro"; Path = (Join-Path $env:USERPROFILE ".kiro\settings\mcp.json"); Namespace = $true },
    @{ Name = "Windsurf"; Path = (Join-Path $env:APPDATA "Windsurf\User\globalStorage\codeium.windsurf\mcp_config.json"); Namespace = $false },
    @{ Name = "Cline"; Path = (Join-Path $env:USERPROFILE ".cline\mcp_settings.json"); Namespace = $false }
)

$McpEntry = @{
    command = $binaryPath.ToString()
    args = @("--repo", $repoPath.ToString())
    disabled = $false
    autoApprove = @("search_code", "get_symbol", "get_file_summary", "get_status")
}

$configured = @()

foreach ($target in $McpTargets) {
    $configDir = Split-Path $target.Path -Parent
    if (-not (Test-Path $configDir)) { continue }

    try {
        $config = @{}
        if (Test-Path $target.Path) {
            $raw = Get-Content $target.Path -Raw -ErrorAction SilentlyContinue
            if ($raw) { $config = $raw | ConvertFrom-Json -AsHashtable -ErrorAction SilentlyContinue }
            if (-not $config) { $config = @{} }
        }

        if ($target.Namespace) {
            if (-not $config.ContainsKey("powers")) { $config["powers"] = @{} }
            if (-not $config["powers"].ContainsKey("mcpServers")) { $config["powers"]["mcpServers"] = @{} }
            $config["powers"]["mcpServers"]["omnicontext"] = $McpEntry
        } else {
            if (-not $config.ContainsKey("mcpServers")) { $config["mcpServers"] = @{} }
            $config["mcpServers"]["omnicontext"] = $McpEntry
        }

        $config | ConvertTo-Json -Depth 10 | Set-Content $target.Path -Encoding UTF8
        $configured += $target.Name
        Write-Host "[OK] $($target.Name): $($target.Path)" -ForegroundColor Green
    } catch {
        Write-Host "[--] $($target.Name): $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

Write-Host ""
if ($configured.Count -gt 0) {
    Write-Host "[OK] Configured $($configured.Count) AI client(s): $($configured -join ', ')" -ForegroundColor Green
    Write-Host "Restart your IDE to load the new configuration."
} else {
    Write-Host "[--] No AI clients detected." -ForegroundColor Yellow
    Write-Host "Install Claude Desktop, Cursor, Continue.dev, Kiro, Windsurf, or Cline and re-run."
}
Write-Host ""
