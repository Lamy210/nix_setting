## ADDED Requirements

### Requirement: Status 診断情報の提供
GUI は host/repo/manifest/tool の存在・パス・バージョン・エラー原因を含む診断 Status を取得 SHALL である。

#### Scenario: repository が存在しない場合の原因表示
- **WHEN** ユーザーが GUI を起動し、`~/nix_setting` が存在しない
- **THEN** Status は `repo_exists: false` と `repo_path` を返す
- **AND** GUI は「Repository not configured」と原因を表示する

#### Scenario: manifest が読めない場合の原因表示
- **WHEN** repository は存在するが config.toml が無い
- **THEN** Status は `manifest_found: false` と `manifest_error` を返す
- **AND** GUI は user を「-」ではなくエラー原因を表示する

### Requirement: ツール検出結果の詳細提供
各ツール（nix/nh/git/homebrew）の available/path/version を返す SHALL である。

#### Scenario: ツール検出
- **WHEN** ユーザーが Status を取得する
- **THEN** 各ツールは `available`/`path`/`version` を持つ
