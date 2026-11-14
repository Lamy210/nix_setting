# Design: Nix OSS導入評価・選定・安全な段階適用

## Context / Evidence (develop @ 2026-11-13)

実際に読んだ既存ファイル:
- `flake.nix`: `flake-parts`、`home-manager`、`nix-darwin`、`treefmt-nix` がinputs。対応systemsは `aarch64-darwin` / `x86_64-linux` / `aarch64-linux`。
- `modules/flake-parts/default.nix`: nixfmt/shfmt/taploによるtreefmt契約。
- `modules/flake-parts/devshells.nix`: `nixd`, `statix`, `deadnix`, `devenv`, `nix-tree`, `nix-melt`, `nix-unit` 等が導入済み。
- `tests/nix-unit.nix`、`justfile`、`.github/workflows/check.yml` のbootstrap-test: unit testの既存入口と `nix run nixpkgs#nix-unit -- tests/nix-unit.nix` のCI実行を確認済み。現行テストの被覆、registry経由のpinとserver-side required指定は別途監査する。
- `modules/programs.nix`: nix-direnv有効。`modules/packages/dev.nix` と `modules/shell.nix`: nh、nix-index、Zshのcommand-not-found候補表示が既存。追加OSSではなく設定・実行・更新経路の改善候補。
- rootと独立テンプレートを監査対象にする。developではdevenvのlock欠落を確認。既存#299の修正範囲と重複させない。
- PR #330の旧HEAD CI run 43432447154ではdevshell-smoke 27分19秒、Python template 24分31秒、flake-check 56秒。uvバックポートのビルド開始後に長いログ間隔がある。単一runの探索的証拠であり、uv単体の正確なビルド時間や恒常的な性能差ではない。
- `.github/workflows/check.yml`: `openspec-check`、`flake-check`、`lint`、`rust-check`、`macos-check`、`release-artifact-check` 等を持つ。現行required contextsの最終的な正本はGitHubサーバー設定で再確認。
- `openspec/specs/release-supply-chain/spec.md`: release artifactsにcosign署名、SLSA provenance、SPDX SBOMが既存要件として存在する。
- `openspec/specs/development-workflow/spec.md`: critical path -25%以上 / runner-time +20%以内という既存のCI最適化目標、必要context保持が規定されている。
- `docs/testing/ci-workflow-timing.md`: job timingの計測器・matched runの要件がある。

この設計の「採用」とは、ツールの導入をこのPRで確定することではない。**優先PoCへ進める判断と、その後の採用判断ゲートを確定すること**を意味する。

## Goals / Non-Goals

### Goals
1. OSS候補を現行機能と重複排除し、各候補の採用状態と判断根拠を維持する。
2. 再現可能なbenchmarkとsecurity reviewによってツールごとの効果・費用・障害モードを評価する。
3. required CI / macOS release artifacts / Windows-WSL境界 / Rust coreの責務を変えずに改善する。
4. 追加ビルド・外部サービス課金・秘密鍵保護・ストレージ増加のコストを測定できるようにする。

### Non-Goals
- CI全体の置き換え、release artifactsの生成経路変更、root Nix daemon更新。
- `nix-fast-build` と `nix flake check` の互換性を推測で宣言すること。
- ソフトウェア脆弱性スキャナーの推定結果だけで既存requiredを失敗させること。
- 未検証のOSSをSchneeForgeの一般ユーザーへ既定インストールすること。
- macOS Intelのrelease support、native Windows Nixの追加、無人でのhost切替。

## D1. OSS採用ステータスと決定マトリクス

| ステータス | OSS | 判断と条件 |
| --- | --- | --- |
| **既存維持（追加導入不要）** | flake-parts, treefmt-nix, nixfmt, nixd, statix, deadnix, devenv, nix-tree, nix-melt, nix-unit, nh, nix-direnv, nix-index, nix-output-monitor, omnix | 導入済み・有効化済み・実行確認・被覆を区別する。nix-unitは異常系とpinの監査を優先 |
| **条件付きPoC（非blocking）** | nix-fast-build | checks属性数・実測ボトルネック・改善見込みを事前確認。required flake-check / macOS / release gateを置換しない |
| **P1優先PoC（ローカル診断）** | nix-diff | derivation変化・キャッシュミスを診断。read-only、任意ツール |
| **P1優先PoC（non-required）** | flake-checker | lockの陳腐化警告を記録。更新を自動適用せず、false positiveとpin理由を審査 |
| **P2評価** | sbomnix | Nix closureのSBOM・依存関係を検査。既存release用syft/SPDX/cosignを置換しない |
| **P2評価** | niks3 | 複数Runner向けS3キャッシュ。read-path・write-path・署名鍵・GCを別設計 |
| **P2比較対象** | ncps / Attic / Celler | キャッシュミス・重複排除・保守状況・復旧手順の比較後に選定 |
| **P3限定PoC** | nix2container | CLI等の検証用OCIのみ。Tauri/DMG配布やnative installを変更しない |
| **採用保留** | Gradient, Hydra | 専用CIの常時運用・DB・Worker増設費が高い。ジョブ増加と便益が明確になるまで見送り |
| **採用保留** | Lix, Determinate Nix | Managed Nix providerはADR-0001と既存changeの責務。別の意思決定が必要 |
| **採用保留** | cargo-dyndrv, Dynamic Derivations | 実験的要素・移植性・長期保守を別PoCで確認するまで主要CIに置かない |
| **別スコープ** | system-manager, disko, nixos-anywhere, microvm.nix, sops-nix, llm-agents.nix | サーバー運用/秘密管理/AI開発者環境用。ユーザー製品や必須CIへの一括導入は禁止 |

