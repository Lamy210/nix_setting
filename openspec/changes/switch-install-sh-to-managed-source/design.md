# Design: install.sh の managed source 化

## D1: bootstrap-manifest.toml の embedded fallback (repo 優先)

`ManagedNix` に build 時 embed した manifest を持たせる:

```rust
pub fn embedded() -> Result<Self, ManagedNixError> {
    BootstrapManifest::parse(include_str!("../../../bootstrap-manifest.toml"))
}
```

解決順は **repo file 優先 → embedded fallback**:

- repo にある file を使う現行挙動 (dev / e2e の synthetic repo) を最優先で保持
- release binary は同一 tag の source tree から build されるため、embedded
  manifest は「その binary 自身の release の manifest」と一致する
  (release unit 保証の強化)
- fresh machine (repo 無し) は embedded で解決 — install.sh の clone 先行
  制約が消える

cargo は `include_str!` の依存 file 変更を tracking するため、manifest 更新は
binary の rebuild を自動的に誘発する (stale embed は起きない)。

採用しない案: raw.githubusercontent からの tag-pinned fetch (§7 の repo file
経路と同じ)。network 依存が install の critical path に 1 本増え、checksum
無しの取得になるため却下。embed は build 時点で固定され検証不要。

## D2: install.sh の flow 分岐 (既存 checkout は従来、fresh は managed)

```
resolve_git (従来通り)
if [ -d "$REPO_DIR/.git" ]; then
    → 既存 checkout: 現行 flow を丸ごと維持 (clone skip / bootstrap.sh)
else
    → fresh: clone しない
      1. sf_bin = fetch_schneeforge_binary (CHECKSUMS 検証済み, user 権限)
      2. Nix 未検出時: install_managed_nix (staging + TOCTOU 検証は現行のまま。
         manifest は CLI embedded 動作になるよう NIX_SETTING_DIR を渡さない)
      3. dotfile backup (bootstrap.sh 相当を install.sh に移植)
      4. "$sf_bin" source init --tag "$SCHNEEFORGE_BOOTSTRAP_REF"
      5. "$sf_bin" apply
fi
```

- `install_managed_nix` が user 側 binary を削除する現状は、sf_bin を apply
  まで使うため **apply 完了まで保持** するよう変更する (staging copy の
  検証・削除は現行のまま)
- `source init` の tag は binary と同一 release の `SCHNEEFORGE_BOOTSTRAP_REF`
  (stable URL との一致は bats の stable URL 検査が既に保証)
- `apply` は **user 権限**で実行する。macOS は darwin-rebuild が内部で sudo
  を要求する (現行 bootstrap.sh の `nix run ... darwin-rebuild` と同じ構図。
  install.sh は terminal 実行のため TTY がある)。root で wrap すると state
  dir が root 側に作られるため却下
- curl|bash の stdin 問題は `nix install` と同様に `/dev/tty` 対策を apply
  にも適用する

## D3: dotfile backup の移植

`bootstrap.sh` が行っている初回 apply 前の dotfile backup
(`hm-bak-<date>`) を fresh 経路の install.sh へ移植する。home-manager 導入で
既存 dotfile が衝突する初回のみの保険で、機能としては shell 6 行のため
CLI 側へは移さない (core の scope 肥大化を避ける)。

## D4: bootstrap.sh は残す

- 既存 checkout の再適用経路 (install.sh 既存 branch が呼ぶ)
- Linux aarch64 の手動 Nix 導入 user への案内先
- dev の `nix run` 直叩き debugger としての価値

廃止は managed 表現への移行が進んだ後の別 change で判断する。

## D5: 互換性と fail-closed

- state に source が既にある (再 install) 場合、`source init --tag` は
  同一 tag なら移行表示、別 tag なら上書き — §7 実装の挙動に従う
- `source init` / `apply` が失敗した場合、install.sh は従来同様 exit 1
  (fail-closed)。中断時の再実行は state 経由で冪等に再開できる
  (Nix 検出 → source init → apply の各 step が冪等)

## D6: scope 外

- GUI wizard (`run_clone_repo`) の managed 化 — 別 change
- checkout 表現の update 経路 (`CheckoutLatestTag`) の廃止 — 既存 user の
  移行が済むまで佷存 (deprecation は別 change)
- `bootstrap.sh` の廃止
