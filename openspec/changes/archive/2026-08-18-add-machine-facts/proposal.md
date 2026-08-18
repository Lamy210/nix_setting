# Change: MachineFacts による machine 依存の repo 外部化 (v2 P0)

## Why

SchneeForge は v0.2 まで configuration repo (config.toml) に
machine 情報 (`user.username`) を持ち、hosts/*/default.nix が
`builtins.fromTOML (readFile ../../config.toml)` で直接読んで
`home.username` / `home.homeDirectory` へ埋め込んでいる。

これは「配布可能な Nix workstation distribution」の設計として
以下の問題を持つ:

1. **Configuration と machine 情報の混合**: repo を fork しても
   `config.toml` の username を書き換えないと他人の PC で動かない。
   Manifest::validate の「username 空 + 実行ユーザー一致」検証は
   この混合を緩和するための防御であり、分離すれば不要になる
2. **host 名の個人化**: `macbook-air` という特定端末名が
   flake configuration 名 (distribution の公開 API) になっている
3. **GUI 初回設定での入力負荷**: wizard が username 入力を求め、
   生成物が repo を書き換える (repo を書き換えない原則に反する)

v2 設計 (「Easy by default, Git-native when desired」) では
machine 依存を **Rust 側 (MachineFacts) で検出し、Nix へは
生成した machine input file を `--override-input` で注入する**。
評価は pure のまま (builtins.getEnv 不採用)、repo は誰の
machine でもそのまま動く。

## What Changes

- **ADDED: MachineFacts 検出 (core)**
  - `MachineFacts { username, home_directory, os, architecture, hostname }`
    を core が自動検出 (実行 user / HOME / uname 相当)
  - GUI / doctor は検出結果を表示 (手入力させない)
- **ADDED: machine input 注入**
  - flake.nix に `inputs.machine = { url = "path:./defaults/machine.nix"; flake = false; }`
    を追加 (placeholder を repo に同梱 → clone 直後も評価可能)
  - core が apply 時に state dir
    (`~/.local/state/schneeforge/machine.nix`) へ facts を生成し
    `--override-input machine <path>` で注入
  - path input は lock に commit 済み hash が無くても運用できる
    (override 時は lock 未使用)
- **REMOVED: config.toml の `[user]`**
  - schema=1 の `user.username` は廃止。`schneeforge.toml`
    (distribution manifest) への置換は後続 change で行うため、
    この change では「username を読まない」状態にするのみ
- **MODIFIED: hosts 一般化**
  - `hosts/macbook-air` → `hosts/darwin-aarch64` (flake
    configuration 名も同様)。linux-generic は現状維持
  - machine model (MacBook Air 等) と platform を分離
- **MODIFIED: bootstrap (First Run) の username 入力廃止**
  - wizard の config 生成 step を MachineFacts 検出表示へ置換
  - repo に対する file 書き込みをしなくなる

## Impact

- **Specs**: core-operations / bootstrap-flow
- **Code**: crates/core (machine.rs 追加, bootstrap.rs, discovery.rs,
  operations.rs) / hosts/ / flake.nix / GUI wizard
- **既存 user への影響**: `macbook-air` configuration 名で
  `darwin-aarch64` への rename が入る (migration note を doctor に表示)
- **非互換**: config.toml の username は無視される。手で
  config.toml を編集していた user は machine.nix 生成により
  自動移行 (username = 実行 user)

## Section

implementation
