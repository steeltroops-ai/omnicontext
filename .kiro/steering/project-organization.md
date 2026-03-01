---
inclusion: always
---

# OmniContext Project Organization Rules

## File Placement Principles

### CRITICAL: Don't Create Redundant Files

1. **Enhance existing files** instead of creating new ones
2. **Update documentation** in place rather than creating new versions
3. **Consolidate related content** into single files
4. **Delete obsolete files** when creating replacements

### Documentation Structure

```
docs/
├── ADR.md                          # Architecture Decision Records (keep)
├── SUPPORTED_LANGUAGES.md          # Language support matrix (keep)
├── TESTING_STRATEGY.md             # Test approach (keep)
├── SECURITY_THREAT_MODEL.md        # Security analysis (keep)
├── CONCURRENCY_ARCHITECTURE.md     # Concurrency patterns (keep)
├── ERROR_RECOVERY.md               # Error handling (keep)
├── EMBEDDING_COVERAGE_FIX.md       # Technical fix documentation (keep)
├── GRAPH_LOADING_FIX.md            # Technical fix documentation (keep)
├── PHASE_A_COMPLETE.md             # Milestone documentation (keep)
├── PROGRESS_SUMMARY.md             # Current progress tracking (keep)
└── reports/                        # Generated reports (gitignored)
    ├── benchmark-*.md              # Benchmark results
    ├── coverage-*.html             # Test coverage reports
    └── performance-*.json          # Performance metrics

docs/local/                         # Local-only docs (gitignored)
├── DEVELOPMENT_ROADMAP.md          # Internal planning
├── EMBEDDING_MODEL_EVALUATION.md   # Research notes
├── OMNICONTEXT_PRODUCT_SPEC.md     # Product planning
└── WEBSITE_DESIGN_SPEC.md          # Marketing materials
```

### Scripts and Tools

```
/ (project root)
├── install-mcp.ps1                 # Installation script (keep)
├── install-mcp-quick.ps1           # Quick install (keep)
├── test-mcp.ps1                    # Test suite (keep)
├── INSTALLATION_COMPLETE.md        # Installation guide (keep)
└── scripts/                        # Additional scripts
    ├── benchmark.sh                # Run benchmarks
    ├── release.sh                  # Release automation
    └── setup-dev.sh                # Dev environment setup
```

### Logs and Temporary Files

```
/ (project root)
├── err.log                         # Error logs (gitignored)
├── debug.log                       # Debug logs (gitignored)
└── *.log                           # All logs (gitignored)
```

## Git Ignore Rules

### What Should Be Gitignored

1. **Generated Reports**
   - `docs/reports/` - Benchmark and test reports
   - `docs/local/` - Local development docs
   - `*.log` - All log files

2. **Build Artifacts**
   - `target/` - Rust build output
   - `*.exe`, `*.dll`, `*.so` - Binaries

3. **Runtime Data**
   - `.omnicontext/` - Index data
   - `models/*.onnx` - Downloaded models

4. **IDE and OS**
   - `.vscode/`, `.idea/` - IDE configs
   - `.DS_Store`, `Thumbs.db` - OS files

5. **Temporary Files**
   - `*.tmp`, `*.temp` - Temporary files
   - `*.swp`, `*.swo` - Editor swap files

### What Should Be Committed

1. **Source Code**
   - All `.rs` files
   - `Cargo.toml` files
   - Configuration templates

2. **Documentation**
   - Technical docs in `docs/`
   - Steering rules in `.kiro/steering/`
   - README files

3. **Scripts**
   - Installation scripts
   - Build scripts
   - Test scripts

4. **Configuration**
   - `.gitignore`
   - CI/CD configs
   - Default configs

## File Creation Rules

### Before Creating a New File, Ask:

1. **Does a similar file already exist?**
   - If yes, enhance it instead
   - Example: Update `PROGRESS_SUMMARY.md` instead of creating `PROGRESS_SUMMARY_V2.md`

2. **Is this file temporary or permanent?**
   - Temporary → Put in `docs/reports/` or `docs/local/`
   - Permanent → Put in appropriate permanent location

3. **Should this be version controlled?**
   - Generated content → Gitignore it
   - Source content → Commit it

4. **Where does this logically belong?**
   - Technical docs → `docs/`
   - Steering rules → `.kiro/steering/`
   - Scripts → Project root or `scripts/`
   - Reports → `docs/reports/`

### Naming Conventions

1. **Documentation Files**
   - Use SCREAMING_SNAKE_CASE for major docs: `PROGRESS_SUMMARY.md`
   - Use kebab-case for technical docs: `embedding-coverage-fix.md`
   - Use descriptive names: `INSTALLATION_COMPLETE.md` not `DONE.md`

2. **Script Files**
   - Use kebab-case: `install-mcp.ps1`
   - Include action in name: `test-mcp.ps1` not `mcp.ps1`
   - Use appropriate extension: `.ps1`, `.sh`, `.py`

3. **Report Files**
   - Include timestamp: `benchmark-2026-03-01.md`
   - Include type: `coverage-report.html`
   - Use consistent format: `performance-{date}.json`

## Documentation Update Rules

### When to Update vs Create

| Scenario | Action | Example |
|----------|--------|---------|
| Progress update | Update existing | Update `PROGRESS_SUMMARY.md` |
| New feature docs | Create new | Create `NEW_FEATURE.md` |
| Bug fix docs | Update existing | Update relevant technical doc |
| Milestone complete | Create new | Create `PHASE_B_COMPLETE.md` |
| Architecture change | Update ADR | Update `ADR.md` |
| Performance improvement | Update existing | Update `PROGRESS_SUMMARY.md` |

