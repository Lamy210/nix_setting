# Proposal: install.sh の managed source 化 (checkout 表現の新規導入廃止, v2 §7 後続)

## Why

v2 §7 (PR #54) で Release source の managed 表現 (flake ref + working
tree-less) が導入されたが、**install.sh の fresh install は依然として
`git clone --branch <tag>` で checkout 表現を作る**。その結果:

- 新規 user が 2 表現の古い方 (checkout) から始まり、`schneeforge update`
  のたびに移行 hint を見ることになる
- install.sh が Managed Nix install のために **manifest 読み込み先行の
  clone を必要とする** (`NIX_SETTING_DIR` → `bootstrap-manifest.toml`)。
  この順序制約が fresh install flow の複雑化の元凶
- fresh machine に不要な working tree (~repo 全体) が残る

§7 の理想形「nix が GitHub から直接 tag pinned で取得」を新規導入でも
成立させ、2 表現佷存を「既存 user の互換のみ」に縮小する。

## What Changes

- **core**: `ManagedNix` が `bootstrap-manifest.toml` を binary に embed
  (`include_str!`)。`nix install` の manifest 解決は repo file 優先 +
  embedded fallback — fresh machine (repo 無し) でも manifest が解決できる
- **install.sh**: repository が既に存在する場合は従来 flow
  (clone skip + `bootstrap.sh`) を維持し、**fresh install は clone しない**:
  1. CLI binary を release asset から検証付き取得 (既存 `fetch_schneeforge_binary`)
  2. Nix 未検出なら Managed Nix install (embedded manifest で動作)
  3. dotfile backup (従来 `bootstrap.sh` 相当を install.sh 内へ)
  4. 取得済み CLI で `schneeforge source init --tag <pin>` (managed source 化)
  5. 同 CLI で `schneeforge apply` (flake ref から build、darwin は
     darwin-rebuild が内部で sudo を要求)
- **bats**: fresh 経路の「clone しない」「source init が pin tag を使う」
  「既存 checkout は bootstrap.sh 経路を維持」を検証。stable URL pin 検査
  は不変
- **bootstrap.sh**: 既存 checkout / dev 用にそのまま残存 (Linux aarch64 の
  手動経路案内も不変)

## Impact

- 既存 user (checkout 表現) は無影響: install.sh 再実行時も既存 checkout を
  使う、update の移行 hint も従来通り
- 「1 release = 1 source tree = 1 checksum set」は強化される — binary と
  embedded manifest が同一 tag から build されるため
- GUI wizard の clone flow (`run_clone_repo`) は本 change の scope 外
- release artifact / version bump 運用に変更なし (`SCHNEEFORGE_BOOTSTRAP_REF`
  / `_VERSION` の 10 箇所 bump は不変)
