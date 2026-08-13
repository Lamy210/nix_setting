## Why

bootstrap.sh の config.toml 生成は、既に個人化済みでも毎回無条件に上書きする。これにより (1) 所有者が手動編集した config.toml が再適用時に失われる、(2) シェル側で username を検証しないため Rust の `generate_config` と非一貫、という問題がある。

## What Changes

- bootstrap.sh の config.toml 生成を冪等化（既に現在ユーザーで個人化済みなら上書きしない）
- username が空の場合はエラーで停止

## Capabilities

### New Capabilities

（なし）

### Modified Capabilities

- `bootstrap-flow`: config.toml 生成の冪等性 requirement を追加

## Impact

- `bootstrap.sh`