候補のライセンス、最終更新、セキュリティ勧告、maintainer数、対応Nix/Lix、サポートsystem、固定可能なrev/NAR hash、推定転送量・ランタイム依存、依存先の信頼境界は**実装PR直前に確認**する。未確認値は `unknown` と記す。

## D2. アーキテクチャ / 運用境界

```text
develop / PR
  ├── 既存required CI（基準線・変更禁止）
  │    ├── openspec-check / flake-check / lint / rust-check
  │    ├── bootstrap-test / managed-nix-e2e / ci-required candidate
  │    └── macOS stable / release-artifact-check / 配布物検証
  └── opt-in tooling PoC（非required、失敗を既存required成功と読み替えない）
       ├── build diagnostic: nix-fast-build / nix-diff
       ├── policy diagnostic: flake-checker / sbomnix
       └── cache experiment: local → ncps or niks3 (isolated credentials)
```

- PoCは**別workflowまたは手動実行**で開始し、CIのrequired job名を改名/削除/skipしない。
- third-party code実行を伴うPRではcache write credentials、OIDC書込み権限、シークレットを露出しない。
- `nix-fast-build` の主なビルド対象はFlake checks属性。 `nix flake check` の評価検査、別途行うHome Manager activation・nix-darwin system build、Linux musl/Windows/DMG成果物の実行検査と**同値ではない**。
- OSごとに `evaluation` / `realization` / `runtime smoke` / `install acceptance` を別列で記録。Linux上のDarwin derivation評価はmacOS実機実行を証明しない。
- optionalツールを製品CLIから実行する場合は、既存 `Toolchain` と解決済み絶対パス経由のみ。ユーザー側 `PATH` に存在するだけで強制起動しない。

## D3. PoC計測と成功条件

| 観測 | 方法・記録 |
| --- | --- |
| Baseline | workloadのSHA・独立lock/対象attr / baselineと候補のharness SHA・ツールpin・許可差分 / runner class・system・Xcode・cache状態・run ID |
| 完了時間 | `docs/testing/ci-workflow-timing.md` のreportを再利用。created→started（スケジューリング等を含む）とrunner実行時間を分離。課金時間とは区別 |
| 性能 | 各比較条件の成功10回未満は探索扱い。5回は予備計測の目安。matched cold/warmを分け、p50/p95・ばらつき・critical path・runner合計時間を記録 |
| マシン負荷 | CPU秒、peak RSS、ストレージ使用量、I/O、ビルド数、再ビルド差分、ネットワーク転送量 |
| Nix成果物 | 評価できたattr件数、実際にbuildしたdrv件数、cache hit/miss、署名検証結果 |
| 品質・再現性 | `nix flake check`、Nix unit、Linux/macOS requiredとrelease gateの結論、同一lockでの再実行 |
| コスト | 追加Runner秒、ホスト電力、S3容量・APIリクエスト・egress・DB/バックアップ運用の実費（未知ならunknown） |

既存 `development-workflow` の最適化目標 **critical path -25%以上かつrunner time +20%以内**を参照。これを満たせない案をCI最適化の「完了」と呼ばない。品質・セキュリティ・互換性に退行がある場合は速度改善より優先して不採用とする。サンプル数不足・異なるrunner・失敗runを成功例に混ぜた比較は無効。

## D4. セキュリティ・信頼境界

1. PoCツールはバージョン/commit固定とライセンス審査を必要とする。外部のmutable branchをproductionの既定経路で直接実行しない。
2. バイナリキャッシュは署名検証済み `trusted-public-keys` を用い、`substituters`・信頼ユーザー・キャッシュ上書き権限をレビューする。開発者PRの成果物を無条件で信頼キャッシュへpublishしない。
3. AWS/S3 API credential、cache signing key、OIDC token、独自シークレットはNix store・ログ・公開artifactへ保存しない。readとwrite権限を分離する。
4. SBOMと脆弱性照合は、候補一致の誤検知・依存の静的/動的リンク差・閉包外パッケージを区別する。existing release attestationを二重生成・上書きしない。
5. 自動GCやpruneは検証用キャッシュでのみ試す。必要なstore pathの参照、復旧・再取得・容量上限を確認するまで本番適用しない。
6. WSLのLinux backend以外にWindows host上のNix executionを追加せず、macOSのLinux Builder鍵・daemon権限の既存監査を維持する。

