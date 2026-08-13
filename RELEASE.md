# RELEASE

SchneeForge のリリース運用。`main`（リリースブランチ）への直接マージは禁止し、必ず `release/*` ブランチ経由で行う。

## ブランチ制約

```
main        直接 push / 直接 merge 禁止（ブランチプロテクションで enforce）
release/*   唯一 main へ merge できる経路
develop     開発統合。feature ブランチの merge 先
```

- `main` への merge は `release/*` ブランチからの PR のみ
- feature ブランチを `main` へ直接 PR しない
- `main` の保護: PR 必須 + 1 承認 + CI status checks + admin も強制

## リリースフロー

```bash
# 1. develop のリリース準備が整ったら release ブランチを切る
git checkout develop && git pull
git checkout -b release/vX.Y.Z

# 2. release ブランチで最終検証（チェックリスト参照）

# 3. main へ PR（release ブランチから）
gh pr create --base main --head release/vX.Y.Z --title "release: vX.Y.Z"

# 4. レビュー + CI green 後に merge → tag を打つ
git checkout main && git pull
git tag -a vX.Y.Z -m "SchneeForge vX.Y.Z"
git push origin vX.Y.Z    # この push が release workflow を発火

# 5. main を develop へ back-merge
git checkout develop
git merge main
git push

# 6. release ブランチを削除
git branch -d release/vX.Y.Z
git push origin --delete release/vX.Y.Z
```

## リリース前チェックリスト

- [ ] 差分確認: `git diff main..develop` でリリース対象を把握
- [ ] 機能漏れ: OpenSpec の tasks.md 未完了項目を確認（`openspec status --change <name>`）
- [ ] デグレ確認: CI 全ジョブ green（`cargo test` / `clippy` / `flake check`）
- [ ] 手動 smoke: 実機で apply / verify / rollback を確認
- [ ] リリースノート: 変更・既知の制限・未完了機能を記載
- [ ] Release asset: binary / DMG / SBOM / checksums が生成される

## リリースノートの必須記載

- 変更点（差分）
- 既知のデグレ・制限
- 未完了の機能漏れ（今後の予定）
- サポート対象 platform

## Homebrew tap（`Lamy210/homebrew-tap`）

SchneeForge の Homebrew formula は本体リポジトリ（この repo）ではなく `Lamy210/homebrew-tap` に置く。

- インストール: `brew tap Lamy210/homebrew-tap && brew install schneeforge`
- formula テンプレート（リリース時に URL/sha256 を差し替えて tap へ push）:

```ruby
class Schneeforge < Formula
  desc "Declarative Developer Workstation Manager (Nix + Home Manager + nix-darwin)"
  homepage "https://github.com/Lamy210/nix_setting"
  url "https://github.com/Lamy210/nix_setting/releases/download/v0.2.0-rc.1/schneeforge-aarch64-darwin"
  version "0.2.0-rc.1"
  sha256 "<release 時に sha256sum を記入>"
  license "MIT"

  def install
    bin.install "schneeforge-aarch64-darwin" => "schneeforge"
  end

  test do
    assert_match "Declarative Developer Workstation Manager", shell_output("#{bin}/schneeforge --help")
  end
end
```

## 現在のリリース状態

- 最新 release: `v0.1.0`（CLI/Nix 中心、GUI は未成熟）
- 次: `v0.2.0-rc.1`（GUI 正常化。`openspec/changes/gui-normalization/` 完了後に切る）
- `develop` 未リリース差分: `gui-normalization`（core 環境モデル / 操作の repo-aware 化 / State 永続化 / nh 非依存 bootstrap / 診断 API / desktop First Run Wizard）
