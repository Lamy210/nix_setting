## Why

SchneeForge には `ToolResolver` があるが、実ロジック（`actions.rs` / `operations.rs` / `bootstrap.rs`）は `Command::new("nix")` / `run_capture("nix", ...)` / `command_succeeds("nix", ...)` で文字列リテラルを直接 spawn している。そのため:

1. **macOS GUI (.app) 実機バグ**: Finder から起動した Tauri アプリは `launchd` 経由で spawn され login shell を経由しないため、`/etc/paths` / `/etc/paths.d/*` / `.zshrc` が読み込まれず、PATH が `/usr/bin:/bin:/usr/sbin:/sbin` 程度になる。その結果 `/nix/var/nix/profiles/default/bin/nix` 等が見えない。Tauri 公式は [`fix-path-env-rs`](https://github.com/tauri-apps/fix-path-env-rs) で `main()` の冒頭 where `fix_path_env::fix()` を呼ぶ解を提示しているが、SchneeForge desktop は未導入。加えて `diagnostics` は `ToolResolver` 経由なので Nix を検出できるが、`preflight` / `plan` / `apply` / `rollback` / `upgrade` / `sync` は素の `Command::new("nix")` を使うため、**Diagnostics が OK でも Apply が "nix: not found" で失敗する**。
2. **Diagnostics と操作で別の Nix を使う**: `ToolResolver` が `/nix/var/nix/.../bin/nix` を返しても、apply は PATH の先頭にある別 nix（存在すれば）を使う。結果として「検証したバイナリと実行したバイナリが違う」状態が生じうる。
3. **Flakes 検出が素の nix 呼び出し**: `preflight` は `command_succeeds("nix", ...)` で判定するため、(1) と同じ PATH 問題に加え、Nix 未検出と flakes 無効を同一視する。
4. **install.sh / bootstrap.sh も `command -v nix` のみ**: Rust 側の known-paths 探索と shell 側で探索ルールが異なる。第三者が GUI と CLI の両方を使うと、GUI で見えて CLI で見えない（その逆も）という謎な挙動になる。
5. **Health Check が貧弱**: Diagnostics の `nix: ToolStatus { available, path, version }` だけでは、store 接続・flakes 有効性・出処（system profile / home profile / Homebrew）が分からない。トラシューに必要な情報が揃わない。

P0-1〜P0-5（ToolResolver 強化 / 全操作の Resolved Tool 化 / Nix Health Check / Flakes 検出修正 / shell installer 探索統一）は、すべてこの「解決済みパスを全操作で使う」という単一の不変条件に帰着する。よって1つの change として扱う。

## What Changes

### Tauri desktop の PATH 修正（P0-1・前提）

- [tauri-apps/fix-path-env-rs](https://github.com/tauri-apps/fix-path-env-rs) を導入し、`apps/desktop/src-tauri/src/main.rs` の `main()` 冒頭（`tauri::Builder` より前）で `fix_path_env::fix()` を呼ぶ。これにより `/etc/paths` / `/etc/paths.d/*` 相当の PATH が整う。
- これは macOS の launchd 起動に起因する PATH 欠損への**一次対応**であり、以下の ToolResolver 強化とは直交するが併用する。`fix-path-env-rs` は Nix 固有のパス（`/nix/var/nix/profiles/default/bin` 等）を知らないため、ToolResolver で fallback する二段構えとする。

### ToolResolver の強化（P0-1）

- `ResolvedTool` 型を新設: `path: PathBuf` + `source: ToolSource`（EnvOverride / Path / XdgStateProfile / NixProfileEnv / NixProfileHome / PerUserProfile / SystemProfile / Homebrew）。available / unavailable を型で表現する。
- `Toolchain` 型を新設: `nix: ResolvedTool` + `git: ResolvedTool` + `homebrew: Option<ResolvedTool>` + `nh: Option<ResolvedTool>`。1回の解決で全ツールを確定し、以降の全操作はこの Toolchain を使う。
- 探索優先度（要件を満たす順序）。Nix 2.x 以降の XDG ベース遷移（[home-manager#4403](https://github.com/nix-community/home-manager/issues/4403)）と、`/nix/var/nix/profiles/per-user/<user>` が root 専用になった点を反映:
  1. `SCHNEEFORGE_NIX_BIN`（テスト用 override、本番では通常未設定）
  2. PATH（`fix-path-env-rs` で補正済みの前提）
  3. `$XDG_STATE_HOME/nix/profile/bin`（設定時）
  4. `~/.local/state/nix/profile/bin`（XSG_STATE_HOME 未設定時の既定）
  5. `$NIX_PROFILE/bin`（設定時）
  6. `~/.nix-profile/bin`
  7. `/etc/profiles/per-user/$USER/bin`
  8. `/nix/var/nix/profiles/default/bin`
  9. `/opt/homebrew/bin` / `/usr/local/bin`（Homebrew）
- 注記: Nix installer は `$XDG_STATE_HOME/nix/profiles` を**自動作成しないことがある**（Home Manager が失敗する有名な罠）。SchneeForge 側でフォルダ欠如を検出した場合は診断/案内する。
- `canonicalize` でシンボリックリンクを解決し、実体パスを保持する。
- version 取得は `<resolved_path> --version` の subprocess で行う。

### 全操作の Resolved Tool 経由化（P0-2）

- `process::run_stream` / `run_capture` / `command_succeeds` を、`&str` ではなく `&ResolvedTool`（または `&Path`）を受け取るシグネチャへ変更。
- `actions::apply` / `rollback` / `upgrade` / `operations::plan` / `sync` / `bootstrap::preflight` / `setup` は `Toolchain` を受け取り、`Command::new("nix")` 等の文字列リテラル spawn を完全に排除する。
- `diagnostics::diagnose` が `Toolchain` を構築し、以降の操作（GUI の Plan / Apply / Verify 等）は同じ Toolchain を再利用する。同一プロセス内で解決結果が揺れないことを保証する。
- CLI（`crates/cli/src/main.rs`）も `Toolchain` を1回解決して全サブコマンドへ伝搬する。
- 再発防止の静的チェック: CI で `crates/` 配下の `Command::new("nix"` / `run_capture("nix"` / `run_stream("nix"` / `command_succeeds("nix"` を grep し、`tool.rs` 内の resolver 実装以外で見つかったら fail する。

### Nix Health Check（P0-3）

- `NixHealth` 型を新設: `installed: bool`, `executable: Option<PathBuf>`, `version: Option<String>`, `store_accessible: bool`, `flakes_available: bool`, `source: Option<ToolSource>`, `error: Option<String>`。
- store 接続確認は `<nix> store ping`（[Nix Reference Manual](https://nix.dev/manual/nix/2.18/command-ref/new-cli/nix3-store-ping)）の exit code で判定する。
- flakes 有効判定は `<nix> flake --version` ではなく、`<nix> config show experimental-features` を実行して出力に `flakes` が含まれるか parse する（[Nix conf-file](https://nix.dev/manual/nix/2.18/command-ref/conf-file) で規定）。`flake --version` は flakes 無効でも通る場合があり不正確なため却下。
- `Diagnostics` に `nix_health: NixHealth` を追加し、従来の `tools.nix: ToolStatus` は deprecated ではなく alias として残す（GUI の serialize 互換）。

### Flakes 検出バグ修正（P0-4）

- `PreflightReport` を分離: `nix_installed: bool` / `flakes_enabled: bool` を別状態にする。現在の `nix: bool` と統合されているのを分離。
- Nix 未検出の場合は「Nix をインストールしてください」、Nix 検出済みで flakes 無効の場合は「flakes を有効化してください（[Enable Flakes]）」と別メッセージを出す。
- flakes 有効化処理（`enable_flakes`）は resolved nix を使って `nix experimental-features` を検査し、必要なら `nix.conf` へ追記する。GUI から呼ぶ場合は enable → 再診断のフローを提供する。

### install.sh / bootstrap.sh の探索統一（P0-5）

- shell 側に `resolve_nix()` 関数を新設: `SCHNEEFORGE_NIX_BIN` → `$NIX_PROFILE/bin` → `$XDG_STATE_HOME/nix/profile/bin` → `~/.local/state/nix/profile/bin` → `~/.nix-profile/bin` → `/etc/profiles/per-user/$USER/bin` → `/nix/var/nix/profiles/default/bin` → `command -v nix` の順で探索し、見つけた絶対パスを `NIX_BIN` として export する。
- `install.sh` は `resolve_nix` で Nix を発見したら再インストールをスキップする（現状は `command -v nix` だけで判定しているため、Finder 起動の GUI から呼ばれた際に Nix が見えない問題と同じ原因で再インストールを試みる危険がある）。
- `bootstrap.sh` も同じ `resolve_nix` を使う。
- shell 側の flakes 判定も `$NIX_BIN flake --help` で行い、文字列 `nix` を直接呼ばないようにする。
- `install.sh` / `bootstrap.sh` / 共通関数（`scripts/resolve-tools.sh` に切り出す）で探索ルールを1箇所に集約する。

## Capabilities

### New Capabilities

- `tool-resolution`: `ResolvedTool` / `Toolchain` / `ToolSource` データモデル、CI による文字列リテラル spawn 防止、shell 側 `resolve_nix` 共通化
- `gui-diagnostics`: `NixHealth` 情報（store 接続 / flakes 有効 / source）

### Modified Capabilities

- `tool-resolution`: 探索順序の明文化（XDG state / NIX_PROFILE / per-user profiles 追加）、canonicalize 義務付け
- `core-operations`: 全操作が `Toolchain` を受け取るシグネチャ変更、`Command::new` の文字列リテラル禁止
- `bootstrap-flow`: `PreflightReport` の nix_installed / flakes_enabled 分離、shell installer の resolve_nix 共通化

## Impact

### コード変更

- `apps/desktop/src-tauri/Cargo.toml`: `fix-path-env-rs` 依存追加
- `apps/desktop/src-tauri/src/main.rs`: `fix_path_env::fix()` を `main()` 冒頭で呼ぶ
- `crates/core/src/tool.rs`: `ResolvedTool` / `Toolchain` / `ToolSource` 追加、`ToolResolver` の探索ロジック強化
- `crates/core/src/process.rs`: `&str` → `&ResolvedTool` / `&Path` へのシグネチャ変更
- `crates/core/src/actions.rs`: 全関数が `&Toolchain` を受け取る
- `crates/core/src/operations.rs`: 全関数が `&Toolchain` を受け取る
- `crates/core/src/bootstrap.rs`: `PreflightReport` 分離、`setup` が `Toolchain` を受け取る
- `crates/core/src/diagnostics.rs`: `NixHealth` 追加、`diagnose` が `Toolchain` を構築
- `crates/cli/src/main.rs`: 起動直後に `Toolchain` を解決して全サブコマンドへ伝搬
- `apps/desktop/src-tauri/src/lib.rs`: 起動直後に `Toolchain` を解決（キャッシュ）、IPC ハンドラが同じ Toolchain を使う
- `scripts/resolve-tools.sh`（新設）: shell 共通関数
- `install.sh` / `bootstrap.sh`: `resolve-tools.sh` を source して使う

### テスト

- `tool.rs`: `ResolvedTool` / `Toolchain` の単体テスト、探索順序のテスト、canonicalize のテスト
- `process.rs`: `&Path` 受け取りのテスト
- 既存テストのシグネチャ追従（`actions` / `operations` のテストも `Toolchain` を渡すように）
- shell テスト: `shellcheck` / `bash -n` に加え、`resolve_nix` の探索順序を bash unit（bats 相当）で検証するか、最小の assert スクリプトを追加

### CI

- `.github/workflows/`: `actions.rs` / `operations.rs` / `bootstrap.rs` / `process.rs` 等に `Command::new("nix"` 等の文字列リテラル spawn が無いか grep するジョブ追加
- 既存の cargo test / clippy / fmt / nix flake check は維持

### 互換性

- CLI / GUI の公開 API（サブコマンド名、IPC コマンド名）は変更しない。内部シグネチャのみ変更。
- `Diagnostics.tools.nix` フィールドは維持（GUI の serialize を壊さない）。`nix_health` フィールドを追加で拡張。
- State ファイル（`state.json`）のスキーマは変更しない。

### リスク

- shell 側 `resolve_nix` の探索順序が Rust 側と厳密一致しないと、GUI と CLI で挙動が分かれる。仕様ドキュメントで1箇所に集約し、両者が同じ順序を参照するようにする。
- `XDG_STATE_HOME` が未設定の場合のデフォルト（`~/.local/state`）の扱いを Rust / shell で一致させる。
