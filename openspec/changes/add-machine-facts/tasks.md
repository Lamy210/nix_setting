# Tasks

## 1. core: MachineFacts

- [x] 1.1 `crates/core/src/machine.rs` を新設: `MachineFacts { username, home_directory, os, architecture, hostname }` と `detect()` (実行 user / HOME / platform・arch は既存 discovery.rs の検出を再利用 / hostname は環境変数 HOSTNAME が無ければ `hostname` command)
- [x] 1.2 検出失敗 (username 空 / HOME 空) は error。空文字の継続は禁止
- [x] 1.3 `machine.nix` 生成関数: facts から attribute set 形式の Nix 式を state dir (`~/.local/state/schneeforge/machine.nix`) へ出力。username の escape 処理
- [x] 1.4 unit test: detect の各項目 (injectable な環境で)、machine.nix 生成の snapshot、escape

## 2. nix: machine input

- [x] 2.1 `flake.nix` へ `inputs.machine = { url = "path:./defaults/machine.nix"; flake = false; }` を追加
- [x] 2.2 `defaults/machine.nix` placeholder を repo に commit (clone 直後の `nix flake check` が通る値。例: username = "schneeforge-user", homeDirectory は system 毎の慣例 path)
- [x] 2.3 `hosts/darwin-aarch64/default.nix` (旧 macbook-air) と `hosts/linux-generic/default.nix` を `builtins.fromTOML (readFile ../../config.toml)` から `inputs.machine` 参照へ変更。`home.username = machine.username; home.homeDirectory = machine.homeDirectory;`
- [x] 2.4 host rename に伴う flake 設定 (modules/flake-parts) の configuration 名更新 (`macbook-air` → `darwin-aarch64`)。`nixosConfigurations` 相当が無い場合は homeConfigurations/darwinConfigurations の key
- [x] 2.5 既存 `hosts/macbook-air/` directory の削除と git mv 履歴の保持
- [x] 2.6 `nix flake check` が placeholder のまま通ること (CI の flake-check job で保証)

## 3. core: 操作の注入

- [x] 3.1 `operations.rs` の apply / plan が `--override-input machine <state-dir>/machine.nix` を nix command へ渡す
- [x] 3.2 CLI 引数や CWD に依存せず state dir を解決 (既存 state.rs の dir 解決を再利用)
- [x] 3.3 apply 前に machine.nix を必ず再生成 (stale 検出は不要、常に上書き)
- [x] 3.4 unit test: 注入引数の構築 (args vector に `--override-input machine <path>` が含まれる)

## 4. core: config.toml 依存の除去

- [x] 4.1 `bootstrap.rs` の config.toml 生成処理を削除 (generate_config 系関数と test)
- [x] 4.2 `manifest.rs` の `[user]` parse を廃止。schema=1 は username 無しで parse 可に (既存 test の username 関連を更新)
- [x] 4.3 `manifest.rs` の「実行ユーザー一致」検証を削除 (machine 情報を読まなくなったため)
- [x] 4.4 repo root の `config.toml` を git rm。`.gitignore` に `config.toml` を追加 (旧 file が手元に残っていても無視)
- [x] 4.5 `discovery.rs` の `home_directory(username)` helper を削除し MachineFacts 側へ集約

## 5. CLI / GUI

- [x] 5.1 CLI: `schneeforge doctor` に MachineFacts 検出結果 (username / home / platform / arch / hostname) を表示
- [x] 5.2 CLI: 旧 configuration 名 `macbook-air` 指定時の migration note 表示 (`darwin-aarch64` への rename を案内)
- [x] 5.3 GUI wizard: username 入力 step を MachineFacts 検出表示へ置換 (invoke 先は既存 diagnostics command または新 command)
- [x] 5.4 GUI: 検出結果の型定義 (`MachineFactsSummary`) を frontend に追加
- [x] 5.5 GUI wizard の regression test (`wizard_reads_preflight_report_fields` 相当) を更新: username 入力が存在しないこと・machine 表示があること

## 6. test / CI

- [x] 6.1 `cargo test` 全 green (core / cli / desktop)
- [ ] 6.2 `nix flake check` green (placeholder machine.nix で)
- [x] 6.3 `tests/install-sh.bats` / `tests/managed-nix-contract.bats` が config.toml 依存の記述を更新して green
- [ ] 6.4 PR 作成 (base: develop)。7 required checks green を確認
