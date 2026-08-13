## Why

SchneeForge GUI は起動するが、ホスト/ユーザーが「-」表示のままボタンが反応しない。原因は Frontend → IPC → Core → Nix の縦方向 integration が未接続で、fresh install では repository 発見・`nh` 依存の循環により設定適用が成立しないため。CLI は完成度が高い一方、GUI/DMG を「インストーラー」として提供するには未完成。

## What Changes

- GUI の Status を単純な値表示から診断 API（host/repo/manifest/tool の存在・パス・エラー原因）に拡張
- PATH 依存を解消する ToolResolver を core に追加（macOS GUI は Terminal と PATH が異なる）
- `nh` への bootstrap 循環依存を解消（core は `nh` に依存せず `nix run nix-darwin` を直接利用）
- repository 未存在時に First Run Wizard へ誘導（clone/init/config.toml 生成）
- Tauri command を非同期化（UI スレッド占有の解消）
- GUI の状態を state machine で管理（Booting/NeedsSetup/Ready/Applying/Failed）

## Capabilities

### New Capabilities
- `gui-diagnostics`: 診断 Status API。host/repo/manifest/tool の存在・パス・バージョン・エラー原因を返す
- `tool-resolution`: PATH と既知パス（/nix/.../bin, ~/.nix-profile/bin, /opt/homebrew/bin 等）からツールを解決
- `bootstrap-flow`: fresh install のセットアップフロー（repository 発見・clone・config 生成・適用・権限・rollback 意味論）
- `gui-operations`: 非同期の scan/plan/apply/rollback/upgrade 操作、progress 表示、操作ロック、CSP
- `core-operations`: repo-aware な plan/apply/verify/rollback/upgrade/sync と State 永続化（CWD 非依存）

### Modified Capabilities

## Impact

- `schneeforge-core`: ToolResolver、bootstrap ロジック追加。actions の `nh` 依存を除去
- `apps/desktop`: Status 拡張、非同期 command、state machine、First Run Wizard
- `apps/desktop/dist/index.html`: 診断型 UI、Setup フロー
- `bootstrap.sh`: `nh` 非依存の適用パスへ統一
- テスト: ToolResolver / Status / bootstrap の unit test、GUI E2E
