# Tasks

## 0. 設計レビュー
- [ ] 0.1 #332 / #340 と canonical development-workflow の適用範囲を確認し、全体 guard の提案を承認する。
- [ ] 0.2 最新HEADで OpenSpec strict と既存CIを確認する。必須レビュー条件を満たす。

## 1. 承認後の独立実装
- [ ] 1.1 run・attempt・workflow revision・matrix/reusable展開・ジョブ責務とサーバー側requiredコンテキスト/App IDのスナップショット記録に対応する。
- [ ] 1.2 server-required gate elapsed、non-required `ci-required`候補のelapsed、Rust sum、全体 sum、execution-only DAG pathを別集計する。
- [ ] 1.3 比較条件、欠損、重複、baselineゼロ、失敗runを扱い、判定不能を区別する。
- [ ] 1.4 既存レポートとの互換性、7つのserver-requiredジョブより候補集約ジョブが後で完了するケース、保護設定不明時の保留、逆方向のguard超過、個別目標と全体目標の混同を回帰検証する。
- [ ] 1.5 既存CI/CodeQL/実機受入の非回帰を確認し、計測手順とrollbackを記録する。
- [ ] 1.6 承認された比較条件・予算内で実測し、探索結果と採用判断を分離する。

実装PRがdevelopへmergeし受入が完了した後、別archive PRでcanonical specを同期する。#332 の archive は独立の責務とする。
