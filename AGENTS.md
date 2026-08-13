# AGENTS

SchneeForge（Declarative Developer Workstation Manager）の開発ルール。チーム開発相当の規律を守る。

## 開発フロー: OpenSpec + ブランチ + PR を必ず使う

main へ直接コミットしない。必ず feature branch → PR → レビュー → merge とする。

```bash
# 1. 現状確認
openspec list                    # 進行中の change 一覧
openspec status --change <name>  # アーティファクト進捗

# 2. ブランチを作成
git checkout -b feat/<kebab-case-name>

# 3. OpenSpec change を作成
openspec new change <kebab-case-name>
# → proposal.md → design.md → specs/ → tasks.md の順に書く
# → openspec validate <name> が通るまで実装しない

# 4. tasks.md の順に実装（チェックを付ける）

# 5. コミット（conventional commits）
git commit -m "feat: ..."

# 6. PR を作成してレビュー後に merge
gh pr create --title "feat: ..."
# → レビュー → merge

# 7. 完了時にアーカイブ
openspec archive <name>
```

## ブランチ・コミット規約

| 種別 | プレフィックス | 例 |
|------|---------------|-----|
| ブランチ | feat/ fix/ refactor/ docs/ test/ chore/ | `feat/gui-diagnostics` |
| コミット | feat: fix: refactor: docs: test: chore: | `fix: resolve button dispatch bug` |

- **main へ直接 push しない**。必ず PR を挟む
- 1 PR = 1 関心事（feature / fix / refactor を混ぜない）
- PR タイトルは conventional commits 形式

## OpenSpec の必須条件

- 機能追加・変更には必ず OpenSpec change を伴う（spec の無い実装はしない）
- requirement には SHALL/MUST、Scenario には WHEN/THEN を必ず含める
- `openspec validate --all` が通るまで実装を始めない
- 手書きの `docs/*.md` spec は作らない（OpenSpec の changes/ を使う）

## アーキテクチャ

```
schneeforge-core (crates/core)   ← 実ロジック唯一の置き場
  ├── actions     (apply/rollback/scan/upgrade)
  ├── discovery   (detect_host/tool検出)
  ├── manifest    (config.toml)
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
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
nix flake check
openspec validate --all
```

## コードレビューチェックリスト

- [ ] OpenSpec change が存在し `openspec validate --all` が通る
- [ ] 実ロジックが core にあり、CLI/GUI に重複していない
- [ ] テストが追加・更新されている
- [ ] conventional commits 形式
- [ ] 1 PR = 1 関心事
- [ ] 既存テスト・CI が green

## 現在進行中

- `openspec/changes/gui-normalization/` — GUI を動く installer へ（tasks.md 63件、Phase 1 の Core Foundation から着手）
