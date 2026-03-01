---
inclusion: always
---

# File Organization & Management Rules

## Core Principle: Update, Don't Duplicate

NEVER create versioned files (*-fixed.*, *-new.*, *-v2.*, *-updated.*). Always update the original file in place.

## Directory Structure

```
docs/                           # Technical documentation
├── ADR.md                      # Architecture decisions
├── SUPPORTED_LANGUAGES.md      # Language support matrix
├── TESTING_STRATEGY.md         # Test approach
├── PROGRESS_SUMMARY.md         # Current progress (update in place)
└── reports/                    # Generated reports (gitignored)

distribution/                   # End-user installation
├── install.sh                  # Unix/Linux/Mac installer
├── install.ps1                 # Windows installer
├── update.ps1                  # Update script (all platforms)
├── uninstall.ps1               # Uninstaller
├── homebrew/                   # Homebrew formula
│   └── omnicontext.rb
└── scoop/                      # Scoop manifest (Windows)
    └── omnicontext.json

scripts/                        # Development scripts (contributors only)
├── install-mcp.ps1             # Build & install MCP from source
├── install-mcp-quick.ps1       # Quick config update (no build)
├── test-mcp.ps1                # MCP integration tests
└── test-mcp-protocol.py        # Protocol compliance tests

tests/                          # Test fixtures and data
├── bench/                      # Benchmark tests
└── fixtures/                   # Test data by language

.kiro/steering/                 # AI assistant rules
├── product.md
├── tech.md
├── structure.md
├── competitive-advantage.md
└── project-organization.md
```

## Installation Flow

**End Users** should use ONE method:
1. Direct install: `install.ps1` (Windows) or `install.sh` (Unix)
2. Package manager: Scoop (Windows) or Homebrew (macOS/Linux)

**Developers** use scripts in `scripts/` for local development.

See `INSTALL.md` for complete installation guide.

## File Placement Decision Tree

Before creating any file, ask these questions in order:

1. Does a similar file exist? → Update it instead
2. Is this temporary? → Use `docs/local/` or `docs/reports/`
3. Is this generated? → Ensure it's gitignored
4. Where does it belong?
   - Technical docs → `docs/`
   - Release installers → `distribution/`
   - Development scripts → `scripts/`
   - Test data → `tests/fixtures/`
   - Steering rules → `.kiro/steering/`

## Naming Conventions

| File Type | Convention | Examples |
|-----------|------------|----------|
| Major docs | SCREAMING_SNAKE_CASE | `PROGRESS_SUMMARY.md`, `INSTALLATION_COMPLETE.md` |
| Technical docs | kebab-case | `embedding-coverage-fix.md`, `graph-loading-fix.md` |
| Scripts | kebab-case + action | `install-mcp.ps1`, `test-mcp.ps1`, `benchmark.sh` |
| Reports | type-timestamp | `benchmark-2026-03-01.md`, `performance-{date}.json` |
| Source code | snake_case | `model_manager.rs`, `search_engine.rs` |

## Gitignore Rules

### Always Gitignore
- `docs/reports/` - Generated benchmark/test reports
- `docs/local/` - Local development documentation
- `*.log` - All log files
- `target/` - Rust build output
- `.omnicontext/` - Index data
- `models/*.onnx` - Downloaded models
- IDE configs (`.vscode/`, `.idea/`)
- OS files (`.DS_Store`, `Thumbs.db`)

### Always Commit
- Source code (`.rs`, `Cargo.toml`)
- Technical documentation in `docs/`
- Release installers in `distribution/`
- Development scripts in `scripts/`
- Configuration templates
- `.gitignore`, CI/CD configs

## Project Root Cleanliness

Only these files belong in project root:
- `Cargo.toml` - Workspace manifest
- `README.md`, `LICENSE`, `CHANGELOG.md` - Project metadata
- `.gitignore`, `.dockerignore` - Configuration

Everything else goes in subdirectories:
- Installation scripts → `distribution/`
- Test scripts → `tests/`
- Documentation → `docs/`

## Update vs Create Decision Matrix

| Scenario | Action | Target File |
|----------|--------|-------------|
| Progress update | Update | `PROGRESS_SUMMARY.md` |
| Bug fix documentation | Update | Relevant technical doc |
| Architecture change | Update | `ADR.md` |
| Performance improvement | Update | `PROGRESS_SUMMARY.md` |
| Major milestone | Create | `PHASE_X_COMPLETE.md` |
| New feature documentation | Create | `docs/NEW_FEATURE.md` |

## Critical Anti-Patterns

### ❌ NEVER Create These

```
❌ install-mcp-fixed.ps1        → Update install-mcp.ps1
❌ PROGRESS_SUMMARY_V2.md       → Update PROGRESS_SUMMARY.md
❌ config-new.toml              → Update config.toml
❌ installation-analysis.md     → Update PROGRESS_SUMMARY.md
❌ bug-fix-summary.md           → Update relevant doc
❌ doc1.md, notes.txt, temp.sh  → Use descriptive names
```

### ✅ Always Do This

```
✅ Update original files in place
✅ Use descriptive, professional names
✅ Place files in correct directories
✅ Search before creating
✅ Gitignore generated content
```

## Pre-Creation Checklist

Before creating ANY file:

1. Search for existing similar files (`ls docs/ distribution/ scripts/ tests/ | grep -i "topic"`)
2. Confirm this is NOT a versioned/fixed variant
3. Confirm this is NOT a redundant analysis document
4. Verify file name is professional and descriptive
5. Verify correct directory per structure above
6. Confirm it will be maintained OR properly gitignored

## Pre-Commit Checklist

Before committing:

1. No log files, build artifacts, or temp files
2. No IDE-specific configs
3. No "*-fixed.*", "*-new.*", or "*-v2.*" files
4. No redundant analysis documents
5. Files in correct directories
6. Names follow conventions
7. No duplicate content

## Steering File Management

Create new steering files only for:
- New major subsystems (e.g., `api-design.md`)
- New language support (e.g., `python-parser.md`)
- New integrations (e.g., `vscode-extension.md`)
- New deployment targets (e.g., `docker-deployment.md`)

Update existing steering files when:
- Project structure changes
- Tech stack evolves
- Best practices are refined
- New conventions are established

## Summary: Absolute Rules

1. NEVER create versioned files - UPDATE THE ORIGINAL
2. NEVER create redundant analysis documents - UPDATE EXISTING DOCS
3. ALWAYS search before creating
4. ALWAYS use professional, descriptive names
5. ALWAYS place files in correct directories
6. ALWAYS gitignore generated content
7. ALWAYS keep project root clean

This is enterprise software. Every file must have a clear purpose, professional name, and correct location.
