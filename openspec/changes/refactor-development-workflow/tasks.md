## 1. OpenSpec workflow policy

- [x] 1.1 Add `development-workflow` delta spec covering topic branches, release merges, back-merges, OpenSpec lifecycle, and CI protection migration.
- [ ] 1.2 Validate `refactor-development-workflow` with strict OpenSpec validation in CI.

## 2. Repository guidance synchronization

- [x] 2.1 Update `AGENTS.md` so implementation PR merge precedes archive PR and release/back-merge use merge commits.
- [x] 2.2 Update `openspec/project.md` Git Workflow section to match the same policy.
- [x] 2.3 Update `CONTRIBUTING.md` with the same branch/OpenSpec lifecycle and merge methods.
- [x] 2.4 Update `RELEASE.md` so release and back-merge are PR-based merge commits.
- [x] 2.5 Update `docs/STATUS.md` with the workflow correction, current change status, and follow-up order.

## 3. Verification

- [x] 3.1 Change `openspec-check` to `openspec validate --all --strict --no-interactive`; the PR run is the executable validation gate.
- [ ] 3.2 Confirm the PR `openspec-check`, `lint`, and other existing required checks are green.
- [x] 3.3 Confirm no runtime code, release workflow, or branch-protection server settings were changed by this PR. The only CI behavior change is strict OpenSpec validation.
- [ ] 3.4 Open PR to `develop` with conventional-commit title and document the follow-up order: `refactor-ci-critical-path` → `add-macos-compatibility-matrix` → `add-windows-wsl2-platform`.
