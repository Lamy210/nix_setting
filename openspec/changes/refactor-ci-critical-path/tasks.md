## 1. Specification and baseline

- [x] 1.1 Create `refactor-ci-critical-path` proposal/design/delta spec before implementation.
- [x] 1.2 Confirm root Cargo workspace contains core/CLI only and desktop dependencies can be isolated.
- [x] 1.3 Confirm GitHub required-check skip/`needs` behavior from current official documentation.
- [x] 1.4 Run strict OpenSpec validation for this change in CI.

## 2. Rust required-check fan-out

- [x] 2.1 Replace the serial Rust implementation job with parallel `rust-quality` and `rust-build-smoke` workers.
- [x] 2.2 Keep Tauri/GTK apt dependencies only in `rust-build-smoke`; `rust-quality` remains desktop-dependency-free.
- [x] 2.3 Make `rust-build-smoke` run CLI release smoke plus Linux desktop `cargo check --release`, while required `release-artifact-check` retains macOS DMG/Tauri full build coverage.
- [x] 2.4 Add required-context-compatible `rust-check` aggregator using `needs` + `always()` and fail on every non-success worker result.
- [x] 2.5 Keep required coverage equivalent without duplicate Linux desktop full link/build work; align CLI sidecar and desktop compile Cargo profiles.

## 3. Shadow unified gate

- [x] 3.1 Keep non-required `ci-required` aggregator for the existing seven required contexts.
- [x] 3.2 Make `ci-required` fail on failure/cancelled/skipped results and succeed only when all seven contexts succeed.
- [x] 3.3 Confirm negative path in PR CI: a Rust worker failure propagates to both `rust-check` and `ci-required` failure.
- [x] 3.4 Keep GitHub branch-protection server settings unchanged in this change; no server-side required-context mutation is performed.

## 4. Documentation and validation

- [x] 4.1 Update `docs/STATUS.md` and `AGENTS.md` so CI critical-path work is current and completed workflow-hardening text is no longer stale.
- [ ] 4.2 Run `actionlint` and strict OpenSpec validation through final PR CI on the latest head.
- [x] 4.3 Confirm no workflow-level `paths` / `paths-ignore` filter was added to the required workflow.

## 5. Performance verification

- [x] 5.1 Record baseline required critical-path / Rust job elapsed duration from successful pre-change run #378: required execution critical path 479s, `rust-check` 448s.
- [x] 5.2 Evaluate initial three-worker experiment and reject it: `rust-quality` 117s + `rust-cli-smoke` 123s + `rust-desktop-smoke` >334s before completion = >574s, exceeding the 537.6s (+20%) guard before completion.
- [x] 5.3 Record post-change measurements: run #393 proved the 2-way Rust total at 426s but missed the primary target with a 387s critical path; after isolating the dominant flake realization cost, run #397 measured `rust-quality=119s`, `rust-build-smoke=320s`, `rust-check=4s`, current required critical path 327s, and shadow `ci-required` completion at 332s.
- [x] 5.4 Evaluate target: run #397 achieves 479s → 327s (31.7% reduction, target >=25%) and Rust runner total 443s (below baseline 448s and <=537.6s guard).
- [x] 5.5 After run #393 missed the critical-path target, identify `flake-check` / Terraform source realization as the next bottleneck, separate default `developer` evaluation from supported `minimal` actual realization, and remeasure successfully in run #397.

## 6. Integration

- [ ] 6.1 Review the final workflow/spec/docs diff for required-context name drift, fail-open behavior, and CI coverage regressions.
- [x] 6.2 Open a conventional-commit draft PR to `develop` from `refactor/ci-critical-path` (#90).
- [ ] 6.3 Merge only after latest-head CI is green; archive this OpenSpec change in a separate `chore/archive-refactor-ci-critical-path` PR afterwards.
