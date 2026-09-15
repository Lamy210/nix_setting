## ADDED Requirements

### Requirement: Required Rust context preservation

Rust CI を並列化するとき、GitHub branch protection が要求する `rust-check` context 名を維持しなければならない (MUST)。実処理を worker job へ分割しても、external merge gate としての `rust-check` は継続して生成されなければならない (MUST)。

#### Scenario: Rust worker を fan-out する
- **WHEN** Rust quality / build smoke を独立 job に分割する
- **THEN** workflow は引き続き `rust-check` context を生成する
- **AND** GitHub branch protection の server-side required context を同時変更しない

### Requirement: Rust CI fail-closed aggregation

`rust-check` aggregator は Rust worker 全件の完了後に必ず評価され、全 worker が `success` の場合のみ success にならなければならない (MUST)。worker が failure、cancelled、skipped のいずれかなら `rust-check` は failure にならなければならない (MUST)。

#### Scenario: 全 Rust worker が成功する
- **WHEN** `rust-quality` と `rust-build-smoke` がすべて success になる
- **THEN** `rust-check` は success になる

#### Scenario: Rust worker が non-success になる
- **WHEN** Rust worker の1件以上が failure、cancelled、または skipped になる
- **THEN** `rust-check` は failure になる
- **AND** dependent-job skip によって merge gate が曖昧にならない

### Requirement: Desktop dependency isolation and artifact gate separation

Tauri/GTK の Linux system dependencies は desktop compile smoke を実行する worker のみに install しなければならない (MUST)。root Cargo workspace の core/CLI quality gate は desktop 専用 system dependency install を要求してはならない (MUST NOT)。Linux desktop gate は CLI sidecar build profile と一致する release-profile compile coverage を提供し、release artifact の full build coverage は required `release-artifact-check` が継続して提供しなければならない (MUST)。

#### Scenario: Core/CLI quality worker を実行する
- **WHEN** `rust-quality` が実行される
- **THEN** Tauri/GTK apt dependency install を実行しない

#### Scenario: Build smoke worker を実行する
- **WHEN** `rust-build-smoke` が実行される
- **THEN** CLI release smoke を実行する
- **AND** desktop build に必要な Tauri/GTK system dependencies を install する
- **AND** Linux desktop manifest を `cargo check --release` で compile gate する
- **AND** CLI sidecar source と desktop compile の Cargo profile を一致させる

#### Scenario: Full desktop artifact gate を実行する
- **WHEN** required `release-artifact-check` が実行される
- **THEN** release workflow と同一 script による macOS DMG/Tauri full build を継続する
- **AND** Linux build-smoke 側で同一目的の full desktop build を重複実行しない

### Requirement: Required Linux Home Manager coverage separation

required `flake-check` は product default `developer` profile の Linux Home Manager activation derivation を評価しなければならない (MUST)。同時に、supported `minimal` profile を明示的な profile input override で actual realization しなければならない (MUST)。この CI optimization のために product manifest の default profile を変更してはならない (MUST NOT)。

#### Scenario: Default developer profile を検証する
- **WHEN** required `flake-check` が Linux configuration を検証する
- **THEN** default `developer` profile の `homeConfigurations.linux.activationPackage.drvPath` を evaluation する
- **AND** module/config/derivation construction error は required gate を failure にする

#### Scenario: Minimal profile を actual realization する
- **WHEN** required `flake-check` が Linux Home Manager build smoke を実行する
- **THEN** CI fixture で `minimal` profile を `--override-input profile` に明示指定する
- **AND** `homeConfigurations.linux.activationPackage` を `nix build` して actual realization を検証する

#### Scenario: Product default を保持する
- **WHEN** CI realization target を `minimal` に override する
- **THEN** `schneeforge.toml` の default `developer` を変更しない
- **AND** override はその CI build invocation のみに適用する

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

#### Scenario: Fan-out が runner-time guard を超過する
- **WHEN** worker + aggregator の runner time が baseline 比 +20% を超える
- **THEN** その分割案を高速化完了として採用してはならない
- **AND** worker granularity または duplicated build を再設計して再計測する

#### Scenario: Critical-path target を満たさない
- **WHEN** runner-time guard は満たすが required critical path の短縮率が25%未満である
- **THEN** その状態を高速化完了として扱ってはならない
- **AND** 次の dominant required gate を計測して再設計する
