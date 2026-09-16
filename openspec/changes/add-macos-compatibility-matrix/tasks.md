## 1. Proposal / specification

- [x] 1.1 Verify current Apple Xcode/macOS support and GitHub runner image availability using primary sources.
- [x] 1.2 Define stable compatibility/current lanes, preview canary isolation, release toolchain pinning, failure modes, and rollback in `design.md`.
- [x] 1.3 Add `development-workflow` delta requirements for explicit macOS/Xcode contracts.
- [x] 1.4 Run `openspec validate add-macos-compatibility-matrix --strict` and `openspec validate --all --strict`.

## 2. Stable macOS matrix

- [x] 2.1 Replace the single mutable `macos-latest` `macos-check` worker with a `macos-stable` matrix containing `macos-15` + Xcode 26.3 and `macos-26` + Xcode 26.6.
- [x] 2.2 Set lane-local `DEVELOPER_DIR` and add OS / architecture / Xcode / macOS SDK diagnostics before builds.
- [x] 2.3 Keep both stable lanes observable with `strategy.fail-fast: false`.
- [x] 2.4 Convert `macos-check` into an `if: always()` fail-closed aggregator that succeeds only when the stable matrix succeeds.
- [x] 2.5 Keep `macos-check` out of `ci-required` and do not change GitHub branch-protection server settings in this change.

## 3. Release host/toolchain pinning

- [x] 3.1 Pin `release-artifact-check` to `macos-26` + Xcode 26.6 and log the selected toolchain.
- [x] 3.2 Pin the macOS CLI build in `.github/workflows/release.yml` to `macos-26` + Xcode 26.6.
- [x] 3.3 Pin the DMG build in `.github/workflows/release.yml` to `macos-26` + Xcode 26.6.
- [x] 3.4 Preserve the existing shared macOS release build scripts so PR artifact gates and tag releases continue using identical build semantics.

## 4. Xcode 27 preview canary

- [x] 4.1 Add a separate `xcode-27` Public Preview canary that is not a dependency of `macos-check`, `ci-required`, or release jobs.
- [x] 4.2 Run the preview canary only for `develop` push, scheduled, or manual workflow execution; do not run it for pull requests.
- [x] 4.3 Log actual OS / architecture / Xcode / SDK identity so the `xcode-27` image's underlying macOS changes remain visible.

## 5. Regression tests and documentation

- [x] 5.1 Extend `tests/ci-scripts.bats` to reject active `runs-on: macos-latest` in `check.yml` / `release.yml`.
- [x] 5.2 Add contract checks for stable runner labels, pinned Xcode 26.3/26.6 paths, preview-canary isolation, and PR-event exclusion.
- [x] 5.3 Update `docs/STATUS.md` to mark CI critical-path work archived and this change active.
- [x] 5.4 Update `AGENTS.md` current-work section without changing the established Git/OpenSpec workflow policy.

## 6. Verification / measurement

- [x] 6.1 Run workflow static gates (`actionlint`, shell/Bats contract tests) and relevant tests; run #424 passed OpenSpec strict, actionlint, ShellCheck, Bats, Rust, flake and E2E gates.
- [x] 6.2 Confirm both stable macOS lanes complete successfully on the implementation PR and `macos-check` aggregates them fail-closed; run #424 passed both lanes and the aggregator, while an earlier cancelled run demonstrated the non-success path fails closed.
- [x] 6.3 Confirm `release-artifact-check` succeeds on `macos-26` + Xcode 26.6 and release workflow uses the same explicit host/toolchain contract; run #424 passed CLI, DMG and release-metadata verification.
- [x] 6.4 Confirm the Xcode 27 preview canary is absent from pull-request execution and cannot affect current required checks; no `xcode-27-canary` job was created for run #424 and no stable/release job depends on it.
- [x] 6.5 Record before/after macOS wall-clock and runner elapsed totals. Previous single-lane baseline: 544s. Run #424: `macos-15` about 635s, `macos-26` about 591s, combined about 1227s (2.26x baseline) with parallel wall-clock about 636s; this is below the 2.5x / 1360s guard.
- [x] 6.6 Run full required CI, shadow `ci-required`, and final diff review; run #424 passed all current required contexts plus `ci-required`, and review found no Critical/Important blocker or server-side branch-protection change.
- [ ] 6.7 Open the implementation PR to `develop`, obtain review, and squash merge only after the final latest-head checks are green.
- [ ] 6.8 After implementation merge, create `chore/archive-add-macos-compatibility-matrix` from latest `develop`, archive with spec sync, validate, and merge the separate archive PR.
