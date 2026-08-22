## ADDED Requirements

### Requirement: 初回設定は machine 情報を repo へ書き込まない

First Run Wizard SHALL は machine 情報 (username 等) の入力を利用者に求めず、MachineFacts の自動検出結果を表示する。repo 内の file を生成・変更しない。

#### Scenario: username 入力 step が存在しない

- **WHEN** wizard の初回設定を開始する
- **THEN** username の入力 field は表示されず、検出された machine 情報の確認表示になる

#### Scenario: repo が書き換えられない

- **WHEN** wizard の初回設定を完了する
- **THEN** configuration repo 内に file は作成されない
- **AND** machine 情報は state dir の machine.nix として管理される

#### Scenario: 検出結果の確認と再検出

- **WHEN** wizard が machine 情報を表示する
- **THEN** username / home directory / OS / architecture を表示する
- **AND** 検出に失敗した場合は error を表示し、先へ進めない

## REMOVED Requirements

### Requirement: install 時の username 個人化

**Reason**: machine 情報は MachineFacts (実行環境検出) と machine input 注入へ移行し、config.toml の username は読まれなくなる。

### Requirement: config.toml 生成の冪等性

**Reason**: config.toml を生成しなくなるため、冪等性の要件自体が不要になる。
