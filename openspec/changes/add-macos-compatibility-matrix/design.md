## Context

現在の `.github/workflows/check.yml` は `macos-check` と `release-artifact-check` を `macos-latest` で実行し、`.github/workflows/release.yml` も macOS CLI / DMG build を `macos-latest` に依存している。`macos-latest` は GitHub 側の image migration で意味が変わるため、OS major / default Xcode / bundled tools が repository の変更なしに切り替わる。

2026-09-16 時点では以下が成立する。

- Apple: Xcode 26.3 は macOS Sequoia 15.6 - Tahoe 26.x を support。
- Apple: Xcode 26.6 は macOS Tahoe 26.2 - 26.x を support。
- GitHub `macos-15` runner: `/Applications/Xcode_26.3.app` を提供する。default Xcode は 16.4 なので明示選択が必要。
- GitHub `macos-26` runner: `/Applications/Xcode_26.6.app` を提供する。
- GitHub `xcode-27` runner: Public Preview。2026-09-10 以降は macOS 27 上で動作し、arm64 のみ。

SchneeForge の product support は macOS aarch64 を中心としているため、stable CI は arm64 GitHub-hosted runner の `macos-15` / `macos-26` で support floor と current stable を同時に検証できる。一方 Xcode 27 / macOS 27 は preview platform であり、merge / release contract へ昇格させるには早い。

現行 `ci-required` は `macos-check` を集約対象に含めていない。本 change は既存 required gate migration と切り離し、macOS verification contract を先に安定化する。

## Goals / Non-Goals

### Goals

- `macos-latest` 依存を macOS CI / release build path から除去する。
- macOS 15 + Xcode 26.3 を compatibility floor として継続検証する。
- macOS 26 + Xcode 26.6 を current stable / release toolchain として継続検証する。
- stable lane のどちらかが non-success なら `macos-check` を fail-closed にする。
- Xcode 27 / macOS 27 preview を stable merge/release path から分離して早期検知だけ行う。
- runner image drift を OS / architecture / Xcode / SDK diagnostics で観測可能にする。
- preview canary を毎PR実行せず、macOS runner resource の不要な3 lane化を避ける。

### Non-Goals

- Xcode 27 / macOS 27 の正式 support 宣言。
- branch protection required context の変更。
- `ci-required` への `macos-check` 追加。
- x86_64-darwin support / Intel artifact 配布。
- Windows / WSL2 support。
- Flutter/iOS simulator full E2E。
- self-hosted macOS runner や patch-level OS image pinning。

## Decisions

### D1: stable macOS verification は2-entry matrixにする

`macos-check` の実作業を `macos-stable` matrix worker へ移す。

| lane | runner | Xcode | purpose |
|---|---|---|---|
| compatibility | `macos-15` | 26.3 | supported floor / older host compatibility |
| current | `macos-26` | 26.6 | current stable / release host compatibility |

両 lane は現在の `macos-check` と同じ core contract を実行する。

1. Nix install/setup
2. `nix flake check --allow-import-from-derivation`
3. `nix build .#homeConfigurations.darwin-aarch64.activationPackage`
4. `nix build .#darwinConfigurations.darwin-aarch64.system`

`strategy.fail-fast: false` とし、一方が失敗しても他方の結果・diagnosticsを取得する。

### D2: `macos-check` は stable matrix の fail-closed aggregator にする

既存 job/context 名 `macos-check` を残し、`needs: [macos-stable]` + `if: ${{ always() }}` で matrix worker の結果を評価する。

- stable matrix result が `success` の場合のみ aggregator success。
- failure / cancelled / skipped は failure。
- `macos-check` は本 change では `ci-required` dependency に追加しない。
- server-side branch protection も変更しない。

これにより future required promotion を行う場合も context 名を再変更せずに済む。

### D3: Xcode は `DEVELOPER_DIR` で明示選択する

runner の default Xcode は利用しない。stable matrix は lane ごとに以下を設定する。

- compatibility: `/Applications/Xcode_26.3.app/Contents/Developer`
- current: `/Applications/Xcode_26.6.app/Contents/Developer`

`sudo xcode-select` で host global state を書き換えるより、job-local `DEVELOPER_DIR` を使う。対象 Xcode が image から消えた場合は fallback せず job を失敗させ、support/toolchain policy の明示更新を要求する。

### D4: macOS job は toolchain diagnostics を build 前に出力する

