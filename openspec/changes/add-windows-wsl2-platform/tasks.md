# Tasks: add-windows-wsl2-platform

## 1. Proposal and design gates

- [x] 1.1 Audit current host/platform, `MachineFacts`, CLI startup, tool resolution, Unix-only code, and release/CI assumptions
- [x] 1.2 Confirm current Microsoft WSL command/filesystem guidance and current GitHub `windows-2025` image capabilities
- [x] 1.3 Define Windows as host/control-plane and WSL2 Linux as execution environment; keep Windows out of Nix system values
- [ ] 1.4 `openspec validate add-windows-wsl2-platform --strict` and `openspec validate --all --strict`
- [ ] 1.5 Written proposal/design/spec reviewed and explicitly approved before implementation

## 2. Host/backend model and WSL inventory (TDD)

- [ ] 2.1 Add failing unit tests for `HostPlatform::{MacOS,Linux,Windows}` and native/WSL2 backend selection
- [ ] 2.2 Implement host detection without changing existing Nix-side `Platform`/`ConfigurationTarget` semantics
- [ ] 2.3 Add fixture tests for UTF-8 and UTF-16LE/NUL/BOM WSL output decoding
- [ ] 2.4 Add quiet/verbose WSL inventory correlation tests, including default marker and distro names containing spaces
- [ ] 2.5 Implement WSL inventory parsing and WSL2 readiness model
- [ ] 2.6 Add selection-precedence tests: `--wsl-distro` > `SCHNEEFORGE_WSL_DISTRO` > WSL default
- [ ] 2.7 Implement fail-closed selection for missing WSL, no distro, unknown explicit distro, no default, and WSL1

## 3. Helper compatibility and safe delegation (TDD)

- [ ] 3.1 Add failing tests for machine-readable Linux helper compatibility probe parsing
- [ ] 3.2 Implement internal helper probe with bridge protocol version, app version, execution OS, and architecture
- [ ] 3.3 Add tests for missing helper, malformed probe, unsupported identity, protocol mismatch, and app-version mismatch
- [ ] 3.4 Implement helper readiness checks; no automatic helper installation/update
- [ ] 3.5 Add tests proving forwarded arguments remain distinct argv entries and launcher-only `--wsl-distro` is stripped
- [ ] 3.6 Implement `wsl.exe -d <distro> -- schneeforge ...` delegation without shell interpolation
- [ ] 3.7 Preserve stdin/stdout/stderr and delegated exit status; distinguish spawn/transport errors from command result

## 4. CLI integration and execution-side ownership (TDD)

- [ ] 4.1 Add global `--wsl-distro` and environment fallback without changing native macOS/Linux behavior
- [ ] 4.2 Move Windows backend dispatch before repo resolution, `ToolInventory::discover`, machine/state discovery, and operation-specific core calls
- [ ] 4.3 Define the command split: help/version local, Windows doctor host-aware, `self-update` explicit unsupported, execution-requiring commands delegated
- [ ] 4.4 Add Windows doctor tests for WSL unavailable, WSL1, no distro, helper missing, helper mismatch, and ready backend
- [ ] 4.5 Implement Windows doctor backend diagnostics with actionable remediation
- [ ] 4.6 Add explicit repo-path tests: omitted repo resolves in WSL, absolute Linux path allowed, drive-letter/UNC paths rejected
- [ ] 4.7 Implement initial execution-side repo/path policy without implicit `wslpath` or `/mnt/c` conversion

## 5. Windows portability audit

- [ ] 5.1 Replace hard-coded PATH `:` splitting with `std::env::split_paths` and add platform-aware tests
- [ ] 5.2 Audit `std::os::unix`, executable permission, HOME/XDG, shell, `sudo`, and path assumptions in core/CLI
- [ ] 5.3 cfg-gate or isolate Unix-only behavior that Windows compilation reaches; do not generalize unrelated Linux-helper-only code
- [ ] 5.4 Ensure core and CLI build/tests run on Windows without adding Windows Desktop/Tauri scope
- [ ] 5.5 Confirm macOS/Linux native tests and semantics are unchanged

## 6. CI and workflow contracts

- [ ] 6.1 Add Bats/static contract tests for pinned `windows-2025`, non-required Windows coverage, and no Windows release dependency
- [ ] 6.2 Add non-required `windows-check` for core/CLI compile/tests, native launcher smoke, and hermetic WSL contract tests
- [ ] 6.3 If practical, add a separate scheduled/develop/manual real-WSL2 canary that is not part of `ci-required` or release gates
- [ ] 6.4 Keep current server-side required contexts unchanged
- [ ] 6.5 Verify `actionlint`, ShellCheck/Bats, and Windows workflow syntax on the latest head

## 7. Documentation and project state

- [ ] 7.1 Update `openspec/project.md` platform description only after the implementation makes experimental Windows/WSL2 support true
- [ ] 7.2 Document Windows prerequisites, distro selection, helper requirement, WSL filesystem recommendation, and initial non-goals
- [ ] 7.3 Update `AGENTS.md` / `docs/STATUS.md` with actual implementation/CI state
- [ ] 7.4 Document that Windows release artifact/self-update/helper bootstrap remain follow-ups

## 8. Verification and review

- [ ] 8.1 `openspec validate --all --strict`
- [ ] 8.2 `cargo test`
- [ ] 8.3 `cargo fmt -- --check`
- [ ] 8.4 `cargo clippy -- -D warnings`
- [ ] 8.5 `nix flake check`
- [ ] 8.6 Windows `windows-check` green on latest implementation head
- [ ] 8.7 Existing seven required contexts + `ci-required` green on latest implementation head
- [ ] 8.8 Final PR diff review: no shell interpolation, no Windows-as-Nix-system drift, no required-context drift, no release-scope drift
- [ ] 8.9 Resolve all review threads / blocking findings

## 9. Merge and archive lifecycle

- [ ] 9.1 Open implementation PR from `feat/add-windows-wsl2-platform` to `develop`
- [ ] 9.2 Squash merge only after latest-head CI and review are green
- [ ] 9.3 Create `chore/archive-add-windows-wsl2-platform` from merged `develop`
- [ ] 9.4 Archive the OpenSpec change and sync canonical specs in a separate PR
- [ ] 9.5 Review/CI the archive PR and squash merge it to `develop`
