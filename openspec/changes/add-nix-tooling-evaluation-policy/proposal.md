# Change: Nix関連OSSの導入評価と段階的な採用ポリシー

## Why

SchneeForge (nix_setting) は Nix/Flakes、Home Manager、nix-darwin、Rust/Tauri、Linux・macOS・実験的Windows/WSL2にまたがる。2026年のOSS探索でCI並列ビルド・依存差分分析・Flakeの健全性監査・SBOM・共有キャッシュ等の候補が見つかったが、既存実装と重複する導入、required CIの置換、未検証の外部OSSを本番や製品に直接組み込むことは危険である。

既存の `development-workflow` は required context 維持・CI性能計測・OpenSpec承認順序を規定済み。これらを継承し、新しいOSSの「何を採用し、いつ、何を証明してから採用するか」を独立したポリシーとして仕様化する。

## What Changes

- **新規capability `nix-tooling-adoption`** を設け、OSSの現状棚卸し・導入判断・再現性・セキュリティ審査・プラットフォーム受入・PoC計測・撤退条件を定義する。
- **維持・再利用**: 既存 `flake-parts`、`treefmt-nix` (nixfmt/shfmt/taplo)、`nixd`、`statix`、`deadnix`、`nix-unit`、`nix-tree`、`nix-melt`、`devenv` を二重導入しない。`nix-unit` は `tests/nix-unit.nix` と `justfile` に存在するため、新規採用ではなく**CI実行範囲の照合**を行う。
- **優先PoC (採用未確定)**: `nix-fast-build` (並列評価・ビルド)、`nix-diff` (derivation差分)、`flake-checker` (古いFlake入力の診断) をpinされた検証用ツールとして別PRで試す。
- **追加検証候補**: `sbomnix` (Nixパッケージ依存/SBOM診断)、`niks3`・`ncps` (共有キャッシュ方式比較)、`nix2container` (OCIビルド) を段階的に評価する。既存の release artifact 用cosign/SLSA/SPDX生成は置き換えない。
- **当面見送り**: `Gradient`/`Hydra`等の専用CIサービス全面移行、`Lix`/`Determinate Nix`へのManaged Nix provider切替、`cargo-dyndrv`/Dynamic Derivations、`system-manager`/OS再構築ツールの製品組み込み、AIエージェント一括導入。別の受入・運用条件とOpenSpec changeが必要。
- PoCは事前採取した同一条件のbaselineと比較し、**required CIと配布物検証の維持**、ゼロ特権昇格、同一commit/flake.lockによる再現性、導入後の切り戻しを必須とする。

## Scope / Out of Scope

**本PRはOpenSpec設計のみ。** `flake.nix`、`flake.lock`、`modules/`、`templates/`、CI workflow、`crates/`、`apps/`、実機・サーバ設定を変更しない。新規OSSの実装・本番キャッシュ・デプロイ・ツールのデフォルト有効化・managed Nix provider変更は承認後の別PRで行う。

本changeは既存 `development-workflow`、`release-supply-chain`、`tool-resolution`、`platform-execution` の規範を変更しない。既存安全要件に抵触する選択肢は不採用とする。

## Impact

- **新規仕様**: `nix-tooling-adoption`。
- **既存仕様**: `development-workflow` の required context と計測目標、`release-supply-chain` の署名/SBOM、`tool-resolution` の経路を維持。
- **将来変更箇所の候補**: `modules/flake-parts/`、`tests/nix-unit.nix`、`scripts/ci/`、`.github/workflows/check.yml`、`docs/testing/` (**このPRでは変更なし**)。
- **進行中PRへの配慮**: #324 のLinux Nix固定CI、#323/#327 のmacOS release artifact gate、#299 のNodeセキュリティ検証、既存の`update-node24-runtime-security`等を迂回しない。
- **対応対象**: aarch64-darwin、x86_64-linux、aarch64-linuxの各評価/実行能力を区別。Windowsはnative Nix導入ではなくWSL2 Linux helper経由の実験的対応のみ。x86_64-darwinの対応拡大は含まない。

