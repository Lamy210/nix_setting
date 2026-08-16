## MODIFIED Requirements

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

## ADDED Requirements

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
