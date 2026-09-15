## ADDED Requirements

### Requirement: Required Rust context preservation

Rust CI を並列化するとき、GitHub branch protection が要求する `rust-check` context 名を維持しなければならない (MUST)。実処理を worker job へ分割しても、external merge gate としての `rust-check` は継続して生成されなければならない (MUST)。

#### Scenario: Rust worker を fan-out する
- **WHEN** Rust quality / CLI smoke / desktop smoke を独立 job に分割する
- **THEN** workflow は引き続き `rust-check` context を生成する
- **AND** GitHub branch protection の server-side required context を同時変更しない

### Requirement: Rust CI fail-closed aggregation

`rust-check` aggregator は Rust worker 全件の完了後に必ず評価され、全 worker が `success` の場合のみ success にならなければならない (MUST)。worker が failure、cancelled、skipped のいずれかなら `rust-check` は failure にならなければならない (MUST)。

#### Scenario: 全 Rust worker が成功する
- **WHEN** `rust-quality`, `rust-cli-smoke`, `rust-desktop-smoke` がすべて success になる
- **THEN** `rust-check` は success になる

#### Scenario: Rust worker が non-success になる
- **WHEN** Rust worker の1件以上が failure、cancelled、または skipped になる
- **THEN** `rust-check` は failure になる
- **AND** dependent-job skip によって merge gate が曖昧にならない

### Requirement: Desktop dependency isolation

Tauri/GTK の Linux system dependencies は desktop build smoke を実行する worker のみに install しなければならない (MUST)。root Cargo workspace の core/CLI quality gate と CLI smoke は desktop 専用 system dependency install を要求してはならない (MUST NOT)。

#### Scenario: Core/CLI quality worker を実行する
- **WHEN** `rust-quality` または `rust-cli-smoke` が実行される
- **THEN** Tauri/GTK apt dependency install を実行しない

#### Scenario: Desktop smoke worker を実行する
- **WHEN** `rust-desktop-smoke` が実行される
- **THEN** desktop build に必要な Tauri/GTK system dependencies を install してから build する

### Requirement: Unified required-check candidate

workflow は現行 branch protection required checks を集約する `ci-required` candidate job を提供しなければならない (MUST)。`ci-required` は現行 required check 全件が success の場合のみ success となり、それ以外の result を fail-closed で扱わなければならない (MUST)。本 change では `ci-required` を server-side required context に昇格してはならない (MUST NOT)。

#### Scenario: 現行 required checks がすべて成功する
- **WHEN** `openspec-check`, `flake-check`, `rust-check`, `lint`, `bootstrap-test`, `managed-nix-e2e`, `release-artifact-check` がすべて success になる
- **THEN** `ci-required` は success になる

#### Scenario: 現行 required check が non-success になる
- **WHEN** 現行 required check の1件以上が failure、cancelled、または skipped になる
- **THEN** `ci-required` は failure になる

### Requirement: Required workflow trigger safety

required context migration 中、workflow-level path filtering 等によって required workflow 自体を trigger しない構成を導入してはならない (MUST NOT)。

#### Scenario: Docs-only PR を作成する
- **WHEN** runtime code を変更しない PR が作成される
- **THEN** current required workflow は通常通り起動する
- **AND** required contexts が trigger skip により Pending のまま残らない

### Requirement: CI optimization measurement

CI critical path optimization は変更前後の実測を記録し、wall-clock と runner resource cost の両方で評価しなければならない (MUST)。目標未達の場合は高速化完了と扱ってはならない (MUST NOT)。

#### Scenario: CI fan-out PR を評価する
- **WHEN** fan-out workflow の成功 run が得られる
- **THEN** baseline と required critical path elapsed time を比較する
- **AND** Rust worker + aggregator の elapsed runner time 合計を比較する
- **AND** 25%以上の critical-path 短縮と +20%以内の runner-time 増加を目標値として結果を記録する
