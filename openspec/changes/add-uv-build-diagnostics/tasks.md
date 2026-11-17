## 0. Approval gate

- [ ] 0.1 当changeのstrict validation、proposal review/approval、#332実装との整合を確認する。
- [ ] 0.2 workload/harness pin、隔離、予算・停止条件、Gate 0と候補差分の許可を記録する。

## 1. Diagnostic implementation

- [ ] 1.1 root Python shell・独立templateのlockとuv候補、入力drvの証拠取得を標準Nixコマンドで実装する。
- [ ] 1.2 実体パス/version対応とunknown/failed判定、secret/pathのallowlistとマスキングを実装する。
- [ ] 1.3 正常系、同version異drv、候補0件/複数、lock欠落、malformed JSON、version不一致、秘密情報fixtureの回帰テストを追加する。
- [ ] 1.4 現行のCI計測手順と整合した診断・証拠・rollback手順を文書化する。検証は既存gateを緩和せず行う。

## 2. Initial comparison and decision

- [ ] 2.1 承認済み隔離環境でbaselineと限定pin候補の予備計測を実施し、独立templateの実行・uv安全性・失敗検出を確認する。
- [ ] 2.2 承認済み予算内で比較可能なcold/warmデータを取得し、成功数・p50/p95・失敗/skip等・費用/unknownを報告する。件数不足は探索としてDeferを記録する。
- [ ] 2.3 #335に証拠とGo(後続changeの提案のみ)/No-Go/Defer、非回帰、rollback、未確認事項を記録する。恒久的なpin/cache/workflow変更は禁止。

## 3. Integration

- [ ] 3.1 最新HEAD strict validationと既存required/全CI、レビューを確認し、実装PRをdevelopへ統合する。
- [ ] 3.2 実装完了後、別archive PRでcanonical spec同期・strict validationを行う。archiveのために未実施タスクを完了扱いしない。将来の最適化採用は別changeに分離する。
