$ErrorActionPreference = "Stop"

function Get-LatestOnnxVersion {
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/microsoft/onnxruntime/releases/latest" -UseBasicParsing
        return $release.tag_name.TrimStart('v')
    } catch {
        return "1.24.3"
    }
}

$version = Get-LatestOnnxVersion
Write-Host "Setting up ONNX Runtime $version"

$arch = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }
$targetDirs = @(
    (Join-Path (Join-Path $PSScriptRoot "..") "target\debug"),
    (Join-Path (Join-Path $PSScriptRoot "..") "target\release"),
    "C:\Users\mayan\.omnicontext\bin"
)

$needsDownload = $true
foreach ($dir in $targetDirs) {
    if (Test-Path $dir) {
        $dllPath = Join-Path $dir "onnxruntime.dll"
        if (Test-Path $dllPath) {
            $verInfo = (Get-Item $dllPath).VersionInfo.ProductVersion
            if ($verInfo -like "$($version.Split('.')[0]).*") {
                Write-Host "OK in $dir"
                $needsDownload = $false
            }
        }
    }
}

if ($needsDownload) {
    $url = "https://github.com/microsoft/onnxruntime/releases/download/v$version/onnxruntime-win-$arch-$version.zip"
    Write-Host "Downloading $version..."
    $tempDir = Join-Path $env:TEMP "onnx_latest"
    if (Test-Path $tempDir) { Remove-Item $tempDir -Recurse -Force }
    New-Item $tempDir -ItemType Directory | Out-Null
    $archive = Join-Path $tempDir "onnx.zip"
    Invoke-WebRequest -Uri $url -OutFile $archive -UseBasicParsing
    Expand-Archive $archive -DestinationPath "$tempDir\ext" -Force
    $lib = Get-ChildItem -Path "$tempDir\ext" -Recurse -Directory -Filter "lib" | Select-Object -First 1
    foreach ($dir in $targetDirs) {
        if (Test-Path $dir) {
            Write-Host "Copying to $dir"
            Get-ChildItem $lib.FullName -File | ForEach-Object {
                Copy-Item $_.FullName -Destination (Join-Path $dir $_.Name) -Force
            }
        }
    }
    Remove-Item $tempDir -Recurse -Force
}

Write-Host "Backend ready."
