# CONTRIBUTING

SchneeForge の開発運用ルール。チーム開発相当の規律を守る。

> リリース運用（release ブランチ・チェックリスト）は [RELEASE.md](./RELEASE.md) を参照。
> 現在の開発状態・デグレ・次の作業は [docs/STATUS.md](./docs/STATUS.md) を参照。

## ブランチ運用（Git Flow 簡略版）

```
main        リリースブランチ（production）。tag はここに打つ。直接 push 禁止
develop     開発ブランチ（統合・default）。直接 push 禁止
feat/*      機能開発（develop から切り、PR で develop へ）
fix/*       バグ修正
docs/*      ドキュメント
refactor/*  リファクタリング
test/*      テスト
chore/*     雑務（CI/設定等）
release/*   リリース準備（develop から切り、main へ merge 後に tag）
hotfix/*    緊急修正（main から切り、main + develop 両方へ）
```

## 大きな OpenSpec change の取り扱い

1 つの OpenSpec change が多数のタスク（例: `gui-normalization` は 63 tasks / 9 phase）に
またがる場合の運用:

- **ブランチ / PR は change 単位で 1 本**にする（phase ごとに切らない）。`feat/<change-name>` を
  develop から切り、全タスク完了まで積み、最後に 1 PR で develop へ merge する。
- change のアーティファクト（proposal/design/specs/tasks）が**既に develop に commit 済み**の場合
  （過去に仕様だけ先行で入れた等）は、その change 用の feature ブランチを develop から切って継続し、
  全タスク完了後に `openspec archive <name>` する。
- **マージ済みの feature/release/hotfix ブランチは削除**する（孤児ブランチを残さない）。
  ```bash
  git push origin --delete <branch>   # リモート削除
  git branch -d <branch>              # ローカル削除
  ```
- セッション開始時は `git fetch --prune` で孤児リモートブランチの状況を確認する。

## 日常の開発フロー

```bash
# 1. develop を最新化
git checkout develop
git pull

# 2. feature ブランチを作成
git checkout -b feat/<kebab-case-name>

# 3. OpenSpec change を作成（必須）
openspec new change <kebab-case-name>
# → proposal.md → design.md → specs/ → tasks.md
# → openspec validate <name> が通るまで実装しない

# 4. 実装（tasks.md の順に、チェックを付ける）

# 5. 品質ゲートをローカルで実行
cargo test && cargo clippy -- -D warnings && cargo fmt -- --check
nix flake check && openspec validate --all

# 6. コミット（conventional commits）
git commit -m "feat: ..."

# 7. PR を作成 → レビュー → develop へ merge
gh pr create --base develop --title "feat: ..."

# 8. 完了時に OpenSpec change をアーカイブ
openspec archive <name>
```

## リリースフロー

```bash
# 1. develop が安定したら release ブランチを切る
git checkout develop
git checkout -b release/vX.Y.Z

# 2. release ブランチで最終検証（CI + 実機 smoke）

# 3. main へ merge して tag を打つ
git checkout main
git merge release/vX.Y.Z
git tag vX.Y.Z
git push origin main --tags

# 4. main を develop へ back-merge
git checkout develop
git merge main
git push

# 5. release ブランチを削除
git branch -d release/vX.Y.Z
```

## コミット規約（conventional commits）

| 型 | 用途 |
|----|------|
| `feat:` | 新機能 |
| `fix:` | バグ修正 |
| `refactor:` | 挙動を変えないリファクタリング |
| `docs:` | ドキュメント |
| `test:` | テスト追加・修正 |
| `chore:` | ビルド・CI・ツール設定 |
| `perf:` | 性能改善 |

例: `feat: add ToolResolver`, `fix: resolve button dispatch bug`

## PR ルール

- 1 PR = 1 関心事
- PR タイトルは conventional commits 形式
- feature/fix には必ず OpenSpec change を伴う
- PR テンプレートにチェックリストあり（`docs` 以外は全項目必須）
- レビュー通過まで merge しない

## ブランチプロテクション

- `main`: direct push 禁止、PR 必須、CI チェック必須
- `develop`: direct push 禁止、PR 必須

## 品質ゲート（CI）

```
openspec-check   openspec validate --all
flake-check      nix flake check + Linux build
macos-check      nix-darwin + HM build
rust-check       cargo test / fmt / clippy
lint             statix / deadnix / actionlint / shellcheck
secret-scan      trufflehog
```

## OpenSpec の必須条件

- 機能追加・変更には必ず OpenSpec change を伴う
- requirement には SHALL/MUST、Scenario には WHEN/THEN
- `openspec validate --all` が通るまで実装しない
- 手書き `docs/*.md` spec は作らない
