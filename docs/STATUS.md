# STATUS（セッション引き継ぎ用）

現在の開発状態・既知のデグレ・機能漏れ・次の作業をまとめる。セッションを切り替えても、ここを読めば再開できる。

最終更新: 2026-09-21

## 完成済み

| 領域 | 内容 |
|------|------|
| Nix 基盤 | flake-parts / hosts / profiles / manifest / 3システム / CI 10+ジョブ |
| Rust core | actions / discovery / execution / diagnostics / manifest / repo / state / time / tool / lock / operations / process / bootstrap / self_update |
| CLI | native macOS/Linux adapter + experimental Windows launcher / WSL2 delegation |
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

### CI critical-path optimization（2026-09-16 archive 済み）

- Rust required gate を `rust-quality` / `rust-build-smoke` の2 workerへ分割し、既存 required context `rust-check` は fail-closed aggregator として維持
- Linux desktop smoke は CLI sidecar と同じ release profile の `cargo check --release` に変更し、full DMG/Tauri build は required `release-artifact-check` に集約
- shadow `ci-required` を追加し、現行 required 7 contexts を fail-closed で集約（server-side branch protection は本 change では変更しない）
- Linux required `flake-check` は default `developer` profile を derivation evaluation、supported `minimal` profile を actual realization。product default は `developer` のまま
- baseline run #378: required critical path 479s / old `rust-check` 448s
- final measured run #397: current required critical path **327s（31.7%短縮）**、Rust runner total **443s**。目標 `<=359s` / `<=537.6s` をともに達成
- `flake-check` は 387s級のボトルネックから61sまで短縮。Terraform source build を毎PRのrequired Linux realizationから外しつつ、default developer evaluationは維持
- 実装 PR #90 と archive/spec-sync PR #92 はともに squash merge 済み

### macOS compatibility matrix（2026-09-16 archive 済み）

- stable compatibility lane: `macos-15` + Xcode 26.3、stable current/shipping lane: `macos-26` + Xcode 26.6
- `macos-check` は stable 2 laneをfail-closedで集約し、既存required contextsは変更していない
- `release-artifact-check` とtag releaseも `macos-26` + Xcode 26.6へpin
- Xcode 27はPR外のpreview canaryとして分離し、required/release dependencyには入れていない
- hosted runner が `macos-15-arm64` / `macos-26-arm64` であることも実runで確認済み
- latest-head run #428でrequired 7 contexts、`ci-required`、release artifact、stable 2 lane、`macos-check`が全てgreen
- old single-lane baseline 544sに対しstable 2 lane totalは約1227s（約2.26x）で、2.5x guard 1360s以内
- 実装 PR #93 と archive/spec-sync PR #94 はともに squash merge 済み

### Windows / WSL2 experimental backend（2026-09-18 merge / archive 済み）

- PR #96 で native Windows launcher + WSL2 delegation を実装し develop へ merge
- PR #98 で `add-windows-wsl2-platform` を archive + canonical spec sync
- PR #99 で Windows checkout 時の CRLF を `.gitattributes` で修正
- PR #100 で WSL delegation を `wsl.exe --exec` へ修正し shell-free argv contract を実境界で成立
- PR #101/#102 で real WSL2 canary harness と静的 contract を hardening
- real WSL2 canary は explicit selector / env selector / literal argv / delegated exit code を実 Windows→WSL2 で green 確認済み
- Windows release asset / coordinated self-update は initial scope 外で、experimental source-build 段階を維持


### macOS Managed Nix release lifecycle helper（2026-09-19 merge / real run 済み）

- PR #105 で manual-only / non-required の `macos-managed-nix-lifecycle.yml` と release-artifact lifecycle verifier を追加
- PR #109 で hosted macOS の synthetic `/nix` cleanup 判定を runtime/state remnants 基準へ修正
- PR #111 で flakes smoke を self-contained local flake に変更し、unauthenticated GitHub API rate limit 依存を除去
- production `develop@6bae1c21` から `v0.2.0-rc.7` を指定した workflow run #5 (`35435044077`) が success
- `macos-15-arm64` fresh host で release CLI checksum → Managed Nix install → receipt/ownership → store/local flake → doctor → `ExistingNixDetected` → uninstall → reinstall → final cleanup を実境界で確認
- これは CLI lifecycle の自動 evidence であり、Finder GUI / `install.sh` D8 / nix-darwin full bootstrap の manual Final Acceptance を置き換えない

### GUI self-update Step 2（2026-09-21 実装 merge 済み / production activation 未実施）