## D5. 障害モード・監視・切り戻し

| 障害モード | 検知 | 予防/復旧 |
| --- | --- | --- |
| fast-buildが一部checksのみ検証 | attr一覧と既存flake-check結果の差分 | 必須gateを保持、optional PoCを停止 |
| Nix lock更新による大量再ビルド | nix-diff, store diff, transfer / elapsed | pinを戻し、最終成功commit/lockへ戻す |
| Cache poisoning / 署名不一致 | signature verification failure | 該当substituterを無効化、信頼鍵を見直す。requiredをskipしない |
| キャッシュ停止・GC誤削除 | HTTP/NAR取得失敗、容量・エラー率 | 公開の信頼済み上流へフォールバック（既定信頼ポリシー維持） |
| API課金/転送量超過 | egress, PUT/GET, disk, S3請求 | upload停止、プロキシ停止、上限と通知 |
| false positive CVE | package/purl/cpe一致と実際のclosure照合 | non-requiredのままmanual triage |
| OOM/CI timeout | peak RSS、exit status、実行時間 | 同時実行数を戻し、PoC workflowだけ無効化 |
| Mac/WSLの互換性破壊 | OS別build/realization/runtime結果 | 元のlock・workflowへ復旧、releaseしない |

監視の最低項目: job結果/step duration、実行ホスト/アーキ、CPU、peak RSS、disk/free、cache hit/miss、転送量、署名検証エラー、アップロード失敗。将来S3サーバーを設置するときはメトリクス・アクセスログ・バックアップ/DR・容量制限・可用性目標を**別のインフラ仕様**で決める。

## D6. 推進順序とPR分割

1. **P0 inventory + benchmark**: 既存ツールと必須gate・Nix test coverageを確定。計測baselineを採取。
2. **P1 diagnostics**: nix-diff / flake-checkerを任意ツールとして追加比較。nix-fast-buildは属性棚卸し・ボトルネックで必要性が示された場合のみ非blocking/manual workflowで計測。各ツール独立のimplementation PR。
3. **P2 security/cache**: sbomnixでNix closureを調査。キャッシュは比較Spikeを先行し、必要が証明されてからniks3等の最小構成を別のOpenSpecで設計。
4. **P3 OCI/研究**: nix2containerのみ隔離環境で評価。ほかは研究として監視。
5. 本ポリシーの提案承認後、開発ガイドへの導線・decision/evidenceテンプレート・記録例/異常系レビューを実装する。実装PRをdevelopへmerge後、別archive PRでcanonical specを同期する。後続PoCの完了は待たない。

## Open Questions (実装前に解消)

- `nix-unit` はbootstrap-testで実行済み。server-side required指定、negative fixtureと実行pinの被覆をどう改善するか。
- `nix-fast-build` はどのattributesで完了時間短縮が見込めるか。単一の小さなchecksでは追加負荷の方が大きくないか。
- flake-checkerのNixpkgs対応branch、例外扱い、false positive抑制ルール。
- Nix SBOMが実際のRust/Tauri releaseの依存範囲をどこまで表現できるか（既存syftとの差異）。
- 共有バイナリキャッシュを必要とするRunner数、総転送量と保管期限。S3互換料金は利用リージョンに依存するため未算出。
- 非required PoCが失敗した場合の記録先と翌月の再評価責任者。

## D7. 導入判定モデル（セキュリティ・品質をスコアより優先）

候補単位で **Gate 0（必須条件）→ Gate 1（比較）→ Gate 2（Go/No-Go）** の順に評価する。

### Gate 0: PoC開始前と正式採用前の二段階審査

PoC開始前は取得元・ライセンス・immutable pin・隔離・最小権限・予算/停止条件・対象範囲・既存gate維持を確認する。実測前の効果・対象systemの実測はunknownでよいが、安全性の必須条件が不明なら実行しない。正式採用前には以下の全条件を再審査し、必要な実測がなければDeferとする。

- OSSライセンス/配布条件、実行コードのimmutable pin（rev/hash/lock）、取得元、保守状態、重要な公開脆弱性の状況を記録できる。利用範囲が不明な項目は `unknown` とし**本番採用禁止**。
- 現行required CIをskip/削除/緩和せず、既存の `managed-nix-e2e`、macOS 15/26、release CLI/DMG、Windows non-required契約の扱いを維持する。
- ソースに含まれる任意コードがroot/CI write token/cache signing keyに接触しない。外部PRの成果物を信頼キャッシュやrelease成果物に昇格させない。
- 対象アーキテクチャ/実行段階ごとのサポート範囲が明示され、少なくとも採用対象のsupported platformで実測がある。
- 破壊的なGC、Nix daemon/Managed Nix provider切替、SSH trusted-user付与、公開ポート追加、秘密鍵配備は本仕様の範囲外。別OpenSpecと明示的な承認が必要。
- 依存ツールが導入されなくても既存製品・ブートストラップ・リリース手順が利用でき、独立したrevert/disable操作が記載される。

