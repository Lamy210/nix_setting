## ADDED Requirements

### Requirement: profile の選択と注入

SchneeForge core SHALL は user が選択した profile を state に保持し、
apply / plan の評価時に flake の `profile` input へ
`--override-input` で注入する。選択が無い場合は manifest の
`profiles.default` を用いる。repo は書き換えない。

#### Scenario: 選択 profile の注入

- **WHEN** state に profile が保存されており apply / plan が実行される
- **THEN** state dir に生成した profile input が `--override-input profile <path>` で渡される
- **AND** flake 内の hosts は `profiles/<選択名>.nix` を import する

#### Scenario: 未選択時は manifest default

- **WHEN** state に profile が保存されていない
- **THEN** manifest の `profiles.default` が選択されたものとして注入される

#### Scenario: 利用不可能な profile の選択

- **WHEN** 保存済み profile が manifest の `profiles.available` に含まれない
- **THEN** core は fail-closed に error を返し、評価を実行しない

#### Scenario: repo 同梱の placeholder で評価できる

- **WHEN** profile input 未生成の状態で repo で `nix flake check` 等が実行される
- **THEN** repo 同梱の placeholder (`defaults/profile.nix` = null) により manifest default 相当の profile で評価が失敗しない

### Requirement: profile 選択の CLI 操作

CLI SHALL は profile の一覧表示・選択・確認を提供する。選択は
manifest 検証後に state へ保存され、repo は書き換えない。

#### Scenario: profile の一覧

- **WHEN** `schneeforge profile list` が実行される
- **THEN** manifest の `profiles.available` と現在の選択 / default が表示される

#### Scenario: profile の選択

- **WHEN** `schneeforge profile set <name>` が manifest の available 内の name で実行される
- **THEN** 選択が state に保存され、以降の apply で反映される

#### Scenario: 不正な profile の選択

- **WHEN** `schneeforge profile set <name>` が available に無い name で実行される
- **THEN** error を返し state は変更されない

#### Scenario: 現在の選択確認

- **WHEN** `schneeforge profile show` が実行される
- **THEN** 解決済み profile (state または manifest default) と出典が表示される
