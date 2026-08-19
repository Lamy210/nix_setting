# Proposal: GUI での profile 切替 (v2 §17 follow-up)

## Why

v2 §17 (PR #48) で profile 選択を state + manifest ベースに移行し、CLI には
`schneeforge profile list|set|clear|show` が揃った。しかし GUI には profile を
切り替える手段がなく、Dashboard / Status は実効 profile の**表示のみ**にとどまる。
GUI user が minimal profile へ変更するには terminal で CLI を叩く必要があり、
「CLI / GUI / flake apps の三面で統一 UX」(project purpose) に反する。

## What Changes

- **core** (`crates/core/src/profile.rs`):
  - `ProfileList` (available / default / selected) を返す `list` / `list_with`
  - manifest の `profiles.available` 検証を行ってから state へ保存する
    `set_selection` / `set_selection_with`。検証 logic を CLI から core へ集約
- **CLI**: `profile set` / `profile list` を core API (`set_selection` /
  `ProfileList`) で構築するよう変更。出力と error message は不変
  (integration test が検証)
- **desktop** (`apps/desktop/src-tauri/src/lib.rs`): async command
  `get_profiles` / `set_profile(name)` / `clear_profile` を追加。
  manifest 取得 (managed source は network fetch) は blocking 実行
- **frontend**: Dashboard card に profile 切替 UI (available からの select +
  適用+ 既定へ)。切替は state のみで、**次回の apply から反映される**旨を
  表示する (repo は書き換えない)

## Impact

- profile 選択の保存先は従来通り state (`XDG_STATE_HOME`) のみ。escalation 不要
- CLI の挙動 (出力形式 / fail-closed error) は不変 — 既存 integration test green
- manifest が解決できない環境 (source 未 init) では切替 UI は使用不可能表示に
  落ち、Dashboard 自体は error にしない
- specs: `gui-operations` に profile 切替 の requirement を追加
