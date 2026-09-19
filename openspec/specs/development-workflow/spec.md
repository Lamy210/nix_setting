# development-workflow Specification

## Purpose

SchneeForge の通常開発・OpenSpec lifecycle・release/back-merge・CI required-check migration における統合経路と履歴保全ルールを定義し、`main` / `develop` の保護と仕様・実装・運用手順の整合性を維持する。

## Requirements

### Requirement: Topic branch integration

SchneeForge の通常開発は `develop` を統合先とし、feature/fix/refactor/docs/test/chore の topic branch は `develop` を base にした Pull Request を経由して統合しなければならない (MUST)。topic branch の統合には squash merge を使用し、1 PR = 1 concern を維持しなければならない (MUST)。

#### Scenario: Feature branch を develop へ統合する
- **WHEN** `feat/*` branch の変更が review と required checks を通過する
- **THEN** PR は `develop` へ squash merge される
- **AND** `main` へ直接統合されない

### Requirement: Release branch ancestry preservation

release は `develop` から `release/vX.Y.Z` branch を作成し、`main` への統合では merge commit を使用しなければならない (MUST)。release branch を `main` へ squash merge または rebase merge してはならない (MUST NOT)。

#### Scenario: Release branch を main へ統合する
- **WHEN** `release/vX.Y.Z` が release acceptance と required checks を通過する
- **THEN** `main` への PR は merge commit で統合される
- **AND** release branch の ancestry が `main` に保持される

### Requirement: Release back-merge ancestry preservation

release 完了後、`main` の release commit は `develop` へ merge commit で back-merge しなければならない (MUST)。back-merge に squash または rebase を使用してはならない (MUST NOT)。過去の diverged history を修正するための force push や history rewrite は行ってはならない (MUST NOT)。

#### Scenario: main を develop へ戻す
- **WHEN** release が `main` に merge され tag/release 処理が完了する
- **THEN** `main` から `develop` への back-merge PR を作成する
- **AND** merge commit で統合して共通 ancestry を保持する

### Requirement: OpenSpec change lifecycle

機能追加、breaking change、architecture change、behavior-changing optimization、security pattern change は OpenSpec change を伴わなければならない (MUST)。change は proposal/design/delta spec/tasks を作成し strict validation と proposal approval を完了してから実装しなければならない (MUST)。実装 PR が `develop` へ merge された後、archive と main spec sync は別 `chore/archive-*` PR で行わなければならない (MUST)。

#### Scenario: OpenSpec change を実装して archive する
- **WHEN** 新規 capability change を開始する
- **THEN** topic branch 上で OpenSpec artifacts を作成し `openspec validate <change-id> --strict` を通す
- **AND** proposal approval 後に実装する
- **AND** 実装 PR merge 後に `chore/archive-<change-id>` branch で archive PR を作成する

### Requirement: CI required-check migration safety

branch protection の required checks を新しい aggregator job へ移行するとき、新しい required-check candidate を既存 required checks と並行稼働させ、成功実績を確認する前に既存 required contexts を削除してはならない (MUST NOT)。

#### Scenario: ci-required aggregator を導入する
- **WHEN** follow-up CI change が `ci-required` aggregator job を追加する
- **THEN** 既存 required checks を維持したまま `ci-required` を複数 PR で検証する
- **AND** 安定性確認後にのみ GitHub branch protection の required contexts を移行する

### Requirement: Workflow documentation consistency

`AGENTS.md`、`openspec/project.md`、`docs/STATUS.md` は topic integration、release merge、back-merge、OpenSpec archive 順序について相互に矛盾してはならない (MUST NOT)。

#### Scenario: Workflow policy を変更する
- **WHEN** Git/OpenSpec workflow policy を変更する
- **THEN** 同一 change で関連する repository guidance を同期する
- **AND** CI または review で古い順序が残っていないことを確認する

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

### Requirement: Explicit stable macOS compatibility lanes

SchneeForge の stable macOS CI は mutable な `macos-latest` に依存してはならず (MUST NOT)、明示的な runner / Xcode 組合せで compatibility floor と current stable の両方を検証しなければならない (MUST)。stable compatibility lane は `macos-15` + Xcode 26.3、stable current lane は `macos-26` + Xcode 26.6 を使用しなければならない (MUST)。

