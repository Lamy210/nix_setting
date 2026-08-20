# Change: GUI wizard の managed source 対応 (clone 前提の撤廃)

## Why

v2 §7 で configuration source に managed 表現 (state + flake ref) が追加され、
install.sh の fresh 経路も clone しなくなった
(`switch-install-sh-to-managed-source`)。しかし GUI (desktop app) の
first-run wizard は依然として:

1. **clone 前提**: stepRepo が `run_clone_repo` (git clone) のみを提供し、
   fresh machine に checkout 表現を作らせる
2. **setup から出られない**: 起動時 gate が `!repo_exists` で setup を出す
   ため、managed source だけ初期化した環境 (repo 無し) だと毎回 wizard が
   表示される
3. **Managed Nix install が repo gate 持ち**: wizard の Nix 導入が
   `repo_exists` を前提にしている (escalated CLI が repo の
   `bootstrap-manifest.toml` を必要としていたため。embedded manifest 化で
   解消済みの前提)

GUI と CLI/install.sh で初期化経路が食い違う状態を解消する。

## What Changes

- **ADDED (gui-diagnostics)**: `Diagnostics` に managed source の状態
  (tag / channel / flake ref) を追加。frontend は `repo_exists ||
  managed_source` で「source 初期化済み」を判定する
- **ADDED (gui-operations)**: wizard stepRepo を「configuration source の
  選択」に再構成。managed source (推奨, channel stable 最新) を default に、
  git clone は fork / 開発者向けの選択肢として残す
- **ADDED (gui-operations)**: `run_source_init` Tauri command
  (core `source_init` の呼び出し)
- **MODIFIED (gui-operations)**: Managed Nix install の `repo_exists` gate
  を削除 (CLI sidecar の embedded manifest で動作)
- **MODIFIED (gui-operations)**: 起動時 setup gate を source 初期化状態基準
  に変更

## Impact

- `crates/core/src/diagnostics.rs` / `apps/desktop/src-tauri/src/lib.rs` /
  `apps/desktop/dist/{main.js,index.html}`
- 既存 checkout user (repo_exists=true) の wizard 挙動は不変
- Dashboard からの update 実行 (本体自己 update) は引き続き scope 外
