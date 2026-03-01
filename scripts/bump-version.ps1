# Enterprise-grade version bumping script for Windows
# Usage: .\scripts\bump-version.ps1 [major|minor|patch|<version>]

param(
    [Parameter(Mandatory=$true)]
    [string]$BumpType
)

# Get current version from Cargo.toml
function Get-CurrentVersion {
    $content = Get-Content Cargo.toml
    $versionLine = $content | Select-String '^version = ' | Select-Object -First 1
    if ($versionLine -match 'version = "([^"]+)"') {
        return $matches[1]
    }
    throw "Could not find version in Cargo.toml"
}

# Parse version into components
function Parse-Version {
    param([string]$Version)
    
    if ($Version -match '(\d+)\.(\d+)\.(\d+)') {
        return @{
            Major = [int]$matches[1]
            Minor = [int]$matches[2]
            Patch = [int]$matches[3]
        }
    }
    throw "Invalid version format: $Version"
}

# Bump version
function Get-NewVersion {
    param(
        [string]$Current,
        [string]$BumpType
    )
    
    switch ($BumpType) {
        'major' {
            $v = Parse-Version $Current
            return "$($v.Major + 1).0.0"
        }
        'minor' {
            $v = Parse-Version $Current
            return "$($v.Major).$($v.Minor + 1).0"
        }
        'patch' {
            $v = Parse-Version $Current
            return "$($v.Major).$($v.Minor).$($v.Patch + 1)"
        }
        default {
            # Custom version provided
            return $BumpType
        }
    }
}

# Update version in Cargo.toml files
function Update-CargoVersion {
    param([string]$NewVersion)
    
    Write-Host "Updating Cargo.toml files..." -ForegroundColor Yellow
    
    # Update workspace Cargo.toml
    (Get-Content Cargo.toml) -replace '^version = ".*"', "version = `"$NewVersion`"" | Set-Content Cargo.toml
    
    # Update all crate Cargo.toml files
    Get-ChildItem -Path crates\*\Cargo.toml | ForEach-Object {
        $content = Get-Content $_.FullName
        $content = $content -replace '^version = ".*"', "version = `"$NewVersion`""
        $content = $content -replace 'omni-core = \{ version = ".*"', "omni-core = { version = `"$NewVersion`""
        $content | Set-Content $_.FullName
    }
    
    Write-Host "✓ Updated Cargo.toml files" -ForegroundColor Green
}

# Update CHANGELOG.md
function Update-Changelog {
    param([string]$NewVersion)
    
    Write-Host "Updating CHANGELOG.md..." -ForegroundColor Yellow
    
    $date = Get-Date -Format "yyyy-MM-dd"
    
    if (-not (Test-Path CHANGELOG.md)) {
        @"
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [$NewVersion] - $date

### Added
- Initial release

"@ | Set-Content CHANGELOG.md
    } else {
        $content = Get-Content CHANGELOG.md -Raw
        $newSection = @"

## [$NewVersion] - $date

### Added
- 

### Changed
- 

### Fixed
- 

"@
        $content = $content -replace '(## \[Unreleased\])', "`$1$newSection"
        $content | Set-Content CHANGELOG.md
    }
    
    Write-Host "✓ Updated CHANGELOG.md" -ForegroundColor Green
}

# Update package manifests
function Update-PackageManifests {
    param([string]$NewVersion)
    
    Write-Host "Updating package manifests..." -ForegroundColor Yellow
    
    # Update Homebrew formula
    if (Test-Path distribution\homebrew\omnicontext.rb) {
        (Get-Content distribution\homebrew\omnicontext.rb) -replace 'version ".*"', "version `"$NewVersion`"" | 
            Set-Content distribution\homebrew\omnicontext.rb
    }
    
    # Update Scoop manifest
    if (Test-Path distribution\scoop\omnicontext.json) {
        (Get-Content distribution\scoop\omnicontext.json) -replace '"version": ".*"', "`"version`": `"$NewVersion`"" | 
            Set-Content distribution\scoop\omnicontext.json
    }
    
    Write-Host "✓ Updated package manifests" -ForegroundColor Green
}

# Verify build
function Test-Build {
    Write-Host "Verifying build..." -ForegroundColor Yellow
    
    $result = cargo check --workspace --quiet
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ Build verification passed" -ForegroundColor Green
        return $true
    } else {
        Write-Host "✗ Build verification failed" -ForegroundColor Red
        return $false
    }
}

# Create git commit and tag
function New-GitCommitAndTag {
    param([string]$NewVersion)
    
    Write-Host "Creating git commit and tag..." -ForegroundColor Yellow
    
    # Stage changes
    git add Cargo.toml Cargo.lock crates\*\Cargo.toml CHANGELOG.md distribution\
    
    # Commit
    git commit -m "chore: bump version to $NewVersion"
    
    # Create tag
    git tag -a "v$NewVersion" -m "Release v$NewVersion"
    
    Write-Host "✓ Created commit and tag v$NewVersion" -ForegroundColor Green
}

# Main script
try {
    $currentVersion = Get-CurrentVersion
    $newVersion = Get-NewVersion -Current $currentVersion -BumpType $BumpType
    
    Write-Host "OmniContext Version Bump" -ForegroundColor Green
    Write-Host "=========================="
    Write-Host "Current version: $currentVersion"
    Write-Host "New version:     $newVersion"
    Write-Host ""
    
    # Confirm
    $confirm = Read-Host "Proceed with version bump? (y/N)"
    if ($confirm -ne 'y' -and $confirm -ne 'Y') {
        Write-Host "Aborted" -ForegroundColor Yellow
        exit 0
    }
    
    # Update versions
    Update-CargoVersion -NewVersion $newVersion
    Update-Changelog -NewVersion $newVersion
    Update-PackageManifests -NewVersion $newVersion
    
    # Update Cargo.lock
    Write-Host "Updating Cargo.lock..." -ForegroundColor Yellow
    cargo update --workspace --quiet
    Write-Host "✓ Updated Cargo.lock" -ForegroundColor Green
    
    # Verify build
    if (-not (Test-Build)) {
        throw "Build verification failed"
    }
    
    # Git operations
    New-GitCommitAndTag -NewVersion $newVersion
    
    Write-Host ""
    Write-Host "✓ Version bump complete!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:"
    Write-Host "  1. Review the changes: git show"
    Write-Host "  2. Push the commit: git push origin main"
    Write-Host "  3. Push the tag: git push origin v$newVersion"
    Write-Host "  4. GitHub Actions will automatically create the release"
    
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
    exit 1
}
