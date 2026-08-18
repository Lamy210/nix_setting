# Tasks

## 1. core: manifest type 置換

- [x] 1.1 `crates/core/src/manifest.rs`: `Manifest` を `schneeforge.toml` 読み込みへ置換 (`schema` / `[distribution]` name / `[profiles]` default+available / `[systems]` map)。`[user]` は削除
- [x] 1.2 `Manifest::validate`: schema == 1 / default ∈ available / 実行 system ∈ systems を検証 (systems は `detect_target` 相当の呼び出し元から渡す)
- [x] 1.3 unit test: 読み込み・parse・検証 (schema 不一致 / default 不整合 / system 未対応 / 有効)、旧 config.toml を読まないこと

## 2. repo: schneeforge.toml + minimal profile

- [x] 2.1 `schneeforge.toml` を作成 (schema 1 / distribution name / profiles default=developer, available=[minimal, developer] / systems 3 種)
- [x] 2.2 `profiles/minimal.nix` を作成 (cli + git のみ)
- [x] 2.3 `nix flake check` が green であることを確認 (profile 追加の影響なし)

## 3. core: diagnostics / State 表示

- [x] 3.1 `diagnostics.rs`: `username` field を廃止し `profile: Option<String>` を追加。`manifest_found` は schneeforge.toml 基準へ
- [x] 3.2 unit test: schneeforge.toml 有り (profile 表示) / 無し (manifest_found false) の両方

## 4. CLI / GUI

- [x] 4.1 `crates/cli/src/main.rs`: `status` / `scan` の `user:` 行を `profile:` へ。`load_manifest` は新 Manifest を返す
- [x] 4.2 desktop `get_status`: username を返さず profile を返す。frontend (`main.js`) の user 表示を profile へ
- [x] 4.3 test: cli integration と desktop unit が新 field で green

## 5. test / CI

- [x] 5.1 `cargo test` 全 green (core / cli / desktop)
- [x] 5.2 `cargo fmt` / `cargo clippy -D warnings` green
- [x] 5.3 `nix flake check` green
- [x] 5.4 openspec validate green / PR 作成 (base: develop)
