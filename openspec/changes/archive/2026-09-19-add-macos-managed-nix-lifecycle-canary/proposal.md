# Change: macOS Apple Silicon Managed Nix lifecycle acceptance helper

## Why

ADR-0001 は macOS aarch64 の実機 smoke を Final Acceptance 条件として
`Accepted provisionally` のまま残している。一方、現在の GitHub hosted
`macos-15` runner は arm64 で disposable であり、Final Acceptance のうち
GUI/Finder 目視を除く Managed Nix CLI lifecycle は安全に自動化できる。

現行の `docs/testing/macOS-final-acceptance-checklist.md` は手順としては
十分だが、fresh host / install / idempotency / uninstall / reinstall の
反復検証が人手依存で、release artifact の lifecycle regression を継続的に
再現しづらい。

そこで release tag の実 artifact を使う **manual / non-required**
acceptance helper を追加する。ただし GUI の Finder 起動・表示確認と
ADR-0001 の最終昇格判断は引き続き手動 gate とし、この workflow の成功だけで
Final Acceptance 完了とは扱わない。

## What Changes

- **ADDED: manual macOS Managed Nix lifecycle workflow**
  - `workflow_dispatch` で release tag を受け取る
  - `macos-15` hosted arm64 runner を使用する
  - PR / push / schedule / required gate から分離する
- **ADDED: release artifact lifecycle script**
  - fresh host precondition (`arm64`, `/nix` 無し, `nix` 無し)
  - release の `schneeforge-aarch64-darwin` と `CHECKSUMS.txt` を取得し SHA256 検証
  - release binary の embedded manifest で `nix install --yes`
  - receipt / ownership / store ping / flakes / `nix doctor` を検証
  - 2 回目 install が `ExistingNixDetected` で fail-closed になることを検証
  - uninstall → cleanup → reinstall → final uninstall で lifecycle 一周を検証
  - current checkout の manifest/state を暗黙利用しない
- **ADDED: workflow contract regression**
  - acceptance workflow が manual-only / non-required であること
  - `cachix/install-nix-action` 等で事前に Nix を入れないこと
  - release tag + checksum verification + install/uninstall/reinstall の marker を静的検証
- **MODIFIED: Final Acceptance docs / STATUS**
  - 自動化 helper がカバーする gate と、残る manual GUI gate を明記
  - Windows/WSL2・issue #16 等の stale status を現状へ同期

## Scope Boundary

本 change は以下を **行わない**:

- Finder からの GUI 起動結果・表示内容の自動判定
- `install.sh` の D8 `/dev/tty` 対話経路を CI で代替
- nix-darwin apply / uninstall の full bootstrap lifecycle
- ADR-0001 の Status を自動で `Accepted` へ変更
- release workflow / branch protection required contexts への依存追加
- DMG offline bundle (#17) の実装

## Impact

- **specs**: `managed-nix-bootstrap`, `development-workflow`
- **workflow**: manual non-required acceptance helper を1本追加
- **script**: release artifact lifecycle verifier を追加
- **test**: `tests/ci-scripts.bats` に workflow contract
- **risk**: 中 — `/nix` install/uninstall を行うため hosted disposable runner のみに限定し、fresh precondition を満たさない環境では fail-closed する
