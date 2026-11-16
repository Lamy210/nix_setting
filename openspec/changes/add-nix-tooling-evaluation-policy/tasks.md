# Tasks: add-nix-tooling-evaluation-policy

> 本PRは提案の確定のみ。承認後、別実装PRで本ポリシーの運用を可能にする。新規OSSの有効化、CI変更、製品実装、将来PoCは対象外。

## 0. 提案の準備（本PR）

- [x] 0.1 既存Flake、Home Manager、devShell、テスト、CI、関連OpenSpecを棚卸しし、導入済みと被覆を区別する。
- [x] 0.2 候補の用途、代替、反証、条件付きPoC、セキュリティ・費用・rollbackを整理する。
- [x] 0.3 提案承認・ポリシー実装・archiveと後続PoCの責務、実測前後のGate、比較条件を明記する。

提案の承認条件（実装完了タスクとは別にPRレビュー/CIで記録）:
- `openspec validate add-nix-tooling-evaluation-policy --strict` および `openspec validate --all --strict` を最新HEADで確認する。
- 現行canonical specとの整合、関連PR、server-side required contexts、最新HEADのCIと未解決レビューを確認し、提案承認を得る。未確認の権限設定は管理者確認とし成功を捏造しない。
- 最新HEADの検証はPRの外部証拠で管理し、チェック更新による自己参照を避ける。

## 1. ポリシーの実装（提案承認後の別実装PR）

- [ ] 1.1 開発ガイドに採用ポリシーと後続change作成の導線、担当者・承認者の役割を記述する。
- [ ] 1.2 decision/evidenceテンプレートを配置し、既知の候補で採用・保留の記録例を作成する。実測前の値はunknownとする。
- [ ] 1.3 安全審査不合格、重複OSS、未計測の採用主張、独立lock欠落の異常系をレビューし、承認/保留の判断を確認する。
- [ ] 1.4 既存規範との整合と実装受入を確認する。

実装PR merge後に別`chore/archive-add-nix-tooling-evaluation-policy` PRでcanonical specを同期する。このchangeは新規capabilityを定義するため`--skip-specs`を用いない。全後続PoCの完了を待たない。

## 2. 後続バックログ（参考・本changeの完了対象外）

| 順位 | change候補 | 受入の要点 / 既存PRとの境界 |
| --- | --- | --- |
| P0 | 開発コマンド整合・nix-unit被覆確認 | 現行Flake属性名との一致、negative fixture、pinとCI被覆。製品変更は別change |
| P0 | 計測baseline・uv重複ビルド | workload/lock/harnessを区別し、属性数、cold/warm、時間、失敗率を計測。安全性backportは維持し#299/#324と重複させない |
| P1 | nix-diff | 同一drvと既知の変更の差分を検証 |
| P1 | flake-checker | 全独立Flakeを列挙し欠落lock・意図的pinを記録。#299の修正を代替しない |
| 条件付き | nix-fast-build | 属性数・ボトルネックが妥当なら非required PoC。既存検証維持、全体最適化目標は-25%/+20% |
| P2 | sbomnix | 既存release SBOMとの範囲比較・誤検知審査 |
| P2 | ncps/niks3/Attic系 | local storeとの比較、署名、GC/DR、TCO、信頼境界 |
| P3 | nix2container | OCIの明示的需要がある場合のみ、既存Dockerfileと比較 |
| 別提案 | zizmor/cargo-deny/sccache、nix-du、Plan/Doctor/世代管理強化 | 現状機能との差分と個別OpenSpecを先に確認。採用・実装は未承認 |
| 保留 | Gradient/Hydra、Dynamic Derivations、Managed Nix provider変更 | 具体的需要・互換性・費用と専用security reviewが必要 |

## 3. 後続PoCの成果物テンプレート（参考、チェックリストではない）

- source/license/release/immutable pin、対象system、現在の代替、利用者価値、事前安全審査。
- workload SHA/独立lock、baseline/候補harness SHA、許可差分、Runnerと実行ステージ・実行方法・結果。
- 全結果、negative case、測定方法、CPU/RAM/disk/time/bytes/API・費用、unknown理由。
- 信頼境界、失敗時の停止/復旧/rollback、レビューとdecision log、再評価トリガー。
- 各個別changeの実装・テスト・merge後、別archive PR。未実施項目を完了扱いしない。
