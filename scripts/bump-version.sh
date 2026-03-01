#!/bin/bash
# Enterprise-grade version bumping script
# Usage: ./scripts/bump-version.sh [major|minor|patch|<version>]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get current version from Cargo.toml
get_current_version() {
    grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'
}

# Parse version into components
parse_version() {
    local version=$1
    echo "$version" | sed 's/\([0-9]*\)\.\([0-9]*\)\.\([0-9]*\).*/\1 \2 \3/'
}

# Bump version
bump_version() {
    local current=$1
    local bump_type=$2
    
    read -r major minor patch <<< "$(parse_version "$current")"
    
    case $bump_type in
        major)
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        minor)
            minor=$((minor + 1))
            patch=0
            ;;
        patch)
            patch=$((patch + 1))
            ;;
        *)
            # Custom version provided
            echo "$bump_type"
            return
            ;;
    esac
    
    echo "${major}.${minor}.${patch}"
}

# Update version in Cargo.toml files
update_cargo_version() {
    local new_version=$1
    
    echo -e "${YELLOW}Updating Cargo.toml files...${NC}"
    
    # Update workspace Cargo.toml
    sed -i.bak "s/^version = \".*\"/version = \"${new_version}\"/" Cargo.toml
    
    # Update all crate Cargo.toml files
    for cargo_file in crates/*/Cargo.toml; do
        sed -i.bak "s/^version = \".*\"/version = \"${new_version}\"/" "$cargo_file"
        # Update workspace dependencies
        sed -i.bak "s/omni-core = { version = \".*\"/omni-core = { version = \"${new_version}\"/" "$cargo_file"
    done
    
    # Remove backup files
    find . -name "Cargo.toml.bak" -delete
    
    echo -e "${GREEN}✓ Updated Cargo.toml files${NC}"
}

# Update CHANGELOG.md
update_changelog() {
    local new_version=$1
    local date=$(date +%Y-%m-%d)
    
    echo -e "${YELLOW}Updating CHANGELOG.md...${NC}"
    
    # Check if CHANGELOG.md exists
    if [ ! -f CHANGELOG.md ]; then
        cat > CHANGELOG.md <<EOF
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [${new_version}] - ${date}

### Added
- Initial release

EOF
    else
        # Insert new version section after the header
        sed -i.bak "/^## \[Unreleased\]/a\\
\\
## [${new_version}] - ${date}\\
\\
### Added\\
- \\
\\
### Changed\\
- \\
\\
### Fixed\\
- " CHANGELOG.md
        
        rm -f CHANGELOG.md.bak
    fi
    
    echo -e "${GREEN}✓ Updated CHANGELOG.md${NC}"
}

# Update package manifests
update_package_manifests() {
    local new_version=$1
    
    echo -e "${YELLOW}Updating package manifests...${NC}"
    
    # Update Homebrew formula (placeholder - will be updated by release workflow)
    if [ -f distribution/homebrew/omnicontext.rb ]; then
        sed -i.bak "s/version \".*\"/version \"${new_version}\"/" distribution/homebrew/omnicontext.rb
        rm -f distribution/homebrew/omnicontext.rb.bak
    fi
    
    # Update Scoop manifest (placeholder - will be updated by release workflow)
    if [ -f distribution/scoop/omnicontext.json ]; then
        sed -i.bak "s/\"version\": \".*\"/\"version\": \"${new_version}\"/" distribution/scoop/omnicontext.json
        rm -f distribution/scoop/omnicontext.json.bak
    fi
    
    echo -e "${GREEN}✓ Updated package manifests${NC}"
}

# Run cargo check to ensure everything compiles
verify_build() {
    echo -e "${YELLOW}Verifying build...${NC}"
    
    if cargo check --workspace --quiet; then
        echo -e "${GREEN}✓ Build verification passed${NC}"
    else
        echo -e "${RED}✗ Build verification failed${NC}"
        exit 1
    fi
}

# Create git commit and tag
create_git_commit_and_tag() {
    local new_version=$1
    
    echo -e "${YELLOW}Creating git commit and tag...${NC}"
    
    # Stage changes
    git add Cargo.toml Cargo.lock crates/*/Cargo.toml CHANGELOG.md distribution/
    
    # Commit
    git commit -m "chore: bump version to ${new_version}"
    
    # Create tag
    git tag -a "v${new_version}" -m "Release v${new_version}"
    
    echo -e "${GREEN}✓ Created commit and tag v${new_version}${NC}"
}

# Main script
main() {
    if [ $# -eq 0 ]; then
        echo -e "${RED}Error: No version bump type specified${NC}"
        echo "Usage: $0 [major|minor|patch|<version>]"
        echo ""
        echo "Examples:"
        echo "  $0 patch          # 0.1.0 -> 0.1.1"
        echo "  $0 minor          # 0.1.0 -> 0.2.0"
        echo "  $0 major          # 0.1.0 -> 1.0.0"
        echo "  $0 0.2.0-alpha    # Set to specific version"
        exit 1
    fi
    
    local bump_type=$1
    local current_version=$(get_current_version)
    local new_version=$(bump_version "$current_version" "$bump_type")
    
    echo -e "${GREEN}OmniContext Version Bump${NC}"
    echo "=========================="
    echo "Current version: ${current_version}"
    echo "New version:     ${new_version}"
    echo ""
    
    # Confirm
    read -p "Proceed with version bump? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}Aborted${NC}"
        exit 0
    fi
    
    # Update versions
    update_cargo_version "$new_version"
    update_changelog "$new_version"
    update_package_manifests "$new_version"
    
    # Update Cargo.lock
    echo -e "${YELLOW}Updating Cargo.lock...${NC}"
    cargo update --workspace --quiet
    echo -e "${GREEN}✓ Updated Cargo.lock${NC}"
    
    # Verify build
    verify_build
    
    # Git operations
    create_git_commit_and_tag "$new_version"
    
    echo ""
    echo -e "${GREEN}✓ Version bump complete!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Review the changes: git show"
    echo "  2. Push the commit: git push origin main"
    echo "  3. Push the tag: git push origin v${new_version}"
    echo "  4. GitHub Actions will automatically create the release"
}

main "$@"
