# Design: Nix OSS導入評価・選定・安全な段階適用

## Context / Evidence (develop @ 2026-11-13)

実際に読んだ既存ファイル:
- `flake.nix`: `flake-parts`、`home-manager`、`nix-darwin`、`treefmt-nix` がinputs。対応systemsは `aarch64-darwin` / `x86_64-linux` / `aarch64-linux`。
- `modules/flake-parts/default.nix`: nixfmt/shfmt/taploによるtreefmt契約。
- `modules/flake-parts/devshells.nix`: `nixd`, `statix`, `deadnix`, `devenv`, `nix-tree`, `nix-melt`, `nix-unit` 等が導入済み。
- `tests/nix-unit.nix`、`justfile`: unit testの既存入口がある。ただしrequired CIで全て実行されているかは独立して検証する。
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
| **既存維持（追加導入不要）** | flake-parts, treefmt-nix, nixfmt, nixd, statix, deadnix, devenv, nix-tree, nix-melt, nix-unit | 既存設定と利用経路を確認。nix-unitはテスト実行の不足だけを調べる |
| **P1優先PoC（非blocking）** | nix-fast-build | checksの評価・ビルドを並列化。required flake-check / macOS / release gateを置換しない |
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
| Baseline | 同一commit SHA / `flake.lock` / runner class / system / Xcode / input pin / 実行コマンド / 成功したrun ID |
| 完了時間 | `docs/testing/ci-workflow-timing.md` のreportを再利用。queue(wait)とrunner実行時間を分離 |
| 性能 | cold/warm cacheそれぞれ成功5回以上を暫定サンプルとし、中央値・分散、critical path、runner合計時間を別々に計測 |
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
2. **P1 diagnostics**: nix-diff / flake-checkerを任意ツールとして追加比較。nix-fast-buildを非blocking/manual workflowで計測。各ツール独立のimplementation PR。
3. **P2 security/cache**: sbomnixでNix closureを調査。キャッシュは比較Spikeを先行し、必要が証明されてからniks3等の最小構成を別のOpenSpecで設計。
4. **P3 OCI/研究**: nix2containerのみ隔離環境で評価。ほかは研究として監視。
5. 実装/必要な運用runbookと受入完了後、archiveを専用PRで行う。

## Open Questions (実装前に解消)

- `nix-unit` はrequired CIで実際に実行されるか。未実行なら既存fast laneへ追加可能か。
- `nix-fast-build` はどのattributesで完了時間短縮が見込めるか。単一の小さなchecksでは追加負荷の方が大きくないか。
- flake-checkerのNixpkgs対応branch、例外扱い、false positive抑制ルール。
- Nix SBOMが実際のRust/Tauri releaseの依存範囲をどこまで表現できるか（既存syftとの差異）。
- 共有バイナリキャッシュを必要とするRunner数、総転送量と保管期限。S3互換料金は利用リージョンに依存するため未算出。
- 非required PoCが失敗した場合の記録先と翌月の再評価責任者。

## D7. 導入判定モデル（セキュリティ・品質をスコアより優先）

候補単位で **Gate 0（必須条件）→ Gate 1（比較）→ Gate 2（Go/No-Go）** の順に評価する。

### Gate 0: 満たさなければ自動的にNo-Go

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

各軸を0〜5段階で根拠つき評価し、`weighted points = sum(weight * rating / 5)` とする。未確認は `unknown` として加点せず、Gate 0に必要な未確認項目があればscoreに関係なくNo-Go。75/100以上は**本番採用審査へ進める暫定目安**に過ぎず、品質・安全性・既存CI目標の違反を免除しない。評価尺度・根拠の変更はdecision logに記録する。

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
| flake-checker | root `flake.lock` と独立 `templates/node/flake.lock`、devenvのpin有無 | 意図的な古い入力、サポート外branch、例外pinをfixture化。ルートのみ検査してテンプレートの安全性を宣言しない。デフォルトの30日警告を一律自動更新と混同しない |
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

