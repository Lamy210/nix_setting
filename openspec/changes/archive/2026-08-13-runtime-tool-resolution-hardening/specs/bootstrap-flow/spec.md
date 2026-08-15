## MODIFIED Requirements

### Requirement: fresh install のセットアップフロー
repository が存在しない場合、通常画面ではなくセットアップフローへ誘導する SHALL である。セットアップフロー SHALL は `Toolchain` を解決した上で、Nix 未検出と flakes 無効を区別して扱う。

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
- **THEN** 「Nix をインストールしてください（curl -L https://nixos.org/nix/install | sh）」を表示する

#### Scenario: Nix 検出済みで flakes 無効
- **WHEN** セットアップフロー中に Nix は検出されたが flakes が無効
- **THEN** 「flakes を有効化してください」と表示し、[Enable Flakes] アクションを提供する
- **AND** enable 後に再診断して次ステップへ進む

### Requirement: install 時の username 個人化
bootstrap SHALL は apply 前に、committed された username ではなく OS の実行ユーザー名から config.toml を生成する。

#### Scenario: 別のユーザーが install する
- **WHEN** 別のユーザー（username が committed 値と異なる）が install.sh / bootstrap.sh を実行する
- **THEN** config.toml の `username` がその実行ユーザー名になる
- **AND** 適用後の homeDirectory が実行ユーザーの HOME に一致する

#### Scenario: 所有者が再適用する
- **WHEN** committed username と一致するユーザーが install.sh / bootstrap.sh を実行する
- **THEN** config.toml を上書きせず、既存の username を維持する

## ADDED Requirements

### Requirement: shell installer のツール解決共通化
`install.sh` / `bootstrap.sh` SHALL は `scripts/resolve-tools.sh` の `resolve_nix` / `resolve_git` / `resolve_brew` 関数を経由し、Rust 側 `ToolResolver` と同じ探索優先度を使う。

#### Scenario: install.sh が resolve_nix を使う
- **WHEN** `install.sh` が Nix の存在を判定する
- **THEN** `command -v nix` ではなく `resolve_nix` 関数の結果を使う

#### Scenario: 既存 Nix の再インストール防止
- **WHEN** PATH に nix が無いが `/nix/var/nix/profiles/default/bin/nix` が存在する状態で install.sh を実行する
- **THEN** `resolve_nix` がそのパスを返すため Nix の再インストールをスキップする

#### Scenario: bootstrap.sh が resolved Nix を使う
- **WHEN** `bootstrap.sh` が `nix run` / `nix build` を実行する
- **THEN** 文字列 `nix` ではなく `$NIX_BIN`（resolve_nix の結果）を使う
