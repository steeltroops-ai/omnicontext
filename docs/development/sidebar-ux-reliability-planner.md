# Sidebar UX and Reliability Planner

## Objective

Ship a clear, reliable OmniContext sidebar experience where users can identify the exact indexed repository folder, navigate quickly to repo/index artifacts, and verify indexing health without ambiguity.

## Scope

- VS Code sidebar repository card UX
- Repo-level actions and reporting
- Indexing reliability feedback and observability
- Local update and MCP sync validation workflow

## Execution Plan

1. Sidebar repository clarity

- [x] Show exact repository path in the repository card
- [x] Expose quick actions for each indexed repo
- [x] Add per-repo report generation from sidebar
- [ ] Add explicit stale-index warning badge based on age threshold

1. Repository actions hardening

- [x] Reveal repo folder in OS explorer
- [x] Open repo in new VS Code window
- [x] Copy repo path to clipboard
- [x] Reveal OmniContext index data folder
- [ ] Add "Re-index this repo" action for non-active repos

1. UX polish and safety

- [x] Escape repo path/name in webview rendering
- [x] Prevent ambiguous folder-only display in Engine Status
- [ ] Add small tooltip/help text for each repo action button
- [ ] Add lightweight empty-state guidance for first-time users

1. Reliability and regression audits

- [x] Audit pass 1: compile + static sanity checks
- [x] Audit pass 2: runtime command flow checks (sidebar actions)
- [x] Audit pass 3: final UX copy and edge-case review

1. Update and MCP readiness

- [ ] Validate `omnicontext.updateBinary` from sidebar and command palette
- [ ] Verify daemon restart and sidebar reconnection behavior after update
- [ ] Verify MCP sync command works post-update
- [ ] Confirm no stale pending IPC requests after update/reconnect

## Validation Checklist

- [x] `bun run compile` succeeds in `editors/vscode`
- [x] No TypeScript errors in changed extension files
- [ ] Sidebar displays full repo path for active workspace
- [ ] Per-repo actions execute and log activity entries
- [ ] Repo report opens with artifact inventory
- [ ] No workflow-breaking changes introduced

## Notes

- Preserve existing user-local changes outside this scope (for example `.gitignore`).
- Keep sidebar actions non-destructive by default; destructive cleanup remains explicit and confirmed.
