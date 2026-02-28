<#
.SYNOPSIS
Installs OmniContext and its required AI embedding model.

.DESCRIPTION
This script downloads the latest release of OmniContext from GitHub,
installs it to your user directory ($HOME\.omnicontext\bin), adds it
to your PATH, and pre-downloads the required ONNX AI model (~550MB)
so it's ready for immediate zero-latency use.

.EXAMPLE
powershell -c "irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install/install.ps1 | iex"
#>

$ErrorActionPreference = "Stop"

# Configuration
$RepoOwner = "steeltroops-ai"
$RepoName = "omnicontext"

try {
    $LatestRelease = Invoke-RestMethod -Uri "https://api.github.com/repos/$RepoOwner/$RepoName/releases/latest" -UseBasicParsing
    $Version = $LatestRelease.tag_name
} catch {
    Write-Host "Warning: Failed to fetch latest version from GitHub. Falling back to explicit alpha version." -ForegroundColor Yellow
    $Version = "v0.1.0-alpha"
}

$OutDir = Join-Path $HOME ".omnicontext\bin"
$OutExe = Join-Path $OutDir "omnicontext.exe"
$OutMcpExe = Join-Path $OutDir "omnicontext-mcp.exe"

# 1. Provide Context
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host " 🚀 Installing OmniContext" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "This script will:"
Write-Host " 1. Download OmniContext $Version binaries"
Write-Host " 2. Add them to your PATH ($OutDir)"
Write-Host " 3. Download the Jina AI code embedding model (~550MB)"
Write-Host "    (This model enables semantic code search and MCP AI agent capability)"
Write-Host ""

# Ensure architecture is x64
if ($env:PROCESSOR_ARCHITECTURE -ne "AMD64") {
    Write-Host "Error: OmniContext currently requires Windows x64." -ForegroundColor Red
    exit 1
}

$AssetFileName = "omnicontext-$Version-x86_64-pc-windows-msvc.zip"
$DownloadUrl = "https://github.com/$RepoOwner/$RepoName/releases/download/$Version/$AssetFileName"
$TempZip = Join-Path $env:TEMP $AssetFileName

# 2. Download Binary
Write-Host "Downloading $AssetFileName..." -ForegroundColor Yellow
try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempZip -UseBasicParsing
} catch {
    Write-Host "Error downloading release. Please check your internet connection or if the release $Version exists." -ForegroundColor Red
    Write-Host "Download URL: $DownloadUrl"
    exit 1
}

# 3. Stop running instances for seamless Auto-Update
Write-Host "Checking for running instances for seamless update..." -ForegroundColor Yellow
$processes = Get-Process -Name "omnicontext", "omnicontext-mcp" -ErrorAction SilentlyContinue
if ($processes) {
    $processes | Stop-Process -Force -ErrorAction SilentlyContinue
}

# 4. Extract and Install
Write-Host "Extracting to $OutDir..." -ForegroundColor Yellow
if (!(Test-Path $OutDir)) {
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
}

try {
    # Extract to a temp staging directory first to properly flat-copy files
    $StagingDir = Join-Path $env:TEMP "omnicontext_staging"
    if (Test-Path $StagingDir) { Remove-Item -Path $StagingDir -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $StagingDir | Out-Null
    
    Expand-Archive -Path $TempZip -DestinationPath $StagingDir -Force
    Remove-Item -Path $TempZip -Force

    # Copy files while preserving necessary relational structures if present
    Copy-Item -Path "$StagingDir\*" -Destination $OutDir -Recurse -Force
    Remove-Item -Path $StagingDir -Recurse -Force
} catch {
    Write-Host "Error extracting zip file." -ForegroundColor Red
    exit 1
}

if (!(Test-Path $OutExe) -or !(Test-Path $OutMcpExe)) {
    Write-Host "Error: Executables not found in the extracted archive." -ForegroundColor Red
    exit 1
}

# 4. Add to PATH
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$OutDir*") {
    Write-Host "Adding $OutDir to User PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("PATH", "$UserPath;$OutDir", "User")
    $env:PATH = "$($env:PATH);$OutDir" # Update current session
}

# 5. Initialize the system / Download the embedding model
Write-Host ""
Write-Host "Initializing OmniContext & downloading Jina AI embedding model..." -ForegroundColor Yellow
Write-Host "This requires a robust internet connection. Please wait while the model downloads."

try {
    # Using 'omnicontext status' in an empty temp directory to force embedder initialization
    # which will trigger the download logic and show the indicatif progress bar.
    $InitTemp = Join-Path $env:TEMP "omnicontext_init_temp"
    if (!(Test-Path $InitTemp)) {
        New-Item -ItemType Directory -Path $InitTemp | Out-Null
    }
    
    # Run status
    Set-Location $InitTemp
    & $OutExe status
    if ($LASTEXITCODE -ne 0) {
        throw "Status command failed with exit code $LASTEXITCODE"
    }
    
    Remove-Item -Path $InitTemp -Recurse -Force
} catch {
    Write-Host ""
    Write-Host "Warning: The model download may have been interrupted or failed." -ForegroundColor Magenta
    Write-Host "You can manually trigger it later by running: omnicontext index ." -ForegroundColor Magenta
}

Write-Host "=========================================" -ForegroundColor Green
Write-Host " ✅ OmniContext installation complete!" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
Write-Host ""
Write-Host "To keep OmniContext updated locally, just re-run this install command anytime!" -ForegroundColor Cyan
Write-Host ""
Write-Host "Where to start indexing:"
Write-Host "  Navigate to your code folder:  cd C:\Path\To\Your\Repo"
Write-Host "  Create the search index:       omnicontext index ."
Write-Host "  Test searching your code:      omnicontext search `"auth`""
Write-Host ""
Write-Host "To connect your MCP (Claude, AI Agents), use this configuration:"
Write-Host "  Command:  omnicontext-mcp"
Write-Host "  Args:     [""--repo"", ""C:\\Path\\To\\Your\\Repo""]"
Write-Host ""
Write-Host "Note: You may need to restart your terminal for PATH changes to take effect."
