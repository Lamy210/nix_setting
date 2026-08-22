## ADDED Requirements

### Requirement: wizard による managed source の初期化

wizard の source 設定 step SHALL は managed source の初期化
(core `source_init` と同等: channel stable の最新 tag 解決 + ReleaseMetadata
検証) を提供する。git clone は fork / 開発者向けの選択肢として維持する。

#### Scenario: fresh machine で clone せず初期化する

- **WHEN** repository も state も無い状態で wizard の source 設定 step で
  managed source (既定) を選ぶ
- **THEN** `run_source_init` が呼ばれ、git clone は発生せず、state に
  managed source が設定される

#### Scenario: clone は選択肢として残る

- **WHEN** source 設定 step で git clone を選ぶ
- **THEN** 従来の `run_clone_repo` (URL 入力付き) が使える

### Requirement: 起動時の setup 表示条件

desktop app 起動時の setup wizard 表示 SHALL は「source が未初期化」
(repository checkout 無し かつ managed source 無し) の場合のみ行う。

#### Scenario: managed source 初期化済みなら setup を表示しない

- **WHEN** managed source のみ初期化済み (repo checkout 無し) の状態で
  app を起動する
- **THEN** setup wizard は表示されず main UI (Dashboard) が表示される

#### Scenario: 未初期化なら従来通り setup

- **WHEN** repository も managed source も無い状態で app を起動する
- **THEN** 従来通り setup wizard を表示する

### Requirement: wizard は GUI から Managed Nix install を実行できる (repository 非依存)

First Run Wizard SHALL は Nix 未導入 (Missing) の場合、ターミナルでの CLI 手打ちに頼らず GUI の操作で Managed Nix install を完了できる。install は core の `ManagedNix::prepare_plan()` / `execute_plan()` 経路 (CLI と同一 policy) を使う。repository checkout は前提としない (escalated CLI sidecar の embedded manifest で動作)。

#### Scenario: Nix 未導入時に install ボタンが表示される

- **WHEN** wizard の前提確認で Nix が未導入 (Missing) と判定される
- **THEN** Managed Nix を導入する操作 (ボタン) が表示される
- **AND** CLI の手打ち案内が escalation が利用できない環境向け fallback として表示される

#### Scenario: repository 未 clone でも install を提供する

- **WHEN** wizard の前提確認で Nix が未導入かつ repository が未 clone である
- **THEN** Managed Nix の導入操作が表示され、そのまま install を開始できる

#### Scenario: detailed plan 表示から最終確認を経て install する

- **WHEN** ユーザーが Managed Nix の導入操作を実行する
- **THEN** plan 生成後に detailed plan (actions 概要) が表示される
- **AND** ユーザーの最終確認操作を受けた場合のみ install が実行される
- **AND** 確認を取り消した場合 `/nix` は変更されない

#### Scenario: install progress が表示される

- **WHEN** install が実行中
- **THEN** phase (download / verify / plan / install / post-install) が順次表示され UI は応答し続ける
- **AND** 完了時に receipt / ownership の確認結果が表示される

## REMOVED Requirements

### Requirement: wizard は GUI から Managed Nix install を実行できる

First Run Wizard SHALL は Nix 未導入 (Missing) の場合、ターミナルでの CLI 手打ちに頼らず GUI の操作で Managed Nix install を完了できる。install は core の `ManagedNix::prepare_plan()` / `execute_plan()` 経路 (CLI と同一 policy) を使う。repository checkout は前提としない (escalated CLI sidecar の embedded manifest で動作)。

#### Scenario: Nix 未導入時に install ボタンが表示される

- **WHEN** wizard の前提確認で Nix が未導入 (Missing) と判定される
- **THEN** Managed Nix を導入する操作 (ボタン) が表示される
- **AND** CLI の手打ち案内が escalation が利用できない環境向け fallback として表示される

#### Scenario: repository 未 clone 時は install を offering しない

- **WHEN** wizard の前提確認で Nix が未導入かつ repository が未 clone である
- **THEN** Managed Nix の導入操作が表示され、そのまま install を開始できる

#### Scenario: detailed plan 表示から最終確認を経て install する

- **WHEN** ユーザーが Managed Nix の導入操作を実行する
- **THEN** plan 生成後に detailed plan (actions 概要) が表示される
- **AND** ユーザーの最終確認操作を受けた場合のみ install が実行される
- **AND** 確認を取り消した場合 `/nix` は変更されない

#### Scenario: install progress が表示される

- **WHEN** install が実行中
- **THEN** phase (download / verify / plan / install / post-install) が順次表示され UI は応答し続ける
- **AND** 完了時に receipt / ownership の確認結果が表示される
