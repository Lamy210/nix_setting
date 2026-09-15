## 1. Proposal / specification

- [x] 1.1 Verify current Apple Xcode/macOS support and GitHub runner image availability using primary sources.
- [x] 1.2 Define stable compatibility/current lanes, preview canary isolation, release toolchain pinning, failure modes, and rollback in `design.md`.
- [x] 1.3 Add `development-workflow` delta requirements for explicit macOS/Xcode contracts.
- [x] 1.4 Run `openspec validate add-macos-compatibility-matrix --strict` and `openspec validate --all --strict`.

## 2. Stable macOS matrix

- [ ] 2.1 Replace the single mutable `macos-latest` `macos-check` worker with a `macos-stable` matrix containing `macos-15` + Xcode 26.3 and `macos-26` + Xcode 26.6.
- [ ] 2.2 Set lane-local `DEVELOPER_DIR` and add OS / architecture / Xcode / macOS SDK diagnostics before builds.
- [ ] 2.3 Keep both stable lanes observable with `strategy.fail-fast: false`.
- [ ] 2.4 Convert `macos-check` into an `if: always()` fail-closed aggregator that succeeds only when the stable matrix succeeds.
- [ ] 2.5 Keep `macos-check` out of `ci-required` and do not change GitHub branch-protection server settings in this change.

## 3. Release host/toolchain pinning

- [ ] 3.1 Pin `release-artifact-check` to `macos-26` + Xcode 26.6 and log the selected toolchain.
- [ ] 3.2 Pin the macOS CLI build in `.github/workflows/release.yml` to `macos-26` + Xcode 26.6.
- [ ] 3.3 Pin the DMG build in `.github/workflows/release.yml` to `macos-26` + Xcode 26.6.
- [ ] 3.4 Preserve the existing shared macOS release build scripts so PR artifact gates and tag releases continue using identical build semantics.

## 4. Xcode 27 preview canary

- [ ] 4.1 Add a separate `xcode-27` Public Preview canary that is not a dependency of `macos-check`, `ci-required`, or release jobs.
- [ ] 4.2 Run the preview canary only for `develop` push, scheduled, or manual workflow execution; do not run it for pull requests.
- [ ] 4.3 Log actual OS / architecture / Xcode / SDK identity so the `xcode-27` image's underlying macOS changes remain visible.

## 5. Regression tests and documentation

- [ ] 5.1 Extend `tests/ci-scripts.bats` to reject active `runs-on: macos-latest` in `check.yml` / `release.yml`.
- [ ] 5.2 Add contract checks for stable runner labels, pinned Xcode 26.3/26.6 paths, preview-canary isolation, and PR-event exclusion.
- [ ] 5.3 Update `docs/STATUS.md` to mark CI critical-path work archived and this change active.
- [ ] 5.4 Update `AGENTS.md` current-work section without changing the established Git/OpenSpec workflow policy.

## 6. Verification / measurement

- [ ] 6.1 Run workflow static gates (`actionlint`, shell/Bats contract tests) and relevant local tests.
- [ ] 6.2 Confirm both stable macOS lanes complete successfully on the implementation PR and `macos-check` aggregates them fail-closed.
- [ ] 6.3 Confirm `release-artifact-check` succeeds on `macos-26` + Xcode 26.6 and release workflow uses the same explicit host/toolchain contract.
- [ ] 6.4 Confirm the Xcode 27 preview canary is absent from pull-request execution and cannot affect current required checks.
- [ ] 6.5 Record before/after macOS wall-clock and runner elapsed totals; if stable matrix total persistently exceeds roughly 2.5x the previous single lane, re-evaluate compatibility-lane cadence/coverage before declaring completion.
- [ ] 6.6 Run full required CI, shadow `ci-required`, and final diff review; confirm no server-side branch-protection settings changed.
- [ ] 6.7 Open the implementation PR to `develop`, obtain review, and squash merge only after latest-head checks are green.
- [ ] 6.8 After implementation merge, create `chore/archive-add-macos-compatibility-matrix` from latest `develop`, archive with spec sync, validate, and merge the separate archive PR.
