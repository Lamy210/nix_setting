# bootstrap-flow Specification

## Purpose
SchneeForge の初回インストールから apply / verify までのフローを定義する。fresh machine では Nix / Git / flakes の前提を整え、既存ユーザーは再適用時に committed username を保持する。shell installer (`install.sh` / `bootstrap.sh`) は Rust 側 `ToolResolver` と同じ探索ルールでツールを解決し、重複インストールを避ける。
## Requirements
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
- **WHEN** committed username と一致するユーザーが install.sh / bootstrap.sh を実行する
- **THEN** config.toml を上書きせず、既存の username を維持する

### Requirement: uninstall は副作用を持たない
uninstall コマンド SHALL は削除レベルと手順を表示するのみで、state や設定を変更しない。

#### Scenario: uninstall を実行しても state が残る
- **WHEN** ユーザーが uninstall コマンドを実行する
- **THEN** 削除レベルと手順が表示される
- **AND** state ファイルは削除されない

### Requirement: config.toml 生成の冪等性
bootstrap の config.toml 生成 SHALL は、既に現在ユーザーで個人化済みなら上書きしない（冪等）。

#### Scenario: 個人化済みの config.toml を保持する
- **WHEN** config.toml が既に現在ユーザーの username で個人化されている
- **THEN** bootstrap は config.toml を上書きしない
- **AND** 手動編集された内容が保持される

#### Scenario: username が確定できない
- **WHEN** OS から username を取得できない（空）
- **THEN** bootstrap はエラーで停止する

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

### Requirement: install.sh の Nix 導入は Managed Nix 経路
`install.sh` SHALL は Nix 未検出時に `curl | sh` で nixos.org installer を直接実行せず、SchneeForge Managed Nix (`schneeforge nix install`) を使う。

#### Scenario: Nix 未検出時に Managed Nix で install する
- **WHEN** `resolve_nix` が失敗する環境で `install.sh` を実行する
- **THEN** repository を clone した上で GitHub Release から schneeforge CLI binary を download する
- **AND** binary の SHA256 を Release の CHECKSUMS.txt と突合してから `sudo schneeforge nix install` を実行する
- **AND** `/nix/schneeforge-managed.json` (ownership record) が作成される

#### Scenario: CHECKSUMS 不一致では実行しない
- **WHEN** download した schneeforge binary の SHA256 が CHECKSUMS.txt の値と一致しない
- **THEN** `install.sh` は error で停止し、binary を実行しない

#### Scenario: 既存 Nix は再 install しない
- **WHEN** Nix が既に導入済みの環境で `install.sh` を実行する
- **THEN** Managed Nix install を行わず、flakes 有効化と dotfiles 適用のみを行う

#### Scenario: curl|bash でも D8 確認が取れる
- **WHEN** `curl | bash` で `install.sh` が実行され stdin が pipe である
- **THEN** `schneeforge nix install` は stdin に `/dev/tty` を繋いで実行され、CLI 側の D8 最終確認 (TTY 必須) が機能する

