## ADDED Requirements

### Requirement: Separate scopes for CI performance measurements

CI 性能評価は server-required gate elapsed、non-required `ci-required` candidate elapsed、Rust worker + aggregator elapsed runner time、対象 Workflow 全ジョブの elapsed runner time、実行時間のみの依存経路、実際の費用を別々に記録しなければならない (MUST)。計測範囲・分母・対象 attempt・ジョブ責務を明示し、異なる指標で既存の受入条件を代替してはならない (MUST NOT)。

#### Scenario: 必須7コンテキストと候補集約ジョブの完了時刻が異なる
- **WHEN** `ci-required` 候補が追加の `macos-check` や受入ジョブを待ち、server-side requiredチェックより遅れて完了する
- **THEN** 同一 `check` workflow run の作成時刻から、ブランチ保護でrequiredとされたコンテキストの最後の成功終了までをserver-required gate elapsedとして測定する
- **AND** `ci-required`候補の終了時刻を別の診断指標として保存し、必須ゲートの終了時刻として代用しない
- **AND** 必須コンテキスト・App ID・run attempt・実行ジョブの対応が不明な場合は達成判定を保留する

#### Scenario: Rust fan-out の評価を行う
- **WHEN** Rust worker の分割案を評価する
- **THEN** 既存の Rust worker + aggregator の +20% guard と required critical path の目標を維持する
- **AND** 全 Workflow runner time を別に報告する

#### Scenario: CI 全体の最適化を評価する
- **WHEN** CI 全体の最適化完了を主張する
- **THEN** required critical path 25%以上短縮および全 Workflow runner time +20%以内を確認する
- **AND** Rust fan-out を伴う場合は既存の Rust guard も維持する
- **AND** 既存検証の非回帰とレビューを確認する

#### Scenario: 個別PoCの目標に達する
- **WHEN** uv やキャッシュ等の個別ジョブの事前承認目標を満たす
- **THEN** 個別評価結果として記録する
- **AND** CI 全体の最適化完了とは扱わない

### Requirement: Comparable CI evidence and unresolved decisions

性能比較は同等の workload、lock、runner、system、toolchain、cache状態、run attempt条件に基づかなければならない (MUST)。候補差分と Workflow revision を記録し、各比較条件の成功runが10件未満なら探索結果として扱い、欠損や異なる指標を成功判定で補ってはならない (MUST NOT)。

#### Scenario: 計測条件が整わない
- **WHEN** baseline が不明またはゼロ、サーバー必須コンテキストのスナップショットが取得できない、必須証拠が欠損、実行ジョブ集合の対応が未確認、または比較可能な成功runが10件未満である
- **THEN** 最適化完了の判定を保留する
- **AND** 欠損・失敗・次の検証を明示する

#### Scenario: 同一runの合計を比較する
- **WHEN** runner time の分布を比較する
- **THEN** runごとに対象ジョブの実行時間を合算してから中央値とp95を計算する
- **AND** matrix / reusable の実行子や重複記録を二重計上しない
- **AND** 実際の費用とrunner時間を区別する