### Gate 1: 候補の比較指標（100点、重みは設計上の仮定）

| 評価軸 | 重み | 計測・判定の根拠 |
| --- | ---: | --- |
| Security / supply chain | 25 | 署名・資格情報・依存・最小権限・脅威モデル |
| Reliability / rollback | 20 | 失敗の再現、復旧手順、既存機能の非回帰 |
| Maintenance / ecosystem | 15 | 最新release、保守履歴、依存の増分、migration |
| Measured benefit | 15 | 性能・検出率・ビルド属性等、目的別の実測 |
| Total cost | 15 | Runner/電力/ストレージ/API/転送/人手の合計 |
| Platform / integration | 10 | Darwin/Linux/WSLの対象別導入コスト |

各軸を0〜5段階で根拠つき評価し、`weighted points = sum(weight * rating / 5)` とする。尺度は0=不適合、1=重大課題、2=一部充足、3=要求を満たす、4=追加便益、5=明確な優位性を根拠で確認。未確認は `unknown` として0点評価とは区別し、加点せず、Gate 0に必要な未確認項目があればscoreに関係なくNo-Go。75/100以上は**本番採用審査へ進める暫定目安**に過ぎず、品質・安全性・既存CI目標の違反を免除しない。評価尺度・根拠の変更はdecision logに記録する。

### Gate 2: 明示的な判定

- **Go (PoC)**: Gate 0のPoC条件を満たし、非requiredで隔離した再現可能なテストを承認。
- **Go (本番利用)**: Gate 0をすべて満たし、Go/No-Go目標、required CI・レビュー・rollbackを確認、候補ごとの実装PRで再承認。
- **Conditional**: 成果はあるが費用/互換/maintenanceが未確認。試験環境のみに保持。
- **No-Go / Defer**: セキュリティ条件を満たさない、優位性がない、検証不十分、既存重複による複雑化が上回る。

## D8. OSS別PoCプロトコルとNegative test

| OSS | PoCで比較する対象 | 成功条件と必須negative case |
| --- | --- | --- |
| nix-fast-build | 現行 `nix flake check`・必要なexplicit `nix build` の実行集合 | **属性単位の検証マトリクス**を作り、evalのみ/build済み/cache hit/skip/failedを区別。1件失敗・unsupported・skippedが検証済みに昇格しない。`--skip-cached`による省略は**対象のビルド実行を証明しない**。現行required全件を存続 |
| nix-diff | commit/lockが異なる2つの明示的drv | 同一drvなら差分なし、既知の入力変更なら説明可能な差分を出す。依存closure変化を0として見逃したら不採用/調査 |
| flake-checker | root `flake.lock` と全独立Flake（rust/node/python/flutter/devenvを含む）のpin有無 | 意図的な古い入力、サポート外branch、例外pinをfixture化。ルートのみ検査してテンプレートの安全性を宣言しない。デフォルトの30日警告を一律自動更新と混同しない |
| sbomnix | Nix closureのSBOM vs 既存release CLI/DMGのsyft/SPDX | 対象範囲、ライブラリ種別とpackage identityを差分比較。未一致を未知として残し、false positiveで公開を誤って止めない |
| ncps/niks3 | Local Nix storeと同一入力・同一hostで3条件 | Cold/Warm時のbytes、API、store容量、キャッシュ署名検証、故障時の再ビルド、GC・再取得、バックアップを再現。料金を推定で断定しない |
| nix2container | 既存Dockerfileで作る隔離テスト用CLIイメージ | digest、サイズ、layer reuse、実行/portability、SBOM scan、署名、baseがない場合の互換性を比較。配布経路は変更しない |

### nix-fast-buildの仕様上の注意

上流READMEでは、標準で `.#checks` を複数systemで評価する一方、buildは現行system中心。`--systems`で他system指定が可能でも、それが実ホストで実行可能とは限らない。`--remote` は遠隔でflakeを評価/ビルドし、上流の説明ではremote側のtrusted userが必要。**最初のPoCでは `--remote`、キャッシュへのupload、`--push-build-closure`、untrustedな`--select`変更を禁止**し、ローカル読取/ビルドのみとする。将来の遠隔ビルドは別security reviewで許可する。

