# Tasks

## 1. OpenSpec

- [x] 1.1 proposal / design / delta specs / tasks
- [x] 1.2 `openspec validate harden-updater-signed-version --strict`
- [x] 1.3 `openspec validate --all --strict`

## 2. TDD contract

- [x] 2.1 RED: add regression test requiring Tauri CLI 2.11.5 pin and `requireSignedVersion: true`
- [x] 2.2 confirm RED against current 2.11.4 / missing config
- [ ] 2.3 GREEN: update CLI pin + config and make contract pass

## 3. Release hardening

- [x] 3.1 pin Tauri CLI 2.11.5
- [x] 3.2 pin reviewed aarch64 macOS release SHA256
- [x] 3.3 keep updater artifact generation behind existing activation gate
- [x] 3.4 keep production private key scoped only to Tauri build step

## 4. Runtime hardening

- [x] 4.1 set updater `requireSignedVersion: true`
- [x] 4.2 keep static config pubkey empty; production key remains compile-time activation input
- [x] 4.3 add regression that signed-version enforcement cannot silently disappear

## 5. Docs / verification

- [x] 5.1 update RELEASE.md activation checklist
- [x] 5.2 update STATUS.md
- [ ] 5.3 cargo test / fmt / clippy / desktop build
- [ ] 5.4 actionlint / shellcheck / Bats / OpenSpec strict / required checks green
- [ ] 5.5 squash merge implementation PR
- [ ] 5.6 archive + canonical spec sync in separate PR
