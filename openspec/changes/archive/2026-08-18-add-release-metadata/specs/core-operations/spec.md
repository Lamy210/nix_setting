## ADDED Requirements

### Requirement: Release Metadata の解釈

core SHALL は release asset `schneeforge-release.json` を ReleaseMetadata
として parse・検証できる。metadata は release の version / channel /
source revision / 最低限必要な schneeforge 版数 / 対応 systems を
machine-readable に表現する (v2 §27)。

#### Scenario: metadata の parse

- **WHEN** schema 1 の `schneeforge-release.json` が与えられる
- **THEN** core は version / channel / source_revision / minimum_schneeforge_version / configuration_schema / systems を持つ ReleaseMetadata を返す

#### Scenario: 未対応 schema

- **WHEN** metadata の `schema` が 1 でない
- **THEN** parse は fail-closed に error を返す

#### Scenario: tag との整合検証

- **WHEN** `validate` が tag (例: `v0.2.0-rc.5`) に対して呼ばれる
- **THEN** version が tag の `v` 接頭辞を除いた部分と一致し、channel が version の prerelease 有無から導出されるものと一致することを検証する
- **AND** 不一致は error を返す

#### Scenario: channel の導出

- **WHEN** version に prerelease suffix (例: `-rc.5`) が含まれる
- **THEN** channel は `preview` と判定される
- **WHEN** prerelease suffix が無い
- **THEN** channel は `stable` と判定される

### Requirement: Release Metadata の取得

CLI SHALL は指定 tag の release asset から ReleaseMetadata を取得して
表示できる。asset が存在しない release (過去 release 等) や network
error は fail-closed に error を返す。

#### Scenario: 存在する tag の metadata 表示

- **WHEN** `schneeforge source metadata <tag>` が metadata asset を持つ tag で実行される
- **THEN** version / channel / source_revision / systems 等が表示される

#### Scenario: asset が無い release

- **WHEN** metadata asset を持たない tag (metadata 導入前の release や存在しない tag) で実行される
- **THEN** error を返し、誤った情報を表示しない