上流 `nix flake check` はNix expressions/outputsのevaluationも実施し、`checks`のderivationをビルドする。build特化ツールはその全部を自動的に保証しない。参照:
- https://github.com/Mic92/nix-fast-build#building-different-flake-attributes
- https://nix.dev/manual/nix/2.35/command-ref/new-cli/nix3-flake-check.html
- https://github.com/DeterminateSystems/flake-checker

## D9. CIの証拠モデル・判定表

PoC runは既存のrequired contextとは**別**に状態を持つ。`success`という1値からOS互換性や成果物品質を推論しない。

| 状態 | 意味 | CI/リリースの扱い |
| --- | --- | --- |
| `verified` | 対象commit・OS・ツールpin・ステージの実測が成功 | そのステージのみの証拠 |
| `failed` | 期待したコマンド/検証が失敗 | 原因分析・未完了 |
| `skipped` | feature gate等により実行しなかった | 検証成功ではない |
| `unsupported` | 対象system/hostで非対応 | 対象外、互換性成功とはしない |
| `unknown` | 時間/環境/結果が不明 | 本番採用の根拠にならない |
| `cancelled` | 実行中止 | 完了と扱わない |

実行方法（`source-built` / `substituted` / `already-present` / `not-run` / `unknown`）は検証結果と別フィールドにする。キャッシュhitだけではソースビルド・本体テストの実行を証明しない。

各PoCのevidence recordは `tool_id`, `tool_pin`, `license`, `source_url`, `workload_sha`, `workload_lock_sha256_by_path`, `harness_sha`, `candidate_delta`, `execution_method`, `runner`, `os_arch`, `xcode_version(該当時)`, `execution_stage`, `attempt`, `job_url`, `cold_warm`, `attributes_expected`, `attributes_evaluated`, `attributes_built`, `attributes_skipped`, `outcome`, `duration`, `runner_seconds`, `cpu_seconds`, `peak_rss_bytes`, `disk_peak_bytes`, `bytes_rx`, `bytes_tx`, `cost_currency`, `cost_amount_or_unknown`, `rollback_reference`, `review_decision` を記録する。各数値の単位、測定方法、unknown理由を記す。個人識別子・秘密情報・絶対home pathは削除/マスクする。証拠ファイルはPRアーティファクトまたはCIへのリンクで参照し、保持期間が終わっても主要集計/判定根拠はPR本文/decision recordに残す。

**同一条件比較の注意:** キャッシュcoldとは実際の関連パスがローカル/候補cacheに無いと確認できた状態、warmとは必要なパスがキャッシュに存在すると確認した状態を指す。クリア方法は隔離storeまたはdisposable runnerを用い、共有Nix storeの破壊的GCはしない。baselineと候補を交互に試し、worker負荷・ネットワーク状態・実行時刻の差を記録。各条件の成功runが10未満なら探索結果扱い。10以上でも効果のばらつき・失敗率を確認し、差が小さい場合はサンプル増加/要因分析を優先する。workload変更を伴う比較は別実験として差分を明示し、同一workload比較と混同しない。

## D10. 詳細なサプライチェーン脅威モデル

| 攻撃対象 | 信頼境界 | 必須対策 |
| --- | --- | --- |
| 外部PRからの任意コード | untrusted PR → Runner | `pull_request`のread-only最小権限。`pull_request_target`/`workflow_run`/comment-triggerでuntrusted checkout・artifact実行と高権限を混在させない |
| Self-hosted Runner | untrusted job → ホスト/内網/永続store | 原則trusted branchのみ。外部PRを実行するならrunnerをジョブごとに破棄し、内部ネットワーク/ホーム/鍵・キャッシュwriteを隔離した別設計を要する |
| Remote builder | PR build → SSH root/trusted users | P1では無効。Nix remoteにtrusted-userを付与する権限の大きさを別審査し、host key固定、最小権限、ネットワーク制限を確認 |
| Binary cache | cache writer → 全substituter利用者 | trusted keysとsubstituter allowlist、PR成果物のpromotion分離、署名鍵のoffline/secret manager管理、失効・ローテーション手順 |
| Nix store/CI artifacts | 任意ビルド/ログ → public artifacts | シークレットをNix式文字列/derivation env/コマンドライン引数/traceへ書かない。ログ・SBOMも情報漏洩検査 |
| 依存・OCI・SBOM | 外部source → release artifact | ソースpin、digest、配布ライセンス、成果物出所、既存attestationの自己検証を維持 |

GitHub公開セキュリティガイド: https://docs.github.com/en/actions/reference/security/securely-using-pull_request_target
Nix 2.35設定・substituterの信頼要件: https://releases.nixos.org/nix/nix-2.35.0/manual/command-ref/conf-file.html

## D11. 費用対効果の判断と限度

**初期PoCの前提:** 既存RunnerのCPU/ストレージを優先し、新しい常時稼働VPS・キャッシュDB・外部S3は購入しない。ネットワーク越しの比較はローカル観測・料金見積が完了してから別承認。

