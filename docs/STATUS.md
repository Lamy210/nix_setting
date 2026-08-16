# STATUS（セッション引き継ぎ用）

現在の開発状態・既知のデグレ・機能漏れ・次の作業をまとめる。セッションを切り替えても、ここを読めば再開できる。

最終更新: 2026-08-16

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

## 進行中

| 項目 | 進捗 | 場所 |
|------|------|------|
| Managed Nix Bootstrap Phase 1 + install 修正 (PR #13/#18) | **develop merge 済み** (a7d4777)。review 4 巡。CI 18/18 green | `openspec/changes/archive/2026-08-14-add-managed-nix-bootstrap/` |
| Spike `nix-bootstrap-provider-evaluation` | 完了 (Linux x86_64 実測済み、macOS aarch64 は ADR final acceptance 条件) | `openspec/changes/spike-nix-bootstrap-provider-evaluation/` |
| NixStatus 状態分類 (issue #15) | **develop merge 済み** (PR #33 / 6c48837)。`NixStatus` 4 状態 model + doctor `[status]` 欄 + GUI `nix_status` 表示・wizard の Managed Nix 案内。`nix repair` は別 change で設計予定 | `openspec/changes/archive/2026-08-16-add-nix-status-classification/` / `2026-08-16-add-gui-managed-nix-status/` |
| `schneeforge nix repair` (issue #15 残件) | 実装完了 (branch `feat/nix-repair`)。`RepairAction` state-driven 修復 (Broken → stale ownership record 削除のみ自動、Degraded → uninstall/手動 cleanup 案内) + upstream `repair {hooks,sequoia}` wrap。E2E 11/11 pass | `openspec/changes/add-nix-repair/` |
| GUI Managed Nix install (issue #16) | 実装完了 (branch `feat/gui-managed-nix-install`、PR #36 CI 19/19 green)。privilege escalation helper (osascript / pkexec、昇格先は bundle 同梱の CLI sidecar) + wizard からの 2 段階 install UI (plan preview → 確認 → install) + install progress の event streaming。`NIX_SETTING_DIR` 昇格先渡し・repo 未 clone 時の gate 付き。CLI fallback 案内維持 | `openspec/changes/add-gui-managed-nix-install/` |
| GUI apply 系の昇格統合 (デグレ #5) | 実装完了 (branch `feat/gui-privileged-apply`)。`run_apply` / `run_rollback` / `run_upgrade` を core 直接呼び出しから CLI sidecar の昇格実行 (osascript / pkexec) へ集約。`EscalatedOp` に Apply/Rollback/Upgrade を追加。lock / state 保存は昇格先 CLI 内。実機 (osascript 昇格での apply) は macOS Final Acceptance で確認 | `openspec/changes/add-gui-privileged-apply/` |
| DMG offline bundle 法務 ADR (issue #17) | ADR-0002 起票・openspec change 作成 (branch `feat/gui-managed-nix-install` に同梱)。無改変再配布 + LICENSE 同梱 + written offer の方針を固定。**実装 (bundle 同梱・offline 経路) は弁護士確認後の別 change** | `docs/adr/0002-dmg-bundle-lgpl-redistribution.md` / `openspec/changes/add-dmg-offline-bundle-licensing/` |

## 既知のデグレ・機能漏れ（要対応）

### 高（Release Blocker）

| # | 問題 | 対応 |
|---|------|------|
| 5 | GUI apply の sudo/TTY 問題（privileged helper 未実装） | **解消** (feat/gui-privileged-apply): `run_apply` / `run_rollback` / `run_upgrade` を CLI sidecar の昇格実行 (osascript / pkexec) へ集約。nix install と同じ `escalate_command()` 経路。実機確認は macOS Final Acceptance に統合 |

### 中

| # | 問題 | 対応 |
|---|------|------|
| 12 | install.sh が main 固定（Stable/Edge 分離無し） | release hardening |
| — | Dependabot alert #2: glib 0.18.5 (GHSA-wrw7-89jp-8q8g / RUSTSEC-2024-0429, medium) | Known upstream dependency risk. 現 dependency tree では Tauri v2 Linux → GTK3 0.18 → glib 0.18.5 経由。macOS / Windows distribution には当該 GTK3 dependency は含まれない。app code からの VariantStrIter 直接利用は確認されていないが、transitive dependency 内の到達可能性まで否定する根拠にはしない。**gtk3-rs は 2026-08-13 に maintenance 再開の動き** (gtk-rs/gtk3-rs#857: gtk 0.19.0 + glib 0.22 への更新が進行中)。Tauri v2 / tao / wry 側の追従状況を monitor し、upstream release 後に再評価。**現時点では dismiss (no_plan_to_fix) せず tracking 継続** |

### 低

| # | 問題 | 対応 |
|---|------|------|
| — | バージョン文字列が `0.1.0` のまま（Cargo.toml / tauri.conf.json / packages.nix） | release/* で v0.2.0-rc.1 に bump |

## 次の作業（推奨順）

1. **macOS Apple Silicon Final Acceptance** (ADR-0001 Accepted 昇格 + PR #11 Finder 実機 smoke を統合した 1 本フロー):
   - fresh / disposable environment・Nix 無し状態から開始
   - install.sh → Managed Nix install
   - receipt (`/nix/receipt.json`) / ownership record (`/nix/schneeforge-managed.json`) 確認
   - self-test / flakes / store ping
   - SchneeForge.app を Finder から起動 (PR #11 の実機 smoke)
   - minimal GUI PATH でも Nix を検出すること (`fix-path-env-rs` 検証)
   - doctor / status 正常終了
   - uninstall → cleanup
   - 通れば ADR-0001 を `Accepted` へ昇格
2. **release/v0.2.0-rc.2**: RELEASE.md checklist に沿って release branch → main → tag。`SCHNEEFORGE_BOOTSTRAP_VERSION` bump (`v0.2.0-rc.2`) + musl asset の実機確認 (Nix-less dry-run)
3. **Phase 2 残作業** (issue #14-17):
   - #14 acceptance 残り: receipt 冪等性 regression
   - #15 Nix 状態分類 (Missing/Healthy/Degraded/Broken): **完了** — 分類 model (`schneeforge nix doctor` の `[status]` 欄) + `schneeforge nix repair` (state-driven 修復、`feat/nix-repair` で PR 提出予定)
   - #16 GUI (Tauri) 統合: `prepare_plan()` / `execute_plan()` API + osascript/pkexec 特権昇格
   - #17 DMG bundle + LGPL-2.1 法務 ADR: **ADR-0002 起票済み** (Accepted provisionally)。実装は弁護士確認後の別 change
4. 残デグレ対応:
   - #5 GUI 特権ヘルパー: **完了** (feat/gui-privileged-apply — apply/rollback/upgrade を sidecar 昇格へ集約)
   - #12 install.sh の Stable/Edge 分離（release 方針の判断）
   - glib advisory (Dependabot #2): gtk3-rs#857 と Tauri v2 側の追従を monitor、upstream release 後に再評価

## 開発フロー

- ブランチ: `git checkout develop` → `feat/*` → PR → develop
- OpenSpec 必須: `openspec new change` → proposal → specs → tasks → 実装 → **archive（PR 前）** → PR
- 品質ゲート: `cargo test` / `clippy` / `fmt` / `nix flake check` / `openspec validate --all`
- 詳細: [CONTRIBUTING.md](../CONTRIBUTING.md) / [RELEASE.md](../RELEASE.md)
