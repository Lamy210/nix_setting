# Tasks

## 1. core: embedded manifest

- [x] 1.1 `managed_nix/mod.rs`: `BootstrapManifest::parse(include_str!(repo root の bootstrap-manifest.toml))` による `ManagedNix::embedded()` を追加
- [x] 1.2 manifest 解決 helper (`load_prefer_repo(repo: Option<&Path>)` 等): repo file 優先 → embedded fallback。CLI `nix_cmd.rs` と desktop (`load_from_repo` 呼び出し) を共通 helper へ寄せ
- [x] 1.3 unit test: repo file 優先 / repo 無しで embedded / embedded の内容が repo の file と同じ schema で parse できること

## 2. install.sh: fresh 経路の managed 化

- [x] 2.1 flow 分岐: `$REPO_DIR/.git` 存在時は従来 flow (clone skip + bootstrap.sh) を丸ごと維持
- [x] 2.2 fresh 経路: clone 削除。`fetch_schneeforge_binary` の binary を apply 完了まで保持 (`install_managed_nix` の user 側 binary 削除を apply 後へ移動)
- [x] 2.3 fresh 経路: `install_managed_nix` へ `NIX_SETTING_DIR` を渡さない (embedded manifest で動作)
- [x] 2.4 dotfile backup を bootstrap.sh から移植 (fresh 経路のみ)
- [x] 2.5 fresh 経路: `"$sf_bin" source init --tag "$SCHNEEFORGE_BOOTSTRAP_REF"` → `"$sf_bin" apply` (user 権限、curl|bash 時の /dev/tty 対策)
- [x] 2.6 step 表示 ([1/4] 等) と message の整合

## 3. bats: install-sh.bats

- [x] 3.1 fresh 経路で clone が呼ばれないこと (stub git で検証)
- [x] 3.2 fresh 経路で `source init --tag <pin>` が呼ばれること (stub sf binary で引数検証)
- [x] 3.3 既存 checkout は bootstrap.sh 経路を維持すること
- [x] 3.4 stable URL pin 検査 (README / `SCHNEEFORGE_BOOTSTRAP_VERSION` 一致) が引き続き green

## 4. test / CI

- [x] 4.1 `cargo test` / `cargo clippy -D warnings` / `cargo fmt` green (local)
- [x] 4.2 shellcheck (install.sh) — local `nix run nixpkgs#shellcheck` または CI
- [x] 4.3 openspec validate green (@fission-ai/openspec@1.8.0)
- [ ] 4.4 bats / bootstrap-test / flake-check / desktop rust-check は CI で green
- [ ] 4.5 PR 作成 (base: develop。stack 元の PR #56 merge 後に rebase して提出)
