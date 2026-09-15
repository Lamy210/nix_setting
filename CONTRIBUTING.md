# CONTRIBUTING

SchneeForge の開発運用ルール。チーム開発相当の規律を守る。

> リリース運用（release ブランチ・チェックリスト）は [RELEASE.md](./RELEASE.md) を参照。
> 現在の開発状態・デグレ・次の作業は [docs/STATUS.md](./docs/STATUS.md) を参照。

## ブランチ運用（Git Flow 簡略版）

```
main        リリースブランチ（production）。tag はここに打つ。直接 push 禁止
develop     開発ブランチ（統合・default）。直接 push 禁止
feat/*      機能開発（develop から切り、PR で develop へ squash merge）
fix/*       バグ修正
refactor/*  リファクタリング
docs/*      ドキュメント
test/*      テスト
chore/*     雑務（CI/設定/ OpenSpec archive 等）
release/*   リリース準備（develop から切り、main へ merge commit）
hotfix/*    緊急修正（main から切り、main へ merge 後 develop へ back-merge）
```

### Merge method

- topic branch (`feat/*`, `fix/*`, `refactor/*`, `docs/*`, `test/*`, `chore/*`) → `develop`: **squash merge**
- `release/vX.Y.Z` → `main`: **merge commit**
- release 後の `main` → `develop`: **merge commit**
- release / back-merge で squash / rebase merge を使わない
- 過去の diverged history を修正するための force push / history rewrite を行わない

## 大きな OpenSpec change の取り扱い

1 つの OpenSpec change が多数のタスクにまたがる場合も、1 PR = 1 concern を維持する。

- **実装 branch / PR は change 単位で 1 本**にする。`feat/<change-name>` 等を develop から切り、対象 tasks を完了させて実装 PR を作る。
- change のアーティファクトが既に develop に commit 済みの場合は、その change 用の topic branch を develop から切って継続する。
- **OpenSpec archive は実装 PR merge 後に別 PR** とする。`develop` から `chore/archive-<change-name>` を切り、`openspec archive <name> --yes` を実行して main spec と archive を同期する。
- tooling-only で main spec を更新しない change のみ `openspec archive <name> --skip-specs --yes` を使う。
- **マージ済みの feature/release/hotfix/archive ブランチは削除**する（孤児ブランチを残さない）。
  ```bash
  git push origin --delete <branch>
  git branch -d <branch>
  ```
- セッション開始時は `git fetch --prune` で孤児リモートブランチの状況を確認する。

## 日常の開発フロー

```bash
# 1. develop を最新化
git checkout develop
git pull

# 2. topic branch を作成
git checkout -b feat/<kebab-case-name>

# 3. OpenSpec change を作成（対象 change では必須）
openspec new change <kebab-case-name>
# → proposal.md → design.md → specs/ → tasks.md
openspec validate <kebab-case-name> --strict
# → validation + proposal review/approval が終わるまで実装しない

# 4. 実装（tasks.md の順に、チェックを付ける）

# 5. 品質ゲートをローカルで実行
openspec validate --all --strict
cargo test && cargo clippy -- -D warnings && cargo fmt -- --check
nix flake check

# 6. コミット（conventional commits）
git commit -m "feat: ..."

# 7. 実装 PR を作成 → review → develop へ squash merge
gh pr create --base develop --title "feat: ..."

# 8. 実装 merge 後、archive branch を develop から作成
git checkout develop && git pull
git checkout -b chore/archive-<kebab-case-name>
openspec archive <kebab-case-name> --yes
git add -A && git commit -m "chore: archive <kebab-case-name> + sync specs"
gh pr create --base develop --title "chore: archive <kebab-case-name>"
# → review → develop へ squash merge
```

## リリースフロー

```bash
# 1. develop が安定したら release branch を切る
git checkout develop && git pull
git checkout -b release/vX.Y.Z

# 2. release branch で最終検証（CI + 実機 smoke）

# 3. main へ PR。merge method は merge commit 固定
gh pr create --base main --head release/vX.Y.Z --title "release: vX.Y.Z"
# review + required checks green 後、GitHub UI/CLI で merge commit を使用

# 4. main の merge commit に tag を打つ
git checkout main && git pull
git tag -a vX.Y.Z -m "SchneeForge vX.Y.Z"
git push origin vX.Y.Z

# 5. main 自体を head にして develop へ back-merge PR。merge commit 固定
gh pr create --base develop --head main --title "chore: back-merge vX.Y.Z"
# review + required checks green 後、merge commit を使用

# 6. release branch を削除
git push origin --delete release/vX.Y.Z
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
- OpenSpec 対象 change は proposal approval + strict validation を実装開始条件とする
- PR テンプレートにチェックリストあり（`docs` 以外は全項目必須）
- required checks と review を通過するまで merge しない
- merge method は branch 種別に従う

## ブランチプロテクション

- `main`: direct push 禁止、PR 必須、CI checks 必須
- `develop`: direct push 禁止、PR 必須、CI checks 必須
- required checks の変更は段階移行する。新しい aggregator/check を既存 required contexts と並行稼働させ、成功実績を確認する前に既存 contexts を外さない

## 品質ゲート（CI）

```
openspec-check   openspec validate --all --strict
flake-check      nix flake check + Linux build
macos-check      nix-darwin + HM build
rust-check       cargo test / fmt / clippy
lint             statix / deadnix / actionlint / shellcheck
secret-scan      trufflehog
```

> `openspec-check` はこの workflow hardening から strict validation を enforce する。CI topology の分割・`ci-required` aggregator は follow-up `refactor-ci-critical-path` で扱う。

## OpenSpec の必須条件

- 機能追加・breaking change・architecture change・behavior-changing optimization・security pattern change には OpenSpec change を伴う
- requirement には SHALL/MUST、Scenario には WHEN/THEN
- 対象 change は `openspec validate <change-id> --strict` を通す
- repository 全体は `openspec validate --all --strict` を gate とする
- proposal approval 前に実装しない
- archive は実装 PR merge 後の separate PR で行う
- 手書き `docs/*.md` spec は作らない
