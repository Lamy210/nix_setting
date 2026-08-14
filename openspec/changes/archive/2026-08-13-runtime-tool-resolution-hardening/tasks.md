## 0. Tauri desktop の PATH 修正（前提・P0-1）

- [x] 0.1 `apps/desktop/src-tauri/Cargo.toml` に `fix-path-env-rs` 依存追加
- [x] 0.2 `apps/desktop/src-tauri/src/main.rs` の `main()` 冒頭で `fix_path_env::fix().ok()` を呼ぶ
- [x] 0.3 `tauri::Builder` より前に呼ばれることを確認（cargo test で検証可能なユニットテスト追加）
- [ ] 0.4 Linux / Windows では noop であることを確認（crate 側で吸収されるが、CI で cross compile確認）
      → macOS CI runner で desktop build smoke が通ることで間接検証済み。Linux/Windows での明示的検証は今後課題。

## 1. ToolResolver 強化（P0-1）

- [x] 1.1 `ToolSource` enum を定義（`EnvOverride` / `Path` / `XdgStateProfile` / `NixProfileEnv` / `NixProfileHome` / `PerUserProfile` / `SystemProfile` / `Homebrew`）
- [x] 1.2 `ResolvedTool` struct を定義（`path: PathBuf` / `source: ToolSource` / `version: Option<String>`）
- [x] 1.3 `Toolchain` struct を定義（`nix: ResolvedTool` / `git: ResolvedTool` / `homebrew: Option<ResolvedTool>` / `nh: Option<ResolvedTool>`）
- [x] 1.4 `ToolResolver::resolve_tool(name) -> Option<ResolvedTool>` を実装
- [x] 1.5 `Toolchain::resolve() -> Result<Self, ToolchainError>` を実装（一括解決）
- [x] 1.6 探索優先度を実装: `SCHNEEFORGE_<NAME>_BIN` env → PATH → `$XDG_STATE_HOME/nix/profile/bin` → `~/.local/state/nix/profile/bin` → `$NIX_PROFILE/bin` → `~/.nix-profile/bin` → `/etc/profiles/per-user/$USER/bin` → `/nix/var/nix/profiles/default/bin` → `/opt/homebrew/bin` → `/usr/local/bin`
- [x] 1.7 `canonicalize` で symlink を解決して実体パスを保持
- [x] 1.8 `<resolved_path> --version` で version を取得
- [x] 1.9 既存 `ToolStatus` を `ResolvedTool` への変換メソッド付きで残す（GUI serialize 互換）
- [x] 1.10 `~/.local/state/nix/profiles` フォルダ欠如（Nix installer が作成しない有名な罠）を検出した場合、`NixHealth.error` に案内を入れる（`ensure_nix_state_dir` で事前保証 + diagnostics で検出）
- [x] 1.11 `tool.rs` の既存テストを `ResolvedTool` ベースへ移行 + 探索順序の回帰テスト追加

## 2. process.rs のシグネチャ変更（P0-2 前準備）

- [x] 2.1 `run_stream(path: &Path, args: &[String]) -> Result<()>` へ変更
- [x] 2.2 `run_capture(path: &Path, args: &[String]) -> Result<String>` へ変更
- [x] 2.3 `command_succeeds(path: &Path, args: &[String]) -> bool` へ変更
- [x] 2.4 既存の `&str` 引数の呼び出しを `&Path` へ修正（actions / operations / bootstrap の全呼び出し箇所）

## 3. 全操作の Toolchain 経由化（P0-2）

- [x] 3.1 `actions::apply(target, repo, toolchain)` シグネチャ変更（`toolchain.nix.path` を使う）
- [x] 3.2 `actions::apply_captured(target, repo, toolchain)` 同上
- [x] 3.3 `actions::rollback(target, repo, toolchain)` シグネチャ変更
- [x] 3.4 `actions::rollback_captured(target, repo, toolchain)` 同上
- [x] 3.5 `actions::upgrade(repo, toolchain)` シグネチャ変更
- [x] 3.6 `actions::upgrade_captured(repo, toolchain)` 同上
- [x] 3.7 `operations::apply / rollback / upgrade / plan / sync` が `&Toolchain` を受け取る
- [x] 3.8 `operations::verify` が `&Toolchain` を受け取り、`which(cmd)` を `toolchain` 経由へ
- [x] 3.9 `actions::scan` が `&Toolchain` を受け取る
- [x] 3.10 `bootstrap::doctor` / `bootstrap::setup` が `&Toolchain` を受け取る
- [x] 3.11 `bootstrap::preflight` が `&Toolchain` を受け取る
- [x] 3.12 既存テストを `Toolchain` を渡すように修正

## 4. Nix Health Check（P0-3）

