# RELEASE

SchneeForge のリリース運用。`main`（リリースブランチ）への直接 push / 直接 merge は禁止し、必ず `release/*` ブランチからの PR を経由する。

## Apple Developer ID 署名・公証なしの配布（Preview / 開発向け）

このリポジトリは、**Apple Developer Program / Apple ID の資格情報を使用しない配布**を選択できる。
これは macOS の **Developer ID 署名・Apple 公証（notarization）を行わない**という意味であり、セキュリティ検証全体を省略する意味ではない。

- 配布対象は現行の Apple Silicon (`aarch64-darwin`) CLI / DMG と Linux (`x86_64-linux-musl`) CLI。Intel Mac 向けバイナリや Windows インストーラーを提供しているとは主張しない。
- `.github/workflows/release.yml` の通常ビルドは Apple の署名用証明書・Apple ID・公証資格情報を要求しない。Tauri / macOS のツールによる ad-hoc 署名の有無は完成品で確認し、Developer ID 署名済みと誤認しない。
- **GUI 自動更新は無効のまま配布する**。リポジトリ変数 `SCHNEEFORGE_UPDATER_ACTIVATED` が未設定または `false` であることを release 前に確認する。コード内の `requireSignedVersion=true` は弱めない。Tauri updater 用署名鍵は Apple Developer ID とは別物だが、未承認の activation はしない。通常の DMG と CLI の手動ダウンロード導線を維持する。
- Apple 未公証のダウンロードアプリは Gatekeeper によって起動を拒否または警告され得る。**macOS の保護機能を全体無効化したり、quarantine を自動削除したりしない**。Finder 起動での挙動を対象 macOS 実機で記録し、制限をリリースノートで明示する。CLI と DMG の CI ビルド成功は Finder 受入を代替しない。
- `CHECKSUMS.txt`、SBOM、GitHub build provenance、cosign keyless attestation / identity pin と各種 portability gate は引き続き必須。これらは Apple の公証の代替ではない。また `install.sh` の初回取得を、同じ HTTPS 経路で得た checksum のみで独立認証できるわけではない（[#268](https://github.com/Lamy210/nix_setting/issues/268)）。
- `v0.2.0-rc.8` の **タグ作成前ゲート**は Node.js 24 P0 修正・実行時依存の検証、Managed Nix の安全性、Updater 無効化、release PR / release-artifact / macOS CI および supply-chain 成果物の生成・検証計画。いずれか未達ならタグを作成しない（[#271](https://github.com/Lamy210/nix_setting/issues/271)）。**macOS Final Acceptance A–J は実際のタグ付き配布アセットが生成された後のゲート**であり、正式承認や安定版への昇格条件として扱う。

未署名プレビューの release notes には必ず「Apple Developer ID 未署名・Apple 公証なし」「Gatekeeper 制約」「GUI 自動更新無効」「手動インストール可能なプラットフォーム」を記載する。

## Release Candidate の二段階ゲート（循環依存の解消）

現行の `.github/workflows/release.yml` は `v*` tag push 時にのみ配布 CLI / DMG / checksum / provenance を生成する。macOS Final Acceptance A–J は **そのタグから作成された正規アセット**を利用するため、**RC の公開前ゲート**と**RC 公開後の昇格ゲート**を分離する。公開される RC は検証途中の *provisional preview* であって、品質保証済み stable ではない。

1. **Tag 前（公開許可の条件）**: root + templates の Node 24 P0 と Brotli/c-ares 依存閉包の修正証拠、Managed Nix provider の新規導入リスクの確認、必須 CI、macOS CLI / DMG の portability / artifact CI、release workflow・SBOM・attestation の必要権限と手順を検証する。Apple Developer ID 署名・公証は使わない。アップデーターは無効とし、`SCHNEEFORGE_UPDATER_ACTIVATED` の repository variable が未設定または `false` と確認する。**未達なら Release Candidate tag 自体を作らない。**
2. **Provisional RC 公開（必要な事前ゲートをすべて通過した場合だけ）**: `release/*` → `main` を merge commit で取り込み、固定した release commit に RC tag を打つ。公開される GitHub Release は prerelease とし、**「A–J 未受入・テスト用・重要な Mac には導入しない」**および未署名・未公証・Gatekeeper 制約・GUI updater 無効を release notes の冒頭に記載する。公開 GitHub Release を作ることは検証前アセットが第三者に見えることを意味するため、リリース担当者の明示的な公開判断を必要とする。
3. **同一 RC で Final Acceptance**: tag / commit SHA、CLI / DMG / checksums / attestations を照合し、使い捨ての Apple Silicon Mac で `docs/testing/macOS-final-acceptance-checklist.md` の A–J（Finder / `install.sh` / `/dev/tty` / nix-darwin cleanup・reinstall 等）を実施して証跡を保存する。ホストのインストール状態と公証状態を推定せず、未確認は未確認とする。
4. **昇格 / 不合格**: 全ゲート PASS の場合のみ ADR-0001 を `Accepted` へ昇格し、安定版・正式な導入推奨を別途決定する。FAIL の場合は RC の失敗と導入不可を公開ノートへ記録し、修正した **新しい RC tag** で再検証する。既存タグの移動、アセットの黙った差し替え、ゲートの省略は禁止する。公開済み RC を正式版として扱わない。

**注意:** Tag 前に A–J を必須とする別の運用へ移行する場合は、tag 不要のリリース候補ビルド・配布と、公開する最終アセットとの同一性検証が可能な独立 workflow を先に実装・検証する必要がある。現行 workflow にはその経路が無い。

## ブランチ制約

```
main        直接 push / 直接 merge 禁止（ブランチプロテクションで enforce）
release/*   唯一 main へ merge できる経路
develop     開発統合。topic branch の merge 先
```

- `main` への merge は `release/*` ブランチからの PR のみ
- feature ブランチを `main` へ直接 PR しない
- `release/*` → `main` は **merge commit** を使用する（squash / rebase 禁止）
- release 後の `main` → `develop` back-merge も **merge commit** を使用する（squash / rebase 禁止）
- back-merge は `main` 自体を head にした PR とし、`chore/*` topic branch に複製しない
- 過去の diverged history を直す目的で force push / history rewrite を行わない

## リリースフロー

```bash
# 1. develop のリリース準備が整ったら release ブランチを切る
git checkout develop && git pull
git checkout -b release/vX.Y.Z

# 2. release ブランチで最終検証（チェックリスト参照）

# 3. main へ PR（release ブランチから）
gh pr create --base main --head release/vX.Y.Z --title "release: vX.Y.Z"
# review + required checks green 後、merge commit で統合する

# 4. main の release merge commit に tag を打つ
git checkout main && git pull
git tag -a vX.Y.Z -m "SchneeForge vX.Y.Z"
git push origin vX.Y.Z    # この push が release workflow を発火

# 5. main 自体を head にして develop へ back-merge PR（merge commit 固定）
gh pr create --base develop --head main --title "chore: back-merge vX.Y.Z"
# review + required checks green 後、merge commit で統合する

# 6. release branch を削除
git push origin --delete release/vX.Y.Z

# 7. Homebrew tap を更新（下記「Homebrew tap 更新」参照）
```

> `main` と `develop` が過去履歴で diverge していても history rewrite はしない。次回 release/back-merge から merge commit を維持し、以後の ancestry を安定させる。

## リリース前チェックリスト

リリース PR (`release/*`) を出す前に **Tag 前ゲート**を確認する。下の「リリース後の手動 smoke」は公開された RC の **正式昇格判定ゲート**であり、タグ作成前に完了したと誤って記録しない。CI 整合性を含む。

### 仕様・機能

- [ ] 差分確認: `git diff main..develop` でリリース対象を把握
- [ ] OpenSpec: `openspec validate --all --strict` が全 spec valid
- [ ] 機能漏れ: 対象 change の `tasks.md` 未完了項目が意図済みか確認
- [ ] OpenSpec archive: 実装 PR merge 済み change は archive PR も merge 済みで、`openspec/specs` が deployed truth と一致している
- [ ] 既知のデグレ: `docs/STATUS.md` の「既知のデグレ・機能漏れ」が最新。release blocker が無いこと

### CI / 品質ゲート（全ジョブ green 必須）

- [ ] `openspec-check`: `openspec validate --all --strict --no-interactive` が成功
- [ ] `flake-check`: `nix flake check --allow-import-from-derivation`
- [ ] `flake-check`: `homeConfigurations.linux.activationPackage` が build できる
- [ ] `flake-check`: `homeConfigurations.linux-arm.activationPackage` が eval できる
- [ ] `macos-check`: `nix flake check` / macOS Home Manager / nix-darwin system build
- [ ] `release-artifact-check`: release workflow と同一 script での macOS CLI build + smoke + portability gate・DMG mounted-app gate・release metadata 生成 script の自己検証
- [ ] `managed-nix-e2e`: musl static build (release workflow と同一 script) + Docker 上での Managed Nix E2E (bats)
- [ ] `docker-check`: Docker image build / flake check / dev shell 起動
- [ ] `lint`: `actionlint` / `shellcheck` / `statix` / `deadnix`
- [ ] `lint`: **forbid raw tool spawns** — `tool.rs` / `cli/tests/` 以外で `Command::new("nix"|"git"|"brew"|"nh")` 等の文字列リテラル spawn が無いこと。shell 側も `$NIX_BIN` / `$GIT_BIN` / `$BREW_BIN` 経由であること
- [ ] `rust-check`: `cargo test` / `cargo fmt --check` / `cargo clippy -- -D warnings`
- [ ] `rust-check`: CLI artifact smoke (`schneeforge --version` / `schneeforge doctor`)
- [ ] `rust-check`: desktop build smoke (`apps/desktop/src-tauri`)
- [ ] `bootstrap-test`: bootstrap/install/resolve-tools/managed-nix contract/ci-scripts bats + `nix-unit`
- [ ] `secret-scan`: `trufflehog filesystem . --only-verified` / image file scan
- [ ] `devshell-smoke`: `nix develop` (default / go / python / node / rust)
- [ ] `template-check`: 全 template (devenv/node/python/rust/flutter) の `nix flake metadata` / `nix develop`

> branch protection の required contexts は一度に置き換えない。follow-up で `ci-required` aggregator を導入する場合、既存 required checks と並行稼働させ、複数 PR の成功実績を確認してから protection 設定を移行する。

### リリース後の手動 smoke（RC の受入 / 正式昇格前）

以下のチェックは公開済み RC の完成アセットを用いた受入項目であり、tag push 前のチェックではない。未完了の間は RC を provisional のままとする。

macOS 実機の full フローは [Final Acceptance 手順書](./docs/testing/macOS-final-acceptance-checklist.md)
(gate A-J。ADR-0001 昇格の条件。rc.6 以降は managed source の gate I を含む):

- [ ] 実機で `schneeforge setup` / `plan` / `apply` / `verify` / `rollback` を実行
- [ ] `schneeforge doctor` / `schneeforge status` が正常終了
- [ ] desktop (Tauri) で Diagnostics → Apply → Verify のフロー
- [ ] fresh 環境で `install.sh` が成功 (rc.6 以降は managed source 経路: clone なし)

### GUI self-update Step 2 activation（macOS aarch64）

GUI updater のコードは production trust root を入れずに先行 merge できる。
**以下を全て満たすまで updater を production activation しない。**

- [ ] macOS Final Acceptance (Finder GUI / install.sh / full bootstrap を含む) が PASS
- [ ] production Tauri updater key pair を release 担当者が生成
- [ ] private key を GitHub Actions secret `TAURI_SIGNING_PRIVATE_KEY` に登録
- [ ] encrypted key の場合は password を `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` に登録
- [ ] private key / password の offline backup を repository 外の安全な保管先へ保存
- [ ] public key を repository variable `SCHNEEFORGE_UPDATER_PUBKEY` に登録し review
- [ ] updater runtime が `requireSignedVersion=true` で、release build が Tauri CLI 2.11.5+ の version-bound signature を生成することを確認
- [ ] 上記完了後にのみ repository variable `SCHNEEFORGE_UPDATER_ACTIVATED=true` を設定
- [ ] tag release が `.app.tar.gz` / `.app.tar.gz.sig` / `latest.json` を生成
- [ ] version N のDMGから signed N+1 をcheck → download/install → restartし N+1 を確認
- [ ] artifactを改変した negative testで signature verification failureとなり、既存appが維持されることを確認

通常のPR / develop buildでは `SCHNEEFORGE_UPDATER_ACTIVATED` は false のままとし、
production signing secretを要求しない。updater disabled buildではDashboardの
自動更新buttonを表示せず、既存のGitHub Releases linkをfallbackとして維持する。

Key rotation時は、旧keyで署名したtransition releaseへ**新public keyを先に埋め込み**、
そのreleaseが十分に配布されるまで旧private keyを破棄しない。その後のreleaseから
新private keyで署名する。private keyをrepoへcommitしてはならない。

### アセット・ノート

- [ ] リリースノート: 変更・既知の制限・未完了機能を記載
- [ ] Release asset: `schneeforge-{aarch64-darwin,x86_64-linux}` / DMG / SBOM / `schneeforge-release.json` / CHECKSUMS.txt が生成される
- [ ] GUI updater activation時のみ: `.app.tar.gz` / `.app.tar.gz.sig` / `latest.json` が生成され、CHECKSUMS / provenance対象に含まれる
- [ ] `vX.Y.Z` の version 表記が `Cargo.toml` / `tauri.conf.json` / flake packages で揃っている
- [ ] **`install.sh` の `SCHNEEFORGE_BOOTSTRAP_VERSION` を今回の `vX.Y.Z` に bump**
- [ ] **README の Stable ワンライナー URL を今回の tag に差し替え**（`raw.githubusercontent.com/Lamy210/nix_setting/vX.Y.Z/install.sh`）
- [ ] Linux asset が musl static であることを release note の CI log で確認
- [ ] provenance: `attest build provenance` step が asset 全てを attest した
- [ ] attestation bundle: `generate attestation bundles` step が配布 artifact ごとに `<asset>.sig.bundle` / `<asset>.provenance.bundle`（CLI は `<asset>.spdx.json` も）を生成し identity pin 自己検証 gate を通過した

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

# Edge (main HEAD を local build) — edge formula は未提供
# brew install schneeforge-edge
```

### Homebrew tap 更新

リリース (`vX.Y.Z`) の tag push 後、GitHub Releases にアセットが揃ったら `Lamy210/homebrew-tap` を更新する。

1. **repo root の `schneeforge.rb` を開く**（tap repo に `Formula/` ディレクトリは無く、formula は root 直下に置く運用）
2. **`version` / `url` / `sha256` を差し替え**（現行 formula は aarch64-darwin アセット 1 本のみで、`on_arm` / `on_intel` セレクタは無い）
   - `sha256` はリリースアセット毎に `sha256sum schneeforge-<target>` で計算（release の `CHECKSUMS.txt` と突合してもよい）
3. **`brew audit --strict schneeforge.rb` を local で通す**
4. **PR を作って review → merge**

> Intel macOS (x86_64-darwin) アセットは未提供。`on_intel` ブロックを足す前に `release.yml` の matrix 拡張が必要。

edge 用 formula（`schneeforge-edge.rb`、`head` 機能で `main` HEAD をローカルビルド）は未提供。

## 現在のリリース状態

- 最新 release: `v0.2.0-rc.7`（2026-08-22。rc.6 は release job の `attest build provenance` が `attestations: write` 権限不足で失敗したため tag 削除のうえ切り直し。managed release source / `schneeforge-release.json` / SLSA provenance / CLI self-update を同梱する最初の release）
- 次候補: `v0.2.0-rc.8`（計画中。**未公開**。詳細は [Issue #271](https://github.com/Lamy210/nix_setting/issues/271)。セキュリティ P0 と公開前 CI / release gate が完了するまで RC tag を作成しない。タグ公開後は Final Acceptance A–J が合格するまで provisional とし、stable・正式受入へ昇格しない）
- `develop` 未リリース差分: rc.7 後の follow-up + workflow hardening。詳細は `docs/STATUS.md`
- GitHub `releases/latest` は現時点で `v0.2.0-rc.1` を返す（過去の rc.1 が `prerelease=false` として登録されているため）。実際の最新公開 prerelease は `v0.2.0-rc.7`。release API の channel 表示は [#271](https://github.com/Lamy210/nix_setting/issues/271) で修正・再検証する。tag の移動・履歴書き換えはしない。
