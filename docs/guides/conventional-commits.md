# Conventional Commits Guide

OmniContext uses [Conventional Commits](https://www.conventionalcommits.org/) for automatic version bumping and changelog generation.

## Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

## Commit Types

### `feat:` - New Features (Minor Bump)

Adds new functionality. Triggers a **minor** version bump (0.1.0 → 0.2.0).

**Examples**:
```bash
git commit -m "feat: add cross-encoder reranking support"
git commit -m "feat(search): implement intent-aware context delivery"
git commit -m "feat(embedder): add batch embedding with retry logic"
```

### `fix:` - Bug Fixes (Patch Bump)

Fixes a bug. Triggers a **patch** version bump (0.1.0 → 0.1.1).

**Examples**:
```bash
git commit -m "fix: resolve ONNX Runtime version mismatch"
git commit -m "fix(parser): handle empty files correctly"
git commit -m "fix(index): prevent race condition in concurrent writes"
```

### `feat!:` or `BREAKING CHANGE:` - Breaking Changes (Major Bump)

Introduces breaking changes. Triggers a **major** version bump (0.1.0 → 1.0.0).

**Examples**:
```bash
git commit -m "feat!: change MCP tool API to use async/await"

git commit -m "feat: rewrite chunker with CAST algorithm

BREAKING CHANGE: Chunker API has changed, old chunk format is incompatible"
```

### Other Types (No Version Bump)

These don't trigger automatic releases but are included in changelog:

- `chore:` - Maintenance tasks (dependencies, configs)
- `docs:` - Documentation changes
- `style:` - Code style changes (formatting, whitespace)
- `refactor:` - Code refactoring without behavior change
- `perf:` - Performance improvements
- `test:` - Adding or updating tests
- `ci:` - CI/CD changes

**Examples**:
```bash
git commit -m "chore: update dependencies to latest versions"
git commit -m "docs: add installation guide for Windows"
git commit -m "refactor: simplify search ranking algorithm"
git commit -m "perf: optimize vector index memory usage"
git commit -m "test: add integration tests for MCP server"
git commit -m "ci: add security audit workflow"
```

## Scopes (Optional)

Scopes provide additional context about what part of the codebase changed:

**Common scopes**:
- `parser` - AST parsing and language support
- `chunker` - Semantic chunking
- `embedder` - Embedding generation
- `index` - Database and indexing
- `search` - Search and ranking
- `graph` - Dependency graph
- `mcp` - MCP server
- `cli` - Command-line interface
- `daemon` - Background daemon
- `vector` - Vector index

**Examples**:
```bash
git commit -m "feat(parser): add support for Ruby language"
git commit -m "fix(embedder): handle model download failures gracefully"
git commit -m "perf(vector): implement quantized vector search"
```

## How Automatic Versioning Works

### 1. You Make Commits

Use conventional commit format when committing:

```bash
git commit -m "feat: add new search feature"
git commit -m "fix: resolve indexing bug"
git commit -m "docs: update README"
```

### 2. Push to Main

```bash
git push origin main
```

### 3. Workflow Analyzes Commits

The `auto-release.yml` workflow:
1. Fetches all commits since last release
2. Analyzes commit messages
3. Determines version bump type:
   - **Major**: If any commit has `!` or `BREAKING CHANGE:`
   - **Minor**: If any commit starts with `feat:`
   - **Patch**: If any commit starts with `fix:`
   - **None**: If only `chore:`, `docs:`, etc.

### 4. Automatic Release

If a version bump is needed:
1. Updates all `Cargo.toml` files
2. Generates changelog from commits
3. Updates `CHANGELOG.md`
4. Creates commit: `chore(release): bump version to X.Y.Z`
5. Creates git tag: `vX.Y.Z`
6. Pushes commit and tag
7. Creates GitHub release with changelog
8. Triggers build workflow to create binaries

## Example Workflow

### Scenario: Adding a new feature and fixing a bug

```bash
# 1. Create feature branch
git checkout -b feature/cross-encoder

# 2. Make changes and commit with conventional format
git add crates/omni-reranker/
git commit -m "feat(search): add cross-encoder reranking

Implements two-stage retrieval with ONNX cross-encoder model.
Expected 40-60% MRR improvement."

# 3. Fix a bug you found
git add crates/omni-core/src/embedder/
git commit -m "fix(embedder): handle empty input gracefully"

# 4. Merge to main
git checkout main
git merge feature/cross-encoder

# 5. Push to main
git push origin main

# 6. Automatic workflow runs:
# - Detects: 1 feat + 1 fix
# - Determines: MINOR bump (feat takes precedence)
# - Bumps: 0.1.0 → 0.2.0
# - Creates release v0.2.0 with changelog
```

### Scenario: Breaking change

```bash
# 1. Make breaking change
git add crates/omni-mcp/
git commit -m "feat!: change MCP tool response format

BREAKING CHANGE: All MCP tools now return structured JSON
instead of plain text. Clients must update to handle new format."

# 2. Push to main
git push origin main

# 3. Automatic workflow runs:
# - Detects: BREAKING CHANGE
# - Determines: MAJOR bump
# - Bumps: 0.2.0 → 1.0.0
# - Creates release v1.0.0 with breaking change notice
```

## Manual Override

If you need to force a specific version bump:

1. Go to GitHub Actions
2. Select "Auto Release" workflow
3. Click "Run workflow"
4. Choose bump type: `major`, `minor`, `patch`, or `auto`
5. Click "Run workflow"

## Checking What Will Be Released

Before pushing, check what version bump will happen:

```bash
# See commits since last tag
git log $(git describe --tags --abbrev=0)..HEAD --oneline

# Check for conventional commits
git log $(git describe --tags --abbrev=0)..HEAD --pretty=format:"%s" | grep -E "^(feat|fix|feat!):"
```

## Best Practices

### ✅ Good Commits

```bash
git commit -m "feat: add quantized vector search"
git commit -m "fix(parser): handle Unicode characters in Python"
git commit -m "feat(mcp): implement context lineage tracking"
git commit -m "perf(index): reduce memory usage by 40%"
```

### ❌ Bad Commits

```bash
git commit -m "update stuff"
git commit -m "fix bug"
git commit -m "WIP"
git commit -m "asdf"
```

### Tips

1. **Be specific**: Describe what changed, not how
2. **Use present tense**: "add feature" not "added feature"
3. **Keep it short**: First line under 72 characters
4. **Add body for complex changes**: Explain why, not what
5. **Reference issues**: Include issue numbers in footer

**Example with body**:
```bash
git commit -m "feat(search): implement graph-based relevance propagation

Uses PageRank-style algorithm to boost related code chunks.
Improves search relevance by propagating scores through
dependency graph.

Closes #123"
```

## Skipping CI

If you need to push without triggering workflows:

```bash
git commit -m "docs: update README [skip ci]"
```

## Troubleshooting

### No release created after push

**Possible reasons**:
1. No conventional commits since last release
2. Only `chore:`, `docs:`, `style:` commits (don't trigger releases)
3. Workflow failed (check GitHub Actions logs)

**Solution**: Check commit messages follow conventional format

### Wrong version bump

**Example**: Expected minor but got patch

**Reason**: No `feat:` commits, only `fix:` commits

**Solution**: Use correct commit type for your changes

### Want to undo a release

```bash
# Delete tag locally and remotely
git tag -d v0.2.0
git push origin :refs/tags/v0.2.0

# Delete release on GitHub
# Go to Releases → Click release → Delete release

# Revert commit
git revert HEAD
git push origin main
```

## Resources

- [Conventional Commits Specification](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)
