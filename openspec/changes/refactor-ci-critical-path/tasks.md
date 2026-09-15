## 1. Specification and baseline

- [x] 1.1 Create `refactor-ci-critical-path` proposal/design/delta spec before implementation.
- [x] 1.2 Confirm root Cargo workspace contains core/CLI only and desktop dependencies can be isolated.
- [x] 1.3 Confirm GitHub required-check skip/`needs` behavior from current official documentation.
- [x] 1.4 Run strict OpenSpec validation for this change in CI.

## 2. Rust required-check fan-out

- [ ] 2.1 Replace the serial Rust implementation job with parallel `rust-quality` and `rust-build-smoke` workers.
- [ ] 2.2 Keep Tauri/GTK apt dependencies only in `rust-build-smoke`; `rust-quality` must remain desktop-dependency-free.
- [ ] 2.3 Make `rust-build-smoke` run CLI release smoke plus Linux desktop `cargo check`, while required `release-artifact-check` retains macOS DMG/Tauri full build coverage.
- [ ] 2.4 Add required-context-compatible `rust-check` aggregator using `needs` + `always()` and fail on every non-success worker result.
- [ ] 2.5 Keep required coverage equivalent without duplicate Linux desktop full link/build work.

## 3. Shadow unified gate

- [ ] 3.1 Keep non-required `ci-required` aggregator for the existing seven required contexts.
- [ ] 3.2 Make `ci-required` fail on failure/cancelled/skipped results and succeed only when all seven contexts succeed.
- [x] 3.3 Confirm negative path in PR CI: a Rust worker failure propagates to both `rust-check` and `ci-required` failure.
- [ ] 3.4 Confirm GitHub branch-protection server settings are unchanged in this change.

## 4. Documentation and validation

- [ ] 4.1 Update `docs/STATUS.md` and `AGENTS.md` so CI critical-path work is current and completed workflow-hardening text is no longer stale.
- [ ] 4.2 Run `actionlint` and strict OpenSpec validation through final PR CI.
- [ ] 4.3 Confirm no workflow-level `paths` / `paths-ignore` filter was added to the required workflow.

## 5. Performance verification

- [x] 5.1 Record baseline required critical-path / Rust job elapsed duration from successful pre-change run #378: required execution critical path ~479s, `rust-check` 448s.
- [x] 5.2 Evaluate initial three-worker experiment and reject it: `rust-quality` 117s + `rust-cli-smoke` 123s + `rust-desktop-smoke` >334s before completion = >574s, exceeding the 537.6s (+20%) guard before completion.
- [ ] 5.3 Record post-change two-worker/aggregator durations from a successful PR run.
- [ ] 5.4 Evaluate target: >=25% required critical-path reduction (<=~359s) and <=20% Rust runner-time increase (<=537.6s).
- [ ] 5.5 If the two-worker cost/performance target is materially missed, adjust build duplication or fan-out granularity again before merge.

## 6. Integration

- [ ] 6.1 Review the final workflow diff for required-context name drift and fail-open behavior.
- [x] 6.2 Open a conventional-commit draft PR to `develop` from `refactor/ci-critical-path` (#90).
- [ ] 6.3 Merge only after existing required checks are green; archive this OpenSpec change in a separate `chore/archive-refactor-ci-critical-path` PR afterwards.