#### Scenario: Stable macOS verification を実行する
- **WHEN** PR または対象 branch push で stable macOS verification が起動する
- **THEN** `macos-15` + Xcode 26.3 lane と `macos-26` + Xcode 26.6 lane の両方を実行する
- **AND** active macOS stable verification は `macos-latest` を runner label として使用しない

### Requirement: Explicit Xcode selection without silent fallback

stable macOS lane と macOS release artifact build は runner default Xcode に依存せず、対象 Xcode の `DEVELOPER_DIR` を明示しなければならない (MUST)。指定 Xcode が runner image に存在しない場合、default Xcode へ silent fallback してはならない (MUST NOT)。

#### Scenario: Pinned Xcode が利用可能である
- **WHEN** compatibility lane が `/Applications/Xcode_26.3.app/Contents/Developer` を指定する
- **THEN** build と `xcrun` は Xcode 26.3 toolchain を使用する

#### Scenario: Pinned Xcode が runner image から消える
- **WHEN** 指定した `DEVELOPER_DIR` が利用できない
- **THEN** macOS verification または release build は failure になる
- **AND** runner default Xcode を使って success に見せてはならない

### Requirement: Stable macOS fail-closed aggregation

stable macOS lane は両方の結果を収集し、一方が失敗しても他方の検証を継続しなければならない (MUST)。`macos-check` context は stable lane 全件が `success` の場合のみ success になり、failure / cancelled / skipped のいずれかを fail-closed で扱わなければならない (MUST)。

#### Scenario: Stable lane がすべて成功する
- **WHEN** `macos-15` + Xcode 26.3 と `macos-26` + Xcode 26.6 がともに success になる
- **THEN** `macos-check` は success になる

#### Scenario: Stable lane の1件が non-success になる
- **WHEN** stable macOS lane の1件以上が failure、cancelled、または skipped になる
- **THEN** 他の stable lane は可能な限り実行を継続する
- **AND** `macos-check` は failure になる

### Requirement: macOS release toolchain pinning

PR の `release-artifact-check` と tag release の macOS CLI / DMG build は同じ explicit current-stable host/toolchain contract を使用しなければならない (MUST)。その contract は `macos-26` + Xcode 26.6 とし、active release path は `macos-latest` に依存してはならない (MUST NOT)。

#### Scenario: PR artifact gate を実行する
- **WHEN** `release-artifact-check` が macOS CLI / DMG を build する
- **THEN** runner は `macos-26` を使用する
- **AND** Xcode 26.6 を明示選択する

#### Scenario: Tag release を実行する
- **WHEN** release workflow が macOS CLI / DMG artifact を build する
- **THEN** PR artifact gate と同じ `macos-26` + Xcode 26.6 contract を使用する
- **AND** 既存の共有 release build script を継続利用する

### Requirement: Preview Xcode canary isolation

Public Preview の `xcode-27` runner は stable macOS verification、`macos-check` aggregator、`ci-required`、release artifact path から分離しなければならない (MUST)。preview canary は PR merge gate として実行してはならず (MUST NOT)、`develop` push、scheduled run、または manual run の観測用途に限定しなければならない (MUST)。

#### Scenario: Pull request を検証する
- **WHEN** `pull_request` event で check workflow が起動する
- **THEN** stable macOS lane は実行される
- **AND** `xcode-27` preview canary は実行されない

#### Scenario: Preview canary が失敗する
- **WHEN** `xcode-27` canary が failure になる
- **THEN** stable `macos-check` と current required checks の結果は canary failure に依存しない
- **AND** release artifact creation pathへ preview result を伝播させない

### Requirement: macOS toolchain diagnostics

macOS stable verification、preview canary、release artifact build は build 前に実際の host/toolchain identity を観測可能にしなければならない (MUST)。少なくとも macOS version、architecture、selected `DEVELOPER_DIR`、Xcode version、macOS SDK version を log しなければならない (MUST)。

#### Scenario: macOS build host を診断する
- **WHEN** macOS CI job が build を開始する
- **THEN** log から `sw_vers` の OS version、`uname -m` の architecture、selected Xcode、macOS SDK version を確認できる
- **AND** runner image drift の調査に必要な toolchain identity が残る

### Requirement: macOS workflow contract regression protection

CI は macOS runner/toolchain policy の static contract を検証し、`macos-latest` の再導入、stable Xcode pin の欠落、preview canary の blocking dependency 混入を検出しなければならない (MUST)。

