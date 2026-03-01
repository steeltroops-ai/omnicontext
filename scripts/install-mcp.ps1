#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Install or update OmniContext MCP server
.DESCRIPTION
    Enterprise-grade installation script for OmniContext MCP server.
    Handles building, installation, configuration, and verification.
.PARAMETER Repo
    Repository path to index (defaults to current directory)
.PARAMETER ConfigPath
    Path to MCP configuration file (defaults to ~/.kiro/settings/mcp.json)
.PARAMETER SkipBuild
    Skip building and use existing binary
.PARAMETER SkipTests
    Skip running tests before installation
.EXAMPLE
    .\install-mcp.ps1
.EXAMPLE
    .\install-mcp.ps1 -Repo "C:\MyProject" -SkipTests
#>

param(
    [string]$Repo = $PWD,
    [string]$ConfigPath = "$env:USERPROFILE\.kiro\settings\mcp.json",
    [switch]$SkipBuild,
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"

# Colors for output
function Write-Success { Write-Host "✓ $args" -ForegroundColor Green }
function Write-Info { Write-Host "ℹ $args" -ForegroundColor Cyan }
function Write-Warning { Write-Host "⚠ $args" -ForegroundColor Yellow }
function Write-Error { Write-Host "✗ $args" -ForegroundColor Red }

Write-Host "`n=== OmniContext MCP Server Installation ===" -ForegroundColor Magenta
Write-Host "Enterprise-grade code context engine`n" -ForegroundColor Gray

# Step 1: Verify prerequisites
Write-Info "Checking prerequisites..."

# Check Rust
try {
    $rustVersion = cargo --version
    Write-Success "Rust installed: $rustVersion"
} catch {
    Write-Error "Rust not found. Please install from https://rustup.rs/"
    exit 1
}

# Check if we're in the right directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Error "Cargo.toml not found. Please run from project root."
    exit 1
}

# Step 2: Run tests (unless skipped)
if (-not $SkipTests) {
    Write-Info "Running test suite..."
    try {
        $testOutput = cargo test -p omni-core --lib --quiet 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Success "All tests passed"
        } else {
            Write-Error "Tests failed. Output:"
            Write-Host $testOutput
            exit 1
        }
    } catch {
        Write-Error "Failed to run tests: $_"
        exit 1
    }
} else {
    Write-Warning "Skipping tests (--SkipTests flag)"
}

# Step 3: Build MCP server (unless skipped)
if (-not $SkipBuild) {
    Write-Info "Building MCP server in release mode..."
    try {
        cargo build -p omni-mcp --release --quiet
        if ($LASTEXITCODE -eq 0) {
            Write-Success "Build completed successfully"
        } else {
            Write-Error "Build failed"
            exit 1
        }
    } catch {
        Write-Error "Failed to build: $_"
        exit 1
    }
} else {
    Write-Warning "Skipping build (--SkipBuild flag)"
}

# Step 4: Verify binary exists
$binaryPath = Join-Path $PWD "target\release\omnicontext-mcp.exe"
if (-not (Test-Path $binaryPath)) {
    Write-Error "Binary not found at: $binaryPath"
    exit 1
}

$binaryInfo = Get-Item $binaryPath
Write-Success "Binary found: $($binaryInfo.Length / 1MB) MB"
Write-Info "Location: $binaryPath"

# Step 5: Test binary execution
Write-Info "Testing binary execution..."
try {
    $testRun = & $binaryPath --help 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Binary executes correctly"
    } else {
        Write-Warning "Binary executed but returned non-zero exit code"
    }
} catch {
    Write-Error "Failed to execute binary: $_"
    exit 1
}

# Step 6: Configure MCP settings
Write-Info "Configuring MCP settings..."

# Ensure config directory exists
$configDir = Split-Path $ConfigPath -Parent
if (-not (Test-Path $configDir)) {
    New-Item -ItemType Directory -Path $configDir -Force | Out-Null
    Write-Success "Created config directory: $configDir"
}

