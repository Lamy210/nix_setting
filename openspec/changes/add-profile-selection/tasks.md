# Tasks

## 1. flake: profile input

- [x] 1.1 `flake.nix`: `profile` input (`path:./defaults/profile.nix`, flake=false) を追加
- [x] 1.2 `defaults/profile.nix` (placeholder): `{ profile = null; }`
- [x] 1.3 `modules/profile-input.nix`: selected profile を `profiles/<name>.nix` へ解決する helper (null → manifest default 相当の developer, 未知 name は throw)
- [x] 1.4 `hosts/darwin-aarch64/default.nix` / `hosts/linux-generic/default.nix`: `../../profiles/developer.nix` の hard-code を profile-input helper へ置換
- [x] 1.5 `nix flake check` green (3 systems)

## 2. core: state + 注入

- [x] 2.1 `state.rs`: `State.profile: Option<String>` 追加 (serde default, 旧 JSON 互換)
- [x] 2.2 `machine.rs` 相当の `write_profile_input(name) -> PathBuf`: state dir に `{ profile = "<name>"; }` を生成
- [x] 2.3 `resolve_profile(repo)`: state.profile → manifest default の解決。manifest available 外は error
- [x] 2.4 `machine_override_args` を拡張し profile の `--override-input` を追加 (apply / plan / upgrade 経路で共通)
- [x] 2.5 unit test: profile 解決 (state あり / なし / available 外 / manifest 無し)、args に profile override が含まれること

## 3. CLI

- [x] 3.1 `schneeforge profile list` (manifest available + 現在選択を表示)
- [x] 3.2 `schneeforge profile set <name>` (manifest 検証 → state 保存)
- [x] 3.3 `schneeforge profile show` (現在の解決結果)
- [x] 3.4 `status` に `profile (selected)` を表示 (manifest default と異なる場合)
- [x] 3.5 integration test: profile set → state 保存 / 不正 name は error

## 4. GUI

- [x] 4.1 `get_status` に `selected_profile` を追加 (state 由来。manifest default と同じなら null)
- [x] 4.2 test: desktop unit で selected_profile の roundtrip

## 5. test / CI

- [x] 5.1 `cargo test` 全 green (core / cli / desktop)
- [x] 5.2 `cargo fmt` / `cargo clippy -D warnings` green
- [x] 5.3 `nix flake check` green
- [x] 5.4 openspec validate green / PR 作成 (base: develop)
