<!-- OPENSPEC:START -->
# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:
- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# AGENTS

SchneeForge（Declarative Developer Workstation Manager）の開発ルール。チーム開発相当の規律を守る。

## セッション引き継ぎ（必ず実行）

セッション開始時と終了時に、状態を引き継ぐ。

### 開始時

1. [docs/STATUS.md](./docs/STATUS.md) を読む（現在の状態・既知のデグレ・次の作業）
2. [openspec/changes/](./openspec/changes/) の進行中 change を `openspec list` / `openspec status --change <name>` で確認
3. MCP の memory サーバ（`search_nodes`）で前セッションのメモリを検索

### 終了時

1. 完了した作業・残タスク・判断・注意点を memory MCP（`create_entities` / `create_relations`）へ保存
2. [docs/STATUS.md](./docs/STATUS.md) の「完成済み」「進行中」「既知のデグレ」を更新

## 開発フロー: OpenSpec + ブランチ + PR を必ず使う

`main` / `develop` へ直接コミットしない。topic branch → PR → review → merge とする。

```bash
# 1. 現状確認
openspec list                    # 進行中の change 一覧
openspec status --change <name>  # アーティファクト進捗

# 2. develop から topic branch を作成
git checkout develop
git checkout -b feat/<kebab-case-name>

# 3. OpenSpec change を作成
openspec new change <kebab-case-name>
# → proposal.md → design.md → specs/ → tasks.md
# → openspec validate <name> --strict が通るまで実装しない
# → proposal 承認後に実装へ進む

# 4. tasks.md の順に実装（チェックを付ける）

# 5. 品質ゲート
openspec validate --all --strict
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
nix flake check

# 6. コミット（conventional commits）
git commit -m "feat: ..."

# 7. 実装 PR を develop へ作成
# topic PR は squash merge
gh pr create --base develop --title "feat: ..."

# 8. 実装 PR merge 後、develop から archive branch を作成
git checkout develop && git pull
git checkout -b chore/archive-<name>
openspec archive <name> --yes
git add -A && git commit -m "chore: archive <name> + sync specs"
gh pr create --base develop --title "chore: archive <name>"
```

## ブランチ・コミット規約

| 種別 | プレフィックス | 例 |
|------|---------------|-----|
| Topic branch | feat/ fix/ refactor/ docs/ test/ chore/ | `feat/gui-diagnostics` |
| Archive branch | chore/archive- | `chore/archive-gui-diagnostics` |
| Release branch | release/ | `release/v0.3.0` |
| コミット | feat: fix: refactor: docs: test: chore: | `fix: resolve button dispatch bug` |

- **main / develop へ直接 push しない**。必ず PR を挟む
- 1 PR = 1 関心事（feature / fix / refactor を混ぜない）
- PR タイトルは conventional commits 形式
- topic branch → `develop` は **squash merge**
- `release/vX.Y.Z` → `main` は **merge commit**（squash / rebase 禁止）
- release 後の `main` → `develop` back-merge は **merge commit**（squash / rebase 禁止）
- 過去の diverged history を直すための force push / history rewrite はしない

## OpenSpec の必須条件

- 機能追加・breaking change・architecture change・behavior-changing optimization・security pattern change には OpenSpec change を伴う
- requirement には SHALL/MUST、Scenario には WHEN/THEN を必ず含める
- `openspec validate <change-id> --strict` と `openspec validate --all --strict` が通ること
- proposal 承認前に実装を開始しない
- change の archive は実装 PR merge 後に別 `chore/archive-*` PR で行う
- tooling-only で main spec を変更しない archive のみ `openspec archive <change-id> --skip-specs --yes` を許可する
- 手書きの `docs/*.md` spec は作らない（OpenSpec の changes/ を使う）

## アーキテクチャ

```
schneeforge-core (crates/core)   ← 実ロジック唯一の置き場
  ├── actions     (apply/rollback/scan/upgrade)
  ├── discovery   (detect_target/Platform/Architecture/tool検出)
  ├── manifest    (schneeforge.toml)
  ├── repo        (repository解決)
  ├── state       (state.json)
  └── time        (時刻)
CLI (crates/cli)                 ← core を呼ぶだけ
Desktop (apps/desktop)           ← Tauri 2。core を呼ぶだけ
```

原則:
- CLI / Desktop に実ロジックを置かない（core へ集約）
- 新規操作は core に置き、CLI/GUI は adapter にする

## 技術スタック

- Nix (flakes, flake-parts) / Home Manager / nix-darwin
- Rust: schneeforge-core / cli
- Tauri 2: desktop GUI
- 配布: flake / install.sh / GitHub Release (binaries + DMG) / Homebrew / cargo install

## 品質ゲート（コミット前にローカル実行）

```bash
openspec validate --all --strict
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
nix flake check
```

## コードレビューチェックリスト

- [ ] OpenSpec change が存在し `openspec validate --all --strict` が通る
- [ ] proposal が承認済み
- [ ] 実ロジックが core にあり、CLI/GUI に重複していない
- [ ] テストが追加・更新されている
- [ ] conventional commits 形式
- [ ] 1 PR = 1 関心事
- [ ] 既存テスト・CI が green
- [ ] merge method が branch 種別に合っている

## 現在進行中

- `openspec/changes/refactor-ci-critical-path/` — PR #90 で最終検証中。required critical path は baseline 479s → 327s（31.7%短縮）を実測済み。merge 後は別 `chore/archive-refactor-ci-critical-path` PR で archive する
- `openspec/changes/add-dmg-offline-bundle-licensing/` — 法務確認待ち
- `refactor-development-workflow` は 2026-09-16 に archive 済み
- 次: `add-macos-compatibility-matrix` — PR #90 と archive 完了後に別 change として開始する
- 状態・既知のデグレ・次の作業は [docs/STATUS.md](./docs/STATUS.md) を参照（セッション開始時に必ず読む）
- リリース運用は [RELEASE.md](./RELEASE.md) を参照
