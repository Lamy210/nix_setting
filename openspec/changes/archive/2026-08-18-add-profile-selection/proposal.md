# Proposal: Profile 選択の flake 注入 (v2 §17)

## Why

PR #46 で `schneeforge.toml` の `[profiles]` (default=developer,
available=[minimal, developer]) と `profiles/minimal.nix` を導入したが、
現状 **host module が `profiles/developer.nix` を hard-code** しており
manifest は表示専用になっている。minimal profile を選択する手段が存在しない。

v2 設計 §17 は Profile を明示的な選択対象とし、§15 の manifest が
その宣言源である。machine input (§13) と同じ `--override-input` pattern
であれば「repo を書き換えない」原則 (P0) を維持したまま実現できる。

## What Changes

- **flake**: 新 input `profile` (`path:./defaults/profile.nix`,
  flake=false, placeholder は `null`)。hosts/*/default.nix は
  `import ../../profile-input` 経由で profile module を import する
  (null なら developer = 既存挙動)
- **core**: `state.json` に `profile: Option<String>` を追加。
  `machine_override_args` を `profile::override_args` へ統合し、
  machine + profile 両 input を `--override-input` で注入
  (apply / plan / rollback で共通利用)
- **bugfix**: file を指す path input の override は bare 絶対 path だと
  nix 2.35 が "not a flake (because it's not a directory)" で拒否する
  ため `path:<abs>` URL 形式へ修正 (**既存の machine input 注入に潜在
  していた bug の fix を含む**。実機 Acceptance 未実施のため未検出だった)
- **core**: manifest の `profiles.available` に無い profile は
  fail-closed で error
- **CLI**: `schneeforge profile show|set <name>|list` を追加。
  `set` は manifest 検証後に state へ保存
- **GUI**: Status (`get_status`) が current profile (state) と
  manifest default を返す。表示は既存 `profile` 行を流用

## Impact

- 既定動作は不変 (state に profile が無ければ manifest default の
  developer を注入 = 現行と同じ module 構成)
- `nix flake check` / template / e2e に影響なしの見込み
- specs: `core-operations` (profile 選択), `gui-diagnostics`
  (Status の profile 出典) を更新