#### Scenario: Workflow contract が意図せず変更される
- **WHEN** `check.yml` または `release.yml` が macOS runner/toolchain contract を破る変更を含む
- **THEN** repository の workflow contract test は failure になる
- **AND** stable / preview / release の責務境界が review 前に検出される

### Requirement: Windows core and CLI portability have a pinned non-required CI gate

The PR workflow SHALL include Windows portability coverage pinned to the explicit `windows-2025` runner label. The Windows gate SHALL compile/test the supported Windows scope (core and CLI), run native launcher smoke coverage, and exercise hermetic WSL backend contract tests. This change MUST NOT add the Windows gate to the current server-side required status-check set.

#### Scenario: Windows code is validated on an explicit runner

- **WHEN** a pull request changes the repository
- **THEN** the Windows portability job runs on `windows-2025`
- **AND** it validates core/CLI Windows compilation and contract tests
- **AND** it does not depend on `windows-latest`

#### Scenario: existing branch protection remains unchanged

- **WHEN** the Windows portability job is introduced
- **THEN** the existing required contexts remain `openspec-check`, `flake-check`, `rust-check`, `lint`, `bootstrap-test`, `managed-nix-e2e`, and `release-artifact-check`
- **AND** Windows coverage is not a required context until a separate migration proves stability and is explicitly approved

### Requirement: Windows CI does not require a registered WSL distro for PR correctness

PR-gating Windows tests SHALL be hermetic with respect to WSL inventory/selection/helper behavior and MUST NOT assume that the hosted runner always has a registered usable Linux distribution. Real WSL2 distro execution MAY be observed in a separate canary, but such a canary MUST remain outside the current required aggregate and release dependencies.

#### Scenario: PR runner has no registered distro

- **WHEN** `windows-2025` provides WSL components but no usable registered WSL2 distro
- **THEN** the core/CLI Windows compile and hermetic backend contract tests can still pass or fail on repository behavior deterministically
- **AND** the PR is not blocked solely by absence of an externally registered distro

#### Scenario: real WSL canary fails

- **WHEN** a scheduled, `develop`, or manually dispatched real-WSL canary cannot provision or reach a WSL2 distro
- **THEN** the result is visible for compatibility tracking
- **AND** it does not fail the existing required `ci-required` aggregate or a release artifact gate

### Requirement: Windows support does not expand release scope in this change

The Windows/WSL2 platform change MUST NOT add Windows release artifacts, Windows Desktop bundles, Windows self-update assets, or a Windows shipping dependency to the existing release workflow. Release behavior for currently supported artifacts SHALL remain unchanged.

#### Scenario: tag release remains unchanged for Windows

- **WHEN** a release tag is processed after this change
- **THEN** no Windows artifact is required or published by this change
- **AND** the existing Linux/macOS release checks remain the shipping contract

### Requirement: macOS destructive acceptance workflow isolation

Managed Nix install/uninstall を行う macOS lifecycle acceptance workflow は
manual / non-required verification として required CI・release path から分離しなければ
ならない (MUST)。fresh-host contract を壊す Nix setup action を acceptance job の
前段で実行してはならない (MUST NOT)。

#### Scenario: Pull request を作成する
- **WHEN** normal PR CI が起動する
- **THEN** destructive macOS lifecycle acceptance workflow は自動実行されない
- **AND** current required contexts は acceptance workflow の結果に依存しない

#### Scenario: develop へ push する
- **WHEN** topic PR が `develop` へ merge される
- **THEN** destructive macOS lifecycle acceptance workflow は自動実行されない
- **AND** operator が `workflow_dispatch` で明示起動した場合のみ lifecycle を開始する

#### Scenario: Fresh-host acceptance job を準備する
- **WHEN** lifecycle job が開始する
- **THEN** job は `cachix/install-nix-action` 等で Nix を事前導入しない
- **AND** script 自身が fresh Nix state を fail-closed 検証する

### Requirement: macOS acceptance workflow contract regression protection

CI SHALL は acceptance workflow の isolation と release-artifact lifecycle marker を
static contract test で検証する。

#### Scenario: Acceptance workflow が required path へ混入する
- **WHEN** workflow に PR/push trigger、required aggregator dependency、または事前 Nix setup が追加される
- **THEN** repository の workflow contract test は failure になる

#### Scenario: Lifecycle coverage が削除される
- **WHEN** checksum verification、install、ExistingNixDetected、uninstall、reinstall のいずれかの marker が script から失われる
- **THEN** repository の workflow contract test は failure になる
