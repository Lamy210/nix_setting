# Project Context

## Purpose
SchneeForge (codename `nix_setting`) — Declarative Developer Workstation Manager。Nix / Home Manager / nix-darwin を用いて macOS / Linux の開発環境を宣言的に管理・再現する。Windows は experimental native CLI launcher から WSL2 Linux execution backend へ委譲して利用する。Nix をユーザーインターフェースではなく再現性エンジンとして利用し、CLI / Tauri GUI / flake apps の三面で統一 UX (Desired State → Plan → Apply → Verify → Rollback) を提供する。

## Tech Stack
- **Backend (Core)**: Rust 2021 (`crates/core/`)
- **CLI**: Rust + clap (`crates/cli/`)
- **Windows execution**: native Rust launcher + `wsl.exe` direct-argv delegation to matching Linux helper (experimental)
- **GUI**: Tauri 2 + TypeScript (`apps/desktop/`)
- **Nix**: flakes + flake-parts, Home Manager, nix-darwin
- **Distribution**: GitHub Releases (binaries + DMG), Homebrew tap (`Lamy210/homebrew-tap`), flake apps/templates
- **CI**: GitHub Actions (cargo / nix / shellcheck / treefmt / bats / openspec validate; non-required `windows-2025` portability lane)
- **Spec-driven**: OpenSpec (`openspec/`)

## Project Conventions

### Code Style
- Rust: `cargo fmt` + `cargo clippy -D warnings`
- Nix: `nixfmt` via treefmt, statix / deadnix lint
- Shell: shellcheck (strict), shfmt
- YAML / JSON: prettier
- Conventional Commits (`feat:` / `fix:` / `chore:` / `docs:` / `refactor:`)

### Architecture Patterns
- Core / CLI / Desktop の 3 層分離。CLI と Desktop は Core の thin adapter。
- Windows では launcher host と execution backend を分離し、Nix/repo/tool/state operation は WSL2 Linux helper 側が所有する。Windows を Nix `system` として扱わない。
- `Toolchain` 解決済みの絶対パスを全操作で使う (`tool.rs` / `scripts/resolve-tools.sh` の探索順序を共有)。
- `StateStore` (`state.rs`) は cross-process flock で保護。
- `Receipt` は upstream (`/nix/receipt.json`) を source of truth とし、SchneeForge 側で複製しない。
- ADR は `docs/adr/` で管理 (lightweight ADR / Michael Nygard 形式)。

### Testing Strategy
- Rust: `cargo test --all`
- Shell: bats (`tests/*.bats`)
- Smoke: Docker container (Linux x86_64) / disposable macOS aarch64
- Windows: `windows-2025` で workspace/core/CLI compile、WSL bridge contract、native launcher smokeを検証。operational Nix testsはWSL/Linux execution-sideの契約としてWindows nativeでは実行しない。
- OpenSpec: `openspec validate <change-id> --strict` + `openspec validate --all --strict`
- CI gate: lint / fmt / clippy / cargo test / nix flake check / openspec validate。Windows laneはexperimental期間中non-required。

### Git Workflow
- `develop` が開発統合先。topic branch (`feat/*`, `fix/*`, `refactor/*`, `docs/*`, `test/*`, `chore/*`) は `develop` から切り、PR で **squash merge** する。
- `main` / `develop` は直接 push 禁止 (branch protection)。1 PR = 1 concern を維持する。
- リリースフロー: `develop` → `release/vX.Y.Z` → `main` PR は **merge commit** → tag push → release workflow → `main` → `develop` back-merge PR も **merge commit**。
- release / back-merge に squash または rebase merge を使用しない。過去の diverged history を直すための force push / history rewrite も行わない。
- OpenSpec change は proposal/design/delta spec/tasks → strict validation → proposal approval → 実装 → 実装 PR を `develop` へ merge → **別 `chore/archive-*` PR で archive + main spec sync** の順に進める。
- tooling-only change で main spec を更新しない archive のみ `openspec archive <change-id> --skip-specs --yes` を許可する。
- CI required-check migration は新しい aggregator を既存 required checks と並行稼働させ、安定確認前に既存 contexts を削除しない。

## Domain Context
- macOS は APFS Volume に Nix store を置く (nix-darwin 標準構成)。
- macOS 15 Sequoia が `_nixbld` user を乗っ取る問題があり、SchneeForge doctor は `repair sequoia` を案内する。
- macOS で Nix を uninstall する前に nix-darwin を外さないと SSL cert が壊れる (nix-installer quirks)。SchneeForge は uninstall 順序を保証する。
- Linux は home-manager standalone + nix-darwin 不要の構成をサポート。
- Windows host は Nix execution environment ではなく、WSL2 Linux backend を選択・検証して委譲するcontrol-planeとして扱う。

## Important Constraints
- **License**: SchneeForge 本体は MIT。NixOS/nix-installer (LGPL-2.1) を subprocess で呼び出し (link 無)、Phase 1 では binary bundle 再配布を行わない (bundle は別 ADR / 法務設計)。
- **Cross-platform**: Nix execution platform は macOS aarch64 / x86_64-linux / aarch64-linux。Windows host support はexperimental WSL2 launcher経由。x86_64-darwin は未サポート。
- **Windows initial scope**: WSL2のみ。Windows Desktop/Tauri、native Windows Nix、WSL/helper自動導入、Windows release asset/self-update、暗黙path変換は未サポート。
- **Offline**: 初回 install は online 必須 (Managed Nix)。2 回目以降はアプリデータ配下キャッシュで offline 動作。
- **Stable/Edge**: install.sh / Homebrew formula は stable のみ。edge 利用者は flake 経由。

## External Dependencies
- **NixOS/nix-installer**: Managed Nix provider (ADR-0001)。version-pinned で GitHub Releases から取得。
- **NixOS/nixpkgs**: パッケージソース。
- **LnL7/nix-darwin**: macOS system management。
- **nix-community/home-manager**: dotfiles / per-user packages。
- **Microsoft WSL2**: experimental Windows hostのLinux execution backend。
- **Lamy210/homebrew-tap**: Homebrew formula 配布先 (本体 repo とは分離)。
- **Cloudflare**: docs / landing (将来)。
