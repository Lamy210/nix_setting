# Change: 独立Python Flakeのuvビルド診断と比較PoC

## Why

[Issue #335](https://github.com/Lamy210/nix_setting/issues/335) で、root devShellと独立Python templateが同一uv 0.13.5のソース・Cargo hashを指定しながら異なるderivationをビルドすることを確認した。CI実行時間を削減する前に、入力差分と実行方法を再現可能な証拠として取得する。

## What Changes

- read-onlyの開発者向け診断とオフライン回帰テストを設計する。最初の対象は x86_64-linux。
- root/templateの独立lock、uvセキュリティbackport、既存CIを維持したbaselineを作る。
- pin整合候補は隔離した使い捨てcheckoutで比較する。結果を採用、Defer、No-Goの判断資料とし、恒久的なlock更新・キャッシュ導入・CI変更は別changeにする。
- #330の採用ポリシー（#332で運用実装中）のGate 0、証拠モデル、独立Flake、negative test、approvalを継承する。ポリシーの未完了部分を完了済みとみなさない。

## Impact

- Affected specs: `development-workflow`（診断・比較に限ったADDED requirements）。
- Affected code (提案): `scripts/ci/` の診断スクリプト、`tests/` のfixture/テスト、`docs/testing/` の実行手順。root/templateへの診断専用出力が必要なら別途レビューし、配布インターフェースに追加しない。
- Non-goals: required checks、runner構成、cache trust設定、release、製品CLI/GUI、公開Flake lockまたはuv更新。新規OSSは導入しない。

本PRはStage 1の提案のみ。strict validation、proposal approval、#332の運用実装および先行するセキュリティ変更との整合確認より前に実装・計測を開始しない。
