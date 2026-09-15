## 1. OpenSpec workflow policy

- [ ] 1.1 Add `development-workflow` delta spec covering topic branches, release merges, back-merges, OpenSpec lifecycle, and CI protection migration.
- [ ] 1.2 Validate `refactor-development-workflow` with `openspec validate refactor-development-workflow --strict`.

## 2. Repository guidance synchronization

- [ ] 2.1 Update `AGENTS.md` so implementation PR merge precedes archive PR and release/back-merge use merge commits.
- [ ] 2.2 Update `openspec/project.md` Git Workflow section to match the same policy.
- [ ] 2.3 Update `docs/STATUS.md` with the workflow correction and current change status.

## 3. Verification

- [ ] 3.1 Run `openspec validate --all --strict` and fix workflow-spec validation errors introduced by this change.
- [ ] 3.2 Run repository documentation/config lint gates affected by the files changed in this PR.
- [ ] 3.3 Confirm no runtime, release workflow, or branch-protection settings were changed by this PR.
- [ ] 3.4 Open PR to `develop` with conventional-commit title and document the follow-up order: `refactor-ci-critical-path` → `add-macos-compatibility-matrix` → `add-windows-wsl2-platform`.
