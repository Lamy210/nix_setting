# bootstrap-flow Specification

## Purpose
TBD - created by archiving change gui-normalization. Update Purpose after archive.
## Requirements
### Requirement: fresh install のセットアップフロー
repository が存在しない場合、通常画面ではなくセットアップフローへ誘導する SHALL である。

#### Scenario: repository が無い場合
- **WHEN** ユーザーが fresh machine で GUI を起動する
- **THEN** GUI は NeedsSetup 状態になり「Set up SchneeForge」を表示する
- **AND** Apply 等の mutating 操作は非表示になる

#### Scenario: セットアップフロー
- **WHEN** ユーザーが Setup を実行する
- **THEN** OS/arch 検出 → Nix/Git 検出 → repository clone → username 確認 → config.toml 生成 → plan（dry-run）→ 明示的確認 → apply → verify の順で進む
- **AND** plan/verify は apply の前後に分離して実行される

### Requirement: nh 非依存の適用
core SHALL は `nh` に依存せず適用できる。

#### Scenario: macOS での適用
- **WHEN** macOS で apply する
- **THEN** pinned な `nix-darwin#darwin-rebuild` を `--inputs-from <repo>` で利用して switch する

#### Scenario: Linux での適用
- **WHEN** Linux で apply する
- **THEN** `homeConfigurations.<host>.activationPackage` を build して activate する

### Requirement: 権限の明示
管理者権限が必要な操作 SHALL は明示的に権限昇格を要求する。

#### Scenario: macOS での system 変更
- **WHEN** nix-darwin switch が管理者権限を必要とする
- **THEN** 事前に権限が必要であることを示し、明示的な昇格を行う

#### Scenario: GUI（.app）から管理者権限を要求
- **WHEN** .app から起動した GUI が管理者権限を必要とする操作を実行する
- **THEN** TTY に依存せず、認証を伴う昇格（sudo プロンプト相当）を明示的に要求してから実行する

### Requirement: rollback の意味論
rollback SHALL は何を戻すかを明確にする（Generation Rollback / Configuration Revert / Restore Pre-install）。

#### Scenario: 世代ロールバック
- **WHEN** 直前の generation へ戻す
- **THEN** Nix/HM/nix-darwin の generation を明示的に選択して戻す

#### Scenario: 導入前の復元
- **WHEN** SchneeForge 導入前の状態へ戻す
- **THEN** 導入前にバックアップした dotfiles を復元する（generation rollback とは別操作）

### Requirement: install 時の username 個人化
bootstrap SHALL は apply 前に、committed された username ではなく OS の実行ユーザー名から config.toml を生成する。

#### Scenario: 別のユーザーが install する
- **WHEN** 別のユーザー（username が committed 値と異なる）が install.sh / bootstrap.sh を実行する
- **THEN** config.toml の `username` がその実行ユーザー名になる
- **AND** 適用後の homeDirectory が実行ユーザーの HOME に一致する

#### Scenario: 所有者が再適用する
- **WHEN** 所有者（username が committed 値と一致）が bootstrap.sh を再実行する
- **THEN** config.toml は実質変化せず、repo に差分が生じない

