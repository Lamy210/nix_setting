# STATUS（セッション引き継ぎ用）

現在の開発状態・既知のデグレ・機能漏れ・次の作業をまとめる。セッションを切り替えても、ここを読めば再開できる。

最終更新: 2026-09-16

## 完成済み

| 領域 | 内容 |
|------|------|
| Nix 基盤 | flake-parts / hosts / profiles / manifest / 3システム / CI 10+ジョブ |
| Rust core | actions / discovery / diagnostics / manifest / repo / state / time / tool / lock / operations / process / bootstrap / self_update（+ 303 unit tests） |
| CLI | 12 コマンド（core 委譲のみの adapter 化済み / `with_toolchain` wrapper） |
| Tauri GUI | 診断 Status + First Run Wizard + 非同期コマンド + CSP + 状態機械 + Plan/Verify ボタン + `fix-path-env-rs` による macOS PATH 補正 |
| OpenSpec | `gui-normalization` / runtime hardening / managed source / release supply-chain / development workflow hardening 等を archive 済み。active change は下記「進行中」を参照 |

### gui-normalization（63/63 完了・PR #4 merge 済み）

- **Phase 0**: spec 整合
- **Phase 1**: Platform/Architecture/ConfigurationTarget 分離、`Manifest::validate`、`ToolResolver`、`RepoResolver`、structured error
- **Phase 2**: atomic `StateStore`、クロスプロセス flock ロック、apply/rollback の State 永続化
- **Phase 3**: repo-aware 操作（plan/apply/verify/rollback/upgrade/sync）+ CLI の core 委譲
- **Phase 4**: preflight、pinned nix-darwin bootstrap、nh 非依存 apply、権限/backup 設計
- **Phase 5**: 診断 API（`Diagnostics` / tool path/version / manifest 検証）
- **Phase 6**: desktop 非同期コマンド、CSP、opener 削除
- **Phase 7**: First Run Wizard（clone/config 生成/plan/confirm/apply/verify/resume）
- **Phase 8**: GUI E2E 計画 + action mapping 静的クロスチェック
- **Phase 9**: CI smoke（CLI/desktop）、Homebrew tap 分離、README/RELEASE 同期

### 追加で対応済み（gui-normalization 後）

| 項目 | PR |
|------|----|
| install.sh / bootstrap.sh の username 個人化（#1） | #5 merge 済み |
| config.toml 生成の冪等化 + username 空ガード（#2/#3） | #9 |
| Ready 画面 Plan/Verify ボタン（#13） | #6 |
| uninstall の副作用排除（#10） | #7 |
| archive-before-pr のドキュメント修正（当時のプロセス改善。2026-09-16 の workflow hardening で後続運用へ更新） | #8 |

### runtime-tool-resolution-hardening（実装完了・merge 済み #11）

P0-1〜P0-5 を 1 change に統合して実装。PR #11 (squash) で develop へ merge 済み。

- **P0-1 ToolResolver 強化**: `ToolSource` / `ResolvedTool` / `Toolchain` / `ToolchainError` 追加。8 段階の探索優先度（env → PATH → XDG state → Nix profile 群 → per-user → system profile → Homebrew）。`canonicalize` で symlink 解決。
- **P0-2 全操作の Toolchain 経由化**: `actions` / `operations` / `bootstrap` の全関数が `&Toolchain` 受け。`process` 系は `&str` → `&Path` へ型変更。
- **P0-3 Nix Health Check**: `NixHealth` struct 追加。`nix store ping` / `nix config show experimental-features` で実環境検証。
- **P0-4 Flakes 検出バグ修正**: `PreflightReport` を `{ nix_installed, flakes_enabled, git_installed }` へ分離。`nix config show` を使って正確に判定。
- **P0-5 install.sh / bootstrap.sh 探索統一**: `scripts/resolve-tools.sh` 新設。Rust 側と同一の探索順序。`tests/resolve-tools.bats` (11 ケース) で回帰テスト。
- **前提**: `fix-path-env-rs` 追加（macOS の Finder/Spotlight 起動時の PATH 欠損を補正）。
- **CI 再発防止**: `lint` job に "forbid raw tool spawns" step 追加。`tool.rs` / `cli/tests/` 以外での文字列リテラル spawn を禁止。
- **GUI 側**: `CachedToolchain` (`Mutex<Option<Toolchain>>`) を `tauri::State` で保持。
- **未完了**: macOS desktop build + Finder launch 実機検証は macOS Final Acceptance に統合。

### docs/release-checklist-and-tap-sync（merge 済み #12）

RELEASE.md のリリースチェックリスト再構築 + weekly workflow の権限修正。

### v2 P0/P1 + Manifest + profile + Release Metadata + GUI Dashboard (2026-08-18 merge 済み)

