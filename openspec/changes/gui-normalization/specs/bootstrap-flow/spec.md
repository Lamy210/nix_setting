## ADDED Requirements

### Requirement: fresh install のセットアップフロー
repository が存在しない場合、通常画面ではなくセットアップフローへ誘導する SHALL である。

#### Scenario: repository が無い場合
- **WHEN** ユーザーが fresh machine で GUI を起動する
- **THEN** GUI は NeedsSetup 状態になり「Set up SchneeForge」を表示する
- **AND** Apply 等の mutating 操作は非表示になる

#### Scenario: セットアップフロー
- **WHEN** ユーザーが Setup を実行する
- **THEN** OS/arch 検出 → Nix/Git 検出 → repository clone → username 確認 → config.toml 生成 → apply の順で進む

### Requirement: nh 非依存の適用
core SHALL は `nh` に依存せず適用できる。

#### Scenario: macOS での適用
- **WHEN** macOS で apply する
- **THEN** `nix run nix-darwin -- switch --flake <repo>#macbook-air` を実行する

#### Scenario: Linux での適用
- **WHEN** Linux で apply する
- **THEN** `homeConfigurations.<host>.activationPackage` を build して activate する