## Acceptance / Approval Gate

1. `openspec validate add-nix-tooling-evaluation-policy --strict` と `openspec validate --all --strict` を実行し、**最新HEADで成功結果を確認するまで仕様承認済みとしない**。
2. `proposal.md`、`design.md`、`tasks.md`、delta specのレビューを受け、ツール採用可否・閾値・ロールバック責任を確定する。
3. 実装PRはPoCごとに分割。既存required checksを並行稼働させ、結果が`failure`/`skipped`/`cancelled`でも緩和しない。
4. 本PRではOSS導入・高速化・セキュリティ改善が「完了」とは表明しない。実装後、別`chore/archive-*`PRでアーカイブする。

## 一次資料

- https://github.com/Mic92/nix-fast-build
- https://github.com/Gabriella439/nix-diff
- https://github.com/DeterminateSystems/flake-checker
- https://github.com/tiiuae/sbomnix
- https://github.com/Mic92/niks3
- https://github.com/kalbasit/ncps
- https://github.com/nlewo/nix2container
- https://github.com/nix-community/nix-unit
- リポジトリの既存規約: `openspec/specs/development-workflow/spec.md`、`openspec/specs/release-supply-chain/spec.md`、`docs/testing/ci-workflow-timing.md`


## Expected user/developer benefits (hypotheses, not implemented functionality)

| Area | 何ができるようになるか | 誰に価値があるか | 期待メリットと反証 |
| --- | --- | --- | --- |
| Build analysis | `nix-diff`で変更前後のdrvの差分・キャッシュミス理由を診断 | Nix設定を変更する開発者 | 不要な再ビルドの原因を調査しやすくなる。既存`nix-melt`等と差がなければ見送り |
| CI acceleration | `nix-fast-build`の並行評価・ビルドを任意CIで試す | CI担当・PRレビュアー | 多数のchecksでは待ち時間短縮が期待できる。既存requiredと同等の被覆を保証するものではなく、効果は要実測 |
| Flake input health | `flake-checker`でroot・独立テンプレートのNixpkgs pinを個別検査 | 開発者・依存更新担当 | 更新漏れを発見する可能性。ただし日数だけの機械更新はしない |
| Supply-chain audit | `sbomnix`でNix closureのSBOMや依存グラフを調べる | セキュリティ・リリース担当 | 既存release SBOMで分からない依存を調査できる可能性。情報が重複するだけなら不採用 |
| Multi-runner cache | `ncps`/`niks3`で上流download/ビルド成果物を複数ホストに再利用 | CI/インフラ担当 | 転送/ビルド削減。単一Runnerでwarm storeが十分なら追加コストが勝る |
| OCI build | `nix2container`でLinux CLIの検証用imageをNixから生成する | コンテナビルド担当 | tarballやlayer再送の削減可能性。OCIを配布/運用しないなら不要 |

**製品利用者への効果は間接的**: このchangeはSchneeForge CLI/GUIへ新規ボタンや自動更新を提供しない。開発者用、CI用、インフラ用を区別し、エンドユーザー向けread-only診断などを作る場合は別OpenSpecとする。

各OSSについて「利用者・操作・代替案・期待メリット・測定方法・運用コスト・不採用条件」を`design.md`に記録し、その記録を採用判断の必須証拠とする。

## Adoption decisions & conditions

- **即時の製品導入は0件**。既存のnix-unit/flake-parts/treefmt/devenv等は再利用する。
- 優先PoCは `nix-fast-build` / `nix-diff` / `flake-checker` をそれぞれ独立changeで検証する。
- `sbomnix` / `ncps` / `niks3` / `nix2container` は利用頻度・増分価値・費用が実測で示された場合のみ次段階へ進む。
- OSSの特性を採用の可否と混同せず、Gate 0（security/maintenance/reproducibility）、Gate 1（比較）、Gate 2（Go/No-Go）を通過させる。
