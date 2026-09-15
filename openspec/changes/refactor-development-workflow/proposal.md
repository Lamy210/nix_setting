# Change: 開発ワークフローを OpenSpec / Git ブランチ / CI の実運用に整合させる

## Why

SchneeForge は `develop` 統合、`release/*` 経由の `main` リリース、OpenSpec change、複数の required CI checks を運用しているが、文書化された OpenSpec archive 順序と直近 PR の実運用が一致していない。また `main` と `develop` の ancestry が diverge しており、release/back-merge に squash を適用すると履歴関係が失われる。今後 Windows/WSL2 対応、macOS compatibility matrix、CI 並列化を安全に進めるため、先に開発ワークフローを一貫した状態へ固定する必要がある。

## What Changes

- feature/fix/refactor/docs/test/chore の topic branch は `develop` を base とし、PR を squash merge する方針を明文化する。
- release branch は `develop` から `release/vX.Y.Z` を作成し、`main` への PR は merge commit を使用する方針を明文化する。
- release 後の `main` → `develop` back-merge は merge commit を使用し、release ancestry を保持する。
- OpenSpec change は proposal/design/spec/tasks を feature branch 上で作成・strict validate し、実装 PR merge 後に archive + spec sync を別 `chore/archive-*` PR で行う方針へ統一する。
- CI の branch protection migration を安全に行えるよう、将来の `ci-required` aggregator 導入手順を規約化する。ただし本 change では既存 required check の削除や branch protection 設定変更は行わない。
- `AGENTS.md`、`openspec/project.md`、`docs/STATUS.md` の workflow 記述を同期する。

## Non-Goals

- Windows/WSL2 の実装。
- macOS 15 / macOS 26 / Xcode version matrix の CI 実装。
- `check.yml` の job 分割や実行時間最適化。
- GitHub branch protection の server-side 設定変更。
- 過去の `main` / `develop` history の rewrite。

## Impact

- Affected specs: `development-workflow` (new)
- Affected docs: `AGENTS.md`, `openspec/project.md`, `docs/STATUS.md`
- Affected CI: none in this change; follow-up `refactor-ci-critical-path` で実装
- Migration: 現在の diverged history は rewrite せず、次回 release 以降で merge commit による ancestry 保存へ切り替える
