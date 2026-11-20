# Design: CI性能評価の集計契約

## 指標

| 指標 | 定義 | 判定 |
| --- | --- | --- |
| server-required gate elapsed | 同一 `check` workflow run の `created_at` から、当該時点のブランチ保護でrequiredの7コンテキストがすべてsuccessとなった最後のジョブ `completed_at` まで。待機・依存関係を含む | 比較可能な baseline と比較 |
| ci-required candidate elapsed | 同一runの `created_at` から、non-required `ci-required` 集約候補ジョブの `completed_at` まで | 診断専用。server-required gate elapsedの代替不可 |
| Rust elapsed runner sum | rust-quality / rust-build-smoke / rust-check の started_at→completed_at 合計。旧baselineの同等責務を明示 | Rust fan-out は +20%以内 |
| workflow elapsed runner sum | 対象check Workflowの全実行ジョブの started_at→completed_at 合計。matrix/reusable の実行子を重複計上しない | 全体最適化完了は +20%以内を維持する提案 |
| execution-only DAG path | 依存グラフに沿う実行時間の最長和 | 診断のみ。gate elapsedの代替不可 |
| paid cost | 実際の請求根拠・runner料金・外部cache費 | 時間から自動換算しない |

現行のブランチ保護でrequiredなのは `openspec-check` / `flake-check` / `rust-check` / `lint` / `bootstrap-test` / `managed-nix-e2e` / `release-artifact-check` の7コンテキストであり、`ci-required` 自体はrequiredではない。後者は `macos-check` や受入ジョブを追加で `needs` に含むため、終了時刻をserver-required完了と誤認しない。必須コンテキストとApp IDは比較時点の保護設定スナップショットに固定し、取得不能・check/jobの対応不明・失敗/skip・run attempt混在・欠損があれば成功判定を保留する。`ci-required`の結果は診断値として別記録する。

既存 required critical path の意味・起点が旧測定と異なる場合は別指標として扱い、実装前に baseline の互換性をレビューする。Rust 対象名は現行構成の例であり、変更時は責務対応表を承認する。

## 比較方法

- 同一 workload、lock、OS/arch、Xcode、runner種別、cache条件、attempt条件を固定し、harness SHA と候補差分を別記録。Workflow revision と展開ジョブ一覧を確認する。
- 各比較条件で成功runを10件以上取得するまでは探索扱い。成功runの中央値とp95、失敗・cancelled・skipped・pending件数、計測欠損を別報告する。
- 短縮率 = (baseline - candidate) / baseline、増加率 = (candidate - baseline) / baseline。各指標のbaseline中央値が0・不明なら判定不能。
- runごとのジョブ合計を算出してから中央値を求める。ジョブごとの中央値の足し算や異なるrunの合成は禁止。
- server-required gate elapsed 25%以上短縮、適用されるguardすべて +20%以内、非回帰、レビューを満たすまで全体最適化完了とはしない。`ci-required`候補の短縮は単独では達成根拠にならない。全体の追加guardの適用は承認時に確定する。
- #340 の uv 個別目標や将来の cache PoC の目標は別 scope。コスト・転送量・安全性・rollback条件を別途定義。

## 反証例

| Rust runner増加 | 全体runner増加 | gate短縮 | 全体最適化の判定案 |
| --- | --- | --- | --- |
| 5% | 30% | 30% | 保留 |
| 25% | 5% | 30% | Rust fan-outを伴う場合は保留 |
| 5% | 5% | 10% | 保留 |
| 5% | 5% | 30% | 他の受入条件も満たした場合のみ候補 |

## 安全・運用

診断は read-only、機密情報を含むログは公開しない。既存の required / CodeQL / release gates、matrix、署名検証を維持する。比較不能・データ欠損の場合は採用保留。障害時は任意の計測処理のみ停止し、通常CIを維持する。追加のrunner利用は予算・停止条件承認後に行う。

## 未決事項

全体 guard の明示的な採用、既存 baseline 起点の対応、#332 archive後の文言整合、計測コホートと予算は提案レビューで確定する。どの条件も承認済みとは扱わない。