### Documentation Lifecycle

1. **Draft** → `docs/local/` (not committed)
2. **Review** → Move to `docs/` (committed)
3. **Obsolete** → Delete or archive
4. **Generated** → `docs/reports/` (not committed)

## Enterprise Software Standards

### File Organization Principles

1. **Predictability**
   - Files are where you expect them
   - Consistent naming across project
   - Clear hierarchy

2. **Maintainability**
   - No duplicate content
   - Clear ownership
   - Easy to find and update

3. **Cleanliness**
   - No clutter in project root
   - Logs and temp files gitignored
   - Build artifacts excluded

4. **Professionalism**
   - No "test123.txt" files
   - No "backup_old_final_v2.md" files
   - No scattered scripts everywhere

### Project Root Cleanliness

**Allowed in Project Root:**
- `Cargo.toml` - Workspace manifest
- `README.md` - Project overview
- `LICENSE` - License file
- `CHANGELOG.md` - Version history
- `INSTALLATION_COMPLETE.md` - Installation guide
- `install-*.ps1` - Installation scripts
- `test-*.ps1` - Test scripts
- `.gitignore` - Git configuration
- `.dockerignore` - Docker configuration

**NOT Allowed in Project Root:**
- Log files (→ gitignore)
- Temporary files (→ gitignore)
- Test data (→ `tests/fixtures/`)
- Documentation (→ `docs/`)
- Build artifacts (→ `target/`)
- IDE configs (→ gitignore)

## Steering Rules Organization

### Current Steering Files

```
.kiro/steering/
├── product.md                  # Product overview and principles
├── tech.md                     # Tech stack and commands
├── structure.md                # Project structure and conventions
├── competitive-advantage.md    # Strategy and roadmap
└── project-organization.md     # This file - file management rules
```

### When to Create New Steering Files

Create a new steering file when:
1. A new major subsystem is added (e.g., `api-design.md`)
2. A new language is supported (e.g., `python-parser.md`)
3. A new integration is added (e.g., `vscode-extension.md`)
4. A new deployment target is added (e.g., `docker-deployment.md`)

### When to Update Existing Steering Files

Update existing files when:
1. Project structure changes
2. Tech stack evolves
3. Best practices are refined
4. New conventions are established

## Workflow for File Management

### Adding New Documentation

```bash
# 1. Check if similar doc exists
ls docs/ | grep -i "topic"

# 2. If exists, update it
# If not, create in appropriate location

# 3. If temporary/local, use docs/local/
# If permanent, use docs/

# 4. Update .gitignore if needed

# 5. Commit with clear message
git add docs/NEW_DOC.md
git commit -m "docs: add NEW_DOC for feature X"
```

### Cleaning Up Old Files

```bash
# 1. Identify obsolete files
# 2. Check if content is still needed
# 3. If needed, merge into current docs
# 4. Delete obsolete files
git rm docs/OBSOLETE.md
git commit -m "docs: remove obsolete OBSOLETE.md (merged into CURRENT.md)"
```

### Managing Reports

```bash
# 1. Generate reports in docs/reports/
cargo bench > docs/reports/benchmark-$(date +%Y-%m-%d).md

# 2. Reports are gitignored automatically
# 3. Keep only recent reports (last 30 days)
find docs/reports/ -mtime +30 -delete
```

## Anti-Patterns to Avoid

### ❌ DON'T DO THIS

1. **Creating versioned files**
   ```
   ❌ PROGRESS_SUMMARY_V1.md
   ❌ PROGRESS_SUMMARY_V2.md
   ❌ PROGRESS_SUMMARY_FINAL.md
   ✅ PROGRESS_SUMMARY.md (update in place)
   ```

2. **Scattering files everywhere**
   ```
   ❌ /test.log
   ❌ /debug_output.txt
   ❌ /my_script.sh
   ✅ Use appropriate directories
   ```

3. **Committing generated files**
   ```
   ❌ git add target/
   ❌ git add docs/reports/
   ❌ git add *.log
   ✅ These should be gitignored
   ```

4. **Creating redundant documentation**
   ```
   ❌ INSTALLATION.md + INSTALL_GUIDE.md + SETUP.md
   ✅ INSTALLATION_COMPLETE.md (single source of truth)
   ```

5. **Unclear file names**
   ```
   ❌ doc1.md, notes.txt, stuff.md
   ✅ EMBEDDING_COVERAGE_FIX.md (descriptive)
   ```

## Checklist for File Operations

### Before Creating a File

- [ ] Does a similar file already exist?
- [ ] Is this the right location?
- [ ] Should this be gitignored?
- [ ] Is the name descriptive and follows conventions?
- [ ] Will this file be maintained or is it temporary?

### Before Committing Files

- [ ] No log files included
- [ ] No build artifacts included
- [ ] No temporary files included
- [ ] No IDE-specific configs included
- [ ] File is in the correct directory
- [ ] File name follows conventions
- [ ] Content is not redundant with existing files

### Before Deleting Files

- [ ] Is the content still needed elsewhere?
- [ ] Are there any references to this file?
- [ ] Should this be archived instead of deleted?
- [ ] Is this file tracked in git?

## Summary

**Golden Rules:**
1. **Enhance, don't duplicate** - Update existing files instead of creating new versions
2. **Organize, don't scatter** - Put files where they belong
3. **Gitignore generated content** - Reports, logs, and build artifacts
4. **Keep root clean** - Only essential files in project root
5. **Be professional** - No test files, no clutter, no mess

**Remember:** As enterprise software, every file should have a clear purpose and location. If you can't explain why a file exists and where it belongs, it probably shouldn't be there.
