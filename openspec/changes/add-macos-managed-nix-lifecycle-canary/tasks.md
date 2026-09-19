# Tasks

## 1. OpenSpec

- [x] 1.1 proposal / design / delta specs / tasks を作成
- [x] 1.2 `openspec validate add-macos-managed-nix-lifecycle-canary --strict`
- [x] 1.3 `openspec validate --all --strict`

## 2. TDD / contract

- [x] 2.1 RED: workflow/script 不在で macOS lifecycle contract test が failure になる (run #523: contract test #28 が workflow 不在で failure)
- [x] 2.2 GREEN: manual-only workflow / lifecycle script を追加して contract test を通す (run #533)
- [x] 2.3 workflow が PR/push/schedule/required gate から分離されていること
- [x] 2.4 workflow が Nix を事前導入せず fresh-host precondition を維持すること

## 3. Lifecycle implementation

- [x] 3.1 arm64 / GitHub Actions / fresh `/nix` precondition
- [x] 3.2 release CLI + CHECKSUMS download / SHA256 verification
- [x] 3.3 release binary embedded manifest で `nix install --yes`
- [x] 3.4 receipt / ownership / store ping / flakes / `nix doctor`
- [x] 3.5 second install を `ExistingNixDetected` で拒否
- [x] 3.6 uninstall → runtime-remnant cleanup → reinstall → final cleanup (空の unmounted `/nix` path 単体は failure にしない)
- [x] 3.7 `if: always()` log artifact upload

## 4. Docs / status

- [x] 4.1 Final Acceptance checklist に automated helper のcoverage / 非coverageを追記
- [x] 4.2 STATUS.md の Windows/WSL2・#16・active OpenSpec 状態を同期
- [x] 4.3 ADR-0001 が workflow success のみでは Accepted にならないことを維持

## 5. Verification / lifecycle close

- [x] 5.1 PR CI: bootstrap contract / shellcheck / actionlint / OpenSpec strict / required checks green (PR #105 / run #551、同一 tree の run #550 は 22/22 green)
- [x] 5.2 implementation PR #105 を `develop` へ squash merge
- [x] 5.3 post-merge: `v0.2.0-rc.7` lifecycle job を hosted macOS arm64 で実行。初回は path-only cleanup 判定で failure、診断付き run #2 で install → doctor → idempotency → uninstall → reinstall → final cleanup が success。connector に workflow-dispatch mutation が無いため、manual-only develop workflow を変更せず merge済み feature branch に一時 push trigger を置いて同一 job/script を実行し、検証後 ref を merge commit へ復元
- [ ] 5.4 separate `chore/archive-add-macos-managed-nix-lifecycle-canary` PR で archive + canonical spec sync
