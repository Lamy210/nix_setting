# Change: required CI の critical path を branch protection を壊さず短縮する

## Why

SchneeForge の PR CI は品質ゲート自体は充実している一方、required context の一部が単一 job 内で直列実行されている。特に `rust-check` は core/CLI の test・fmt・clippy、CLI release smoke、Tauri desktop build smoke を 1 runner 上で順番に実行しており、desktop 専用の GTK/Tauri system dependency install まで job 全体の前処理になっている。

また branch protection は既存の context 名 (`rust-check`, `flake-check`, `lint`, `bootstrap-test`, `managed-nix-e2e`, `release-artifact-check`, `openspec-check`) を required としているため、単純な job rename / path filter / required context 削除は merge gate を壊す可能性がある。

Windows/WSL2 と macOS compatibility matrix を追加する前に、既存 required checks の critical path を安全に短縮し、将来の `ci-required` 1本化を観測可能な状態にする。

初期案の3-way fan-out (`rust-quality` / `rust-cli-smoke` / `rust-desktop-smoke`) は実 PR run で検証したが、desktop worker の cold full build により worker runner time が上限を完了前に超過した。そのため計測結果に基づき、2-way fan-out と desktop gate の重複排除へ設計を修正する。

## What Changes

- `rust-check` の実処理を `rust-quality` / `rust-build-smoke` の2 worker に分割し、並列実行する。
- `rust-quality` は root Rust workspace の test・fmt・clippy を担当し、Tauri/GTK system dependency install を行わない。
- `rust-build-smoke` は CLI release build/smoke と Linux desktop compile smoke (`cargo check`) を担当し、Tauri/GTK system dependencies はこの worker のみに限定する。
- Linux desktop smoke は full link/build の重複を避けて compile gate とし、required `release-artifact-check` が release workflow と同一 script で実行する macOS DMG/Tauri full build を実 artifact gate とする。
- branch protection 互換性のため、既存名 `rust-check` は fail-closed aggregator job として残す。
- `ci-required` aggregator job を追加し、現行 required 7 context の結果を fail-closed で集約する。ただし本 change では GitHub branch protection の required contexts は変更しない。
- aggregator は `needs` と `if: always()` を使い、worker/check が failure / cancelled / skipped のいずれでも aggregator 自身が success にならないようにする。
- required workflow 全体への `paths` / `paths-ignore` は導入しない。required workflow が trigger されず Pending のまま残る状態を避ける。
- CI run の before/after を計測し、wall-clock と runner duration の両方を評価する。

## Success Criteria

- `rust-check` context 名を維持したまま branch protection が継続して機能する。
- `rust-quality` / `rust-build-smoke` のいずれかが non-success の場合、`rust-check` も failure になる。
- 現行 required 7 checks がすべて success の場合のみ `ci-required` が success になる。
- Linux では desktop Rust/Tauri code を compile gate し、macOS では required release artifact gate が実 DMG full build を継続する。
- 直近 baseline と比較して required critical path を 25% 以上短縮することを目標とする。
- Rust 系 worker + aggregator の合計 elapsed runner time 増加は baseline 比 +20% 以内を目標とする。超える場合は分割粒度または build 重複を再評価する。

## Non-Goals

- GitHub branch protection の server-side required contexts 変更。
- `flake-check`, `release-artifact-check`, `bootstrap-test` の本格分割（本 change の計測結果を見て後続判断）。
- release artifact scripts / release workflow の build semantics 変更。
- macOS 15 / macOS 26 / Xcode matrix。
- Windows / WSL2 support。
- 新しい third-party cache provider の導入。
- workflow-level path filtering。

## Impact

- Affected spec: `development-workflow`
- Affected workflow: `.github/workflows/check.yml`
- Affected tests: `tests/ci-scripts.bats`
- Affected docs: `docs/STATUS.md`, `AGENTS.md`
- GitHub branch protection: server-side 設定変更なし
- Follow-up: CI 実測 → `add-macos-compatibility-matrix` → `add-windows-wsl2-platform`
