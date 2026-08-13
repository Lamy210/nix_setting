## Why

`uninstall` コマンドは「削除レベル」と手順を表示するだけの案内コマンドなのに、実際に state ファイルを削除する副作用を持つ（表示コマンドなのに state を消す）。表示と副作用が混在しており、ユーザーがレベルを選択する前に state が消えてしまう。

## What Changes

- `uninstall` を純粋な表示コマンド（削除レベル・手順の案内のみ）にし、state 削除の副作用を排除する
- 使用されなくなる core の `uninstall`（state 削除）を削除する

## Capabilities

### New Capabilities

（なし）

### Modified Capabilities

- `bootstrap-flow`: uninstall は副作用を持たない requirement を追加

## Impact

- `crates/cli/src/main.rs`: uninstall から state 削除を除去
- `crates/core/src/bootstrap.rs`: `uninstall` を削除
- テスト: 既存テスト更新
