## Context

SchneeForge GUI は起動するが、Frontend → IPC → Core → Nix の縦方向 integration が未接続。CLI/Nix 構成は完成度が高いが、GUI/DMG を「インストーラー」として提供するには P0（IPC/button/success は修正済み）と P1（診断・ToolResolver・nh 循環・非同期）の解消が必要。

## Goals / Non-Goals

**Goals:**
- 診断 Status API で「なぜ `-` か」を可視化
- ToolResolver で PATH 非依存のツール解決
- `nh` 循環依存の解消（fresh install を成立させる）
- Tauri command の非同期化
- First Run Wizard と state machine

**Non-Goals:**
- React/Vite 等の frontend stack 導入（Vanilla JS を維持）
- package/profile editor
- GUI streaming output（P2 に先送り）
- 署名/notarization

## Decisions

- **core は `nh` に依存しない**: macOS は `nix run --inputs-from <repo> nix-darwin#darwin-rebuild -- switch --flake <repo>#<host>`（repo の flake.lock に pin、registry 非依存）。Linux は `homeConfigurations.<host>.activationPackage` を build して activate する。`nh` は「環境構築後の便利 CLI」へ降格
- **Linux 適用の比較検討（4.3）**: `nix run --inputs-from <repo> home-manager -- switch`（home-manager コマンド経由）と `activationPackage` build + activate（flake output 直接）を比較。**後者を採用** — home-manager コマンドさえ経由せず、flake output の activate を直接実行する方が「core は nh/home-manager に依存しない」に合致し、fresh install での可用性が高い。トレードオフは 2 段階実行になる点
- **ToolResolver を core に追加**: PATH → /nix/var/nix/profiles/default/bin → ~/.nix-profile/bin → /opt/homebrew/bin → /usr/local/bin の順で解決
- **Status を診断 API に拡張**: `ToolStatus { available, path, version }` と repo/manifest/state の存在・エラーを返す
- **Tauri command を `spawn_blocking` で非同期化**: plan/apply/verify/rollback/upgrade の重い操作のみ（scan/status は同期）
- **操作ロックはクロスプロセス flock**: ロックファイル（state と同ディレクトリの `operation.lock`）に対する排他 flock で、CLI（別 terminal）と GUI の同時実行を直列化。取得失敗時は Busy エラーを返す
- **権限昇格（4.4）**: CLI は TTY があるため従来どおり sudo（ユーザーが `sudo schneeforge apply`、または darwin-rebuild が TTY 経由で昇格）を要求。GUI（.app）は TTY が無いため、core は「実行可能ならそのまま、要昇格なら `Error::Precondition`（privilege required）」を返し、実際の昇格は desktop 層の privileged helper（Phase 6）が担う。core が sudo を直接 spawn しない
- **backup/restore 設計（4.5）**: apply 前に対象の dotfiles（`.zshrc` / `.gitconfig` / `starship.toml` 等）を `~/.local/state/schneeforge/backup/<timestamp>/` へコピーして退避。restore（uninstall / rollback「導入前の復元」）時にこれを復元する。generation rollback（Nix 側）とは独立した「導入前状態」の復元として扱う。実装は uninstall 改善（別 change）に含める
- **Vanilla JS を維持**: `withGlobalTauri: true` で `window.__TAURI__` を利用

## Risks / Trade-offs

- `nix run --inputs-from <repo>` は repo の flake.lock が正しいことを前提とする（lock が無い場合は要 `nix flake update`）。bootstrap 専用のため許容
- ToolResolver の既知パスはハードコード。platform 追加時に更新が必要
- Linux apply は build + activate の 2 段階。build 出力のリアルタイム表示は `--out-link` のシンボリックリンク経由で担保
- First Run Wizard の clone 先は `~/nix_setting` 固定（`NIX_SETTING_DIR` で上書き可）
- 非同期化は `spawn_blocking` のスレッド管理コストを伴うが、UI 応答性を優先
