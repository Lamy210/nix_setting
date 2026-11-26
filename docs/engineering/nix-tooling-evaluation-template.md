# Nix tooling adoption: decision / evidence テンプレート

各候補の個別OpenSpecにコピーして使用する。これは記録用テンプレートであって承認や実行スクリプトではない。unknownには理由と調査先を記載し、未計測の値を0や成功に置き換えない。

## Decision record

| 項目 | 記入欄 |
| --- | --- |
| Change ID / proposal approval URL | unknown |
| Tool / version / immutable pin / source URL / license / review date | unknown |
| Maintainer状況・脆弱性・配布条件と根拠 | unknown |
| 利用者・現在の不便・期待便益（仮説） | unknown |
| 代替案・重複調査（導入済み/有効/実行/被覆） | unknown |
| 導入面（developer/CI/infra/product）と対象外 | unknown |
| 各system・実行段階の範囲・受入条件 | unknown |
| 事前比較方法・KPI・反証・必要negative test | unknown |
| Scope / 費用・時間・容量上限 / 停止条件 | unknown |
| Owner / reviewer / approver / operator / review date | unknown |
| Evidence links / 採用判定 / 未解決事項 / 再評価トリガー | unknown |
| Disable / revert SHA・復元対象 / 再検証 | unknown |

## Gate 0（PoC前） / 正式採用前再審査

各項目に pass/fail/unknown と根拠を入れる。対象外は理由を明示し審査する。PoC前に必要な安全性の根拠がないなら実行しない。正式採用には実測、対象OS検証、運用費、停止/rollbackを含める。

| 審査項目 | PoC前 | 採用前 | 根拠/確認先 |
| --- | --- | --- | --- |
| 固定取得元、license、供給網 | unknown | unknown | unknown |
| 隔離、最小権限、秘密鍵/credential、artifact promotion | unknown | unknown | unknown |
| scope、予算、停止条件、既存required維持 | unknown | unknown | unknown |
| 配布範囲、保守、security情報 | unknown | unknown | unknown |
| 対象platform実測とnegative test | 計測予定 | unknown | unknown |
| 効果・費用・既存代替との比較 | 計測予定 | unknown | unknown |
| rollback、限定導入、運用復旧 | 停止計画を審査 | unknown | unknown |

Gate 0を満たさない場合はNo-Go/Defer。PoC前の効果/実測が未知でも安全審査を独立して判断する。

## Gate 1 / 2

| 軸 | Weight | 評価 0–5 または unknown | 根拠 |
| --- | ---: | --- | --- |
| Security / supply chain | 25 | unknown | unknown |
| Reliability / rollback | 20 | unknown | unknown |
| Maintenance / ecosystem | 15 | unknown | unknown |
| Measured benefit | 15 | unknown | unknown |
| Total cost | 15 | unknown | unknown |
| Platform / integration | 10 | unknown | unknown |

0=不適合、1=重大課題、2=一部充足、3=必要水準、4=追加便益、5=優位性確認。weighted points = sum(weight × rating / 5)、unknownは加点しない。75点は暫定的な審査目安で安全性を代替しない。

判定: unknown（Go (PoC) / Go (本番利用) / Conditional / No-Go / Defer のいずれかを根拠付きで選択）
承認者、日付、理由、残課題、再評価条件、対象PRとrollback: unknown

## 独立Flake棚卸し（該当する候補）

rootを含む独立flake.nixを列挙し、各パスのlock、内容ハッシュ、Nixpkgs入力と例外、関連PR、実行確認を記録。未固定/欠落は完了扱いしない。root-only検査の成功を全体成功としない。

| Flake path | lock status/hash | pin/例外理由 | 検証結果・関連PR |
| --- | --- | --- | --- |
| unknown | unknown | unknown | unknown |

## Evidence record（run・attribute・stageごとに記録）

| Key | Value |
| --- | --- |
| tool_id / tool_pin / license / source_url | unknown |
| workload_sha / workload_lock_sha256_by_path | unknown |
| harness_sha / candidate_delta | unknown |
| runner / os_arch / xcode_version | unknown |
| execution_stage / attempt / job_url / cold_warm | unknown |
| attributes_expected / attributes_evaluated / attributes_built / attributes_skipped | unknown |
| execution_method (source-built/substituted/already-present/not-run/unknown) | unknown |
| outcome (verified/failed/skipped/unsupported/cancelled/unknown) | unknown |
| duration / runner_seconds / cpu_seconds | unknown |
| peak_rss_bytes / disk_peak_bytes | unknown |
| bytes_rx / bytes_tx | unknown |
| cost_currency / cost_amount_or_unknown | unknown |
| rollback_reference / review_decision | unknown |
| Measurement methods/units, unknown reasons, negative test, failure causes | unknown |

OS別にevaluation / realization / runtime smoke / install acceptanceを分ける。比較対象のworkloadと候補harnessを区別し負荷、時刻、cache実態、runner条件を揃える。別workloadは別実験として記録。cold/warmや成功/失敗を混ぜず、10 successful runs未満は探索扱い。p50/p95、全結果数、失敗率、queueとrunner実行時間、required critical pathとrunner時間の合計を区別する。ログ・artifactsの秘密や個人識別子はマスクし、保持期限後も判断根拠をPRに残す。

## TCO / rollback

runner/電力/SSD/保存/API/転送/インフラ/運用工数の見積、前提・通貨・時点、上限、監視、停止・復旧手順: unknown。
