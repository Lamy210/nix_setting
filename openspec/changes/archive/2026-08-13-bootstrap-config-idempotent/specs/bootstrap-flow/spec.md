## ADDED Requirements

### Requirement: config.toml 生成の冪等性
bootstrap の config.toml 生成 SHALL は、既に現在ユーザーで個人化済みなら上書きしない（冪等）。

#### Scenario: 個人化済みの config.toml を保持する
- **WHEN** config.toml が既に現在ユーザーの username で個人化されている
- **THEN** bootstrap は config.toml を上書きしない
- **AND** 手動編集された内容が保持される

#### Scenario: username が確定できない
- **WHEN** OS から username を取得できない（空）
- **THEN** bootstrap はエラーで停止する
