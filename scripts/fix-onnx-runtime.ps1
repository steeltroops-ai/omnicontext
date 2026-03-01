#!/usr/bin/env pwsh
# Fix ONNX Runtime version mismatch by downloading compatible binaries
# This script downloads ONNX Runtime 1.23.0 which is compatible with ort 2.0.0-rc.11

$ErrorActionPreference = "Stop"

Write-Host "🔧 Fixing ONNX Runtime version mismatch..." -ForegroundColor Cyan

# Detect architecture
$arch = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }
$os = if ($IsWindows -or $env:OS -eq "Windows_NT") { "win" } 
      elseif ($IsMacOS) { "osx" } 
      else { "linux" }

Write-Host "Detected: $os-$arch" -ForegroundColor Gray

# ONNX Runtime 1.23.0 download URLs
$version = "1.23.0"

# Check if correct version already exists
$targetDir = Join-Path $PSScriptRoot ".." "target" "debug"
$dllPath = Join-Path $targetDir "onnxruntime.dll"

if (Test-Path $dllPath) {
    try {
        $fileVersion = (Get-Item $dllPath).VersionInfo.FileVersion
        if ($fileVersion -like "1.23.*") {
            Write-Host "✅ ONNX Runtime $version already installed!" -ForegroundColor Green
            Write-Host "   Found version: $fileVersion" -ForegroundColor Gray
            Write-Host ""
            Write-Host "🧪 Now run: cargo test -p omni-core --lib" -ForegroundColor Cyan
            exit 0
        } else {
            Write-Host "⚠️  Found incompatible version: $fileVersion" -ForegroundColor Yellow
            Write-Host "   Upgrading to version $version..." -ForegroundColor Yellow
        }
    } catch {
        Write-Host "⚠️  Could not read version info, reinstalling..." -ForegroundColor Yellow
    }
}
$urls = @{
    "win-x64" = "https://github.com/microsoft/onnxruntime/releases/download/v$version/onnxruntime-win-x64-$version.zip"
    "win-x86" = "https://github.com/microsoft/onnxruntime/releases/download/v$version/onnxruntime-win-x86-$version.zip"
    "linux-x64" = "https://github.com/microsoft/onnxruntime/releases/download/v$version/onnxruntime-linux-x64-$version.tgz"
    "osx-x64" = "https://github.com/microsoft/onnxruntime/releases/download/v$version/onnxruntime-osx-x64-$version.tgz"
    "osx-arm64" = "https://github.com/microsoft/onnxruntime/releases/download/v$version/onnxruntime-osx-arm64-$version.tgz"
}

$platform = "$os-$arch"
if ($IsMacOS -and [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq "Arm64") {
    $platform = "osx-arm64"
}

$url = $urls[$platform]
if (-not $url) {
    Write-Host "❌ Unsupported platform: $platform" -ForegroundColor Red
    exit 1
}

Write-Host "📥 Downloading ONNX Runtime $version for $platform..." -ForegroundColor Yellow
Write-Host "URL: $url" -ForegroundColor Gray

# Create temp directory
$tempDir = Join-Path $env:TEMP "onnxruntime-$version"
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null

# Download
$archiveFile = Join-Path $tempDir "onnxruntime.$($url.Split('.')[-1])"
try {
    Invoke-WebRequest -Uri $url -OutFile $archiveFile -UseBasicParsing
    Write-Host "✅ Downloaded successfully" -ForegroundColor Green
} catch {
    Write-Host "❌ Download failed: $_" -ForegroundColor Red
    exit 1
}

# Extract
Write-Host "📦 Extracting..." -ForegroundColor Yellow
$extractDir = Join-Path $tempDir "extracted"
if ($url.EndsWith(".zip")) {
    Expand-Archive -Path $archiveFile -DestinationPath $extractDir -Force
} else {
    # For .tgz on Windows, use tar if available
    if (Get-Command tar -ErrorAction SilentlyContinue) {
        tar -xzf $archiveFile -C $extractDir
    } else {
        Write-Host "❌ tar command not found. Please install tar or manually extract $archiveFile" -ForegroundColor Red
        exit 1
    }
}

# Find the lib directory
$libDir = Get-ChildItem -Path $extractDir -Recurse -Directory -Filter "lib" | Select-Object -First 1
if (-not $libDir) {
    Write-Host "❌ Could not find lib directory in extracted files" -ForegroundColor Red
    exit 1
}

# Copy DLLs to project target directory
$targetDir = Join-Path $PSScriptRoot ".." "target" "debug"
New-Item -ItemType Directory -Force -Path $targetDir | Out-Null

Write-Host "📋 Copying binaries to $targetDir..." -ForegroundColor Yellow

$copied = 0
Get-ChildItem -Path $libDir.FullName -File | ForEach-Object {
    $destPath = Join-Path $targetDir $_.Name
    Copy-Item -Path $_.FullName -Destination $destPath -Force
    Write-Host "  ✓ Copied $($_.Name)" -ForegroundColor Gray
    $copied++
}

# Also copy to release directory if it exists
$releaseDir = Join-Path $PSScriptRoot ".." "target" "release"
if (Test-Path $releaseDir) {
    Get-ChildItem -Path $libDir.FullName -File | ForEach-Object {
        Copy-Item -Path $_.FullName -Destination (Join-Path $releaseDir $_.Name) -Force
    }
    Write-Host "  ✓ Also copied to release directory" -ForegroundColor Gray
}

# Cleanup
Remove-Item -Path $tempDir -Recurse -Force

Write-Host ""
Write-Host "✅ ONNX Runtime $version installed successfully!" -ForegroundColor Green
Write-Host "   Copied $copied file(s) to target directories" -ForegroundColor Gray
Write-Host ""
Write-Host "🧪 Now run: cargo test -p omni-core --lib" -ForegroundColor Cyan
Write-Host "   Or: cargo run -p omni-mcp" -ForegroundColor Cyan
