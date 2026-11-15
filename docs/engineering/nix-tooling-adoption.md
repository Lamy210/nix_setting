# Nix tooling導入ガイド

このガイドはNix OSS導入評価ポリシー（提案PR #330）の運用手順であり、独立した仕様や新規ツールの採用承認ではない。規範はOpenSpecのnix-tooling-adoption capabilityと既存development-workflow / release-supply-chain / tool-resolution / platform-executionに従う。archive前はopenspec/changes/add-nix-tooling-evaluation-policy/、archive後はopenspec/specs/nix-tooling-adoption/ の同期済み仕様を参照する。本文と規範が矛盾した場合は勝手に条件を緩めず、レビューで解消する。

## 1. 提案から運用まで

| 手順 | 担当と作業 | 次へ進む条件 |
| --- | --- | --- |
| 棚卸し | 提案者が現行Flake・Home Manager・devShell・テスト・CI・OpenSpec・関連PRの同等機能を確認 | 導入済み/有効化済み/実行確認/被覆を区別し、利用目的と差分メリットを記録 |
| Gate 0: PoC前 | 提案者が個別OpenSpecを作り、レビュー担当者が取得元・ライセンス・immutable pin・権限・隔離・予算/停止条件・対象範囲・既存gate維持を確認 | 安全性必須条件にfail/unknownがなく、承認を記録してから隔離PoCへ |
| Go (PoC) / 比較 | 実装担当者が別changeの承認後、任意の手動/非required経路で比較とnegative testを行う | 実行と結果が追跡可能。失敗・skipも記録。効果や対象platform実測がunknownでもPoC前安全審査と混同しない |
| Gate 1 / Gate 0再審査 | レビュー担当者が実測、比較可能性、費用、OS別受入、供給網、復旧を確認 | 重要項目のunknown、既存gate非回帰違反は正式採用不可 |
| Gate 2 | 承認者が採用/Conditional/No-Go/Deferを決める | 根拠・責任者・再評価条件を記録。PoC成功を本番採用に読み替えない |
| 限定導入・運用 | 実装担当/運用担当者が個別PRで非回帰、停止・復旧を確認 | 最新HEADのrequired checksとレビュー・承認が揃ってからsquash merge |
| archive | 実装PR merge後、別chore/archive-* PR | canonical specを同期する。新規capabilityでは--skip-specsを使用しない |

提案者・実装担当・レビュー担当・承認者・運用担当は記録上の役割。単独開発でも誰が何を判断したかをPRに残し、未実施のレビューを承認済みとしない。GitHubサーバー側のrequired設定が取得できない場合は管理者確認事項として残し、保護の緩和・迂回をしない。

## 2. 最低限の成果物

[decision/evidenceテンプレート](./nix-tooling-evaluation-template.md) を個別changeの記録にコピーし、上流の一次資料・固定revision・計測証拠を紐付ける。値の空欄、未知、未実行、対象外は区別し、unknownの理由と次の確認手順を記す。[記録例と異常系レビュー](./nix-tooling-evaluation-examples.md) は手順理解用であり、実際の採用済み結果ではない。

- 既存機能で十分なら追加しない。nix-unit、nix-direnv、nix-index等を新規導入と誤記しない。
- Flake監査はrootと独立Flakeを動的に棚卸しし、rust/node/python/flutter/devenv等のlock有無・intentional pinを個別に記録。古いという理由だけで更新せず、既存Nodeセキュリティchangeと重複させない。
- nix-fast-buildはchecks属性・ボトルネックを先に把握し、需要が認められた場合だけ非required PoC。既存nix flake checkやrelease/macOS/Linux実環境検証を置き換えない。
- CI全体の最適化完了判定では既存development-workflowの-25% critical path/+20% runner-timeを維持。個別ジョブの改善は事前目標で別評価する。比較条件ごとの成功runが10未満なら探索扱い。時刻差だけから請求額や実ビルド時間を断定しない。

## 3. 安全に実行・停止する

CI/ローカルの任意PoCは既存の信頼境界とrequired CIを維持する。untrusted PRへwrite token・署名鍵・リリースOIDC資格情報を渡さない。外部PRの成果物を無条件でpromotionしない。永続self-hosted Runner・remote builderの特権化、公開ポート追加、Managed Nix provider切替、破壊的GC、本番cache・秘密鍵の配備・外部有料サービス契約はこのガイドだけで実行しない。必要なら独立changeで承認する。

障害時はPoCの専用経路を停止し、候補input/lockをrevertし、既存CIを再検証する。cache漏洩/署名不一致では書込停止、旧鍵の信頼・取得済み成果物の影響調査と隔離、信頼済み経路からの再取得または再ビルド・検証を行う。鍵削除だけで既存storeが安全と判定しない。個別の監視・復旧・容量・費用上限は各採用changeで定義する。
