# Change: Distribution Manifest (`schneeforge.toml`) の導入 (v2 §14/§15/§17)

## Why

v2 P0 (PR #43) で `config.toml` の `[user]` を廃止し repo から git rm した
結果、`Manifest::load` は常に失敗する状態になった。diagnostics の
`manifest_found` が常時 `false` で、fresh clone と manifest 破損が区別できず、
CLI `status` / GUI Status の表示も実質 dead code 化している。

v2 設計は repository の自己記述を「誰が使うか (machine)」ではなく
「この repository が何を提供するか (distribution)」へ分離することを定めて
おり、その载体として `schneeforge.toml` を定義する (§14/§15)。あわせて
Profile 体系 (§17: minimal / developer) を manifest で宣言し、GUI
Dashboard (§28) が表示する Profile / channel 情報の供給源とする。

## What Changes

- **ADDED: `schneeforge.toml` (repo root)**
  - `schema = 1`
  - `[distribution]` name
  - `[profiles]` default / available (`minimal`, `developer`)
  - `[systems]` 対応 system の map (`aarch64-darwin` 等)
- **MODIFIED: `Manifest` type (core)**
  - 読み込み先を `config.toml` から `schneeforge.toml` へ
  - `[user]` 読み込みを削除 (v1 config.toml は読み込み対象外)
  - `validate`: schema 検査に加え default profile が available に含まれる
    こと・実行 system が `[systems]` に含まれることを検証
- **ADDED: `profiles/minimal.nix`**
  - §17 の Profile 体系。`developer` は既存のまま
- **MODIFIED: diagnostics**
  - `username` field を廃止し `profile` (manifest の default) を返す
  - `manifest_found: false` は schneeforge.toml が無い/壊れている場合のみ
- **MODIFIED: CLI `status` / `scan`**
  - `user:` 行を `profile:` 行へ (machine 情報は MachineFacts が別途表示)
- **MODIFIED: GUI (`get_status`)**
  - `username` を返さず `profile` を返す。frontend の user 表示を profile へ

## Impact

- **Specs**: `core-operations` (manifest 読み込み) / `gui-diagnostics`
  (Status schema の username → profile) を更新
- **Core**: `manifest.rs` 全面置換、`diagnostics.rs` の field 変更
- **Repo**: `schneeforge.toml` / `profiles/minimal.nix` 追加。
  host 側 profile 選択 (flake への profile input 注入) は本 change の
  対象外 — manifest は宣言のみで、適用は現行 (`developer` 固定) のまま
- **互換性**: v1 `config.toml` は読み込まない。schema 検証で明確に
  error を返すため silent な誤動作は無い

## Scope / Non-goals

- flake への profile 選択の注入 (`--override-input` 拡張) は別 change
- Release Metadata (§27, release asset 側の JSON) は別 change
- GUI Dashboard 本体 (§28) は別 change。本 change はその data 供給のみ
