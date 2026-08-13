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

- **core は `nh` に依存しない**: macOS は `nix run nix-darwin -- switch --flake <repo>#macbook-air`、Linux は `homeConfigurations.<host>.activationPackage` build + activate。`nh` は「環境構築後の便利 CLI」へ降格
- **ToolResolver を core に追加**: PATH → /nix/var/nix/profiles/default/bin → ~/.nix-profile/bin → /opt/homebrew/bin → /usr/local/bin の順で解決
- **Status を診断 API に拡張**: `ToolStatus { available, path, version }` と repo/manifest/state の存在・エラーを返す
- **Tauri command を `spawn_blocking` で非同期化**: plan/apply/verify/rollback/upgrade の重い操作のみ（scan/status は同期）
- **操作ロックはクロスプロセス flock**: ロックファイル（state と同ディレクトリの `operation.lock`）に対する排他 flock で、CLI（別 terminal）と GUI の同時実行を直列化。取得失敗時は Busy エラーを返す
- **権限昇格**: GUI（.app）は TTY が無いため sudo 直接実行せず、認証を伴う昇格（macOS の authorization / 昇格ヘルパー）を経由する。CLI は従来どおり sudo を要求
- **Vanilla JS を維持**: `withGlobalTauri: true` で `window.__TAURI__` を利用

## Risks / Trade-offs

- `nix run nix-darwin` は registry 版（unpinned）。bootstrap 専用とし、その後の適用は pinned を保つため `nh` を推奨 CLI として残す
- ToolResolver の既知パスはハードコード。platform 追加時に更新が必要
- First Run Wizard の clone 先は `~/nix_setting` 固定（`NIX_SETTING_DIR` で上書き可）
- 非同期化は `spawn_blocking` のスレッド管理コストを伴うが、UI 応答性を優先
