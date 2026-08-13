## ADDED Requirements

### Requirement: 非同期操作
apply/rollback/upgrade/verify SHALL は UI スレッドを占有せず非同期で実行する。

#### Scenario: apply 実行中も UI が応答する
- **WHEN** ユーザーが apply を実行する
- **THEN** スピナーが表示され、UI は応答し続ける
- **AND** 完了時に出力が表示される

### Requirement: 実行状態の可視化
操作中 SHALL は Running 状態を明示する。

#### Scenario: 実行中
- **WHEN** scan/apply を実行中
- **THEN** スピナーと進捗表示が表示され、ボタンは disable になる

#### Scenario: 失敗
- **WHEN** 操作が失敗する
- **THEN** エラーが表示され、ボタンは再度有効になる

### Requirement: 状態機械
GUI SHALL は Booting/NeedsSetup/Ready/Applying/Failed の状態を持つ。

#### Scenario: NeedsSetup 状態
- **WHEN** repository が存在しない
- **THEN** NeedsSetup 状態になり Setup のみ表示する
