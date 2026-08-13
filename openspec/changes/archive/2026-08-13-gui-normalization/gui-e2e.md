# GUI E2E テスト計画（gui-normalization）

GUI の E2E テストケース。自動化は tauri-driver + WebDriver が必要なため、現状は手動チェックリストとして運用する。バックエンド側の各シナリオ相当は core の unit test で担保済み（表の「バックエンド担保」参照）。

## 事前準備

- ビルド: `cargo build --release --manifest-path apps/desktop/src-tauri/Cargo.toml`
- 起動: `apps/desktop/src-tauri/target/release/schneeforge-desktop`（または `nix run .#schneeforge-desktop`）
- ケースによっては `~/nix_setting` を一時退避して fresh 状態を作る

## テストケース

| # | シナリオ | 手順 | 期待結果 | バックエンド担保 |
|---|---------|------|---------|----------------|
| 8.1 | boot | アプリを起動する | クラッシュせず UI が表示される | — |
| 8.2 | host/status 表示 | Ready 状態で起動し、status カードを確認 | host/platform/arch/nix/homebrew/applied が「-」でなく実際の値を表示 | `diagnostics::diagnose` unit test |
| 8.3 | missing repo | `~/nix_setting` を無い状態で起動 | NeedsSetup モードになり「Set up SchneeForge」を表示。Apply 等の mutating ボタン非表示 | `diagnose_nonexistent_repo` |
| 8.4 | action mapping | 各ボタンをクリックし、どの command が呼ばれるか確認 | apply→run_apply, rollback→run_rollback, upgrade→run_upgrade, scan→run_scan | `frontend_commands_match_backend`（静的クロスチェック）|
| 8.5 | backend failure | 存在しない repo で Apply を実行 | エラーが output に表示され、クラッシュしない | operations のエラー伝搬 unit test |
| 8.6 | spinner/disable | Apply 実行中に状態を確認 | スピナー表示 + ボタン disable、完了後に復帰 | — |
| 8.7 | state refresh | Apply 成功後に status を確認 | applied_revision が更新され、status カードに反映 | `operations::apply` + StateStore unit test |

## First Run Wizard フロー（Phase 7）

1. 前提条件（8.2/8.3 の後）: Nix/Git/flakes の OK/NG 表示。NG なら install 手順表示
2. repo clone: URL 入力 → `git clone`（不正 URL は拒否）
3. username 確認: OS のユーザー名が入力欄に prefill
4. plan: dry-run 結果表示
5. confirm: 「適用する／キャンセル」の明示確認
6. apply: 実行（数分）。失敗時「再試行」で resume
7. verify: チェック結果表示。全 OK で「完了」

## 自動化の方針

- action mapping（8.4）は `frontend_commands_match_backend` で静的クロスチェック済み
- 完全な GUI E2E（実クリック・実描画）は tauri-driver + WebDriver の CI 導入が必要（Phase 9 以降で検討）
