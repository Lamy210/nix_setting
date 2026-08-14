## MODIFIED Requirements

### Requirement: fresh install のセットアップフロー
repository が存在しない場合、通常画面ではなくセットアップフローへ誘導する SHALL である。セットアップフロー SHALL は `Toolchain` を解決した上で、Nix 未検出と flakes 無効を区別して扱い、Nix が未検出の場合は curl|sh ではなく SchneeForge Managed Nix (`schneeforge nix install` / NixOS/nix-installer) の実行を案内する。

#### Scenario: repository が無い場合
- **WHEN** ユーザーが fresh machine で GUI を起動する
- **THEN** GUI は NeedsSetup 状態になり「Set up SchneeForge」を表示する
- **AND** Apply 等の mutating 操作は非表示になる

#### Scenario: セットアップフロー
- **WHEN** ユーザーが Setup を実行する
- **THEN** OS/arch 検出 → Nix/Git 検出（Toolchain 解決）→ repository clone → username 確認 → config.toml 生成 → plan（dry-run）→ 明示的確認 → apply → verify の順で進む
- **AND** plan/verify は apply の前後に分離して実行される

#### Scenario: Nix 未検出時のメッセージ
- **WHEN** セットアップフロー中に Nix が未検出になる
- **THEN** curl|sh の案内ではなく、SchneeForge Managed Nix (`schneeforge nix install` を起動) の案内と [Install Nix] アクションを提供する
- **AND** SchneeForge Managed Nix は NixOS/nix-installer を用いて receipt付きで導入する旨を表示する

#### Scenario: Nix 検出済みで flakes 無効
- **WHEN** セットアップフロー中に Nix は検出されたが flakes が無効
- **THEN** 「flakes を有効化してください」と表示し、[Enable Flakes] アクションを提供する
- **AND** enable 後に再診断して次ステップへ進む
