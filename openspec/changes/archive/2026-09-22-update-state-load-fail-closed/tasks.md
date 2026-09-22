## 1. StateStore strict load contract

- [x] 1.1 Add a dedicated `Error::State(String)` variant for state read/parse failures.
- [x] 1.2 Change `StateStore::load()` to `Result<Option<State>>`.
- [x] 1.3 Return `Ok(None)` only for missing state file.
- [x] 1.4 Return `Error::State` for malformed JSON and non-NotFound read failures without embedding state contents.
- [x] 1.5 Preserve valid legacy JSON compatibility.
- [x] 1.6 Add deterministic unit tests for missing / legacy / malformed / read-error cases.

## 2. Core call-site migration

- [x] 2.1 Audit every `StateStore::load()` callsite and remove lossy `unwrap_or_default` / `and_then` fallbacks on errors.
- [x] 2.2 Make source/manifest/profile resolution propagate state errors rather than checkout/default fallback.
- [x] 2.3 Make self-update / Dashboard channel selection distinguish missing state from corrupt state.
- [x] 2.4 Keep diagnostics useful while exposing state read failure explicitly.

## 3. Mutation ordering

- [x] 3.1 Load required previous state before apply/rollback effects.
- [x] 3.2 Load state before update/source-init/profile mutation side effects.
- [x] 3.3 Add regressions proving malformed existing state is not overwritten by a default state.
- [x] 3.4 Add regressions proving managed source/profile semantics never silently fall back when state exists but is invalid.

## 4. Adapter migration

- [x] 4.1 Update CLI status/source/self-update/doctor paths for strict state load errors.
- [x] 4.2 Update Desktop Dashboard/manifest/state consumers.
- [x] 4.3 Keep error messages actionable and include state path/context without exposing sensitive content.

## 5. Verification and lifecycle

- [x] 5.1 Run `openspec validate update-state-load-fail-closed --strict`.
- [x] 5.2 Run `openspec validate --all --strict --no-interactive`.
- [x] 5.3 Run cargo tests / fmt / clippy and repository required checks.
- [x] 5.4 Merge implementation PR to `develop` with squash merge after proposal approval.
- [x] 5.5 Archive + canonical spec sync in a separate `chore/archive-update-state-load-fail-closed` PR.
