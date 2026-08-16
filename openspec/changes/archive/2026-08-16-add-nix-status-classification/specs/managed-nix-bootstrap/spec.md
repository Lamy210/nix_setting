## ADDED Requirements

### Requirement: Nix 状態分類 (NixStatus)

SchneeForge SHALL は install 済み環境を `Missing` / `Healthy` / `Degraded` / `Broken` の 4 状態に分類する `NixStatus` model を持つ。分類は installation marker (`/nix/store`, `/nix/var/nix`, `/nix/receipt.json`)、receipt の内容、ownership record、runtime 検証 (`nix store ping`) の組合せで決定する。

#### Scenario: Missing — Nix 未導入

- **WHEN** installation marker が一切存在しない
- **THEN** `NixStatus::Missing` に分類する
- **AND** 次アクションとして `schneeforge nix install` を案内する

#### Scenario: Healthy — 完全に稼働する install

- **WHEN** marker が存在し、receipt が読め、`nix store ping` が成功する
- **THEN** `NixStatus::Healthy` に分類する
- **AND** 次アクションとして「対応不要」を表示する

#### Scenario: Degraded — marker 残存だが不完全

- **WHEN** installation marker は存在するが receipt が読めない、または `nix store ping` が失敗する
- **THEN** `NixStatus::Degraded` に分類する
- **AND** 次アクションとして修復手段 (現時点では `schneeforge nix uninstall` + 手動確認、将来は `nix repair`) を案内する
- **AND** install は `ExistingNixDetected` で拒否する (fail-closed を維持)

#### Scenario: Broken — ownership と実態の不一致

- **WHEN** ownership record が存在するが `/nix` 配下の実態が削除されている (またはその逆)
- **THEN** `NixStatus::Broken` に分類する
- **AND** 手動での調査を要する旨と、不一致の内容 (どちら側が残っているか) を表示する

### Requirement: NixStatus の分類 input は injectable である

SchneeForge SHALL は NixStatus の分類 input (marker path 群・receipt path・ownership path・ping 結果) を引数で差し替え可能にする。実環境の `/nix` に依存した test は書かない。

#### Scenario: unit test は実 /nix に依存しない

- **WHEN** NixStatus の unit test を実行する
- **THEN** tempdir 上に marker / receipt を配置して分類を検証する
- **AND** test の成败が実行環境の Nix 有無に影響されない

### Requirement: doctor は NixStatus を表示する

`schneeforge nix doctor` SHALL は診断の冒頭に NixStatus 分類と次アクションを表示する。既存の receipt / runtime 診断項目は維持する。

#### Scenario: doctor が分類を冒頭に表示

- **WHEN** `schneeforge nix doctor` を実行する
- **THEN** `[status]` 欄に 4 状態のいずれかと次アクション案内が表示される
- **AND** 既存の receipt / runtime 診断が引き続き出力される

#### Scenario: doctor はどの状態でも正常終了する

- **WHEN** いずれの状態 (Missing を含む) で `schneeforge nix doctor` を実行する
- **THEN** doctor は非 zero exit で異常終了しない (診断コマンドであるため)
