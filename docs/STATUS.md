# STATUS（セッション引き継ぎ用）

現在の開発状態・既知のデグレ・機能漏れ・次の作業をまとめる。セッションを切り替えても、ここを読めば再開できる。

最終更新: 2026-08-20

## 完成済み

| 領域 | 内容 |
|------|------|
| Nix 基盤 | flake-parts / hosts / profiles / manifest / 3システム / CI 10+ジョブ |
| Rust core | actions / discovery / diagnostics / manifest / repo / state / time / tool / lock / operations / process / bootstrap（+ 98 unit tests） |
| CLI | 11 コマンド（core 委譲のみの adapter 化済み / `with_toolchain` wrapper） |
| Tauri GUI | 診断 Status + First Run Wizard + 非同期コマンド + CSP + 状態機械 + Plan/Verify ボタン + `fix-path-env-rs` による macOS PATH 補正 |
| OpenSpec | `gui-normalization` 63/63 merge 済み・アーカイブ済み / `runtime-tool-resolution-hardening` 実装完了（archive 待ち）/ main specs 5件 |

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
| archive-before-pr のドキュメント修正（プロセス改善） | #8 |

### runtime-tool-resolution-hardening（実装完了・merge 済み #11）

P0-1〜P0-5 を 1 change に統合して実装。PR #11 (squash) で develop へ merge 済み。

- **P0-1 ToolResolver 強化**: `ToolSource` / `ResolvedTool` / `Toolchain` / `ToolchainError` 追加。8 段階の探索優先度（env → PATH → XDG state → Nix profile 群 → per-user → system profile → Homebrew）。`canonicalize` で symlink 解決。
- **P0-2 全操作の Toolchain 経由化**: `actions` / `operations` / `bootstrap` の全関数が `&Toolchain` 受け。`process` 系は `&str` → `&Path` へ型変更。
- **P0-3 Nix Health Check**: `NixHealth` struct 追加。`nix store ping` / `nix config show experimental-features` で実環境検証。
- **P0-4 Flakes 検出バグ修正**: `PreflightReport` を `{ nix_installed, flakes_enabled, git_installed }` へ分離。`nix config show` を使って正確に判定。
- **P0-5 install.sh / bootstrap.sh 探索統一**: `scripts/resolve-tools.sh` 新設。Rust 側と同一の探索順序。`tests/resolve-tools.bats` (11 ケース) で回帰テスト。
- **前提**: `fix-path-env-rs` 追加（macOS の Finder/Spotlight 起動時の PATH 欠損を補正）。
- **CI 再発防止**: `lint` job に "forbid raw tool spawns" step 追加。`tool.rs` / `cli/tests/` 以外での文字列リテラル spawn を禁止。
- **GUI 側**: `CachedToolchain` (`Mutex<Option<Toolchain>>`) を `tauri::State` で保持。フロントエンド型定義の更新（`NixHealth` / `ToolchainSummary` 表示）は後続の GUI P1 変更で対応。
- **未完了**: macOS desktop build + Finder launch 実機検証 (CI の macos-check は green だが実機 smoke は別途)。→ 「次の作業」1 の macOS Apple Silicon Final Acceptance に統合済み。

### docs/release-checklist-and-tap-sync（merge 済み #12）

RELEASE.md のリリースチェックリスト再構築 + weekly workflow の権限修正。

### v2 P0/P1 + Manifest + profile + Release Metadata + GUI Dashboard (2026-08-18 merge 済み)

