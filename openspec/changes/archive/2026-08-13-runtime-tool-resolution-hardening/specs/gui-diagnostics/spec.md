## ADDED Requirements

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
