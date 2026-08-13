# gui-operations Specification

## Purpose
TBD - created by archiving change gui-normalization. Update Purpose after archive.
## Requirements
### Requirement: 非同期操作
plan/apply/verify/rollback/upgrade SHALL は UI スレッドを占有せず非同期で実行する。scan と status は軽量のため同期実行でよい。

#### Scenario: apply 実行中も UI が応答する
- **WHEN** ユーザーが apply を実行する
- **THEN** スピナーが表示され、UI は応答し続ける
- **AND** 完了時に出力が表示される

#### Scenario: plan/verify の非同期実行
- **WHEN** ユーザーが plan または verify を実行する
- **THEN** スピナーが表示され、UI は応答し続ける
- **AND** 完了時に結果が表示される

### Requirement: 実行状態の可視化
操作中 SHALL は Running 状態を明示する。

#### Scenario: 実行中
- **WHEN** scan/apply を実行中
- **THEN** スピナーと進捗表示が表示され、ボタンは disable になる

#### Scenario: 失敗
- **WHEN** 操作が失敗する
- **THEN** エラーが表示され、ボタンは再度有効になる

### Requirement: 状態機械
GUI SHALL は SetupState（NeedsSetup/Ready）と OperationState（Idle/Running/Failed）の 2 軸で状態を持つ。

#### Scenario: NeedsSetup 状態
- **WHEN** repository が存在しない
- **THEN** NeedsSetup 状態になり Setup のみ表示する

#### Scenario: Ready + Running の合成
- **WHEN** Ready 状態で apply を実行する
- **THEN** Running(Apply) 状態になり、他の mutating 操作は disable になる

### Requirement: Tauri API 初期化
GUI SHALL は起動時に Tauri IPC が利用可能か検証する。

#### Scenario: Tauri API が無い場合
- **WHEN** `window.__TAURI__` が利用できない
- **THEN** 分かりやすいエラーを表示し、例外で固まらない

### Requirement: ボタンと IPC の対応
操作ボタン SHALL は DOM ID・表示ラベル・backend command を分離して定義する。

#### Scenario: ボタンクリック
- **WHEN** ユーザーがスキャンボタンを押す
- **THEN** 期待した IPC command（run_scan）が dispatch される

### Requirement: 操作結果の判定
backend の CommandOutput.success SHALL に基づいて成功/失敗を表示する。

#### Scenario: backend が失敗を返す
- **WHEN** CommandOutput.success が false
- **THEN** GUI は失敗としてエラーを表示する

### Requirement: プロセス間操作ロック
mutating 操作 SHALL はプロセス間で共有されるロック（ロックファイルの flock）で直列化する。

#### Scenario: CLI と GUI の同時実行
- **WHEN** GUI で apply 実行中に別 terminal から upgrade を実行する
- **THEN** 後発の操作はロックにより拒否または待機する

### Requirement: セキュリティ設定
GUI SHALL は CSP を null にせず、frontend からの system operation を必要最小限の capability に制限する。

#### Scenario: 未使用 plugin
- **WHEN** opener plugin を使う機能が無い
- **THEN** opener の capability と plugin 初期化を削除する

### Requirement: Ready 画面からの Plan/Verify
Ready 状態の GUI SHALL は Plan（dry-run）と Verify（検証）を実行できる。

#### Scenario: Ready 画面で Plan を実行する
- **WHEN** ユーザーが Ready 画面で Plan ボタンを押す
- **THEN** `run_plan` コマンドが dispatch され、dry-run 結果が表示される

#### Scenario: Ready 画面で Verify を実行する
- **WHEN** ユーザーが Ready 画面で Verify ボタンを押す
- **THEN** `run_verify` コマンドが dispatch され、チェック結果が表示される

