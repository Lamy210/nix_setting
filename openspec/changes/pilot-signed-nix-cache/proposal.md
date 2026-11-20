# Change: 署名付き Nix CI キャッシュの段階的検証

## Why

macOS 15/26 の既存成功ジョブ4件で Home Manager build の35件のderivationと415件の公開cache取得パスが共通だった。再利用可能性はあるが、短縮率・実転送量は未計測。Atuin単体を初回実験の対象とする。

## What Changes

- Phase A: 固定revisionからnative macOS ARM64で評価・realize・一時cacheへの署名、closure照合、新版機械証跡の受入。
- Phase B: 別の使い捨てRunnerでの実取得、破損・誤鍵・miss・outage検証。A受入後に独立した実装承認。
- Phase C: private R2の限定実験。配布条件、資格情報分離、予算、GCを別承認。
- Phase D: 実測に基づくCI採用判定。既存gateと両macOSレーンを維持して通常buildへの復帰を検証。

## Impact

- ADDED requirements: development-workflow。canonical specは変更しない。
- #332 の採用審査と #342 の性能評価契約を参照し、未承認・未マージを承認済みと扱わない。#340 のuv専用PoCとは分離。
- 本PRは設計のみ。実装候補は scripts/ci/nix-cache-pilot/。検証対象checkoutと実装の配置・identityを分離する。
- 公式strict検証と設計承認後に独立実装PRを作る。CI、製品、lock、R2、実機、秘密鍵、保護設定には触れない。

## Evidence

- #332 macOS 26: https://github.com/Lamy210/nix_setting/actions/runs/44014260065/job/133808274001
- #332 macOS 15: https://github.com/Lamy210/nix_setting/actions/runs/44014260065/job/133808274187
- #333 macOS 26: https://github.com/Lamy210/nix_setting/actions/runs/44014350163/job/133808572175
- #333 macOS 15: https://github.com/Lamy210/nix_setting/actions/runs/44014350163/job/133808572638
