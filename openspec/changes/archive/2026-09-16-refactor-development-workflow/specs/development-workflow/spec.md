## ADDED Requirements

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
