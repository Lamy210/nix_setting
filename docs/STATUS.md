# STATUS（セッション引き継ぎ用）

現在の開発状態・既知のデグレ・機能漏れ・次の作業をまとめる。セッションを切り替えても、ここを読めば再開できる。

最終更新: 2026-08-14

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
- **未完了**: macOS desktop build + Finder launch 実機検証 (CI の macos-check は green だが実機 smoke は別途)。

### docs/release-checklist-and-tap-sync（merge 済み #12）

RELEASE.md のリリースチェックリスト再構築 + weekly workflow の権限修正。

## 進行中

| 項目 | 進捗 | 場所 |
|------|------|------|
| Managed Nix Bootstrap (NixOS/nix-installer 統合) | OpenSpec change `add-managed-nix-bootstrap` 起票・ADR-0001 provisionally accepted | `feat/managed-nix-bootstrap` branch |
| Spike `nix-bootstrap-provider-evaluation` | 完了 (Linux x86_64 実測済み、macOS aarch64 は ADR final acceptance 条件) | `openspec/changes/spike-nix-bootstrap-provider-evaluation/` |

## 既知のデグレ・機能漏れ（要対応）

### 高（Release Blocker）

| # | 問題 | 対応 |
|---|------|------|
| 5 | GUI apply の sudo/TTY 問題（privileged helper 未実装） | desktop: 昇格ヘルパー実装（設計は design.md に済み。macOS authorization / osascript）。※ runtime-tool-resolution-hardening で「GUI 起動時に Nix が見つからない」副次症状は解消（`fix-path-env-rs` + `Toolchain` 解決） |

### 中

| # | 問題 | 対応 |
|---|------|------|
| 12 | install.sh が main 固定（Stable/Edge 分離無し） | release hardening |

### 低

| # | 問題 | 対応 |
|---|------|------|
| — | バージョン文字列が `0.1.0` のまま（Cargo.toml / tauri.conf.json / packages.nix） | release/* で v0.2.0-rc.1 に bump |

## 次の作業（推奨順）

1. **Managed Nix Bootstrap (Phase 1) の実装** — `add-managed-nix-bootstrap` tasks.md に従い、Core `managed_nix` module → CLI `schneeforge nix install/doctor/uninstall` → `bootstrap-manifest.toml` → release bump CI → spec 更新 → test → docs → archive → PR
2. **ADR-0001 Final acceptance**: macOS aarch64 disposable env で smoke test (install / self-test / flakes / receipt / uninstall / cleanup) を実施し、Status を `Accepted` へ昇格
3. **PR #11 の macOS Finder 実機 smoke** (CI は green だが未実施)
4. 残デグレ対応:
   - #5 GUI 特権ヘルパー（macOS authorization）— privileged-gui-operations で Managed Nix と統合
   - #12 install.sh の Stable/Edge 分離（release 方針の判断）
   - バージョン bump（release/* ブランチで）

## 開発フロー

- ブランチ: `git checkout develop` → `feat/*` → PR → develop
- OpenSpec 必須: `openspec new change` → proposal → specs → tasks → 実装 → **archive（PR 前）** → PR
- 品質ゲート: `cargo test` / `clippy` / `fmt` / `nix flake check` / `openspec validate --all`
- 詳細: [CONTRIBUTING.md](../CONTRIBUTING.md) / [RELEASE.md](../RELEASE.md)
