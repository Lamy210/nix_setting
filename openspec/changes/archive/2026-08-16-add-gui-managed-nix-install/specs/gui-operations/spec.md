## ADDED Requirements

### Requirement: wizard は GUI から Managed Nix install を実行できる

First Run Wizard SHALL は Nix 未導入 (Missing) の場合、ターミナルでの CLI 手打ちに頼らず GUI の操作で Managed Nix install を完了できる。install は core の `ManagedNix::prepare_plan()` / `execute_plan()` 経路 (CLI と同一 policy) を使う。

#### Scenario: Nix 未導入時に install ボタンが表示される

- **WHEN** wizard の前提確認で Nix が未導入 (Missing) と判定される
- **AND** repository が既に clone 済みである
- **THEN** Managed Nix を導入する操作 (ボタン) が表示される
- **AND** CLI の手打ち案内が escalation が利用できない環境向け fallback として表示される

#### Scenario: repository 未 clone 時は install を offering しない

- **WHEN** wizard の前提確認で Nix が未導入かつ repository が未 clone である
- **THEN** Managed Nix の導入操作は表示されず repository 設定 step への誘導が表示される
- **AND** install 操作を提供しない理由として repository の clone が必要であることが表示される

#### Scenario: detailed plan 表示から最終確認を経て install する

- **WHEN** ユーザーが Managed Nix の導入操作を実行する
- **THEN** plan 生成後に detailed plan (actions 概要) が表示される
- **AND** ユーザーの最終確認操作を受けた場合のみ install が実行される
- **AND** 確認を取り消した場合 `/nix` は変更されない

#### Scenario: install progress が表示される

- **WHEN** install が実行中
- **THEN** phase (download / verify / plan / install / post-install) が順次表示され UI は応答し続ける
- **AND** 完了時に receipt / ownership の確認結果が表示される

### Requirement: GUI install の privilege escalation

GUI process SHALL は自身を root にせず、特権が必要な操作を別 process として昇格実行する。macOS は osascript、Linux は pkexec を使う。

#### Scenario: 非 root で install 操作を実行する

- **WHEN** GUI が root 以外で動作しておりユーザーが install を確認した
- **THEN** GUI bundle に同梱された SchneeForge CLI sidecar (`schneeforge nix install --yes`) が管理者権限で再実行される
- **AND** GUI process 自身は root 権限を取得しない
- **AND** 昇格先の process に `NIX_SETTING_DIR` (repo 位置) が引き継がれる

#### Scenario: 昇格が拒否された場合は fallback 案内を出す

- **WHEN** ユーザーが昇格の認証をキャンセルする、または osascript / pkexec が利用できない
- **THEN** install を実行せずエラーを表示する
- **AND** CLI (`sudo schneeforge nix install`) での実行案内を表示する

### Requirement: GUI install の確認責任

GUI SHALL は upstream installer を `--no-confirm` 相当で呼ぶ場合、detailed plan 表示とユーザーの明示的な最終確認を確認 gate とする (D8 の GUI 版)。

#### Scenario: 確認操作なしに install が始まらない

- **WHEN** detailed plan が表示されている
- **THEN** ユーザーの確認操作を受け取るまで install phase へ遷移しない
