# Tasks

## 1. core: profile API の集約

- [x] 1.1 `crates/core/src/profile.rs`: `ProfileList` (available / default / selected) と `list` / `list_with` を追加
- [x] 1.2 `set_selection` / `set_selection_with` を追加: manifest の `profiles.available` 検証 → `save_selection`。検証 error は "not in manifest" を含む文字列にする (CLI integration test 互換)
- [x] 1.3 unit test: list (selected あり / なし / manifest 無し)、set_selection (保存成功 / available 外は拒否かつ state 不変 / manifest 無しは error)

## 2. CLI: core API への寄せ

- [x] 2.1 `profile set` を core `set_selection` で構築 (出力・error は不変)
- [x] 2.2 `profile list` を core `ProfileList` で構築 (表示形式は不変)
- [x] 2.3 既存 integration test (list / set / 拒否 / show) が green

## 3. desktop: commands

- [x] 3.1 `apps/desktop/src-tauri/src/lib.rs`: async command `get_profiles` / `set_profile` / `clear_profile` を追加し `generate_handler` に登録 (manifest 取得は blocking 実行)
- [x] 3.2 regression test: `ProfileList` serialize key + `main.js` の invoke / key 参照 + `index.html` の DOM id の 3 層検証

## 4. frontend: 切替 UI

- [x] 4.1 `index.html`: Dashboard card に profile select (`dash-profile-select`) + 適用 (`profile-set`) + 既定へ (`profile-clear`) + 結果表示 (`profile-note`) を追加
- [x] 4.2 `main.js`: `get_profiles` で select を構築 (selected > default を初期選択)、`set_profile` / `clear_profile` の wiring、成功時は「次回の apply から反映」を表示して status / dashboard を更新、manifest 不在は使用不可表示

## 5. test / CI

- [x] 5.1 `cargo test` (core / cli) + `cargo clippy -D warnings` + `cargo fmt -- --check` green (desktop は CI rust-check で検証)
- [x] 5.2 openspec validate green (CI と同一 version 1.8.0 で)
- [x] 5.3 PR 作成 (base: develop)
