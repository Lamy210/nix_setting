## 1. StateStore strict load contract

- [ ] 1.1 Change `StateStore::load()` to `Result<Option<State>>`.
- [ ] 1.2 Return `Ok(None)` only for missing state file.
- [ ] 1.3 Return structured errors for malformed JSON and non-NotFound read failures.
- [ ] 1.4 Preserve valid legacy JSON compatibility.
- [ ] 1.5 Add deterministic unit tests for missing / legacy / malformed / read-error cases.

## 2. Core call-site migration

- [ ] 2.1 Audit every `StateStore::load()` callsite and remove lossy `unwrap_or_default` / `and_then` fallbacks on errors.
- [ ] 2.2 Make source/manifest/profile resolution propagate state errors rather than checkout/default fallback.
- [ ] 2.3 Make self-update / Dashboard channel selection distinguish missing state from corrupt state.
- [ ] 2.4 Keep diagnostics useful while exposing state read failure explicitly.

## 3. Mutation ordering

- [ ] 3.1 Load required previous state before apply/rollback effects.
- [ ] 3.2 Load state before update/source-init/profile mutation side effects.
- [ ] 3.3 Add regressions proving malformed existing state is not overwritten by a default state.
- [ ] 3.4 Add regressions proving managed source/profile semantics never silently fall back when state exists but is invalid.

## 4. Adapter migration

- [ ] 4.1 Update CLI status/source/self-update/doctor paths for strict state load errors.
- [ ] 4.2 Update Desktop Dashboard/manifest/state consumers.
- [ ] 4.3 Keep error messages actionable and include state path/context without exposing sensitive content.

## 5. Verification and lifecycle

- [ ] 5.1 Run `openspec validate update-state-load-fail-closed --strict`.
- [ ] 5.2 Run `openspec validate --all --strict --no-interactive`.
- [ ] 5.3 Run cargo tests / fmt / clippy and repository required checks.
- [ ] 5.4 Merge implementation PR to `develop` with squash merge after proposal approval.
- [ ] 5.5 Archive + canonical spec sync in a separate `chore/archive-update-state-load-fail-closed` PR.
