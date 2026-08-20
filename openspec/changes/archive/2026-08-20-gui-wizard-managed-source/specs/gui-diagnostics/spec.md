## ADDED Requirements

### Requirement: managed source 状態の診断報告

`get_status` (Diagnostics) SHALL は state の managed Release source を
tag / channel / flake ref 付きで報告する。frontend は
「source が初期化済み」を `repo_exists || managed_source` で判定できる。

#### Scenario: managed source 初期化済みの machine

- **WHEN** state が managed Release source (repo checkout 無し) を示す
- **THEN** `managed_source` に tag / channel / flake ref が設定され、
  frontend は setup wizard を表示せず main UI を表示する

#### Scenario: checkout 表現 / 未初期化

- **WHEN** state に managed source が無い (checkout 表現または未初期化)
- **THEN** `managed_source` は null で、従来通り `repo_exists` のみで
  判断する
