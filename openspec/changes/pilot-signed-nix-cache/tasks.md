# Tasks (unapproved)

## 1. 設計承認
- [ ] 1.1 #332 / #342 の状態、canonical specとの整合、proposal承認
- [ ] 1.2 最新HEADの個別・全体OpenSpec strictと既存CI成功を確認
- [ ] 1.3 Phase A実装配置、承認済みPoC配布、Runner隔離を決定

## 2. Phase A
- [ ] 2.1 承認候補版の回帰テストとCLI/daemon互換性確認
- [ ] 2.2 macOS 15/26で固定drv/outを評価・realize
- [ ] 2.3 両レーンで正鍵/誤鍵/closureを検証し証跡保存
- [ ] 2.4 新版operator record・独立manifest・原観測・原ログ照合と異常系
- [ ] 2.5 両レーンのclosure同一性と独立実行を原証跡で確認
- [ ] 2.6 Phase A受入レビュー

## 3. Phase B
- [ ] 3.1 export/consumer契約を別承認し、短命鍵と成果物を分離
- [ ] 3.2 fresh consumerの独自cache substitutionを確認
- [ ] 3.3 破損・誤鍵・miss・outageを隔離fixtureで検証
- [ ] 3.4 Phase B受入レビュー

## 4. Phase C
- [ ] 4.1 配布許可・private R2/API/権限・予算・GCを承認
- [ ] 4.2 限定cacheの転送・署名・削除・資格情報回転を検証
- [ ] 4.3 Phase C受入レビュー

## 5. Phase D
- [ ] 5.1 cache固有閾値、全体評価との関係、予算、責任者の承認
- [ ] 5.2 matched benchmark・既存CI全検証・rollback・鍵漏えい訓練
- [ ] 5.3 導入PRのレビュー・CI・mergeを確認

実装受入完了後に別archive PRでcanonical specを同期する。各Phaseは先行Phase受入と別承認を条件とする。
