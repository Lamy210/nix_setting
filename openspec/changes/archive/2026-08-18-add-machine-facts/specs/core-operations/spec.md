## ADDED Requirements

### Requirement: MachineFacts の自動検出

SchneeForge core SHALL は machine 固有情報 (username / home directory / OS / architecture / hostname) を `MachineFacts` として実行環境から自動検出する。利用者に入力させず、configuration repo から読まない。

#### Scenario: 検出は実行環境から行う

- **WHEN** core が MachineFacts を検出する
- **THEN** username は実行 user、home directory は実効 HOME、OS / architecture は実行環境の値を返す
- **AND** configuration repo 内の file から username を読まない

#### Scenario: 検出不能な項目は error にする

- **WHEN** username または home directory が検出できない
- **THEN** error を返し、空文字のまま処理を続けない

### Requirement: machine input の生成と注入

SchneeForge core SHALL は apply / plan の評価時に MachineFacts から `machine.nix` を生成し、flake の `machine` input へ `--override-input` で注入する。評価は pure (builtins.getEnv 不使用) を維持する。

#### Scenario: apply 時に machine input が注入される

- **WHEN** apply が flake 評価を実行する
- **THEN** state dir に生成した machine.nix が `--override-input machine <path>` で渡される
- **AND** flake 内の `inputs.machine` は hosts が参照する username / homeDirectory をその machine の値で解決する

#### Scenario: repo は書き換えられない

- **WHEN** apply / plan が実行される
- **THEN** configuration repo 内の file (config.toml 含む) は作成・変更されない
- **AND** machine.nix は repo 外の state dir に生成される

#### Scenario: clone 直後の repo も評価できる

- **WHEN** machine.nix 未生成の状態で `nix flake check` 等が repo で実行される
- **THEN** repo 同梱の placeholder (`defaults/machine.nix`) により評価が失敗しない

## MODIFIED Requirements

### Requirement: repo-aware 操作

全操作（plan/apply/verify/rollback/upgrade/sync）SHALL は repository path を明示的に受け取り、CWD に依存しない。

#### Scenario: upgrade が repo を指定する

- **WHEN** 別ディレクトリから upgrade を実行する
- **THEN** `nix flake update --flake <repo>` を実行し、CWD ではなく repo を更新する

#### Scenario: sync が repo を指定する

- **WHEN** 別ディレクトリから sync を実行する
- **THEN** `git -C <repo>` で操作する

#### Scenario: apply は machine input を注入する

- **WHEN** apply が実行される
- **THEN** MachineFacts から生成した machine input を `--override-input` で注入する
