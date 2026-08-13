## ADDED Requirements

### Requirement: repo-aware 操作
全操作（plan/apply/verify/rollback/upgrade/sync）SHALL は repository path を明示的に受け取り、CWD に依存しない。

#### Scenario: upgrade が repo を指定する
- **WHEN** 別ディレクトリから upgrade を実行する
- **THEN** `nix flake update --flake <repo>` を実行し、CWD ではなく repo を更新する

#### Scenario: sync が repo を指定する
- **WHEN** 別ディレクトリから sync を実行する
- **THEN** `git -C <repo>` で操作する

### Requirement: 操作の core 集約
CLI と GUI SHALL は同じ core operation を呼ぶ。実ロジックを CLI/GUI に重複させない。

#### Scenario: CLI と GUI の apply
- **WHEN** CLI と GUI が apply する
- **THEN** 両者とも同じ `core::operations::apply` を呼ぶ

### Requirement: State 永続化
apply 成功後 SHALL は State（host/revision/applied_at）を core 内で保存する。

#### Scenario: GUI apply 後の State 更新
- **WHEN** GUI から apply が成功する
- **THEN** State が保存され、applied_revision が更新される

#### Scenario: State 保存エラー
- **WHEN** State 保存に失敗する
- **THEN** エラーを返し、成功と偽らない

### Requirement: 同期の安全性
sync SHALL は dirty working tree を検出して競合を防ぐ。

#### Scenario: dirty な repo
- **WHEN** repo に未コミット変更がある状態で sync する
- **THEN** 処理を中止し、先にローカル変更の解決を促す

#### Scenario: 更新の反映
- **WHEN** リモートに更新がある
- **THEN** `--ff-only` で fast-forward のみ反映する
