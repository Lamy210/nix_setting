# STATUS（セッション引き継ぎ用）

現在の開発状態・既知のデグレ・機能漏れ・次の作業をまとめる。セッションを切り替えても、ここを読めば再開できる。

最終更新: 2026-08-14

## 完成済み

| 領域 | 内容 |
|------|------|
| Nix 基盤 | flake-parts / hosts / profiles / manifest / 3システム / CI 10+ジョブ |
| Rust core | actions / discovery / diagnostics / manifest / repo / state / time / tool / lock / operations / process / bootstrap（+ 69 unit tests） |
| CLI | 11 コマンド（core 委譲のみの adapter 化済み） |
| Tauri GUI | 診断 Status + First Run Wizard + 非同期コマンド + CSP + 状態機械 |
| OpenSpec | `gui-normalization` 63/63 tasks 完了（未アーカイブ） |
| 運用 | Git Flow / ブランチ保護 / CONTRIBUTING / PR テンプレ / OpenSpec CI |

### gui-normalization（63/63 完了）

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

## 進行中

| 項目 | 進捗 | 場所 |
|------|------|------|
| （なし） | — | — |

## 既知のデグレ・機能漏れ（要対応）

### 高（Release Blocker）

| # | 問題 | 対応 |
|---|------|------|
| 1 | install.sh が repo の config.toml（username=lamy210）をそのまま適用（第三者に誤適用） | bootstrap-flow: 導入時に username/HOME を OS から取得して config 生成 |
| 5 | GUI apply の sudo/TTY 問題（privileged helper 未実装） | desktop: 昇格ヘルパー実装（設計は design.md に済み） |

### 中

| # | 問題 | 対応 |
|---|------|------|
| 10 | uninstall に副作用（表示コマンドなのに state 削除）+ darwin-uninstaller が古い | 別 change で対応 |
| 12 | install.sh が main 固定（Stable/Edge 分離無し） | release hardening |

### 低

| # | 問題 | 対応 |
|---|------|------|
| 13 | GUI メイン画面（Ready）に Plan/Verify ボタンが無い（First Run Wizard のみ） | GUI 設計 |

## 次の作業（推奨順）

1. `openspec archive gui-normalization`（63/63 完了済み）
2. ブランチ `feat/gui-normalization` を PR → develop へ merge
3. 残デグレ対応:
   - #1 install.sh の username 個人化（bootstrap-flow の残り）
   - #13 Ready 画面への Plan/Verify ボタン追加
   - #10 uninstall の副作用修正（別 change）

## 開発フロー

- ブランチ: `git checkout develop` → `feat/*` → PR → develop
- OpenSpec 必須: `openspec new change` → proposal → specs → tasks → 実装 → archive
- 品質ゲート: `cargo test` / `clippy` / `fmt` / `nix flake check` / `openspec validate --all`
- 詳細: [CONTRIBUTING.md](../CONTRIBUTING.md) / [RELEASE.md](../RELEASE.md)
