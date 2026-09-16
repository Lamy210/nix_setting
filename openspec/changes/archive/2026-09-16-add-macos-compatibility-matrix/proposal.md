# Change: macOS / Xcode compatibility lanes を明示し preview toolchain を分離する

## Why

SchneeForge の macOS CI / release は現在 `macos-latest` に依存している。GitHub-hosted runner の `macos-latest` は基盤 OS / default Xcode が更新されるため、同じ workflow 定義でも時間経過で build host の意味が変わり、macOS support floor と release toolchain の再現性が曖昧になる。

2026-09-16 時点の一次情報では、Apple は Xcode 26.3 を macOS Sequoia 15.6 から Tahoe 26.x までサポートし、Xcode 26.6 は Tahoe 26.2 以降を要求する。GitHub runner image は `macos-15` に Xcode 26.3、`macos-26` に Xcode 26.6 を搭載している。一方 `xcode-27` runner は Public Preview であり、2026-09-10 から macOS 27 上で動作するため、stable compatibility contract と同じ merge gate に含めるべきではない。

Windows / WSL2 support に進む前に、macOS の OS / Xcode compatibility contract を明示し、stable support と preview observation を分離する。

Primary references (verified 2026-09-16):
- Apple Xcode system requirements: https://developer.apple.com/xcode/system-requirements
- GitHub runner images: https://github.com/actions/runner-images
- Xcode 27 public preview announcement: https://github.blog/changelog/2026-07-16-xcode-27-runner-image-now-in-public-preview/
- Xcode 27 macOS 27 migration: https://github.blog/changelog/2026-09-10-xcode-27-runner-image-now-runs-on-macos-27/

## What Changes

- PR/push の stable macOS verification を明示的な2 lane にする。
  - compatibility lane: `macos-15` + Xcode 26.3
  - current-stable lane: `macos-26` + Xcode 26.6
- stable 2 lane は同じ macOS Nix/Home Manager/nix-darwin verification contract を実行し、`macos-check` aggregator が fail-closed で両方を集約する。
- `macos-check` の context 名は維持する。ただし本 change では server-side branch protection / `ci-required` の required set を変更しない。
- Xcode 選択は runner default に依存せず、lane ごとの `DEVELOPER_DIR` で固定する。
- 各 stable lane は build 前に OS / architecture / Xcode / SDK の diagnostics を出力し、runner image drift を観測可能にする。
- `release-artifact-check` と release workflow の macOS build host を `macos-26` + Xcode 26.6 に固定し、release artifact の toolchain を `macos-latest` から切り離す。
- Xcode 27 は `xcode-27` Public Preview canary として stable aggregator / `ci-required` / release path から分離する。
- preview canary は PR ごとの macOS runner 消費を増やさないよう、`develop` 更新後または scheduled/manual 経路で観測する。
- workflow contract test を追加し、active `runs-on: macos-latest` の再導入、stable lane の version drift、preview canary の blocking dependency 混入を検出する。

## Success Criteria

- stable macOS verification が `macos-15` + Xcode 26.3 と `macos-26` + Xcode 26.6 の両方で成功する。
- stable lane のいずれかが failure/cancelled/skipped の場合、`macos-check` は failure になる。
- `macos-check` context 名を維持し、GitHub branch protection の server-side settings は変更しない。
- `check.yml` / `release.yml` の macOS build host が active `macos-latest` に依存しない。
- release artifact build は `macos-26` + Xcode 26.6 で実行され、PR gate と tag release で同じ host/toolchain contract を共有する。
- `xcode-27` canary の failure は current required checks / `ci-required` / release artifact creation を block しない。
- CI diagnostics から `sw_vers`, architecture, selected Xcode, SDK version を追跡できる。
- macOS runner total の増加を計測し、preview lane を毎PR実行しないことで不要な3倍化を避ける。

## Non-Goals

- Xcode 27 / macOS 27 を正式 support と宣言すること。
- Xcode 27 を release artifact build に使用すること。
- GitHub branch protection の required contexts を変更すること。
- `ci-required` に `macos-check` を追加すること。
- x86_64-darwin release asset の追加。
- Windows / WSL2 support。
- Flutter/iOS simulator を用いた full integration test の追加。
- macOS patch version を固定した self-hosted runner 運用。

## Impact

- Affected spec: `development-workflow`
- Affected workflows: `.github/workflows/check.yml`, `.github/workflows/release.yml`
- Affected tests: `tests/ci-scripts.bats`（workflow contract checks）
- Affected docs: `docs/STATUS.md`, `AGENTS.md`
- GitHub branch protection: server-side 設定変更なし
- Resource trade-off: stable macOS verification は1 laneから2 laneへ増える。Xcode 27 preview はPR外に分離し、毎PRのmacOS runner usageを3 lane化しない
- Follow-up: stable lane 実績確認後に `macos-check` の required promotion を別 change で検討可能。その後 `add-windows-wsl2-platform` へ進む