2026-08-18 に develop へ squash merge 済み (PR #43-#52)。詳細は各 archive 済み change を参照。

- **v2 P0/P1 (PR #43/#44/#45)**: MachineFacts / ConfigurationSource / archive 整備
- **Distribution Manifest (PR #46/#47)**: `schneeforge.toml` (schema 1 / profiles / systems) で旧 config.toml 置換
- **profile 選択の flake 注入 (PR #48/#49)**: flake input `profile` + `modules/profile-input.nix`、CLI `profile list/set/clear/show`
- **Release Metadata (PR #50)**: `schneeforge-release.json` parse/validate/fetch + CLI `source metadata`
- **GUI Dashboard §28 (PR #52/#53)**: core `dashboard.rs` + desktop `get_dashboard` + Dashboard card

### managed source 3 change + 初期化経路 unified (2026-08-20 merge 済み, develop a0bc0c5)

CLI / install.sh / GUI wizard の初期化経路が unified (v2 §7 完結)。

- **GUI profile 切替 (PR #56 / 0fddc8f + archive #58)**: core `ProfileList` / `set_selection`、desktop profile 操作、Dashboard profile 切替 UI
- **HOME env race 修正 (PR #57 / d6cd751)**: MachineFacts detect の env 差し込みで test race を解消
- **install.sh fresh 経路の managed source 化 (PR #59 / 77a4d20 + archive #61)**: release asset CLI → source init --tag pin → apply
- **GUI wizard の managed source 対応 (PR #60 / 60e2846 + archive #61)**: managed source を既定経路へ追加

### GUI source 更新 + test 隔離 + ドキュメント整備 + provenance (2026-08-21 merge 済み, develop d1f049a)

PR #62-#68 を sequential chain で merge。

- **GUI からの source 更新 (PR #62 / f0a880b + archive #68)**: `run_update` Tauri command +「ソース更新」ボタン
- **cli test の state/network 隔離 (PR #63 / a6621e2)**: XDG_STATE_HOME 隔離 + local origin 化
- **docs (PR #64-#66)**: STATUS / Final Acceptance / RELEASE checklist 更新
- **release asset provenance (PR #67 / a1fb376 + archive #68)**: `actions/attest-build-provenance` を追加

### v0.2.0-rc.7 release (2026-08-22)

**rc.7 を release 済み** (PR #77 / main f6a1f33 / tag v0.2.0-rc.7)。

- rc.6 は `attestations: write` 不足で release workflow が失敗し、rc.7 へ切り直し (PR #76)
- asset 6 個の CHECKSUMS 一致、provenance attestation subject を確認
- managed release source / `schneeforge-release.json` / SLSA provenance / CLI self-update を初同梱

### CLI 自己更新 self-update (2026-08-22 merge 済み, develop 653fc1f)

- `schneeforge self-update`: channel latest tag 解決 → release binary download → checksum verify → atomic replace
- core は純関数と effect を分離し hermetic test 化
- GUI 内自己更新は別 change。Step 1 Releases link は PR #81 で実装済み

### rc.7 後 follow-up (2026-08-23〜08-30)

- Homebrew tap を rc.7 へ更新 (`homebrew-tap#1`)
- RELEASE.md の tap 節を実態へ修正 (PR #80)
- GUI 自己更新 Step 1 Releases link button (PR #81)
- self-update UpToDate 誤表示 fix (PR #84)
- release attestation bundles 実装 PR #86 → archive/spec sync PR #87

### GUI 自己更新 Step 2 の設計判断確定 (2026-08-23)

- **鍵管理**: GitHub Actions secret + offline backup
- **Linux GUI**: updater 対象外
- **latest.json**: release workflow 自前生成
- **タイミング**: v0.3 で別 change

### 開発ワークフロー hardening（2026-09-16 archive 済み）

- topic → `develop` は squash merge、`release/*` → `main` と `main` → `develop` back-merge は merge commit に統一
- OpenSpec archive は実装 PR merge 後の separate `chore/archive-*` PR に統一
- `main` / `develop` の既存 diverged history は rewrite/force push せず、今後の ancestry を merge commit で維持
- `AGENTS.md` / `CONTRIBUTING.md` / `RELEASE.md` / OpenSpec workflow を同期

### CI critical-path optimization（PR #90 最終検証）

- Rust required gate を `rust-quality` / `rust-build-smoke` の2 workerへ分割し、既存 required context `rust-check` は fail-closed aggregator として維持
- Linux desktop smoke は CLI sidecar と同じ release profile の `cargo check --release` に変更し、full DMG/Tauri build は required `release-artifact-check` に集約
- shadow `ci-required` を追加し、現行 required 7 contexts を fail-closed で集約（server-side branch protection は本 change では変更しない）
- Linux required `flake-check` は default `developer` profile を derivation evaluation、supported `minimal` profile を actual realization。product default は `developer` のまま
- baseline run #378: required critical path 479s / old `rust-check` 448s
- final measured run #397: current required critical path **327s（31.7%短縮）**、Rust runner total **443s**。目標 `<=359s` / `<=537.6s` をともに達成
- `flake-check` は 387s級のボトルネックから61sまで短縮。Terraform source build を毎PRのrequired Linux realizationから外しつつ、default developer evaluationは維持
- PR #90 merge 後は separate `chore/archive-refactor-ci-critical-path` PR で OpenSpec archive/spec sync を行う

## 進行中

| 項目 | 進捗 | 場所 |
|------|------|------|
| **CI critical-path optimization** | PR #90 最終CI / review待ち。性能目標は run #397 で達成済み。merge後に別archive PR | `openspec/changes/refactor-ci-critical-path/` |
| DMG offline bundle 法務 ADR (issue #17) | ADR-0002 起票済み。実装は弁護士確認後 | `openspec/changes/add-dmg-offline-bundle-licensing/` |
| macOS Apple Silicon Final Acceptance | rc.7 での実機 acceptance 未完了 | `docs/testing/macOS-final-acceptance-checklist.md` |

## 既知のデグレ・機能漏れ（要対応）

### 高（Release Blocker）

現時点でコード上の既知 release blocker は無し。macOS Final Acceptance は release readiness gate として未完了。

### 中

| # | 問題 | 対応 |
|---|------|------|
| — | Dependabot alert #2: glib 0.18.5 (GHSA-wrw7-89jp-8q8g / RUSTSEC-2024-0429, medium) | Tauri v2 Linux → GTK3 0.18 → glib 0.18.5 経由。upstream stable release を monitor し、gtk3-rs stable 更新後に再評価。dismiss せず tracking 継続 |
| — | `main` と `develop` の Git history が diverge | **history rewrite しない**。次回 release から `release/*`→`main` と `main`→`develop` を merge commit 固定し、以後の ancestry を維持 |

### 低

| # | 問題 | 対応 |
|---|------|------|
| — | バージョン文字列の同期（現在 `0.2.0-rc.7`） | 次回 release で RELEASE.md checklist に従い同期 |
| — | Intel macOS release asset 未提供 | `add-x86_64-darwin-support` を Windows/macOS compatibility 基盤後に検討 |

## 次の作業（推奨順）

1. **PR #90 `refactor-ci-critical-path` を完了**
   - latest head で OpenSpec strict / lint / Rust / flake / release artifact / E2E を再度 green にする
   - final diff review 後に `develop` へ squash merge
   - merge 後、`chore/archive-refactor-ci-critical-path` PR で archive + spec sync
2. **`add-macos-compatibility-matrix`**
   - macOS 15 + 対応可能な Xcode version
   - macOS 26 + Xcode 26.6
   - `macos-latest` 依存を減らし OS/Xcode を明示
   - Flutter/iOS doctor に Xcode / SDK / simulator runtime diagnostics を追加検討
3. **`add-windows-wsl2-platform`**
   - Windows host と Nix execution backend を分離
   - WSL2 Linux を Nix execution target とする
   - Windows compile portability audit (`std::env::split_paths`, Unix-only paths/permissions/locking 等)
   - Windows-native package manager backend は初期 scope 外
4. **macOS Apple Silicon Final Acceptance**
   - rc.7 を使い `docs/testing/macOS-final-acceptance-checklist.md` gate A-J を実施
5. Phase 2/E 残作業
   - GUI self-update Step 2 (v0.3)
   - #17 DMG bundle + LGPL-2.1 法務確認後の実装

※ issue #14/#15 は close 済み。#16 は Final Acceptance 状況を確認、#17 は弁護士確認後に close。

## 開発フロー

- Topic branch: `develop` → `feat|fix|refactor|docs|test|chore/*` → PR → `develop` (**squash merge**)
- Release: `develop` → `release/vX.Y.Z` → PR → `main` (**merge commit**) → tag → release workflow
- Back-merge: `main` → PR → `develop` (**merge commit**)。squash/rebase/force rewrite は使わない
- OpenSpec: change 作成 → proposal/design/spec/tasks → `openspec validate <id> --strict` → proposal approval → 実装 → `openspec validate --all --strict` → 実装 PR merge → `chore/archive-<id>` で archive + spec sync PR
- 品質ゲート: `cargo test` / `clippy` / `fmt` / `nix flake check` / OpenSpec strict validation
- branch protection required checks の変更は段階移行。新 aggregator を既存 required contexts と並行稼働させてから切り替える
- 手動 CLI 実行時は `XDG_STATE_HOME` を temp に向け、実 state を汚染しない
- 詳細: [CONTRIBUTING.md](../CONTRIBUTING.md) / [RELEASE.md](../RELEASE.md)
