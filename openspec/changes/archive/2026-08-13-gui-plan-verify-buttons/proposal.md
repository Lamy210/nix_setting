## Why

GUI の Ready 画面（通常画面）には Scan/Apply/Rollback/Upgrade ボタンがあるが、Plan（dry-run）と Verify（検証）のボタンが無い。Plan/Verify は First Run Wizard でのみ実行可能で、通常画面では利用できない。

## What Changes

- Ready 画面に Plan / Verify ボタンを追加
- 既存の `run_plan` / `run_verify` コマンドを button → IPC に配線

## Capabilities

### New Capabilities

（なし）

### Modified Capabilities

- `gui-operations`: Ready 画面から Plan/Verify を実行できる requirement を追加

## Impact

- `apps/desktop/dist/index.html`: ボタン追加
- `apps/desktop/dist/main.js`: button → IPC 配線 + VerifyReport 表示
