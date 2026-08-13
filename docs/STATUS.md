# STATUS（セッション引き継ぎ用）

現在の開発状態・既知のデグレ・機能漏れ・次の作業をまとめる。セッションを切り替えても、ここを読めば再開できる。

最終更新: 2026-08-13

## 完成済み

| 領域 | 内容 |
|------|------|
| Nix 基盤 | flake-parts / hosts / profiles / manifest / 3システム / CI 10+ジョブ |
| Rust core | actions / discovery / manifest / repo / state / time（+ 42 unit tests） |
| CLI | 11 コマンド（doctor/scan/setup/status/plan/apply/rollback/upgrade/sync/verify/uninstall） |
| Tauri GUI | 雛形 + 操作ボタン（P0 修正済み） |
| 配布 | flake apps/templates / install.sh / GitHub Release / DMG / Homebrew(要修正) / cargo install |
| OpenSpec | 初期化済み + gui-normalization change（proposal/design/specs/tasks） |
| 運用 | Git Flow（develop default）/ ブランチ保護 / CONTRIBUTING / PR テンプレ / OpenSpec CI |

## 進行中

| 項目 | 進捗 | 場所 |
|------|------|------|
| GUI 正常化 | 0/63 tasks | `openspec/changes/gui-normalization/tasks.md` |

## 既知のデグレ・機能漏れ（要対応）

### 高（Release Blocker）

| # | 問題 | 対応 |
|---|------|------|
| 1 | install.sh が username `lamy210` をハードコード（第三者に誤適用） | bootstrap-flow: username/HOME を OS から取得 |
| 2 | GUI apply 後に State が保存されない（core に無い） | core-operations: state persistence を core へ |
| 3 | upgrade/sync が CWD 依存 | core-operations: repo-aware |
| 4 | Homebrew formula が broken（v0.1.0 binary 不在 + homebrew/ 場所不正） | release gate: Lamy210/homebrew-tap へ分離 |
| 5 | GUI apply が sudo 問題未解決（.app に TTY 無し） | bootstrap-flow: privilege strategy |

### 中

| # | 問題 | 対応 |
|---|------|------|
| 6 | Host 検出が aarch64-darwin → macbook-air 固定（汎用製品でない） | core: Platform/ConfigTarget 分離 |
| 7 | Manifest の実行時検証が無い（schema/username 不一致を検出しない） | core: Manifest::validate |
| 8 | nix-darwin bootstrap が unpinned（registry 版） | bootstrap: --inputs-from <repo> |
| 9 | State 保存エラーが握り潰される（`let _ =`） | core: atomic StateStore + エラー伝搬 |
| 10 | uninstall に副作用（表示コマンドなのに state 削除）+ darwin-uninstaller が古い | 別 change で対応 |
| 11 | enable_flakes 判定が甘い（文字列存在チェック） | bootstrap: preflight で実検証 |
| 12 | install.sh が main 固定（Stable/Edge 分離無し） | release hardening |

### 低

| # | 問題 | 対応 |
|---|------|------|
| 13 | GUI メイン画面に Plan/Verify ボタンが無い（First Run Wizard のみ） | GUI 設計 |
| 14 | CSP が null | desktop: CSP 設定 |
| 15 | opener plugin 未使用（capability 最小化） | desktop: 削除 |

## 次の作業（推奨順）

1. `openspec/changes/gui-normalization/tasks.md` の Phase 1（Core environment model）から着手
   - 1.1 Platform/Architecture/ConfigurationTarget 分離
   - 1.2 Manifest::validate 追加
   - 1.3 ToolResolver / 1.4 RepoResolver
2. Phase 2（State and operation safety）
3. Phase 3（Core operations repo-aware）

## 開発フロー

- ブランチ: `git checkout develop` → `feat/*` → PR → develop
- OpenSpec 必須: `openspec new change` → proposal → specs → tasks → 実装 → archive
- 品質ゲート: `cargo test` / `clippy` / `fmt` / `nix flake check` / `openspec validate --all`
- 詳細: [CONTRIBUTING.md](../CONTRIBUTING.md) / [RELEASE.md](../RELEASE.md)
