# Tasks

## 1. core

- [x] 1.1 `release_metadata.rs` に `release_page_url(repo_url, version)` 純関数を追加 (`.git` trim + `v` prefix)
- [x] 1.2 unit test: default URL / `.git` 付き URL / 上書き URL の各 case

## 2. desktop backend

- [x] 2.1 `tauri-plugin-opener = "2"` を Cargo.toml に追加し Cargo.lock を更新
- [x] 2.2 `open_release` command (version → core 純関数 → opener) を実装
- [x] 2.3 plugin 初期化と `generate_handler` への登録

## 3. desktop frontend

- [x] 3.1 `index.html` に `dash-release-link` button (初期 hidden) を追加
- [x] 3.2 `main.js` refreshDashboard で `update_available` のときのみ button を表示
- [x] 3.3 click handler (`open_release` invoke・error 表示)

## 4. 回帰検証

- [x] 4.1 lib.rs に DOM id / JS 参照の静的検証 test を追加
- [x] 4.2 既存 `frontend_commands_match_backend` が新 command を cover することを確認

## 5. 検証 / PR

- [x] 5.1 local 検証: workspace `cargo test` / `clippy` / `fmt` + desktop lib.rs の rustfmt parse 検査 + `openspec validate --all`
- [ ] 5.2 feature branch → PR (base develop) → CI (macos-check で desktop compile を含む全 gate green)
