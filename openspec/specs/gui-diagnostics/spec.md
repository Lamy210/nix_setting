# gui-diagnostics Specification

## Purpose
desktop (Tauri) が表示する環境診断情報のスキーマと取得方法を定義する。`Diagnostics` は解決済み `Toolchain`、`NixHealth`（store 接続性 / flakes 有効性 / XDG state フォルダ欠如等）、manifest 検証結果を含み、ユーザーが apply 実行前に環境が整っているか判断できるようにする。
## Requirements
### Requirement: Status 診断情報の提供
GUI は host/repo/manifest/tool の存在・パス・バージョン・エラー原因を含む診断 Status を取得 SHALL である。machine 情報 (username) は MachineFacts 検出に移行済みのため、Status は distribution manifest (`schneeforge.toml`) の profile を返す。

#### Scenario: repository が存在しない場合の原因表示
- **WHEN** ユーザーが GUI を起動し、`~/nix_setting` が存在しない
- **THEN** Status は `repo_exists: false` と `repo_path` を返す
- **AND** GUI は「Repository not configured」と原因を表示する

#### Scenario: manifest が読めない場合の原因表示
- **WHEN** repository は存在するが `schneeforge.toml` が無い、または parse に失敗する
- **THEN** Status は `manifest_found: false` と `manifest_error` を返す
- **AND** GUI は profile を「-」ではなくエラー原因を表示する

#### Scenario: profile の表示
- **WHEN** Status を取得した際 manifest が読み込めている
- **THEN** Status は manifest の `[profiles]` default を `profile` として返す

### Requirement: ツール検出結果の詳細提供
各ツール（nix/nh/git/homebrew）の available/path/version を返す SHALL である。

#### Scenario: ツール検出
- **WHEN** ユーザーが Status を取得する
- **THEN** 各ツールは `available`/`path`/`version` を持つ

### Requirement: Platform と ConfigurationTarget の分離
OS/arch 検出（Platform）と、どの configuration を使うか（ConfigurationTarget）を分けて返す SHALL である。

#### Scenario: 異なるハードウェア
- **WHEN** M1 Mac mini と M4 MacBook Air で Status を取得する
- **THEN** Platform はどちらも macOS/arm64 だが、ConfigurationTarget は別々に識別できる

### Requirement: manifest の実行時検証
Status SHALL は manifest の parse だけでなく、schema / profiles / systems の実行時検証結果も返す。username の検証は MachineFacts 移行により廃止された (旧 scenario「空 username」「実行ユーザーとの不一致」は削除)。

#### Scenario: schema 不一致
- **WHEN** `schneeforge.toml` の schema が未対応の版数
- **THEN** Status は validation error を返し、有効とみなさない

#### Scenario: 空 username
- **WHEN** (廃止) username は manifest に存在しなくなったため本 scenario は適用されない
- **THEN** username 起因の validation error は発生しない

#### Scenario: 実行ユーザーとの不一致
- **WHEN** (廃止) username は manifest に存在しなくなったため本 scenario は適用されない
- **THEN** username 不一致の警告は発生しない

#### Scenario: default profile が未定義
- **WHEN** `schneeforge.toml` の `[profiles]` に `default` が無い
- **THEN** Status は validation error を返し、有効とみなさない

#### Scenario: 未対応 system
- **WHEN** 実行中の system が manifest の `[systems]` に含まれない
- **THEN** Status は validation error を返す

### Requirement: Nix Health Check 情報の提供

Diagnostics SHALL は `NixHealth`（installed / executable / version / store_accessible / flakes_available / source / error）を返し、単なる available/path/version より詳細な状態を GUI へ提供する。

#### Scenario: Nix が完全に healthy
- **WHEN** nix が解決済みで store に接続でき flakes が有効
- **THEN** `NixHealth.installed == true` / `store_accessible == true` / `flakes_available == true` になる
- **AND** `executable` / `version` / `source` が設定される

#### Scenario: store へ接続できない
- **WHEN** nix バイナリは存在するが daemon が止まっている
- **THEN** `NixHealth.installed == true` / `store_accessible == false` になる
- **AND** `error` に `<nix> store ping` の stderr が入る

#### Scenario: flakes が無効
- **WHEN** nix は存在するが `<nix> config show experimental-features` の出力に `flakes` が含まれない
- **THEN** `NixHealth.installed == true` / `flakes_available == false` になる

#### Scenario: store ping が失敗
- **WHEN** `<nix> store ping` が非ゼロ exit で返る
- **THEN** `NixHealth.store_accessible == false` になる
- **AND** `NixHealth.error` に ping の stderr が入る

#### Scenario: Nix が未検出
- **WHEN** いかなる探索先にも nix が存在しない
- **THEN** `NixHealth.installed == false` になる
- **AND** `executable` / `version` / `source` は None になる
- **AND** `error` に「Nix is not installed」が入る

### Requirement: NixStatus 分類の提供

Diagnostics SHALL は `NixStatus` 分類 (`Missing` / `Healthy` / `Degraded` / `Broken`) と次アクション文案を `nix_status` field として含める。分類 logic は managed-nix-bootstrap capability の `NixStatus` model に一本化し、GUI 独自の判定をしない。

#### Scenario: Missing 環境で GUI を起動
- **WHEN** installation marker が存在しない環境で GUI の Status を取得する
- **THEN** `nix_status.status == "Missing"` が返る
- **AND** `nix_status` に `schneeforge nix install` を案内する next action が含まれる

#### Scenario: degraded install が NixHealth だけの表現と区別される
- **WHEN** `/nix/store` のみ残存し receipt が読めない環境で Status を取得する
- **THEN** `nix_status.status == "Degraded"` が返る
- **AND** GUI は単なる「Nix あり」と同じ表示で扱わない

### Requirement: wizard は Managed Nix install を案内する

First Run Wizard SHALL は Nix 未導入 (Missing) の場合、legacy な `curl -L https://nixos.org/nix/install | sh` ではなく SchneeForge 自身の Managed Nix install (`sudo schneeforge nix install`) を案内する。

#### Scenario: Missing 状態で wizard の案内を表示
- **WHEN** Missing 環境で wizard の前提条件 step を表示する
- **THEN** `sudo schneeforge nix install` の案内が表示される
- **AND** legacy な `curl | sh` の command は表示されない

#### Scenario: Degraded / Broken 状態では分類を表示する
- **WHEN** Degraded または Broken 環境で wizard の前提条件 step を表示する
- **THEN** NixStatus の分類 label と next action が表示される
- **AND** wizard は install 案内だけで済ませず、修復 / 手動調査の案内を出す

