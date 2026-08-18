## ADDED Requirements

### Requirement: 利用可能 release の解決

core SHALL は指定 channel の最新 release tag を remote の tag 列から
解決できる。tag 列は `git ls-remote --tags` (ToolResolver 経由の解決済み
git) から取得し、channel 毎の最新選択は既存の `latest_tag_for_channel`
(stable は prerelease を含まない、preview は prerelease のみ、semver
降順の先頭) に従う。解決した tag の `ReleaseMetadata` は §27 の fetch
(parse + tag 整合検証) で取得する。

#### Scenario: channel の最新 tag 解決

- **WHEN** tag 列 (`v0.1.0`, `v0.2.0-rc.5`, `v0.2.0-rc.4`, `v0.3.0`) と channel が与えられる
- **THEN** stable は `v0.3.0`、preview は `v0.2.0-rc.5` を返す

#### Scenario: channel に該当 tag が無い

- **WHEN** tag 列に channel に合う release tag が存在しない
- **THEN** 解決は fail-closed に error を返す (誤った available を表示しない)

#### Scenario: metadata 取得失敗

- **WHEN** 解決した tag の metadata asset が存在しない (metadata 導入前の release) か network error が発生する
- **THEN** error 種別と理由を呼び出し元に返す (Dashboard は available を未知として表示できる)

### Requirement: Dashboard snapshot の構築

core SHALL は GUI Dashboard (§28) 表示のための snapshot を構築できる。
snapshot は次で構成する:

- `installed`: 実行 binary version / 実効 profile (state 選択 >
  manifest default) / channel (state の source channel、無ければ
  `stable`) / applied revision / applied_at
- `available`: channel の最新 release の ReleaseMetadata。解決失敗時は
  None と理由 (`available_error`)
- `update_available`: available version が実行 binary version より
  新しい場合に限り true

network access は snapshot 構築から分離し、呼び出し元が解決結果を
差し込めること (hermetic test 可能)。

#### Scenario: available が新しい場合

- **WHEN** 実行 version 0.2.0 に対し available が 0.3.0
- **THEN** `update_available` は true

#### Scenario: available が同等以下の場合

- **WHEN** 実行 version 0.2.0-rc.5 に対し available が 0.2.0-rc.5 以下
- **THEN** `update_available` は false

#### Scenario: offline 時の snapshot

- **WHEN** available 解決が network error で失敗する
- **THEN** snapshot は `available: None` と `available_error` に理由を持ち、installed は値を保持する (snapshot 全体は失敗しない)

#### Scenario: channel の決定

- **WHEN** state に source 情報がある場合はその channel、無い場合は `stable` を channel として使う
