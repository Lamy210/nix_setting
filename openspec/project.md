# Project Context

## Purpose
SchneeForge (codename `nix_setting`) — Declarative Developer Workstation Manager。Nix / Home Manager / nix-darwin を用いて macOS / Linux の開発環境を宣言的に管理・再現する。Nix をユーザーインターフェースではなく再現性エンジンとして利用し、CLI / Tauri GUI / flake apps の三面で統一 UX (Desired State → Plan → Apply → Verify → Rollback) を提供する。

## Tech Stack
- **Backend (Core)**: Rust 2021 (`crates/core/`)
- **CLI**: Rust + clap (`crates/cli/`)
- **GUI**: Tauri 2 + TypeScript (`apps/desktop/`)
- **Nix**: flakes + flake-parts, Home Manager, nix-darwin
- **Distribution**: GitHub Releases (binaries + DMG), Homebrew tap (`Lamy210/homebrew-tap`), flake apps/templates
- **CI**: GitHub Actions (cargo / nix / shellcheck / treefmt / bats / openspec validate)
- **Spec-driven**: OpenSpec (`openspec/`)

## Project Conventions

### Code Style
- Rust: `cargo fmt` + `cargo clippy -D warnings`
- Nix: `nixfmt` via treefmt, statinix / deadnix lint
- Shell: shellcheck (strict), shfmt
- YAML / JSON: prettier
- Conventional Commits (`feat:` / `fix:` / `chore:` / `docs:` / `refactor:`)

### Architecture Patterns
- Core / CLI / Desktop の 3 層分離。CLI と Desktop は Core の thin adapter。
- `Toolchain` 解決済みの絶対パスを全操作で使う (`tool.rs` / `scripts/resolve-tools.sh` の探索順序を共有)。
- `StateStore` (`state.rs`) は cross-process flock で保護。
- `Receipt` は upstream (`/nix/receipt.json`) を source of truth とし、SchneeForge 側で複製しない。
- ADR は `docs/adr/` で管理 (lightweight ADR / Michael Nygard 形式)。

### Testing Strategy
- Rust: `cargo test --all` (98+ unit tests in core, 9 in cli)
- Shell: bats (`tests/resolve-tools.bats`)
- Smoke: Docker container (Linux x86_64) / disposable macOS aarch64
- OpenSpec: `openspec validate --strict`
- CI gate: lint / fmt / clippy / cargo test / nix flake check / openspec validate

### Git Workflow
- `develop` が開発統合先。feature ブランチ (`feat/*`, `fix/*`, `docs/*`) → PR → squash merge。
- `main` は直接 push 禁止 (branch protection)。`release/*` 経由のみ merge 可。
- リリースフロー: `release/vX.Y.Z` ブランチ → `main` へ PR → tag push → workflow 発火 → `develop` へ back-merge。
- OpenSpec change は proposal → 実装 → **archive (PR 前)** → PR の順 (archive 済みの change を specs/ へ反映した上で PR を出す)。

## Domain Context
- macOS は APFS Volume に Nix store を置く (nix-darwin 標準構成)。
- macOS 15 Sequoia が `_nixbld` user を乗っ取る問題があり、SchneeForge doctor は `repair sequoia` を案内する。
- macOS で Nix を uninstall する前に nix-darwin を外さないと SSL cert が壊れる (nix-installer quirks)。SchneeForge は uninstall 順序を保証する。
- Linux は home-manager standalone + nix-darwin 不要の構成をサポート。

## Important Constraints
- **License**: SchneeForge 本体は MIT。NixOS/nix-installer (LGPL-2.1) を subprocess で呼び出し (link 無)、Phase 1 では binary bundle 再配布を行わない (bundle は別 ADR / 法務設計)。
- **Cross-platform**: macOS aarch64 / x86_64-linux / aarch64-linux をサポート。x86_64-darwin は未サポート。
- **Offline**: 初回 install は online 必須 (Managed Nix)。2 回目以降はアプリデータ配下キャッシュで offline 動作。
- **Stable/Edge**: install.sh / Homebrew formula は stable のみ。edge 利用者は flake 経由。

## External Dependencies
- **NixOS/nix-installer**: Managed Nix provider (ADR-0001)。version-pinned で GitHub Releases から取得。
- **NixOS/nixpkgs**: パッケージソース。
- **LnL7/nix-darwin**: macOS system management。
- **Glanvia/home-manager**: dotfiles / per-user packages。
- **Lamy210/homebrew-tap**: Homebrew formula 配布先 (本体 repo とは分離)。
- **Cloudflare**: docs / landing (将来)。
