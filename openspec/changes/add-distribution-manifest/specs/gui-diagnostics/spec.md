## MODIFIED Requirements

### Requirement: Status 診断情報の提供
GUI は host/repo/manifest/tool の存在・パス・バージョン・エラー原因を含む診断 Status を取得 SHALL である。machine 情報 (username) は MachineFacts 検出に移行済みのため、Status は distribution manifest (`schneeforge.toml`) の profile を返す。

#### Scenario: repository が存在しない場合の原因表示
- **WHEN** ユーザーが GUI を起動し、`~/nix_setting` が存在しない
- **THEN** Status は `repo_exists: false` と `repo_path` を返す
- **AND** GUI は「Repository not configured」と原因を表示する

#### Scenario: manifest が読めない場合の原因表示
- **WHEN** repository は存在するが `schneeforge.toml` が無い、または parse に失敗する
- **THEN** Status は `manifest_found: false` と `manifest_error` を返す
- **AND** GUI は profile を「-」ではなくエラー原因を表示する

#### Scenario: profile の表示
- **WHEN** Status を取得した際 manifest が読み込めている
- **THEN** Status は manifest の `[profiles]` default を `profile` として返す

### Requirement: manifest の実行時検証
Status SHALL は manifest の parse だけでなく、schema / profiles / systems の実行時検証結果も返す。

#### Scenario: schema 不一致
- **WHEN** `schneeforge.toml` の schema が未対応の版数
- **THEN** Status は validation error を返し、有効とみなさない

#### Scenario: 未対応 system
- **WHEN** 実行中の system が manifest の `[systems]` に含まれない
- **THEN** Status は validation error を返す
