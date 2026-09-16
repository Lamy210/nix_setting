## ADDED Requirements

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