月次総費用の比較式:

`TCO_month = runner_hours * runner_effective_hourly_cost + storage_GiB_month * storage_price + requests * request_price + egress_GiB * egress_price + infra_fixed_cost + estimated_maintenance_hours * labor_rate`

外部サービス固有の無料枠・課金単価・超過料金・更新時費用はリージョンと契約を明記して一次資料で確認し、未確認なら `unknown`。無料の自前Runnerでも電力/SSD摩耗/障害対応は0と断定しない。導入前に想定月間jobs、平均artifact size、hit率、再実行率、データ保管期限、復旧回数をシナリオ（低・標準・高）として試算する。

便益は `runner hours saved`、`developer wait time saved`、`prevented regressions` を別々に扱い、単純にトークンや売上へ換算しない。常時稼働費・秘密管理負担が増す案は定量便益が示せなければ見送る。

## D12. ADR / 運用責任 / 巻き戻しの実施手順

採用・不採用を判断するとき、各候補のdecision logに以下を保存する:

```text
Tool / Version / Source / License / Last reviewed:
Existing alternative / Tested systems and stages:
Workload SHA + lock hashes by path / baseline and PoC harness SHA + candidate delta:
Evidence links / Attributes and outcomes / Cost estimate:
Security Gate 0 (pass/fail/unknown):
Weighted scoring + rationale:
Go / Conditional / No-Go / Defer:
Owner / Reviewer / Review date / Re-evaluation trigger:
Disable procedure / Revert commit / Restore accepted state:
Unresolved questions:
```

- optional PoCの無効化: **専用の実行スイッチ/手動workflowを停止** → ツールinput・lockの変更をrevert → baselineの `nix flake check`、unit、Rust、macOS、release gateを再確認。failed requiredのskip・context変更で見かけ上greenにしない。
- cache rolloutの無効化: write権限停止 → 新substituter削除/信頼鍵revocation検討 → 上流の安全なcache/ソースビルドへ復帰 → キャッシュ破損時は影響artifactを隔離。署名鍵漏洩時は別セキュリティインシデントとして処理する。旧鍵を信頼する消費者の設定と取得済み成果物を追跡し、影響期間の成果物を隔離・再取得/再ビルド・検証する。鍵削除だけで既存storeが安全になったとは扱わない。
- UI/CLIへの昇格は Core/CLI/Desktopの責務分離と独立Toolchain契約を再設計する**別**OpenSpec changeとする。単なるツール発見やOSへの自動インストールをこのchangeから推奨しない。
- baselineと採用条件が満たされない候補は `No-Go` の理由を記録し、恒久的なcron/runner資格情報を残さない。

## D13. 追加レビューで解消するべき境界

1. セキュリティ更新用に意図的に固定しているroot・Node・devenvのinputsを「30日を超えた」だけで弱く扱わない。pin理由と実体runtime/closureを検証する。
2. GitHub branch protectionのrequired contextsをサーバ側で確定し、CI YAMLの名前から推測しない。
3. `devenv` や `nix-unit` はすでにdevShellで配布され、nix-unitはbootstrap-testで実行済みだが、利用可否・実行成功・server-side required指定は異なる。導入済み数とテスト被覆率を混同しない。
4. Windows/WSL2はLinuxバックエンドで実行することと、Windows native CLIの配布検証を別扱いにする。
5. 実装PRを1 tool = 1 concernで分け、この**導入評価ポリシーchange自体**は方針承認後にD6のポリシー実装と受入を完了し、実装PRがdevelopへmergeされた段階で別archive PRへ進める。全PoC・新OSSの本番導入完了まで本changeのarchiveを不必要に遅らせない。各PoCの動作変更には別OpenSpec change/PRを作り、その実装・archiveはそのchange内で管理する。

## D14. 機能・導入メリット・利用者・導入面のトレーサビリティ

ツールの存在を理由に導入しない。**誰のどの行動がどう改善するか（Job-to-be-done）**を候補ごとに先に宣言する。下表のメリットは製品実装済みの事実ではなく、**PoCで検証する期待効果**である。数値・互換性・現状にないGUI表示は実装前には保証しない。

