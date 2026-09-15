## 1. Specification and baseline

- [x] 1.1 Create `refactor-ci-critical-path` proposal/design/delta spec before implementation.
- [x] 1.2 Confirm root Cargo workspace contains core/CLI only and desktop dependencies can be isolated.
- [x] 1.3 Confirm GitHub required-check skip/`needs` behavior from current official documentation.
- [ ] 1.4 Run strict OpenSpec validation for this change in CI.

## 2. Rust required-check fan-out

- [ ] 2.1 Replace the serial Rust implementation job with parallel `rust-quality`, `rust-cli-smoke`, and `rust-desktop-smoke` workers.
- [ ] 2.2 Keep Tauri/GTK apt dependencies only in `rust-desktop-smoke`.
- [ ] 2.3 Add required-context-compatible `rust-check` aggregator using `needs` + `always()` and fail on every non-success worker result.
- [ ] 2.4 Keep existing commands/coverage equivalent: test, fmt, clippy, CLI release smoke, CLI debug sidecar build, desktop build smoke.

## 3. Shadow unified gate

- [ ] 3.1 Add non-required `ci-required` aggregator for the existing seven required contexts.
- [ ] 3.2 Make `ci-required` fail on failure/cancelled/skipped results and succeed only when all seven contexts succeed.
- [ ] 3.3 Confirm GitHub branch-protection server settings are unchanged in this change.

## 4. Documentation and validation

- [ ] 4.1 Update `docs/STATUS.md` to mark workflow hardening archived and CI critical-path work active.
- [ ] 4.2 Run `actionlint` and strict OpenSpec validation through PR CI.
- [ ] 4.3 Confirm no workflow-level `paths` / `paths-ignore` filter was added to the required workflow.

## 5. Performance verification

- [ ] 5.1 Record baseline required critical-path / Rust job elapsed duration from a successful pre-change run.
- [ ] 5.2 Record post-change Rust worker/aggregator durations from the PR run.
- [ ] 5.3 Evaluate target: >=25% required critical-path reduction and <=20% Rust runner-time increase.
- [ ] 5.4 If the cost/performance target is materially missed, adjust fan-out granularity before merge rather than declaring the optimization complete.

## 6. Integration

- [ ] 6.1 Review the workflow diff for required-context name drift and fail-open behavior.
- [ ] 6.2 Open a conventional-commit PR to `develop` from `refactor/ci-critical-path`.
- [ ] 6.3 Merge only after existing required checks are green; archive this OpenSpec change in a separate `chore/archive-refactor-ci-critical-path` PR afterwards.
