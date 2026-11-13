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
