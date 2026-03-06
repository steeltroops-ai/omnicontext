# System Engineering Constraints & Contribution Protocols

OmniContext enforces deterministic build flows, formal syntax structures, and semantic deployment checks via pre-commit and CI chains. Adhere rigidly.

## Standard Formats

Semantic versioning triggers via **Conventional Commits**:

- `feat:` (Minor jump)
- `fix:` (Patch jump)
- `feat!:` or `BREAKING CHANGE:` (Major ABI jump)

Do not bypass formatting or naming standards. See `docs/CONVENTIONAL_COMMITS.md`.

## Quality Constraints

Code must compile warning-free and execute with zero failing conditions:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo test --workspace`

Pre-commit hooks block execution of invalid states. Use automation setup points to bind these locally to your `.git/hooks` namespace:

- **Unix**: `./scripts/setup-dev.sh`
- **Windows**: `.\scripts\setup-dev.ps1`

## Continuous Integration Policy

The `steeltroops-ai/omnicontext/ci.yml` matrix prevents non-compliant branches from executing `main` merges. CI validates correctness across `msvc` and `gnu` targets simultaneously.

## Module Structure Principles

See `.kiro/steering/structure.md` and `.kiro/steering/tech.md`. Submodules must isolate responsibilities (e.g., semantic resolution via `jina-embeddings` within `omni-core`).

## Protocol Compliance

Commit streams explicitly. Document test matrices. Avoid regressions.