2026-08-18 に develop へ squash merge 済み (PR #43-#52)。詳細は各 archive 済み change を参照。

- **v2 P0/P1 (PR #43/#44/#45)**: MachineFacts (repo を書かない machine input 生成) / ConfigurationSource (ADR-0003 Accepted, SourceKind 5 種 + update dispatch) / archive 整備
- **Distribution Manifest (PR #46/#47)**: `schneeforge.toml` (schema 1 / profiles / systems) で旧 config.toml 置換
- **profile 選択の flake 注入 (PR #48/#49)**: flake input `profile` + `modules/profile-input.nix`、CLI `profile list/set/clear/show`。file 指す path input の override は `path:<abs>` URL 形式が必須 (nix 2.35)
- **Release Metadata (PR #50)**: `schneeforge-release.json` (schema 1) の parse/validate/fetch + CLI `source metadata`。metadata asset は次回 release (rc.6 以降) から同梱
- **GUI Dashboard §28 (PR #52/#53)**: core `dashboard.rs` (snapshot + version 比較純関数) + desktop `get_dashboard` + Dashboard card。network は引数差し込みで hermetic

### managed source 3 change + 初期化経路 unified (2026-08-20 merge 済み, develop a0bc0c5)

CLI / install.sh / GUI wizard の初期化経路が unified (v2 §7 完結)。詳細は各 archive 済み change を参照。

- **GUI profile 切替 (PR #56 / 0fddc8f + archive #58)**: core `ProfileList` / `set_selection`、desktop `get_profiles`/`set_profile`/`clear_profile`、Dashboard の profile 切替 UI (選択 → 適用 / 既定へ)
- **HOME env race 修正 (PR #57 / d6cd751)**: `MachineFacts::detect_with_home_from(|k| std::env::var_os(k))` で env 差し込み可能にし、detect 中の HOME 読みを hermetic 化
- **install.sh fresh 経路の managed source 化 (PR #59 / 77a4d20 + archive #61)**: 初回 install での git clone を廃止。`fetch_schneeforge_binary` (release asset 取得) → `source init --tag pin` → `apply`。dotfile backup、`ManagedNix::load_prefer_repo` (repo 無し時は embedded fallback)
- **GUI wizard の managed source 対応 (PR #60 / 60e2846 + archive #61)**: `Diagnostics.managed_source`、wizard の source 選択 step (managed 既定 + clone 選択肢)、boot gate `!repo_exists && !managed_source`、Managed Nix install の repo gate 削除

## 進行中

| 項目 | 進捗 | 場所 |
|------|------|------|
| **v2 §7 Managed Release Source** (working tree-less) | **develop merge 済み** (PR #54 / 00d9b98)。Release source の表現に flake ref `github:<owner>/<repo>/<tag>` を追加 (state の `managed` flag。旧 state.json 互換)。repo file は `raw.githubusercontent.com` tag-pinned 取得 + state dir 無期限 cache。`schneeforge source init [--channel/--tag]` で移行、update は state 更新のみ、sync は案内 no-op。install.sh / bootstrap-flow は不改変 (2 表現佷存) | `openspec/changes/archive/2026-08-19-add-managed-release-source/` |
| Managed Nix Bootstrap Phase 1 + install 修正 (PR #13/#18) | **develop merge 済み** (a7d4777)。review 4 巡。CI 18/18 green | `openspec/changes/archive/2026-08-14-add-managed-nix-bootstrap/` |
| Spike `nix-bootstrap-provider-evaluation` | 完了 (Linux x86_64 実測済み、macOS aarch64 は ADR final acceptance 条件) | `openspec/changes/spike-nix-bootstrap-provider-evaluation/` |
| NixStatus 状態分類 (issue #15) | **develop merge 済み** (PR #33 / 6c48837)。`NixStatus` 4 状態 model + doctor `[status]` 欄 + GUI `nix_status` 表示・wizard の Managed Nix 案内。`nix repair` は別 change で設計予定 | `openspec/changes/archive/2026-08-16-add-nix-status-classification/` / `2026-08-16-add-gui-managed-nix-status/` |
| `schneeforge nix repair` (issue #15 残件) | **develop merge 済み** (PR #35 / 6470700)。`RepairAction` state-driven 修復 (Broken → stale ownership record 削除のみ自動、Degraded → uninstall/手動 cleanup 案内) + upstream `repair {hooks,sequoia}` wrap。E2E 11/11 pass | `openspec/changes/archive/2026-08-16-add-nix-repair/` |
| GUI Managed Nix install (issue #16) | **develop merge 済み** (PR #36 / f17604a)。privilege escalation helper (osascript / pkexec、昇格先は bundle 同梱の CLI sidecar) + wizard からの 2 段階 install UI (plan preview → 確認 → install) + install progress の event streaming。`NIX_SETTING_DIR` 昇格先渡し・repo 未 clone 時の gate 付き。CLI fallback 案内維持 | `openspec/changes/archive/2026-08-16-add-gui-managed-nix-install/` |
| GUI apply 系の昇格統合 (デグレ #5) | **develop merge 済み** (PR #37 / 1134a31)。`run_apply` / `run_rollback` / `run_upgrade` を core 直接呼び出しから CLI sidecar の昇格実行 (osascript / pkexec) へ集約。`EscalatedOp` に Apply/Rollback/Upgrade を追加。lock / state 保存は昇格先 CLI 内。実機 (osascript 昇格での apply) は macOS Final Acceptance で確認 | `openspec/changes/archive/2026-08-16-add-gui-privileged-apply/` |
| GUI nix repair / uninstall (issue #16 残作業) | **develop merge 済み** (PR #40 / 72fbd20。旧 PR #39 は base branch 削除で close → 再提出)。`EscalatedOp` に NixRepair/NixUninstall を追加し、wizard の Degraded/Broken 表示に「修復を試みる」ボタン・Ready 画面に確認付き「Nix を削除」ボタン。`--force` は GUI から渡さない (fail-closed 維持) | `openspec/changes/archive/2026-08-16-add-gui-nix-repair-uninstall/` |
| DMG offline bundle 法務 ADR (issue #17) | ADR-0002 起票・openspec change 作成 (branch `feat/gui-managed-nix-install` に同梱)。無改変再配布 + LICENSE 同梱 + written offer の方針を固定。**実装 (bundle 同梱・offline 経路) は弁護士確認後の別 change** | `docs/adr/0002-dmg-bundle-lgpl-redistribution.md` / `openspec/changes/add-dmg-offline-bundle-licensing/` |
| **GUI からの source 更新** (`gui-source-update`) | **PR #62 (c71fa9b): CI 全 green (required 7 gate + macos-check)、merge 承認待ち**。`run_update` Tauri command (core `update()` を昇格なし in-process 実行 = sync と同じ扱い) +「ソース更新」ボタン (id=`update`) + managed source での「アップグレード」隠蔽 (core が fail-closed のため)。merge 後は archive PR が必要 | `openspec/changes/gui-source-update/` (PR #62 branch) |
| **cli test の state/network 隔離** (`cli-test-state-isolation`) | **PR #63 (36e20c9): CI 全 green、merge 承認待ち**。`state_dir()` helper で state 読み得る全起動を XDG_STATE_HOME 隔離 (手動 CLI 実行による実 state 汚染で無関係 test が落ちる事故の再発防止) + `source_init_with_tag` の ls-remote を SCHNEEFORGE_REPO_URL で local origin 化 (133s network hang 解消)。#62 merge 後は rebase 必要 (strict up-to-date) | PR #63 branch |

## 既知のデグレ・機能漏れ（要対応）

### 高（Release Blocker）

| # | 問題 | 対応 |
|---|------|------|
| 5 | GUI apply の sudo/TTY 問題（privileged helper 未実装） | **解消** (feat/gui-privileged-apply): `run_apply` / `run_rollback` / `run_upgrade` を CLI sidecar の昇格実行 (osascript / pkexec) へ集約。nix install と同じ `escalate_command()` 経路。実機確認は macOS Final Acceptance に統合 |

### 中

| # | 問題 | 対応 |
|---|------|------|
| 12 | install.sh が main 固定（Stable/Edge 分離無し） | **解消・merge 済み** (PR #38 / e95d899): README の Stable ワンライナーを tag 固定 URL に分離 + tag と `SCHNEEFORGE_BOOTSTRAP_VERSION` pin の一致を `tests/install-sh.bats` で回帰保証。RELEASE.md の bump checklist に README URL 差し替えを追加 |
| — | Dependabot alert #2: glib 0.18.5 (GHSA-wrw7-89jp-8q8g / RUSTSEC-2024-0429, medium) | Known upstream dependency risk. 現 dependency tree では Tauri v2 Linux → GTK3 0.18 → glib 0.18.5 経由。macOS / Windows distribution には当該 GTK3 dependency は含まれない。app code からの VariantStrIter 直接利用は確認されていないが、transitive dependency 内の到達可能性まで否定する根拠にはしない。**gtk3-rs は 2026-08-13 に maintenance 再開の動き** (gtk-rs/gtk3-rs#857: gtk 0.19.0 + glib 0.22 への更新が進行中)。Tauri v2 / tao / wry 側の追従状況を monitor し、upstream release 後に再評価。**現時点では dismiss (no_plan_to_fix) せず tracking 継続** |

### 低

| # | 問題 | 対応 |
|---|------|------|
| — | バージョン文字列が `0.1.0` のまま（Cargo.toml / tauri.conf.json / packages.nix） | release/* で v0.2.0-rc.1 に bump |

## 次の作業（推奨順）

1. **PR #62 / #63 の merge** (user の承認待ち。両方 CI 全 green):
   - #62 (gui-source-update) を merge → `gui-source-update` change の archive PR を作成
   - #63 (cli-test-state-isolation) は #62 merge 後に `git rebase origin/develop` (strict up-to-date 対応。upstream 済み commit が conflict したら `git rebase --skip`) してから merge
2. **macOS Apple Silicon Final Acceptance** (ADR-0001 Accepted 昇格 + PR #11 Finder 実機 smoke を統合した 1 本フロー):
   - fresh / disposable environment・Nix 無し状態から開始
   - install.sh → Managed Nix install
   - receipt (`/nix/receipt.json`) / ownership record (`/nix/schneeforge-managed.json`) 確認
   - self-test / flakes / store ping
   - SchneeForge.app を Finder から起動 (PR #11 の実機 smoke)
   - minimal GUI PATH でも Nix を検出すること (`fix-path-env-rs` 検証)
   - doctor / status 正常終了
   - **PR #37/#40 の実機 smoke**: osascript 昇格での apply、wizard の「修復を試みる」ボタン・Ready 画面の「Nix を削除」ボタン (confirm dialog)
   - uninstall → cleanup
   - 通れば ADR-0001 を `Accepted` へ昇格
3. **release/v0.2.0-rc.6** (Final Acceptance PASS 後。現行 release は rc.5): RELEASE.md checklist に沿って release branch → main → tag。rc.6 から `schneeforge-release.json` asset 同梱 (PR #50)。`SCHNEEFORGE_BOOTSTRAP_VERSION` bump + **README の Stable ワンライナー URL を新 tag へ差し替え** (bats test が検知する) + musl asset の実機確認 (Nix-less dry-run)
4. Phase 2 残作業:
   - #17 DMG bundle + LGPL-2.1 法務 ADR: **ADR-0002 起票済み** (Accepted provisionally)。実装 (bundle 同梱・offline 経路、`add-dmg-offline-bundle-licensing` tasks 4.x) は弁護士確認後の別 change
5. 残デグレ対応:
   - glib advisory (Dependabot #2): gtk3-rs#857 と Tauri v2 側の追従を monitor、upstream release 後に再評価

※ issue #14/#15 は close 済み。#16 は Final Acceptance 済み次第 close、#17 は弁護士確認後に close。

## 開発フロー

- ブランチ: `git checkout develop` → `feat/*` → PR → develop (squash merge)
- OpenSpec 必須: `openspec new change` → proposal → specs → tasks → 実装 → PR → merge 後に **archive の separate PR** (change dir → `openspec/changes/archive/<date>-<name>/`)
- 品質ゲート: `cargo test` / `clippy` / `fmt` / `nix flake check` / `openspec validate --all` (local は `npx -y @fission-ai/openspec@1.8.0`)
- desktop (`apps/desktop`) は GTK 依存のため dev machine で compile 不可。lib.rs 変更時は `rustfmt --edition 2021 --check apps/desktop/src-tauri/src/lib.rs` を最低限実行 (parse error を push 前に検出できる)
- CI watch は `gh api repos/.../commits/$SHA/check-runs` で `status != "completed"` を数える (この gh version は `gh pr checks --json` 非対応、`statusCheckRollup` は pending を conclusion:null で返さないことがある)
- **手動 CLI 実行時は `XDG_STATE_HOME` を temp に向ける**: 向けないで `source init` 等を動かすと実 state (`~/.local/state/schneeforge/state.json`) を汚染し、原因の特定が難しい test 落ちを引き起こす (2026-08-20 に発生)。汚染したら state.json を削除すれば戻る。test 側の隔離は PR #63 で対応済み
- 詳細: [CONTRIBUTING.md](../CONTRIBUTING.md) / [RELEASE.md](../RELEASE.md)
