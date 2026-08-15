## MODIFIED Requirements

### Requirement: 操作の core 集約
CLI と GUI SHALL は同じ core operation を呼ぶ。実ロジックを CLI/GUI に重複させない。全操作 SHALL は `Toolchain` を受け取り、文字列リテラルによる `Command::new` を使わない。

#### Scenario: CLI と GUI の apply
- **WHEN** CLI と GUI が apply する
- **THEN** 両者とも同じ `core::operations::apply` を呼ぶ
- **AND** 両者とも同じ `Toolchain` の nix の絶対パスを spawn する

#### Scenario: apply が Toolchain を使う
- **WHEN** `core::operations::apply` が nix を実行する
- **THEN** `toolchain.nix.path` を `process::run_stream` / `run_capture` へ `&Path` として渡す

#### Scenario: plan の dry-run が Toolchain を使う
- **WHEN** `core::operations::plan` が `nix build --dry-run` を実行する
- **THEN** `toolchain.nix.path` を使う

#### Scenario: sync が Toolchain の git を使う
- **WHEN** `core::operations::sync` が `git pull` を実行する
- **THEN** `toolchain.git.path` を使う

#### Scenario: verify が Toolchain を使う
- **WHEN** `core::operations::verify` が nix / git の存在を検査する
- **THEN** `which(cmd)` ではなく `Toolchain` の解決済みパスを使う
