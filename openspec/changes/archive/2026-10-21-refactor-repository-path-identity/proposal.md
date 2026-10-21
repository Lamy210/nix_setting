# Change: Preserve OS-native repository path identity

## Why

SchneeForge の local repository path は現在 `String` / `&str` として解決・伝播しており、`HOME` / `NIX_SETTING_DIR`、manifest 読み取り、git の working directory、Nix argv など複数の境界で filesystem path を UTF-8 text に強制している。

直近の path hardening で tool path・HOME-derived path・nix-darwin marker 等は OS-native path を維持するよう改善したが、repository path は `RepoResolver` を起点に core 全体へ String-based API が残っている。このため Unix 上で有効な non-UTF-8 repository path を通常の filesystem/core execution path が正しく表現できない。

repository filesystem identity と managed release の textual flake reference を型として分離し、filesystem operation から process argv まで OS-native path identity を保持する。

## What Changes

- **BREAKING**: `schneeforge-core` の local repository path API を `String` / `&str` から `PathBuf` / `&Path` ベースへ移行する。
- `Repo` / `RepoResolver` に path-native representation を導入し、`HOME` / `NIX_SETTING_DIR` を `var_os` で解決する。
- local checkout path と managed `github:<owner>/<repo>/<tag>` flake reference を typed boundary で分離する。
- manifest / profile / source resolution / operations / actions / git revision inspection が local repository を `&Path` として扱うようにする。
- Nix / subprocess argv の repository path を `OsString` として運べるよう process boundary を拡張する。
- Diagnostics は filesystem 判定には native path を使い、serialized `repo_path` は presentation boundary でのみ display string へ変換する。
- Windows native launcher / WSL2 delegation の既存 Unicode argv contract は変更しない。repo 未指定時は引き続き WSL2 Linux helper が Linux 側で解決する。
- GUI privilege escalation のように UTF-8 textual transport を必須とする既存境界は、non-UTF-8 repository path を lossy 変換せず fail-closed で拒否する。
- Unix integration tests で non-UTF-8 repository path の resolution / manifest / source / process path identity を検証する。

## Impact

- Affected specs:
  - `core-operations`
  - `gui-diagnostics`
  - `managed-nix-bootstrap`
- Primary affected code:
  - `crates/core/src/repo.rs`
  - `crates/core/src/manifest.rs`
  - `crates/core/src/profile.rs`
  - `crates/core/src/source.rs`
  - `crates/core/src/source_files.rs`
  - `crates/core/src/operations.rs`
  - `crates/core/src/actions.rs`
  - `crates/core/src/process.rs`
  - `crates/core/src/diagnostics.rs`
  - `crates/core/src/discovery.rs` (flake target construction boundary as needed)
  - `crates/cli/src/main.rs` / `legacy_main.rs`
  - `apps/desktop/src-tauri/src/lib.rs`
- Compatibility:
  - CLI command names, config precedence, source semantics, Diagnostics JSON field names, Windows WSL2 policy, and privilege-escalation fail-closed policy remain unchanged.
  - Rust callers of String-based repository APIs require migration in the same change.
