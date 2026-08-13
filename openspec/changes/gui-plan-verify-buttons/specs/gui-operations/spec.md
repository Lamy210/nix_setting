## ADDED Requirements

### Requirement: Ready 画面からの Plan/Verify
Ready 状態の GUI SHALL は Plan（dry-run）と Verify（検証）を実行できる。

#### Scenario: Ready 画面で Plan を実行する
- **WHEN** ユーザーが Ready 画面で Plan ボタンを押す
- **THEN** `run_plan` コマンドが dispatch され、dry-run 結果が表示される

#### Scenario: Ready 画面で Verify を実行する
- **WHEN** ユーザーが Ready 画面で Verify ボタンを押す
- **THEN** `run_verify` コマンドが dispatch され、チェック結果が表示される
