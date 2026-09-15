## Context

SchneeForge は `develop` を既定ブランチとし、topic branch から PR を作成して統合している。リリースは `release/vX.Y.Z` から `main` へ進め、リリース後に `main` の内容を `develop` へ戻す。OpenSpec は change 単位で proposal / design / specs / tasks を管理する。

現状の問題は、文書上の「archive を PR 前に行う」規約と、直近の実運用（実装 PR merge 後に archive PR）が一致していないこと、また release/back-merge で squash を使うと ancestry が保持されず `main` と `develop` が Git 上 diverge しやすいことである。

## Goals / Non-Goals

### Goals

- topic 開発、release、back-merge の merge method を明確に分ける。
- OpenSpec lifecycle を実運用と一致させる。
- branch protection の required checks を将来 `ci-required` aggregator へ安全に移行できる手順を定義する。
- `AGENTS.md`、`openspec/project.md`、`docs/STATUS.md` の記述を一致させる。
- 既存 history を rewrite せず、今後の履歴を安定させる。

### Non-Goals

- CI job の実際の分割・高速化。
- Windows / WSL2 backend。
- macOS / Xcode compatibility matrix。
- GitHub settings の自動変更。

## Decisions

### D1: topic branch は `develop` に squash merge

`feat/*`, `fix/*`, `refactor/*`, `docs/*`, `test/*`, `chore/*` は `develop` を base にする。1 PR = 1 concern を維持し、統合時は squash merge を使う。

理由:
- `develop` の履歴を PR 単位に保つ。
- feature branch 内の WIP commit を統合履歴へ持ち込まない。
- Conventional Commits の PR title を squash commit title として利用しやすい。

### D2: release → main は merge commit

`release/vX.Y.Z` は `develop` から作成し、`main` への PR は merge commit で統合する。release branch を squash merge しない。

理由:
- `main` がどの release branch を取り込んだか ancestry で追跡できる。
- release 後の back-merge で共通祖先を維持できる。

### D3: main → develop back-merge は merge commit

release 完了後は `main` を `develop` へ merge commit で戻す。squash / rebase は使わない。

既に diverge している過去 history は rewrite しない。必要な conflict を通常の merge で解消し、それ以降の ancestry を維持する。

### D4: OpenSpec archive は実装 merge 後の別 PR

change lifecycle は以下とする。

1. topic branch 上で proposal / design / delta spec / tasks を作成する。
2. `openspec validate <change-id> --strict` を通す。
3. proposal 承認後、同じ topic branch で実装する。
4. tests / CI / `openspec validate --all --strict` を通し、実装 PR を `develop` へ merge する。
5. `chore/archive-<change-id>` branch を `develop` から作成する。
6. `openspec archive <change-id> --yes`（tooling-only かつ main spec を変更しない場合のみ `--skip-specs`）を実行する。
7. archive PR を `develop` へ merge する。

この方式により、実装中 change と deployed truth (`openspec/specs`) の境界を維持する。

### D5: branch protection migration は二段階

将来 `ci-required` aggregator job を導入するとき、既存 required checks を先に削除しない。

1. `ci-required` を追加し、既存 required checks と並行して成功させる。
2. 複数 PR で安定性を確認する。
3. GitHub branch protection を `ci-required` 中心へ変更する。
4. その後にのみ、冗長な required contexts を外す。

本 change は D5 の規約化のみを行い、GitHub server-side 設定は変更しない。

## Risks / Trade-offs

- release/back-merge に merge commit を使うため、`develop` は完全な linear history にはならない。
  - Mitigation: topic branch は squash merge を維持し、merge commit は release 境界に限定する。
- archive が実装 merge 後になるため、一時的に `openspec/changes/<id>` が develop に残る。
  - Mitigation: archive PR を実装 merge 直後に作成し、短時間で閉じる。
- branch protection の required checks 移行時に誤設定すると merge が止まる可能性がある。
  - Mitigation: aggregator 導入と protection 設定変更を別段階にし、既存 checks を先に外さない。

## Migration Plan

1. workflow 文書を新方針へ同期する。
2. 次回 topic PR から squash-to-develop を継続する。
3. 次回 release から release→main と main→develop を merge commit に固定する。
4. `refactor-ci-critical-path` で `ci-required` aggregator を追加する。
5. CI 安定確認後に GitHub branch protection を手動更新する。

## Rollback

この change は開発規約・文書の変更であり、runtime behavior を変更しない。問題があれば文書を以前の方針へ revert できる。Git history の rewrite は行わないため、runtime/release artifact への rollback は不要。