少なくとも以下を記録する。

- `sw_vers -productVersion`
- `uname -m`
- `echo "$DEVELOPER_DIR"`
- `xcodebuild -version`
- `xcrun --sdk macosx --show-sdk-version`
- `xcrun --find clang`

これにより GitHub image の patch update や toolchain drift が failure log から追跡できる。

### D5: release artifact path は `macos-26` + Xcode 26.6 に固定する

`.github/workflows/check.yml` の `release-artifact-check` と `.github/workflows/release.yml` の macOS CLI / DMG build を同じ current-stable contractへ固定する。

- runner: `macos-26`
- `DEVELOPER_DIR=/Applications/Xcode_26.6.app/Contents/Developer`
- build scripts: 既存 `scripts/ci/build-release-macos-cli.sh` / `build-release-macos-dmg.sh` を変更せず共有

PR時 artifact gate と tag release で host/toolchain contract を揃え、`macos-latest` migrationによる release drift を防ぐ。

### D6: Xcode 27 は separate preview canary とする

`xcode-27` label は Xcode major を意味する preview image であり、基盤 OS はすでに macOS 27 へ移行している。これを stable matrix の3番目の entry にはしない。

canary は以下に限定する。

- `develop` push 後
- scheduled run
- manual `workflow_dispatch`

PR event では実行しない。stable `macos-check`、`ci-required`、release jobs の `needs` に含めない。canary failure は signal として赤く残してよいが、未merge PRの required checks を block しない。

canary の初期 coverage は stable macOS Nix verification と同等、または実装時に runner availability/cost を計測して縮小可能とする。ただし preview toolchain が少なくとも flake evaluation と Darwin configuration construction を通ることは検証する。

### D7: workflow contract test で暗黙 drift を防ぐ

`tests/ci-scripts.bats` に static contract checks を追加する。

- active `runs-on: macos-latest` が `check.yml` / `release.yml` に存在しない。
- stable runner labels が `macos-15` / `macos-26` である。
- pinned Xcode paths が 26.3 / 26.6 である。
- `xcode-27` preview job が stable `macos-check` / `ci-required` / release path の dependency に入らない。
- PR event で preview canary を実行しない条件が保持される。

新しい YAML parser / action dependency は追加せず、既存 Bats contract-test pattern を拡張する。

### D8: resource usage を計測する

stable macOS lane が1→2になるため、PRごとの macOS runner elapsed total は増える。実装PRでは変更前の `macos-check` と変更後2 laneの wall-clock / runner totalを記録する。

preview canary はPR外へ分離し、毎PRを3 laneにしない。stable 2 lane total が旧 single lane の概ね2.5倍を継続的に超える場合は、compatibility lane の cadence または coverage縮小を別 change ではなく本 change 内で再評価する。

## Failure Modes

- **GitHub image から pinned Xcode が削除される**: `DEVELOPER_DIR` target 不在で fail。default Xcodeへsilent fallbackしない。
- **macOS 15 runner が deprecateされる**: compatibility laneが明示的に失敗する。support floor引き上げは一次情報確認 + 別のpolicy判断として行う。
- **stable lane片方だけ失敗する**: `fail-fast: false` で両方完走し、`macos-check` aggregatorはfailure。
- **preview canaryが壊れる**: stable/required/release pathは影響を受けない。preview issueとして追跡する。
- **GitHubが `xcode-27` の基盤OSを再変更する**: label semanticsに従い、diagnosticsで実OSを記録する。canaryを正式supportの証明には使わない。
- **releaseとPR artifact gateが別toolchainになる**: contract testで `macos-26` + Xcode 26.6 の一致を固定する。
- **matrix化でmacOS runner usageが過大になる**: before/after計測し、previewはPR外を維持。必要なら compatibility coverage を再設計する。

## Rollback

問題発生時も `macos-latest` へ戻さない。

1. preview canary固有の問題なら canary jobのみ停止/縮小する。
2. compatibility lane固有のrunner障害なら current `macos-26` + Xcode 26.6 laneを維持し、compatibility laneの cadence/coverageを一時縮小する。
3. matrix aggregator固有の問題なら一時的に single explicit `macos-26` + Xcode 26.6 `macos-check`へ戻す。
4. release pathは `macos-26` + Xcode 26.6 pinを維持し、`macos-latest`へのrollbackは行わない。
