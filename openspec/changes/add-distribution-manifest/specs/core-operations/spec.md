## ADDED Requirements

### Requirement: Distribution Manifest の読み込み

core SHALL は repository root の `schneeforge.toml` を distribution
manifest として読み込む。machine 情報を持たず、repository が何を提供
するか (distribution 名 / profiles / 対応 systems) のみを記述する。

#### Scenario: schneeforge.toml の読み込み

- **WHEN** repository に `schneeforge.toml` が存在する
- **THEN** core はこれを parse し distribution 名と profiles を返す

#### Scenario: schneeforge.toml が無い場合

- **WHEN** repository に `schneeforge.toml` が存在しない
- **THEN** manifest 読み込みは構造化 error を返し、`manifest_found` は false になる

#### Scenario: 旧 config.toml は読み込まない

- **WHEN** repository に v1 形式の `config.toml` のみが存在する
- **THEN** core は `config.toml` を読み込まず schneeforge.toml 無しと同じ扱いにする

### Requirement: Distribution Manifest の検証

manifest SHALL は読み込み後に実行時検証される。検証は schema 版数、
default profile の妥当性、実行 system の対応を含む。

#### Scenario: schema 不一致

- **WHEN** `schneeforge.toml` の `schema` が 1 でない
- **THEN** validation error を返し有効とみなさない

#### Scenario: default profile が available に無い

- **WHEN** `[profiles]` の `default` が `available` に含まれない
- **THEN** validation error を返す

#### Scenario: 実行 system が未対応

- **WHEN** 実行中の system が `[systems]` に含まれない
- **THEN** validation error (warning ではなく error) を返す

#### Scenario: 有効な manifest

- **WHEN** schema = 1 / default ∈ available / 実行 system が systems に含まれる
- **THEN** validation は valid になる
