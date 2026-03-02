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

# Fetch version from source code (Cargo.toml)
Write-Host "Fetching latest version from source..." -ForegroundColor Cyan
try {
    $CargoTomlUrl = "https://raw.githubusercontent.com/$RepoOwner/$RepoName/main/Cargo.toml"
    $CargoContent = Invoke-RestMethod -Uri $CargoTomlUrl -UseBasicParsing
    
    if ($CargoContent -match 'version\s*=\s*"([^"]+)"') {
        $SourceVersion = $Matches[1]
        $Version = "v$SourceVersion"
        Write-Host "Latest version from source: $Version" -ForegroundColor Green
    } else {
        throw "Could not parse version from Cargo.toml"
    }
} catch {
    Write-Host "Warning: Failed to fetch version from source. Trying GitHub releases..." -ForegroundColor Yellow
    
    # Fallback: Check GitHub releases
    try {
        $Releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$RepoOwner/$RepoName/releases" -UseBasicParsing
        if ($Releases.Count -eq 0) {
            Write-Host "=========================================" -ForegroundColor Red
            Write-Host " No Pre-Built Releases Available Yet" -ForegroundColor Red
            Write-Host "=========================================" -ForegroundColor Red
            Write-Host ""
            Write-Host "OmniContext doesn't have pre-built releases yet." -ForegroundColor Yellow
            Write-Host "You'll need to build from source." -ForegroundColor Yellow
            Write-Host ""
            Write-Host "To build from source:" -ForegroundColor Cyan
            Write-Host "  1. Install Rust: https://rustup.rs/" -ForegroundColor White
            Write-Host "  2. Clone the repo: git clone https://github.com/steeltroops-ai/omnicontext.git" -ForegroundColor White
            Write-Host "  3. Build: cd omnicontext && cargo build --release" -ForegroundColor White
            Write-Host "  4. Binaries will be in: target/release/" -ForegroundColor White
            Write-Host ""
            Write-Host "For detailed instructions, see:" -ForegroundColor Cyan
            Write-Host "  https://github.com/steeltroops-ai/omnicontext/blob/main/CONTRIBUTING.md" -ForegroundColor White
            Write-Host ""
            exit 1
        }
        
        $LatestRelease = $Releases[0]
        $Version = $LatestRelease.tag_name
        
        # Verify the release has assets
        if ($LatestRelease.assets.Count -eq 0) {
            Write-Host "Warning: Latest release $Version has no binary assets. Checking for older releases..." -ForegroundColor Yellow
            $ReleaseWithAssets = $Releases | Where-Object { $_.assets.Count -gt 0 } | Select-Object -First 1
            if ($ReleaseWithAssets) {
                $Version = $ReleaseWithAssets.tag_name
                Write-Host "Using release $Version which has binaries available." -ForegroundColor Green
            } else {
                Write-Host "Error: No releases with binary assets found. Please build from source." -ForegroundColor Red
                Write-Host "See: https://github.com/steeltroops-ai/omnicontext/blob/main/CONTRIBUTING.md" -ForegroundColor Yellow
                exit 1
            }
        }
    } catch {
        Write-Host "Error: Could not determine version. Please build from source." -ForegroundColor Red
        Write-Host "See: https://github.com/steeltroops-ai/omnicontext/blob/main/CONTRIBUTING.md" -ForegroundColor Yellow
        exit 1
    }
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
    # Create temporary directory with dummy file to trigger model download
    $InitTemp = Join-Path $env:TEMP "omnicontext_init_temp"
    if (!(Test-Path $InitTemp)) {
        New-Item -ItemType Directory -Path $InitTemp | Out-Null
    }
    
    # Create dummy source file
    "// Dummy file for model download" | Out-File -FilePath "$InitTemp\dummy.rs" -Encoding UTF8
    "fn main() {}" | Out-File -FilePath "$InitTemp\dummy.rs" -Append -Encoding UTF8
    
    # Run index command to trigger model download
    Set-Location $InitTemp
    & $OutExe index .
    if ($LASTEXITCODE -ne 0) {
        throw "Index command failed with exit code $LASTEXITCODE"
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
