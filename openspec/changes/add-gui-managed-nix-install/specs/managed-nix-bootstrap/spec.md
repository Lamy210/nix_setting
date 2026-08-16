## ADDED Requirements

### Requirement: GUI 向け privilege escalation helper

SchneeForge SHALL は GUI から特権操作を委譲するための escalation helper を core に持つ。helper は macOS では osascript、Linux では pkexec を使う command を構築し、実行する command は SchneeForge 自身の binary に限定する。

#### Scenario: macOS で osascript 経由の command を構築する

- **WHEN** macOS で SchneeForge CLI を管理者権限で再実行する command を構築する
- **THEN** `osascript -e 'do shell script "…" with administrator privileges'` 形式の引数列が構築される
- **AND** 実行する文字列に含まれる quote 等が escape される

#### Scenario: Linux で pkexec 経由の command を構築する

- **WHEN** Linux で SchneeForge CLI を管理者権限で再実行する command を構築する
- **THEN** `pkexec env <env-assignments…> <schneeforge-binary> nix install --yes` 形式の引数列が構築される
- **AND** GUI 表示に必要な環境変数 (DISPLAY / XAUTHORITY / WAYLAND_DISPLAY) が引き継がれる

#### Scenario: 任意の command は実行しない

- **WHEN** helper に SchneeForge binary 以外の実行対象を渡す要求がある
- **THEN** 構築を拒否する、または SchneeForge の subcommand 引数として安全に escape された形式のみを受け付ける

### Requirement: GUI から昇格再実行する install は CLI と同一 policy に従う

GUI 経由で昇格実行される `schneeforge nix install --yes` SHALL は CLI 実行と同一の policy (既存 Nix 拒否 / plan 生成 / ownership 記録 / post-install gate) に従う。GUI 経由であることを理由に確認や検証を省略しない。

#### Scenario: GUI 経由の install も既存 Nix を上書きしない

- **WHEN** GUI から昇格実行された install が既存 Nix を検出する
- **THEN** install は ExistingNixDetected で失敗する
- **AND** GUI は失敗を表示し `/nix` は変更されない

#### Scenario: GUI 経由の install も ownership record を記録する

- **WHEN** GUI 経由の install が成功する
- **THEN** CLI と同一の ownership record が書き込まれ uninstall 対称性が保たれる