# Read or create config
$config = @{
    mcpServers = @{}
    powers = @{
        mcpServers = @{}
    }
}

if (Test-Path $ConfigPath) {
    try {
        $existingConfig = Get-Content $ConfigPath -Raw | ConvertFrom-Json
        $config = $existingConfig
        Write-Success "Loaded existing configuration"
    } catch {
        Write-Warning "Failed to parse existing config, will create new one"
    }
}

# Add/update omnicontext server
$repoPath = (Resolve-Path $Repo).Path
$config.powers.mcpServers.omnicontext = @{
    command = $binaryPath
    args = @("--repo", $repoPath)
    disabled = $false
    autoApprove = @(
        "search_code",
        "get_symbol",
        "get_file_summary",
        "get_status"
    )
}

# Save config
try {
    $config | ConvertTo-Json -Depth 10 | Set-Content $ConfigPath -Encoding UTF8
    Write-Success "Configuration saved to: $ConfigPath"
} catch {
    Write-Error "Failed to save configuration: $_"
    exit 1
}

# Step 7: Initial indexing
Write-Info "Running initial index of repository..."
Write-Info "Repository: $repoPath"

try {
    $indexStart = Get-Date
    $indexOutput = & $binaryPath --repo $repoPath 2>&1
    $indexDuration = (Get-Date) - $indexStart
    
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Initial indexing completed in $($indexDuration.TotalSeconds) seconds"
    } else {
        Write-Warning "Indexing completed with warnings"
        Write-Host $indexOutput
    }
} catch {
    Write-Error "Failed to run initial indexing: $_"
    Write-Warning "You may need to run indexing manually"
}

# Step 8: Verification
Write-Info "Verifying installation..."

# Check if data directory was created
$dataDir = Join-Path $env:USERPROFILE ".omnicontext"
if (Test-Path $dataDir) {
    $dbPath = Join-Path $dataDir "index.db"
    $vectorPath = Join-Path $dataDir "vectors.bin"
    
    if (Test-Path $dbPath) {
        $dbSize = (Get-Item $dbPath).Length / 1KB
        Write-Success "SQLite index created: $([math]::Round($dbSize, 2)) KB"
    }
    
    if (Test-Path $vectorPath) {
        $vectorSize = (Get-Item $vectorPath).Length / 1KB
        Write-Success "Vector index created: $([math]::Round($vectorSize, 2)) KB"
    }
} else {
    Write-Warning "Data directory not found at: $dataDir"
}

# Step 9: Print summary
Write-Host "`n=== Installation Summary ===" -ForegroundColor Magenta
Write-Host "Status: " -NoNewline
Write-Success "COMPLETE"
Write-Host "Binary: $binaryPath"
Write-Host "Config: $ConfigPath"
Write-Host "Repository: $repoPath"
Write-Host "Data Directory: $dataDir"

Write-Host "`n=== Next Steps ===" -ForegroundColor Magenta
Write-Host "1. Restart your IDE/editor to load the new MCP configuration"
Write-Host "2. Test the MCP server with: " -NoNewline
Write-Host "search_code" -ForegroundColor Yellow
Write-Host "3. Check status with: " -NoNewline
Write-Host "get_status" -ForegroundColor Yellow
Write-Host "4. View logs in your MCP client for any issues"

Write-Host "`n=== Troubleshooting ===" -ForegroundColor Magenta
Write-Host "If you encounter issues:"
Write-Host "- Check logs in your MCP client"
Write-Host "- Verify binary executes: " -NoNewline
Write-Host "$binaryPath --help" -ForegroundColor Yellow
Write-Host "- Re-run with: " -NoNewline
Write-Host ".\install-mcp.ps1 -SkipBuild" -ForegroundColor Yellow
Write-Host "- Report issues: https://github.com/steeltroops-ai/omnicontext/issues"

Write-Host "`n✓ Installation complete!`n" -ForegroundColor Green
