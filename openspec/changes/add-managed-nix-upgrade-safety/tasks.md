# Tasks: add-managed-nix-upgrade-safety

## 0. Design gate

- [x] 0.1 Review Issue #256, installer provider vs runtime Nix distinction, current ownership/receipt and nix-darwin declarations.
- [x] 0.2 Compare documentation-only, safe inventory-first, and direct auto-upgrade alternatives.
- [x] 0.3 Draft proposal/design/delta spec with explicit non-goals and Phase B gate.
- [ ] 0.4 `openspec validate add-managed-nix-upgrade-safety --strict` and `openspec validate --all --strict` pass in CI.
- [ ] 0.5 Review/approve this proposal **before implementing a new upgrade command**.

## 1. Core: read-only preflight (separate implementation PR)

- [ ] 1.1 Define versioned `UpgradeInspection` with explicit known/unknown/error and no implicit default to safe.
- [ ] 1.2 Implement injected filesystem/ownership/receipt / tool observations in Core.
- [ ] 1.3 Resolve `nix` through existing `ToolInventory`; fixed argv and timeout; no shell, no network.
- [ ] 1.4 Probe daemon reachability separately; only parse version with verified per-version contract, else mark unknown.
- [ ] 1.5 Determine upgrade owner using actual host evidence, never only a repository declaration.
- [ ] 1.6 Add human-readable and `--json` CLI display as a thin Core adapter.
- [ ] 1.7 Preserve Windows native launcher → WSL2 helper routing, exact `--json` stdout, and error exit propagation without adding a Windows Nix execution backend.

## 2. Tests and safety gate

- [ ] 2.1 Unit tests for missing/valid/invalid/legacy receipt and ownership (including PR #258 corrupt receipt path).
- [ ] 2.2 Tool/path tests for version output, daemon probe failure, timeout, non-UTF-8 and failure reporting.
- [ ] 2.3 Test that inspect never invokes upgrade/switch/uninstall/daemon restart or writes/contacts network.
- [ ] 2.4 CLI integration JSON schema and non-zero error behavior.
- [ ] 2.4a Windows launcher transport/handshake failure and JSON output purity contract tests (Nix execution remains on Linux helper).
- [ ] 2.5 Run full Rust/Nix/OpenSpec CI; review macOS 15/26 outcomes and release artifacts.
- [ ] 2.6 Document supported environment matrix, remaining unknowns, operational use and Issue #256 linkage.

## 3. Follow-up Phase B (informational only; separate OpenSpec change)

The items below are **not acceptance tasks for this Phase A change**. They must
be tracked as a separate future proposal after Phase A acceptance.

- Confirm supported in-place upgrades per effective host owner (Linux systemd and macOS nix-darwin).
- Pin verified target artifact and independent signature/attestation trust root.
- Design privileged lock, approval, two-stage commit/journal, recoverability and interrupted-host safety.
- Implement platform-specific update only after disposable acceptance/rollback evidence.
- Verify actual daemon version and CVE remediation; add release acceptance communications.

## 4. Archive (after Phase A implementation PR merge)

- [ ] 4.1 Create separate `chore/archive-add-managed-nix-upgrade-safety` PR and sync canonical spec.
