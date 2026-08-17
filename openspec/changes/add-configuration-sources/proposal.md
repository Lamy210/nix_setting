# Change: ConfigurationSource モデルと update 体系の整理 (v2 P1)

## Why

SchneeForge v0.2 までの configuration は単一の Git checkout のみを想定し、
source の種別を表現する model が無い。v2 設計 (ADR-0003) が定める
「Easy by default, Git-native when desired」を実装するには、
4 種の source (Release Stable / Release Preview / Git / Local) を
core が解決し、update 操作を source の実態に応じて dispatch する
必要がある。

現状の具体的な問題:

1. **`schneeforge upgrade` の名前と動作の不一致**: 常に
   `nix flake update` (dependency 更新) を実行するが、install.sh が
   release tag へ pinned clone した checkout では flake.lock の変更は
   「CI で検証された release 単位」(CHECKSUMS に flake.lock hash を
   同梱) を壊す
2. **`schneeforge sync` は Git source にしか意味がない**: pinned
   checkout (detached HEAD) では no-op note を返すだけで、利用者は
   自分がどの source を使っているかを認識できない
3. **State に source 情報が無い**: GUI dashboard が
   installed / available / applied を表示するには source の現在状態と
   適用済み状態の区別が必要

## What Changes

- **ADDED: `source` module (core)**
  - `SourceKind` enum: `ReleaseStable` / `ReleasePreview` /
    `GitTracking` / `GitPinned` / `Local`
  - `SourceResolver::detect(repo, git)`: checkout の実態から kind を
    検出する純粋な分類 (detached HEAD + `v*` tag → Release。
    prerelease suffix の有無で Stable/Preview を判別。branch →
    GitTracking。tag/commit 固定 → GitPinned。`.git` 無し → Local)
- **ADDED: `schneeforge update` (CLI)**
  - source kind 毎の dispatch:
    - ReleaseStable/Preview: fetch tags → 次の release tag へ checkout
      (同じ channel 内のみ。Stable が prerelease に昇格しない)
    - GitTracking: `git fetch` + `git pull --ff-only` (dirty は中止)
    - GitPinned/Local: no-op + 案内表示
  - flake.lock は更新しない (release 単位の検証を保持)
- **ADDED: `schneeforge source sync` / `schneeforge source deps update` (CLI)**
  - `source sync`: 従来の `sync` (git pull --ff-only) を Advanced 扱いへ
    移動。Tracking source でのみ動作
  - `source deps update`: 従来の `upgrade` (`nix flake update`) を
    Advanced 扱いへ移動。Stable で実行した場合は release 検証から
    外れる旨の警告を表示
- **MODIFIED: State 拡張**
  - `state.json` に `source: { kind, ref, channel }` を追加
    (default なし = 従来 state との互換。読み込みは Option 扱い)
- **MODIFIED: 旧 command の alias 化**
  - `upgrade` / `sync` は v0.3 まで動作するが、deprecation note を
    表示して新 command へ案内する
- **ADDED: `schneeforge source status` (CLI)**
  - 現在の source kind / ref / channel / remote 差分を表示

## Impact

- **Specs**: `core-operations` に source 解決と update semantics を
  追加。`upgrade`/`sync` の requirement は alias として MODIFIED
- **CLI**: `Update` / `Source(SourceSub)` subcommand を追加。
  既存 `Upgrade` / `Sync` は残置 (deprecation note 付き)
- **Core**: `crates/core/src/source.rs` 新設。`operations.rs` に
  update dispatch を追加。`state.rs` に source field
- **GUI**: 本 change では対象外 (dashboard は P2)。ただし
  `get_status` が返す JSON に source 情報が増えるため破壊なしの
  追加のみ
- **ADR-0003 を同時に Proposed → 実装完了時に Accepted へ更新**

## Scope / Non-goals

- Release checkout の working tree-less 化 (Nix Store 直接取得) は
  後続 change。本 change では pinned checkout を Release の表現として
  使う
- `schneeforge.toml` (distribution manifest) への置換は別 change
- GUI dashboard / wizard の source 選択 UI は P2
