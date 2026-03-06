# OmniContext Distribution

This directory manages the packaging, distribution manifests, and cross-platform installation scripts for OmniContext.

## Components

- `install.ps1` / `install.sh`: Native remote execution scripts designed for zero-dependency bootstrapping.
- `update.ps1` / `update.sh`: Lifecycle management scripts for seamless version syncing.
- `uninstall.ps1` / `uninstall.sh`: Uninstallation scripts with data preservation arguments.
- `homebrew/`: Formula definitions for macOS/Linux (`brew tap steeltroops-ai/omnicontext`).
- `scoop/`: App manifests for Windows (`scoop bucket add omnicontext ...`).

## Release Automation

Asset distribution is governed entirely by CI/CD (`.github/workflows/release.yml`). Upon version bump:

1. Cross-compilation targets build release binaries.
2. Archives (`.zip`, `.tar.gz`) are generated alongside SHA256 checksums.
3. Package manifests are automatically patched with exact hashes and version tags.
4. Assets are published to the GitHub Release.

For end-user installation, reference the root [`INSTALL.md`](../INSTALL.md).
