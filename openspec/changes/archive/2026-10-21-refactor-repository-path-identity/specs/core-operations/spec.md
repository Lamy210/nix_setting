## MODIFIED Requirements

### Requirement: repo-aware 操作

全操作（plan/apply/verify/rollback/update/sync）SHALL は repository path を明示的に受け取り、CWD に依存しない。local repository filesystem path は OS-native path identity を保持し、SchneeForge core は filesystem/process 境界へ到達する前に valid UTF-8 であることを要求してはならない。

#### Scenario: update が repo を指定する

- **WHEN** 別ディレクトリから update を実行する
- **THEN** 対象 repo のみを更新し、CWD は変更しない

#### Scenario: upgrade が repo を指定する

- **WHEN** 別ディレクトリから upgrade (alias) を実行する
- **THEN** `nix flake update --flake <repo>` を実行し、CWD ではなく repo を更新する

#### Scenario: sync が repo を指定する

- **WHEN** 別ディレクトリから sync を実行する
- **THEN** `git -C <repo>` で操作する

#### Scenario: non-UTF-8 local repository path

- **WHEN** supported Unix execution environment が non-UTF-8 bytes を含む local repository path を解決する
- **THEN** core の filesystem operation はその OS-native path identity を使用する
- **AND** SchneeForge は UTF-8 へ変換できないことだけを理由に local filesystem operation を拒否しない

## ADDED Requirements

### Requirement: local repository path と managed flake reference の型分離

SchneeForge core MUST distinguish local repository filesystem paths from managed textual flake references. Local checkout identity SHALL be represented with a path-native type, while managed release references SHALL remain validated canonical UTF-8 flake reference strings.

#### Scenario: local checkout source

- **WHEN** effective source が local checkout である
- **THEN** filesystem consumers は OS-native repository path を受け取る
- **AND** path を display string へ変換した値を filesystem identity として再利用しない

#### Scenario: managed release source

- **WHEN** effective source が managed Release である
- **THEN** Nix reference consumers は canonical `github:<owner>/<repo>/<tag>` textual flake reference を受け取る
- **AND** managed flake reference を local filesystem path として扱わない

### Requirement: repository process argument の OS-native preservation

SchneeForge SHALL pass local repository process arguments without requiring UTF-8 conversion. Process helpers and action argument builders MUST be capable of carrying `OsStr` / `OsString`-equivalent repository values to the spawned process.

#### Scenario: fake process に non-UTF-8 repository argument を渡す

- **WHEN** supported Unix test environment で non-UTF-8 repository path を process argument または current directory として渡す
- **THEN** spawned process boundary で SchneeForge は元の OS-native path identity を保持する
- **AND** test は fake executable が観測した argv/current-directory を用いて preservation を検証する

#### Scenario: upstream Nix の追加制約

- **WHEN** SchneeForge が preserved repository argument を Nix へ渡した後に upstream Nix が flake-reference 制約で拒否する
- **THEN** SchneeForge はその upstream failure を通常の command error として報告する
- **AND** SchneeForge 自身の都合で事前に lossy path へ置換しない
