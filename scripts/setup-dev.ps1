# Development environment setup script for Windows
# Run this after cloning the repository

Write-Host "Setting up OmniContext development environment..." -ForegroundColor Cyan

# Install git hooks
Write-Host "Installing git hooks..." -ForegroundColor Yellow
git config core.hooksPath .githooks

Write-Host "✅ Git hooks installed successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "The following hooks are now active:" -ForegroundColor Cyan
Write-Host "  - pre-commit: Runs cargo fmt and clippy checks"
Write-Host "  - pre-push: Runs cargo test before pushing"
Write-Host ""
Write-Host "To bypass hooks (not recommended), use:" -ForegroundColor Yellow
Write-Host "  git commit --no-verify"
Write-Host "  git push --no-verify"
Write-Host ""
Write-Host "Development environment setup complete!" -ForegroundColor Green
