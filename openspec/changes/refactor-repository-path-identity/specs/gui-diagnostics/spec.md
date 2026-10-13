## ADDED Requirements

### Requirement: repository path の presentation-boundary conversion

Diagnostics SHALL perform repository existence and repository-local checks using the resolved OS-native repository path. The serialized `repo_path` field SHALL remain a `String` for CLI/Desktop/frontend compatibility and SHALL be derived only at the presentation boundary.

#### Scenario: non-UTF-8 repository diagnostics

- **WHEN** supported Unix execution environment が UTF-8 に lossless 変換できない repository path を解決する
- **THEN** `repo_exists` と repository-local diagnostics は元の OS-native path を使って判定する
- **AND** `repo_path` は presentation 用の lossy display representation を返してよい
- **AND** その display representation を manifest/profile/source の filesystem path として再利用しない

#### Scenario: UTF-8 repository diagnostics compatibility

- **WHEN** repository path が valid UTF-8 である
- **THEN** `repo_path` の既存 JSON field name と通常の表示内容は維持される
- **AND** frontend は schema migration 無しで既存 Status を利用できる
