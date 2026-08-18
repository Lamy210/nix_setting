# Design: Profile 選択の flake 注入

## Context

- flake は既に machine input を `path:./defaults/machine.nix` +
  `--override-input` で注入する構成 (PR #43)。これと同じ pattern を
  profile に適用する
- host module (`hosts/darwin-aarch64/default.nix`,
  `hosts/linux-generic/default.nix`) が `profiles/developer.nix` を
  hard-code しているのが置き換え対象

## Goals / Non-Goals

- Goals: manifest で宣言した profile を user が選択でき、apply 時に
  反映されること。repo を書き換えないこと。既定では現行と同じ
  developer が入ること
- Non-Goals: GUI での profile 切替 UI (後続 change)、profile 固有の
  override 機構、custom profile の動的定義

## Decisions

### D1: flake input `profile` は path input + placeholder `null`

`defaults/machine.nix` と同じ構成。placeholder を `null` にすることで
「override なし = default (developer)」を表現する。host module 側は

```nix
profile-input = if isNull inputs.profile-imported then ../../profiles/developer.nix else ...
```

ではなく、`lib.importProfile` helper (modules/profile-input.nix) で
`null` → developer への fallback を一元化する。

### D2: profile の選択状態は state.json に保存

MachineFacts と違い profile は「検出」できない「選択」なので state
(`~/.local/state/schneeforge/state.json`) に持つ。`profile:
Option<String>` を追加 (serde default で旧 JSON 互換)。

### D3: 注入は profile.nix file 経由

`--override-input` は path input に対して行うため、state dir に

```nix
# ~/.local/state/schneeforge/profile.nix (生成物)
../../<repo>/profiles/<name>.nix を import した結果
```

ではなく **repo 内 path を文字列として返す** のは eval 時の CWD
問題がある。代わりに machine.nix と同様、Rust 側で
`{ profile = "<name>"; }` の attribute set を生成し、flake 側の
helper が `profiles/${name}.nix` を import する。

```nix
# modules/profile-input.nix
{ inputs, ... }:
let
  selected = import inputs.profile; # { profile = "developer"; }
in
  if builtins.pathExists (./../profiles + "/${selected.profile}.nix")
  then import (./../profiles + "/${selected.profile}.nix")
  else throw "unknown profile: ${selected.profile}"
```

※ `builtins.pathExists` で存在確認して throw することで、Rust 側の
検証 (manifest available) と二重防御にする。

### D4: default は manifest、fallback chain は state → manifest default

`resolve_profile()`:
1. state.profile があればそれ (manifest available に含まれること)
2. 無ければ manifest の profiles.default
3. manifest が読めなければ error (fail-closed。developer 決め打ち
   はしない — manifest が source of truth)

### Risks / Trade-offs

- `--override-input` が 2 組 (machine + profile) になる。args 構築は
  helper に集約済みなので影響は小さい
- profile file が repo に無い場合、eval 時に throw される。Rust 側で
  事前に file 存在 + manifest 検証するため通常は到達しない

## Migration Plan

- state.json に profile が無い全 user は現状通り developer。
  破壊的変更なし
- openspec は ADDED requirement (core-operations) + MODIFIED
  (gui-diagnostics) で進める

## Open Questions

なし (§17 の minimal/developer 2 profile 構成は PR #46 で確定済み)