- PR #113 で macOS aarch64 向け Tauri 2 signed updater の実装準備を `develop` へ merge
- backend が pending update を保持し、`fetch_app_update` / `install_app_update` / progress event / restart を提供
- Dashboard は updater capability が有効な build のみ自動更新 button を表示し、GitHub Releases link を fallback として維持
- release pipeline は `SCHNEEFORGE_UPDATER_ACTIVATED=true` の場合だけ signed `.app.tar.gz` / `.sig` / `latest.json` を生成する staged activation
- updater signature は app version と暗号学的にbindし、runtime は signed version と endpoint announced version の一致を必須化する（Tauri CLI 2.11.5+ / `requireSignedVersion=true`）
- production signing private key は Tauri build step のみに scope し、通常 PR/develop build は secret-free / updater-disabled
- **未完了**: macOS manual Final Acceptance、production key pair/secret/public-key provision、signed N→N+1 E2E、tampered artifact signature mismatch E2E
- 上記 activation gate 完了までは production updater を有効化せず、placeholder/test trust root は shipping しない

## 進行中

| 項目 | 進捗 | 場所 |
|------|------|------|
| macOS Apple Silicon Final Acceptance | rc.7 の CLI lifecycle 自動化後も Finder GUI / install.sh 対話 / full bootstrap manual gate は未完了 | `docs/testing/macOS-final-acceptance-checklist.md` |
| GUI self-update Step 2 activation | 実装は merge 済み。production key provision / Final Acceptance / N→N+1・tamper E2E 完了まで disabled | `RELEASE.md`, `openspec/changes/add-gui-self-update-step2/` |
| DMG offline bundle 法務 ADR (issue #17) | ADR-0002 / OpenSpec は archive 済み。binary bundle / offline install 実装は弁護士確認後 | `docs/adr/0002-dmg-bundle-lgpl-redistribution.md`, `openspec/changes/archive/2026-09-17-add-dmg-offline-bundle-licensing/` |

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
| — | Windows release asset / coordinated self-update 未提供 | PR #96 initial scope外。launcher/helperを同一versionで手動用意するexperimental source-build段階。distribution/update設計をfollow-up changeで行う |
| — | バージョン文字列の同期（現在 `0.2.0-rc.7`） | 次回 release で RELEASE.md checklist に従い同期 |
| — | Intel macOS release asset 未提供 | `add-x86_64-darwin-support` を Windows/macOS compatibility 基盤後に検討 |

## 次の作業（推奨順）

1. **macOS Apple Silicon Final Acceptance の残り manual gate**
   - Finder pre-bootstrap / post-bootstrap GUI smoke
   - `install.sh` D8 `/dev/tty` 経路
   - nix-darwin apply 済み full uninstall ordering
   - 全 gate 完了後にのみ ADR-0001 を `Accepted` へ昇格
2. **GUI self-update Step 2 production activation**
   - Final Acceptance PASS 後に production updater key pair を生成し、private key/password を Actions secret + offline backupへ保管
   - public key を review して production config に固定
   - signed N→N+1 updater E2E と tampered artifact signature mismatch E2E を通してから `SCHNEEFORGE_UPDATER_ACTIVATED=true`
3. Phase 2/E follow-up
   - Windows release asset / coordinated launcher-helper update設計
   - Intel macOS release asset の検討
4. **issue #17**
   - LGPL-2.1 再配布条件の弁護士確認後に DMG offline bundle 実装を開始

※ issue #14/#15/#16/#91 は close 済み。#17 は法務確認待ち。

## 開発フロー

- Topic branch: `develop` → `feat|fix|refactor|docs|test|chore/*` → PR → `develop` (**squash merge**)
- Release: `develop` → `release/vX.Y.Z` → PR → `main` (**merge commit**) → tag → release workflow
- Back-merge: `main` → PR → `develop` (**merge commit**)。squash/rebase/force rewrite は使わない
- OpenSpec: change 作成 → proposal/design/spec/tasks → `openspec validate <id> --strict` → proposal approval → 実装 → `openspec validate --all --strict` → 実装 PR merge → `chore/archive-<id>` で archive + spec sync PR
- 品質ゲート: `cargo test` / `clippy` / `fmt` / `nix flake check` / OpenSpec strict validation
- Windows backend: `windows-2025` non-required laneでnative launcher compile/contract/smokeを確認し、Nix operational behaviorはWSL/Linux execution-sideで保持する
- branch protection required checks の変更は段階移行。新 aggregator を既存 required contexts と並行稼働させてから切り替える
- 手動 CLI 実行時は `XDG_STATE_HOME` を temp に向け、実 state を汚染しない
- 詳細: [CONTRIBUTING.md](../CONTRIBUTING.md) / [RELEASE.md](../RELEASE.md) / [Windows / WSL2 guide](./windows-wsl2.md)