| ツール / ステータス | 誰が何をできるか（利用シナリオ） | 得られる可能性のあるメリット | 対象面 / 非対象面 | 比較基準・反証 |
| --- | --- | --- | --- | --- |
| 既存 **nix-unit** | 設定を編集した開発者が、Nix関数の境界値・誤入力・評価失敗を個別テストする | 実機適用前の回帰検出、修正局所化 | 開発時/CI。**現行GUI機能ではない** | `tests/nix-unit.nix` の既存テストとrequired CI実行の差分。未実行なら既存ツールの活用を優先 |
| 既存 **flake-parts/treefmt-nix/nixd** | 開発者がFlake構成を分割し補完・フォーマット・静的検査を使う | 設定変更レビューと保守性の向上 | devShell/開発者。追加の常時サーバー不要 | 既存機能を重複導入すると変更面積・維持費が増えるため不採用 |
| **nix-fast-build**（優先PoC） | CI担当が多数の独立derivationを評価し、準備できたものから並行ビルド・結果を取得する | 大きいチェック集合で待ち時間短縮、失敗属性の把握、機械可読結果 | 開発者/CI。製品のapply/rollbackやrelease gateを置換しない | 実行属性の網羅、同条件critical path・runner-time・メモリ。属性が少ない、キャッシュが温かい場合は不利な可能性 |
| **nix-diff**（優先PoC） | 設定変更でキャッシュミスが起きたとき、前後のderivation入力差分を説明する | 無駄な再ビルド・レビュー負担の原因特定 | ローカル/CI診断。公開ユーザーへの自動実行は別設計 | 同一drv→差分なし、入力を意図的に変える→検出。メンテナンス状況と出力の理解しやすさも評価 |
| **flake-checker**（優先PoC） | 依存管理担当がroot/独立テンプレートのNixpkgs pinの古さ・サポート状況を確認する | セキュリティパッチの取りこぼし候補の早期発見、更新計画の可視化 | 開発者/非blocking policy check。自動lock更新は対象外 | 健全/古い/意図的固定のfixtureで誤検知率を測定。root-only検査を全テンプレート監査と誤記しない |
| **sbomnix**（P2） | セキュリティ担当がNix closureのSBOM・依存グラフ・古いパッケージを調べる | 依存追跡と脆弱性調査の説明性、影響範囲の可視化 | 開発・監査の補助。既存release署名/SPDXの代替ではない | syft/SPDXとNix依存閉包のカバレッジ差・false positiveを比較 |
| **ncps**（P2比較） | 同一ネットワークの複数開発機/Runnerが上流cacheから取るNARをローカルに再利用する | 重複ダウンロード量/待ち時間低下の可能性 | キャッシュproxy。単一Runnerのwarm storeには余剰になり得る | 上流download byte/host、ヒット率、障害時fallback、管理DB/ストレージ負担。初期開発によるデータ整合リスクに注意 |
| **niks3**（P2比較） | 複数Runnerが構築済み成果物を署名付きS3系cacheへuploadし共有する | Runner間の再ビルド削減、read pathのスケールアウト | self-hosted cache、S3とPostgreSQL/GCの運用が必要 | 実ビルド削減時間 vs S3 API/保存/egress/DB/鍵管理。ビルダー1台で利得なしなら不採用 |
| **nix2container**（P3） | CI担当がNix closureからOCIイメージを作り、不要なtarballをNix storeに保存せずにlayerを再利用する | OCI配布が必要な場合のビルド/転送効率と再現性 | **Linux CLIのテスト用OCI**。macOS DMG/Tauri製品配布は対象外 | 既存Dockerfileとimage digest、サイズ、cold/warm push、runtime smoke、SBOM/脆弱性評価 |
| **Gradient/Hydra等**（保留） | 多数のホストにビルドを分散しジョブを集約管理 | 大規模CIでのスループット、可視化、運用統制 | 常時CIサーバー/DB/Worker。現行Runnerの単純化より優先しない | 同時ジョブ数/待ち時間が単純な自前Runner+GitHub Actionsで満たせない時だけ再評価 |

### 具体的な利用シナリオ（検証しやすい順）

1. **変更理由を説明**：Flake lock更新でHome Manager activationのdrvが変化したら、`nix-diff` が何の入力に起因するか表示。開発者は不要な依存更新と本質的な変更を切り分ける。
2. **既存のテストを確実に実行**：`tests/nix-unit.nix` に「想定外のmachine input」などnegative fixtureを置き、CI実行経路に組み込まれているか確認。`justfile` に存在するだけでrequired実行済みとみなさない。
3. **Flake入力のセキュリティ更新を計画**：root、`templates/node`、`templates/devenv` を別々に監査。意図的なセキュリティbackport pinは除外理由を明記し、実際のruntimeを検査。
4. **CIの待ち時間を削減**：一つのchecks集合を`nix-fast-build` に試験実行。評価のみ、ビルド、substitute、skipを区別し、前後のwall-clockとRunner総時間を比較。
5. **複数Runnerを将来増設**：cold buildの成果物を2台目が取得する条件で`niks3`などを検証。**1台のみのローカルstoreと比較して改善がなければキャッシュサーバーを新設しない**。
6. **セキュリティ調査**：実リリースbinary/DMGの既存SBOMとNix closureの`sbomnix`出力を比較し、どちらに何が載るか区別して説明できるようにする。

## D15. 機能の利用面と製品への導入禁止境界

