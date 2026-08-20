# Tasks

## 1. core: diagnostics 拡張

- [x] 1.1 `diagnostics.rs`: `ManagedSourceSummary` (tag / channel / flake_ref) と `Diagnostics.managed_source` を追加 (`Serialize` snake_case は既存 field に合わせる)
- [x] 1.2 unit test: managed state → Some (tag/channel/ref 一致) / checkout・未初期化 → None

## 2. desktop: command / gate

- [x] 2.1 `run_source_init(channel, tag)` Tauri command (core `source_init` を `spawn_blocking` で呼ぶ。`CommandOutput` 返却) を登録
- [x] 2.2 `nix_prepare_plan_blocking` の `bootstrap-manifest.toml` 存在 check 削除 (embedded manifest fallback に寄せた残存 gate) — stack base (switch-install-sh-to-managed-source) で `ManagedNix::load_prefer_repo` 化済み、残存 gate なしを確認
- [x] 2.3 3 層 test: serialize key (`managed_source` 等) / handler 登録 / `source_init` 呼び出し

## 3. frontend: wizard / boot

- [x] 3.1 stepRepo を source 選択 step に再構成 (managed 既定・推奨 + 「git clone (fork / 開発者向け)」選択)。既存 checkout は登録済み表示
- [x] 3.2 起動 gate を `!repo_exists && !managed_source` へ変更
- [x] 3.3 stepPrereq の `repo_exists` gate (「まず repository の clone が必要です」) 削除
- [x] 3.4 JS 参照 / DOM id の 3 層 test 更新 (`wizard_gates_nix_install_on_repo_exists` を `wizard_nix_install_is_repo_independent` へ置換、`wizard_source_step_offers_managed_init` / `boot_gate_accepts_managed_source` 追加)

## 4. test / CI

- [x] 4.1 `cargo test` / `clippy -D warnings` / `cargo fmt` green (local — desktop は GTK 依存のため CI で確認)
- [x] 4.2 openspec validate green (@fission-ai/openspec@1.8.0)
- [x] 4.3 CI 7 gate green (stack 解除後: PR #56 / install.sh change merge 後に rebase + base develop)
- [x] 4.4 PR 作成 (base: develop)