各PoCのevidence recordは `tool_id`, `tool_pin`, `license`, `source_url`, `repo_sha`, `flake_lock_sha256`, `runner`, `os_arch`, `xcode_version(該当時)`, `execution_stage`, `attempt`, `job_url`, `cold_warm`, `attributes_expected`, `attributes_evaluated`, `attributes_built`, `attributes_skipped`, `outcome`, `duration`, `runner_seconds`, `cpu_seconds`, `peak_rss_bytes`, `disk_peak_bytes`, `bytes_rx`, `bytes_tx`, `cost_currency`, `cost_amount_or_unknown`, `rollback_reference`, `review_decision` を記録する。各数値の単位、測定方法、unknown理由を記す。個人識別子・秘密情報・絶対home pathは削除/マスクする。証拠ファイルはPRアーティファクトまたはCIへのリンクで参照し、保持期間が終わっても主要集計/判定根拠はPR本文/decision recordに残す。

**同一条件比較の注意:** キャッシュcoldとは実際の関連パスがローカル/候補cacheに無いと確認できた状態、warmとは必要なパスがキャッシュに存在すると確認した状態を指す。クリア方法は隔離storeまたはdisposable runnerを用い、共有Nix storeの破壊的GCはしない。baselineと候補を交互に試し、worker負荷・ネットワーク状態・実行時刻の差を記録。成功runが各条件で5未満なら探索結果扱い。リリース判定には10以上を目標として、差が小さい場合はサンプル増加/要因分析を優先する。

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
Baseline SHA + lock SHA / PoC SHA + lock SHA:
Evidence links / Attributes and outcomes / Cost estimate:
Security Gate 0 (pass/fail/unknown):
Weighted scoring + rationale:
Go / Conditional / No-Go / Defer:
Owner / Reviewer / Review date / Re-evaluation trigger:
Disable procedure / Revert commit / Restore accepted state:
Unresolved questions:
```

- optional PoCの無効化: **専用の実行スイッチ/手動workflowを停止** → ツールinput・lockの変更をrevert → baselineの `nix flake check`、unit、Rust、macOS、release gateを再確認。failed requiredのskip・context変更で見かけ上greenにしない。
- cache rolloutの無効化: write権限停止 → 新substituter削除/信頼鍵revocation検討 → 上流の安全なcache/ソースビルドへ復帰 → キャッシュ破損時は影響artifactを隔離。署名鍵漏洩時は別セキュリティインシデントとして処理する。
- UI/CLIへの昇格は Core/CLI/Desktopの責務分離と独立Toolchain契約を再設計する**別**OpenSpec changeとする。単なるツール発見やOSへの自動インストールをこのchangeから推奨しない。
- baselineと採用条件が満たされない候補は `No-Go` の理由を記録し、恒久的なcron/runner資格情報を残さない。

## D13. 追加レビューで解消するべき境界

1. セキュリティ更新用に意図的に固定しているroot・Node・devenvのinputsを「30日を超えた」だけで弱く扱わない。pin理由と実体runtime/closureを検証する。
2. GitHub branch protectionのrequired contextsをサーバ側で確定し、CI YAMLの名前から推測しない。
3. `devenv` や `nix-unit` はすでにdevShellで配布されているが、利用可否とrequired CI実行は異なる。導入済み数とテスト被覆率を混同しない。
4. Windows/WSL2はLinuxバックエンドで実行することと、Windows native CLIの配布検証を別扱いにする。
5. 実装PRを1 tool = 1 concernで分け、この**導入評価ポリシーchange自体**は方針が承認され、その仕様整合を確認できた段階で閉じられる。全PoC・新OSSの本番導入完了まで本changeのarchiveを不必要に遅らせない。各PoCの動作変更には別OpenSpec change/PRを作り、その実装・archiveはそのchange内で管理する。
