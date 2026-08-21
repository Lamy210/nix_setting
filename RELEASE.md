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

# 7. Homebrew tap を更新（下記「Homebrew tap 更新」参照）
```

## リリース前チェックリスト

リリース PR (`release/*`) を出す前にリリース担当者が全項目を確認する。CI 整合性を含む。

### 仕様・機能

- [ ] 差分確認: `git diff main..develop` でリリース対象を把握
- [ ] OpenSpec: `openspec validate --all` が全 spec valid
- [ ] 機能漏れ: 対象 change の `tasks.md` 未完了項目（`openspec archive` 前なら `openspec status --change <name>`）が意図済みか確認
- [ ] 既知のデグレ: `docs/STATUS.md` の「既知のデグレ・機能漏れ」が最新。release blocker が無いこと

### CI / 品質ゲート（全ジョブ green 必須）

- [ ] `openspec-check`: `openspec validate --all --no-interactive`
- [ ] `flake-check`: `nix flake check --allow-import-from-derivation`
- [ ] `flake-check`: `homeConfigurations.linux.activationPackage` が build できる
- [ ] `flake-check`: `homeConfigurations.linux-arm.activationPackage` が eval できる
- [ ] `macos-check`: `nix flake check` / `homeConfigurations.macbook-air` / `darwinConfigurations.macbook-air`
- [ ] `release-artifact-check`: release workflow と同一 script での macOS CLI build + smoke + portability gate・DMG mounted-app gate・release metadata 生成 script の自己検証 (RC.4 の `/nix/store` libiconv link 事故の再発防止)
- [ ] `managed-nix-e2e`: musl static build (release workflow と同一 script) + Docker 上での Managed Nix E2E (bats)
- [ ] `docker-check`: Docker image build / flake check / dev shell 起動
- [ ] `lint`: `actionlint` / `shellcheck` / `statix` / `deadnix`
- [ ] `lint`: **forbid raw tool spawns** — `tool.rs` / `cli/tests/` 以外で `Command::new("nix"|"git"|"brew"|"nh")` 等の文字列リテラル spawn が無いこと。shell 側も `$NIX_BIN` / `$GIT_BIN` / `$BREW_BIN` 経由であること
- [ ] `rust-check`: `cargo test` / `cargo fmt --check` / `cargo clippy -- -D warnings`
- [ ] `rust-check`: CLI artifact smoke (`schneeforge --version` / `schneeforge doctor`)
- [ ] `rust-check`: desktop build smoke (`apps/desktop/src-tauri`)
- [ ] `bootstrap-test`: `bats tests/bootstrap.bats` / `bats tests/resolve-tools.bats` / `nix-unit`
- [ ] `secret-scan`: `trufflehog filesystem . --only-verified` / image file scan
- [ ] `devshell-smoke`: `nix develop` (default / go / python / node / rust)
- [ ] `template-check`: 全 template (devenv/node/python/rust/flutter) の `nix flake metadata` / `nix develop`

### 手動 smoke

macOS 実機の full フローは [Final Acceptance 手順書](./docs/testing/macOS-final-acceptance-checklist.md)
(gate A-J。ADR-0001 昇格の条件。rc.6 以降は managed source の gate I を含む):

- [ ] 実機で `schneeforge setup` / `plan` / `apply` / `verify` / `rollback` を実行
- [ ] `schneeforge doctor` / `schneeforge status` が正常終了
- [ ] desktop (Tauri) で Diagnostics → Apply → Verify のフロー
- [ ] fresh 環境で `install.sh` が成功 (rc.6 以降は managed source 経路: clone なし)

### アセット・ノート

- [ ] リリースノート: 変更・既知の制限・未完了機能を記載
- [ ] Release asset: `schneeforge-{aarch64-darwin,x86_64-linux}` / DMG / SBOM / `schneeforge-release.json` (v2 §27 metadata。release workflow が tag ref から自動生成) / CHECKSUMS.txt が生成される
- [ ] `vX.Y.Z` の version 表記が `Cargo.toml` / `tauri.conf.json` / flake packages で揃っている
- [ ] **`install.sh` の `SCHNEEFORGE_BOOTSTRAP_VERSION` を今回の `vX.Y.Z` に bump**（release PR 内で実施。bootstrap が download する CLI version と pin する config ref (`SCHNEEFORGE_BOOTSTRAP_REF`, VERSION に連動。rc.6 以降の fresh install は managed source としてこの tag を pin する) が release と一致する保証になる。latest 任せにすると rc が拾われる）
- [ ] **README の Stable ワンライナー URL を今回の tag に差し替え**（`raw.githubusercontent.com/Lamy210/nix_setting/vX.Y.Z/install.sh`。Stable は「script 取得元 tag == pin 先 release」の一致が意味になるため、bump 忘れは test `install-sh.bats` の stable URL 検査で検知される）
- [ ] Linux asset が musl static であることを release note の CI log で確認（`verify static binary` step）

## リリースノートの必須記載

- 変更点（差分）
- 既知のデグレ・制限
- 未完了の機能漏れ（今後の予定）
- サポート対象 platform

## Homebrew tap（`Lamy210/homebrew-tap`）

SchneeForge の Homebrew formula は本体リポジトリ（この repo）ではなく `Lamy210/homebrew-tap` に置く。

### インストール

```bash
# Stable
brew tap Lamy210/homebrew-tap
brew install schneeforge

# Edge (main HEAD を local build)
brew install schneeforge-edge
```

### Homebrew tap 更新

リリース (`vX.Y.Z`) の tag push 後、GitHub Releases にアセットが揃ったら `Lamy210/homebrew-tap` を更新する。

1. **`Formula/schneeforge.rb` を開く**
2. **`version` / `url` / `sha256` を差し替え**（`on_arm` / `on_intel` セレクタ配下）
   - `sha256` はリリースアセット毎に `sha256sum schneeforge-<target>` で計算
3. **`brew audit --strict Formula/schneeforge.rb` を local で通す**
4. **PR を作って review → merge**

> Intel macOS (x86_64-darwin) アセットは未提供。`on_intel` ブロックの `od_unsupported` を外す前に `release.yml` の matrix 拡張が必要。

`Formula/schneeforge-edge.rb` は `head` 機能で `main` HEAD をローカルビルドするため、リリース時の更新は不要。

## 現在のリリース状態

- 最新 release: `v0.2.0-rc.5`（2026-08-15。rc.4 の DMG `/nix/store` link 事故の修正: DMG を host build 化 + `release-artifact-check` に mounted-app gate 追加）
- 次候補: `v0.2.0-rc.6`（macOS Final Acceptance PASS 後に cut。**managed release source (v2 §7) を同梱する最初の release** — fresh install が clone なしの managed source 経路になる。`schneeforge-release.json` asset の同梱もこの release から)
- `develop` 未リリース差分: 2026-08-18 以降に merge した v2 系 change 全て（MachineFacts / ConfigurationSource / manifest / profile / Release Metadata / GUI Dashboard / managed source 3 change ほか）。詳細は `docs/STATUS.md`
