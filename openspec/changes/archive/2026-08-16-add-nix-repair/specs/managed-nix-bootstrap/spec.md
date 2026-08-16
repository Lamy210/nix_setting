## ADDED Requirements

### Requirement: nix repair は NixStatus に基づいて修復 action を決定する

SchneeForge SHALL は `schneeforge nix repair` command を持ち、`NixStatus` 分類を入力として状態ごとの修復 action を決定する。repair は SchneeForge 単独で安全に実行できる操作 (stale record の削除・案内表示) のみを行い、破壊的な uninstall / 再 install の自動実行は行わない。

#### Scenario: Broken 状態で stale ownership record を削除する

- **WHEN** ownership record は存在するが installation marker が一切存在しない (Broken) 状態で `schneeforge nix repair` を実行する
- **THEN** stale ownership record を削除する
- **AND** 削除後に `schneeforge nix doctor` が `Missing` を表示する状態へ復帰する

#### Scenario: Degraded 状態で receipt 有りは uninstall を案内する

- **WHEN** marker は存在し receipt が読めるが store ping が失敗する (Degraded) 状態で `schneeforge nix repair` を実行する
- **THEN** `schneeforge nix uninstall` による削除と再 install を案内する
- **AND** uninstall を自動実行しない

#### Scenario: Degraded 状態で receipt 無しは手動手順を案内する

- **WHEN** marker のみ残存し receipt が読めない (Degraded) 状態で `schneeforge nix repair` を実行する
- **THEN** upstream が revert できない旨と `sudo schneeforge nix uninstall --force` (build users の手動削除を含む手順) を表示する
- **AND** `/nix` 配下や build users の削除を自動実行しない

#### Scenario: Healthy / Missing は対応不要を表示して正常終了する

- **WHEN** Healthy または Missing 状態で `schneeforge nix repair` を実行する
- **THEN** Healthy は「対応不要」、Missing は install 案内を表示する
- **AND** いずれも file system を変更せず正常終了する

### Requirement: nix repair は dry-run を持つ

`schneeforge nix repair` SHALL は `--dry-run` で実行予定の action を表示するのみで file system を変更しない。

#### Scenario: dry-run は stale record を削除しない

- **WHEN** Broken 状態で `schneeforge nix repair --dry-run` を実行する
- **THEN** 削除対象の ownership record path と実行内容を表示する
- **AND** ownership record は削除されず Broken 状態が維持される

### Requirement: upstream repair hooks / sequoia を wrap する

SchneeForge SHALL は upstream `nix-installer repair hooks` (shell profile 修復) と `repair sequoia` (macOS Sequoia の `_nixbld` 回復) を subprocess 呼び出しする option (`--hooks` / `--sequoia`) を持つ。SchneeForge 側で修復 logic を再実装しない (uninstall と同じ委譲方針)。

#### Scenario: repair hooks は upstream を呼び出す

- **WHEN** `schneeforge nix repair --hooks` を実行する
- **THEN** `nix-installer repair hooks` 相当の upstream command を `/nix/nix-installer` (または cached binary) 経由で subprocess 実行する
- **AND** upstream の stderr を利用者に表示する

#### Scenario: sequoia は明示指定のみで実行する

- **WHEN** `schneeforge nix repair` を option 無しで実行する
- **THEN** `repair sequoia` を自動実行しない (Sequoia 乗っ取りの検出・判定は行わない)
- **AND** macOS 15 環境向けの手順として `--sequoia` の存在を案内に含める場合のみ表示する

### Requirement: doctor の次アクションは repair を案内する

`schneeforge nix doctor` SHALL は Degraded / Broken の次アクション文案として `schneeforge nix repair` を含める。

#### Scenario: Degraded の案内が repair を指す

- **WHEN** Degraded 状態で `schneeforge nix doctor` を実行する
- **THEN** 次アクションに `schneeforge nix repair` が含まれる

#### Scenario: Broken の案内が repair を指す

- **WHEN** Broken 状態で `schneeforge nix doctor` を実行する
- **THEN** 次アクションに `schneeforge nix repair` が含まれる
