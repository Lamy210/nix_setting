# Design

## 基準と範囲

基準 develop 3f13aa45d7c9d551e4020ab14178d6538bad24de。対象 drv /nix/store/813iflmhmlb5sb7pk1iw6sz3lsds7kcn-atuin-18.23.0.drv。実ログで観測済み、基準の再評価は未実機確認。Nix 2.33以上の対応CLIを仮定し、CLI・daemonの互換性を先に確認する。両macOSレーン (15/Xcode 26.3, 26/Xcode 26.6) のnative ARM64を使う。失敗時にpinを自動更新しない。

## 方式と信頼境界

A: file:// のprivate一時cache、短命鍵、外部送信なし。B: 別の隔離Runnerと限定HTTP配信又は承認済み転送経路（別承認）。C: private R2のS3 API（互換性は実測）。D: PR消費者はread-only、publisherはレビュー済みdevelop専用。未検証PR、任意artifact、ユーザー設定、未審査closureを署名しない。R2資格情報や本番秘密鍵をA/Bに配置しない。既存cache.nixos.orgとrequire-sigsを維持し、trusted=trueやtrusted-usersの追加を回避する。

## Phase A

Git状態とdrv/out一致、署名済みclosureのメタデータ、正鍵の受理、誤鍵のroot単独・closure両方の拒否を検証する。誤鍵の拒否は破損や通信失敗と区別する。現行PoCは破損NARを注入しないためBで実装する。一時cacheは終了時に削除し、Bの転送元は別export契約で実装する。

### 機械証跡・受入契約（未承認）

- 旧v6向けoperator record (schema 1、旧ZIP固定SHA-256) は履歴検証専用。新版 local-command-observation schema 1 と混同しない。
- 独立に承認した実装identity: 固定実装commitと全実行・検証ファイルの相対パス/SHA-256一覧をreview記録に固定する。launcher、shell、closure検査、ローカル証跡生成・検証、比較器、新operator record検査器を含む。検証対象checkoutと実装commitは別identity。自己申告hashのみでは受入不可。
- 新版operator record schema 2: 実測OS/arch/Xcode/Nix CLI・daemon、runner/run/job/attempt（他環境では隔離実行ID）、対象checkout・lock identity、実装identity、drv/out、開始・終了時Git検査、独立実行・資格情報非使用の確認、原ログhash、承認参照。欠測は未確認。旧recordを自動変換しない。
- 原観測: closure-before.json、closure-after.json、observation.jsonのdigestとroot/count/終了コードを照合。独立manifestに照合し、原JSONから再計算する。両観測の全store path/NAR hash/size/referencesを比較。同一rootでも内容が異なればBを保留する。
- reviewerは原ログ・実行identity・出力物との対応を確認する。別directoryへの複製は独立実行の証拠ではない。hashは整合性であり真正性・承認の自動証明ではない。模擬記録を実機受入に使わない。すべての検査成功、人間の受入記録までB未承認。
- 秘密鍵・token・非公開情報を証跡に混入させず、承認された非公開保管先と保持期間を決める。編集ログは原ログと別identityで管理する。実行全体失敗時は成功証跡を公開しない。

## Phase B

fresh consumerでrootが未realize、同一drv/out、承認された公開鍵による独自cacheからのsubstitutionログと取得後hash/size/referencesを保存。既存storeや公式cacheからの取得を独自cache成功に数えない。exportは承認closureのみ、秘密鍵は含めない。故障試験は隔離fixtureを使う。誤鍵・破損受理・証跡欠落は採用停止。cache miss/outageは通常buildへ復帰し既存検査を維持。署名不正は警報・停止とし、通常missと混同しない。

## 計測と採用

固定workload/lock、workflow revision、OS/Xcode/Nix、runnerクラス、attempt、cold/warmで matched sample。各条件・各レーン10成功以上、失敗/取消も別記録。全CI経過、required gate elapsed、macOS build runtime、runner sum、転送実測、R2操作数/費用を別集計する。既存fan-out -25% / Rust workers+aggregator +20%制約を変更しない。#342の全体guardと個別目標の区別を採用判断の前提としてレビューする。cache固有閾値・予算は未承認。未決・測定不足・安全性失敗なら本番採用しない。

## 運用・障害

鍵漏えい: publisher停止、鍵信頼削除、資格情報失効、影響store path隔離・再構築・新鍵配布。鍵削除だけで既取得物は消えない。容量・費用上限未決ならR2実験不可。narinfo/NAR整合性を維持するGC、復旧、監視を別設計。実験終了後はRunner廃棄。通常CI変更前にrollback手順と責任者を確認する。
