## ADDED Requirements

### Requirement: install 時の username 個人化
bootstrap SHALL は apply 前に、committed された username ではなく OS の実行ユーザー名から config.toml を生成する。

#### Scenario: 別のユーザーが install する
- **WHEN** 別のユーザー（username が committed 値と異なる）が install.sh / bootstrap.sh を実行する
- **THEN** config.toml の `username` がその実行ユーザー名になる
- **AND** 適用後の homeDirectory が実行ユーザーの HOME に一致する

#### Scenario: 所有者が再適用する
- **WHEN** 所有者（username が committed 値と一致）が bootstrap.sh を再実行する
- **THEN** config.toml は実質変化せず、repo に差分が生じない