| 面 | 初期導入方針 | 例と制約 |
| --- | --- | --- |
| developer-only | `nix develop` / `justfile` / 選択的Nix package等、**opt-in** | nix-diff等は説明性に価値があるが、SchneeForgeの全ユーザーへ既定インストールしない |
| CI-only | 手動dispatchまたはnon-required isolated workflow | nix-fast-build/flake-checker/sbomnix。required contextsを保存し、任意PoCのskipを受入証拠へ昇格させない |
| infrastructure | バックアップ/復旧/権限分離を備えた別OpenSpec | niks3/ncps等。ネットワーク公開、書込鍵、S3/DB新規契約を本仕様だけで自動実施しない |
| product surface | **今回対象外。将来の製品仕様変更を要する** | GUIに依存差分、健康診断、キャッシュ状況を出すならRust Coreの責務・権限・非Nixホスト挙動・受入条件を新規changeで定義 |

期待メリットが「開発者だけの作業の短縮」であれば、まず開発用CLI・CIで実現すべきであり、製品GUI・常駐サービスを追加しない。将来製品化する場合は「診断表示」だけのread-onlyユースケースを先に切り出し、適用・更新などの破壊的操作と混ぜない。

## D16. メリットの定量化と導入しない条件

| 価値仮説 | 計測指標 | 採用に進む条件 | 導入しない具体例 |
| --- | --- | --- | --- |
| CIが速くなる | 同一条件のcritical path、runner time、peak RAM、失敗率 | 既存`development-workflow`の-25% / +20%以内の目標を満たし、検証被覆の低下0 | checksが少なくfast-buildの評価オーバーヘッドの方が大きい |
| 変更原因を特定しやすい | 既知変更fixtureの原因特定率・調査時間 | 正/負fixtureで正しく差分が出て、開発者が結果を理解できる | 既存`nix-melt`/`nix-tree`と実質的に重複し、追加負担が上回る |
| 入力更新漏れが減る | rootと全独立Flakeの監査対象coverage、true positive/false positive | 意図的pinを尊重し、欠落lockを`unknown`と判定 | 期限超過を理由に有効な安全性backportを無条件に置換する |
| SBOM監査が改善する | 対象closureのpackage coverageと既存release SBOM差 | 監査用途の新しい情報が得られ、誤検知をtriage可能 | 同じ出力を二重生成するだけで保守費が増す |
| ビルド/通信が減る | cache hit、bytes、rebuild time、S3 API/容量/egress/電力/保守費 | 現行ローカルstoreに対し総費用と時間の便益が検証できる | 1台Runnerでwarm storeが使え、共有cacheがコストと障害点を増やす |
| OCI作成が簡単になる | OCI image size/digest/re-push時間、実動作、SBOM | Linux CLIの検証・配布の具体的需要があり、安全性が同等 | 配布対象はDMG/native binaryのみでOCIを実際に使わない |

各KPIの目標は**事前に**baseline/PoC開始前に記録し、後から都合の良い対象や条件だけ選ぶことを禁止する。既存CIの閾値を他領域（SBOM/差分検査）へ機械的に適用しない。非数値便益は意思決定ログで仮説として扱い、導入効果の「実績」とは表示しない。

## D17. OSS一次資料と採用前再確認事項（2026-11-13）

以下は調査時の参照先・機能候補であり、採用時は固定するversion/revの一次資料と実環境で再確認する。保守状況・既定値・プラットフォーム互換性を現時点の確定値として扱わない。

- nix-fast-build: https://github.com/Mic92/nix-fast-build — 属性別ビルド、machine-readable result、cache upload/remote optionを提供。高速化は負荷や属性数依存。
- nix-diff: https://github.com/Gabriella439/nix-diff — drv間の差分説明。上流READMEでは新機能開発予定なしと説明されており、保守継続性を別途審査する。
- flake-checker: https://github.com/DeterminateSystems/flake-checker — `flake.lock`のroot-level Nixpkgs入力を検査。明示パスを指定可能、標準の経過日数閾値は30日。root-onlyで全依存監査とは言えない。
- nix-unit: https://github.com/nix-community/nix-unit — 評価失敗をテスト項目ごとに扱える。Lixは別実装`lix-unit`。
- sbomnix: https://github.com/tiiuae/sbomnix — Nix由来SBOM、dependency graph、outdated、provenance等。既存release assetの証明とは異なる。
- ncps: https://github.com/kalbasit/ncps — 上流cache proxyとローカル署名・監視。上流はデータ整合・復旧の保証がない早期開発状態と明記している。
- niks3: https://github.com/Mic92/niks3 — S3直接読込と署名、OIDC、GC。metadata/GCのためPostgreSQL等の構成が追加される。
- nix2container: https://github.com/nlewo/nix2container — tarballをNix storeに保存せずOCIを構築できる。ただし実行環境と配布要件を別検証する。