- [x] 4.1 `NixHealth` struct を定義（`installed` / `executable` / `version` / `store_accessible` / `flakes_available` / `source` / `error`）
- [x] 4.2 `nix_health(toolchain: &Toolchain) -> NixHealth` を実装
- [x] 4.3 store 接続確認は `<resolved_nix> store ping`（[Nix Manual](https://nix.dev/manual/nix/2.18/command-ref/new-cli/nix3-store-ping)）の exit code で判定
- [x] 4.4 flakes 有効判定は `<resolved_nix> config show experimental-features` を実行し、出力に `flakes` が含まれるか parse（[Nix conf-file](https://nix.dev/manual/nix/2.18/command-ref/conf-file)）。`flake --version` 方式は不正確のため採用しない
- [x] 4.5 `Diagnostics` に `nix_health: NixHealth` フィールドを追加
- [ ] 4.6 GUI が `nix_health` を表示できるようフロントエンド型定義を更新
      → バックエンド (`ToolchainSummary` / `NixHealth`) は Serialize 済み。フロント型更新は後続対応（GUI P1 変更で実施予定）。
- [x] 4.7 `NixHealth` の単体テスト（mock ではなく実際に nix を叩く integration test は `#[cfg(test)]` で skippable）

## 5. Flakes 検出バグ修正（P0-4）

- [x] 5.1 `PreflightReport` を `{ nix_installed: bool, flakes_enabled: bool, git_installed: bool }` へ分離
- [x] 5.2 `preflight(toolchain) -> PreflightReport` 実装（`command_succeeds(&toolchain.nix.path, ...)` を使用）
- [x] 5.3 Nix 未検出時のメッセージと、Nix 検出済み / flakes 無効のメッセージを分離
- [x] 5.4 `enable_flakes` が `&Toolchain` を受け取る
- [ ] 5.5 GUI フロントエンドが「Nix OK / Flakes NG」を正しく区別して表示
      → バックエンド側スキーマは分離済み。フロント表示は後続対応（GUI P1 変更で実施予定）。
- [ ] 5.6 enable flakes → 再診断フローを GUI で提供
      → 後続対応（GUI P1 変更で実施予定）。
- [x] 5.7 `PreflightReport` の既存テストを新スキーマへ移行

## 6. install.sh / bootstrap.sh の探索統一（P0-5）

- [x] 6.1 `scripts/resolve-tools.sh` を新設（`resolve_nix` / `resolve_git` / `resolve_brew` 関数）
- [x] 6.2 探索順序を Rust 側（`tool.rs` の `default_known_paths` 相当）と一致させる
- [x] 6.3 `install.sh` が `resolve-tools.sh` を source し、`resolve_nix` で発見した場合は再インストールをスキップ
- [x] 6.4 `bootstrap.sh` が `resolve-tools.sh` を source し、文字列 `nix` 直接呼び出しを排除
- [x] 6.5 shellcheck と `bash -n` を通す
- [x] 6.6 `resolve_nix` の探索順序テスト（最小の assert スクリプト or bats）→ `tests/resolve-tools.bats` (11 test cases)

## 7. CLI / GUI の Toolchain 伝搬

- [x] 7.1 `crates/cli/src/main.rs` が起動直後に `Toolchain` を解決し、全サブコマンドへ渡す（`with_toolchain` wrapper）
- [x] 7.2 `apps/desktop/src-tauri/src/lib.rs` が起動直後に `Toolchain` を解決し、IPC ハンドラで共有（`CachedToolchain` via `tauri::State`）
- [x] 7.3 Toolchain 解決失敗時のエラーハンドリング（Nix が見つからない場合は setup へ誘導）→ `doctor_fails_gracefully_without_nix` テストで検証

## 8. CI: 文字列リテラル spawn 防止（再発防止）

- [x] 8.1 `.github/workflows/check.yml` に lint ジョブ追加: `crates/` 配下で `Command::new("nix"` / `run_capture("nix"` / `run_stream("nix"` / `command_succeeds("nix"` / `Command::new("git"` 等の文字列リテラル spawn を grep（`lint` job の "forbid raw tool spawns" step）
- [x] 8.2 例外として `crates/core/src/tool.rs` の resolver 実装は許可（grep で除外）
- [x] 8.3 同様に shell 側の `nix` / `git` / `brew` 直接呼び出しを grep し、`scripts/resolve-tools.sh` 経由を強制（`$NIX_BIN` / `$GIT_BIN` / `$BREW_BIN` 以外を弾く）

## 9. 品質ゲート

- [x] 9.1 `cargo test` 全通過（core 98 + cli 9）
- [x] 9.2 `cargo clippy -- -D warnings` 通過
- [x] 9.3 `cargo fmt --check` 通過
- [ ] 9.4 `nix flake check` 通過 → CI の `flake-check` / `docker-check` job で検証（ローカル未実行）
- [x] 9.5 `openspec validate runtime-tool-resolution-hardening` 通過
- [x] 9.6 `openspec validate --all` 通過（6 items）

## 10. ドキュメント

- [ ] 10.1 `docs/STATUS.md` の「完成済み」「既知のデグレ」を更新（GUI Nix 検出バグ / Flakes 検出バグを解決済みへ）→ 直後のステップで対応
- [ ] 10.2 探索順序の仕様を `specs/tool-resolution/spec.md` の Purpose に集約（archive 時）
