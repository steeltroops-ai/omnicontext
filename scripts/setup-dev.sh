#!/bin/bash
# Development environment setup script
# Run this after cloning the repository

set -e

echo "Setting up OmniContext development environment..."

# Install git hooks
echo "Installing git hooks..."
git config core.hooksPath .githooks

# Make hooks executable
chmod +x .githooks/pre-commit
chmod +x .githooks/pre-push

echo "✅ Git hooks installed successfully!"
echo ""
echo "The following hooks are now active:"
echo "  - pre-commit: Runs cargo fmt and clippy checks"
echo "  - pre-push: Runs cargo test before pushing"
echo ""
echo "To bypass hooks (not recommended), use:"
echo "  git commit --no-verify"
echo "  git push --no-verify"
echo ""
echo "Development environment setup complete!"
