# Tasks

## 1. core: dashboard snapshot

- [x] 1.1 `crates/core/src/dashboard.rs`: `DashboardSnapshot` (installed: version / profile / channel / applied_revision / applied_at、available: Option<ReleaseMetadata> + available_error、update_available) の定義と構築。available は引数から差し込める形 (network 分離) にして hermetic test 可能にする
- [x] 1.2 `update_available` 判定の純関数 (`version_is_newer` / `compare_versions`) を dashboard.rs に実装。semver 準拠 (正式版 > 同一 core の prerelease、suffix は数値比較で `rc.10` > `rc.9`)。tag 選択には既存 `latest_tag_for_channel` をそのまま使用
- [x] 1.3 `git ls-remote --tags` の tag 列 → `latest_tag_for_channel` → tag 文字列の解決 (`latest_tag_from_ls_remote` 純関数 + `remote_tags` / `fetch_available`)。`run_capture` 経由の解決済み git (lint gate「forbid raw tool spawns」準拠)
- [x] 1.4 unit test: snapshot 組み立て (installed 各 field / available あり・None+error / update_available true・false)、tag 列解決 (stable・preview・peel 行・tag 無し)、version 比較 (rc を含む昇順・桁違い・rc.10 vs rc.9)

## 2. desktop: get_dashboard command

- [x] 2.1 `apps/desktop/src-tauri/src/lib.rs`: async command `get_dashboard` を登録 (generate_handler 追加)。ls-remote + `ReleaseMetadata::fetch` は blocking 実行
- [x] 2.2 test: `DashboardSnapshot` の serialize key (installed/available/update_available/available_error + installed 各 field) が存在すること、`frontend_commands_match_backend` が通ること (※ desktop の test 実行は CI の rust-check で実施)

## 3. frontend: Dashboard 表示

- [x] 3.1 `index.html`: ready view に Dashboard card を追加 (installed / available / update 案内)
- [x] 3.2 `main.js`: `get_dashboard` を呼び installed・available・update 案内を描画。available 取得失敗時は理由を表示し installed は出す
- [x] 3.3 test: frontend が `d.installed.*` / `d.available.*` 等 snapshot key を参照していること + 参照先 DOM id が index.html に存在することの regression test

## 4. test / CI

- [x] 4.1 `cargo test` 全 green (core 261 / cli 25 は local、desktop は CI rust-check で green — PR #52)
- [x] 4.2 `cargo fmt` / `cargo clippy -D warnings` green (workspace local / desktop は CI で green)
- [x] 4.3 `nix flake check` green (CI green — PR #52)
- [x] 4.4 openspec validate green / PR 作成 (base: develop) — PR #52 を squash merge 済み
