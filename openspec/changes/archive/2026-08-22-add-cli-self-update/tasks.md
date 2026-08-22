# Tasks

- [x] 1. core: `Error::SelfUpdate` variant 追加 (Display / From 整備)
- [x] 2. core: `self_update.rs` — 純関数 (`platform_asset` /
      `expected_sha256` / `release_asset_url` / `plan`) を実装
- [x] 3. core: `self_update.rs` — `run` (remote_tags → plan → download →
      verify → atomic replace) と `SelfUpdateStatus` を実装
- [x] 4. core: unit test (hermetic: 純関数全系 + fs 置換の成功/失敗)
- [x] 5. cli: top-level `self-update` command + handler (state の channel、
      ToolInventory 経由の git、結果表示)
- [x] 6. cli: test (引数 wiring / git 未検出の fail-closed)
- [x] 7. 検証: `cargo test` / `clippy` / `fmt`、openspec validate
      (`npx -y @fission-ai/openspec@1.8.0 validate --all --no-interactive`)
