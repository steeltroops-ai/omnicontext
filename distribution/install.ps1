<#
.SYNOPSIS
Installs OmniContext and its required AI embedding model.

.DESCRIPTION
This script downloads the latest release of OmniContext from GitHub,
installs it to your user directory ($HOME\.omnicontext\bin), adds it
to your PATH, pre-downloads the required ONNX AI model (~550MB),
and auto-configures MCP for detected AI clients (Claude, Cursor,
Continue.dev, Kiro, Windsurf, Cline).

.EXAMPLE
powershell -c "irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex"
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
            Write-Host "==========================================" -ForegroundColor Red
            Write-Host " No Pre-Built Releases Available Yet" -ForegroundColor Red
            Write-Host "==========================================" -ForegroundColor Red
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
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host " Installing OmniContext" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "This script will:"
Write-Host " 1. Download OmniContext $Version binaries"
Write-Host " 2. Add them to your PATH ($OutDir)"
Write-Host " 3. Download the Jina AI code embedding model (~550MB)"
Write-Host " 4. Auto-configure MCP for detected AI clients"
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
    $StagingDir = Join-Path $env:TEMP "omnicontext_staging"
    if (Test-Path $StagingDir) { Remove-Item -Path $StagingDir -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $StagingDir | Out-Null
    
    Expand-Archive -Path $TempZip -DestinationPath $StagingDir -Force
    Remove-Item -Path $TempZip -Force

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

# 5. Add to PATH
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$OutDir*") {
    Write-Host "Adding $OutDir to User PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("PATH", "$UserPath;$OutDir", "User")
    $env:PATH = "$($env:PATH);$OutDir"
}

# 6. Initialize / Download the embedding model
Write-Host ""
Write-Host "Initializing OmniContext & downloading Jina AI embedding model..." -ForegroundColor Yellow
Write-Host "This requires a robust internet connection. Please wait while the model downloads."

try {
    $InitTemp = Join-Path $env:TEMP "omnicontext_init_temp"
    if (!(Test-Path $InitTemp)) {
        New-Item -ItemType Directory -Path $InitTemp | Out-Null
    }
    
    "// Dummy file for model download" | Out-File -FilePath "$InitTemp\dummy.rs" -Encoding UTF8
    "fn main() {}" | Out-File -FilePath "$InitTemp\dummy.rs" -Append -Encoding UTF8
    
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

# 7. Auto-Configure MCP for detected AI clients
Write-Host ""
Write-Host "Configuring MCP for detected AI clients..." -ForegroundColor Yellow

$McpBinary = $OutMcpExe
$McpConfigured = @()

$McpTargets = @(
    @{ Name = "Claude Desktop"; Path = (Join-Path $env:APPDATA "Claude\claude_desktop_config.json"); Namespace = $false },
    @{ Name = "Cursor"; Path = (Join-Path $env:APPDATA "Cursor\User\globalStorage\cursor.mcp\config.json"); Namespace = $false },
    @{ Name = "Continue.dev"; Path = (Join-Path $env:USERPROFILE ".continue\config.json"); Namespace = $false },
    @{ Name = "Kiro"; Path = (Join-Path $env:USERPROFILE ".kiro\settings\mcp.json"); Namespace = $true },
    @{ Name = "Windsurf"; Path = (Join-Path $env:APPDATA "Windsurf\User\globalStorage\codeium.windsurf\mcp_config.json"); Namespace = $false },
    @{ Name = "Cline"; Path = (Join-Path $env:USERPROFILE ".cline\mcp_settings.json"); Namespace = $false }
)

$McpEntry = @{ command = $McpBinary; args = @("--repo", "."); disabled = $false }

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
        $McpConfigured += $target.Name
        Write-Host "  [OK] $($target.Name)" -ForegroundColor Green
    } catch {
        Write-Host "  [--] $($target.Name): $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

if ($McpConfigured.Count -eq 0) {
    Write-Host "  No AI clients detected. Install Claude/Cursor/etc and re-run." -ForegroundColor Yellow
} else {
    Write-Host "  Configured $($McpConfigured.Count) client(s): $($McpConfigured -join ', ')" -ForegroundColor Green
}

# Summary
Write-Host ""
Write-Host "==========================================" -ForegroundColor Green
Write-Host " OmniContext installation complete!" -ForegroundColor Green
Write-Host "==========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Quick Start:" -ForegroundColor Cyan
Write-Host "  cd C:\Path\To\Your\Repo"
Write-Host "  omnicontext index ."
Write-Host "  omnicontext search `"authentication`""
Write-Host ""
if ($McpConfigured.Count -gt 0) {
    Write-Host "MCP Auto-Configured: $($McpConfigured -join ', ')" -ForegroundColor Cyan
    Write-Host "  Default: '--repo .' (current directory). Edit config for specific repos."
} else {
    Write-Host "MCP Manual Setup:" -ForegroundColor Cyan
    Write-Host "  Command:  $McpBinary"
    Write-Host "  Args:     [""--repo"", ""C:\\Path\\To\\Your\\Repo""]"
}
Write-Host ""
Write-Host "Update: Re-run this script anytime." -ForegroundColor Cyan
Write-Host "Restart your terminal for PATH changes."
