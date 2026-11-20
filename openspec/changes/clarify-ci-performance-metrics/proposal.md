# Change: CI性能評価の集計範囲と採用判定を分離する

## Why

既存 development-workflow の CI optimization measurement は Rust fan-out の worker + aggregator を runner-time guard の対象とする。一方、#330 の nix-tooling-adoption 提案（#332 は運用実装）には CI 全体の最適化に対する runner 合計時間 +20% の記述がある。両者の分母・対象ジョブを混同すると採用判定が異なる。#340 の uv 診断と将来の署名付きキャッシュ PoC にも共通の計測契約が必要である。

## What Changes

- ブランチ保護上の必須7コンテキストの完了を `server-required gate elapsed` として測定し、non-required `ci-required` 候補の完了を別指標にする。`ci-required` は追加の受入ジョブにも依存するため両者を同一視しない。
- Rust 固有の guard と全 Workflow の runner-time を別指標として定義する。
- CI 全体の最適化完了を主張する場合は、既存 -25% required critical path 目標と関連する guard をすべて維持する。全体 +20% も維持する案をレビューし、既存規範を緩和しない。
- 個別 PoC の目標は独立して事前承認し、全体最適化完了と混同しない。
- 比較単位、実行 attempt、計測欠損、非回帰、証跡の境界を規定する。

## Impact

- development-workflow への ADDED requirements のみ。nix-tooling-adoption の archive 後の整合確認は実装前に別途行う。
- #332 の実装・承認条件や #340 の uv 固有目標は変更しない。
- 本PRは設計レビュー用。コード、Workflow、lock、保護設定、キャッシュ設定、canonical spec は変更しない。
- 公式 strict 検証、提案承認後に別実装 PR を作成する。実装の受入完了後に別 archive PR を作成する。

## References

- openspec/specs/development-workflow/spec.md (CI optimization measurement)
- openspec/changes/add-nix-tooling-evaluation-policy/specs/nix-tooling-adoption/spec.md
- docs/testing/ci-workflow-timing.md
- #330 / #332 / #340 / #252
