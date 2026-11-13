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
