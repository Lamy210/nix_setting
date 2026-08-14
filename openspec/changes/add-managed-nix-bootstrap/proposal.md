# Change: Managed Nix Bootstrap (NixOS/nix-installer 統合)

## Why

SchneeForge は Nix を前提とするが、fresh machine への Nix インストール手段を持たない。現在は `bootstrap-flow` 仕様が「curl -L https://nixos.org/nix/install | sh」の表示を定めるのみで、これには uninstall・receipt・plan が一切無く、doctor / rollback / ownership ledger の前提が崩れる。

ADR-0001 (provisionally accepted) で NixOS/nix-installer を default provider に選定した (Spike Report: `docs/spikes/2026-08-14-nix-bootstrap-provider-evaluation/spike-report.md`)。本 change は SchneeForge Core へ Managed Nix を実装し、CLI (`schneeforge nix install / doctor / uninstall`) で安定させた上で GUI (First Run Wizard) へ接続する。

## What Changes

- **ADDED capability: `managed-nix-bootstrap`**
  - `bootstrap-manifest.toml` による version + SHA256 pinning
  - `BootstrapDownloader` (reqwest, `tauri-plugin-http` 不使用) による online download とアプリデータ配下のキャッシュ
  - SLSA provenance + SHA256SUMS 検証 (release bump CI のみ, runtime は local SHA256 比較)
  - nix-installer subprocess 実行 (`--enable-flakes --logger json --no-confirm`)
  - stderr JSON Lines の best-effort parse (SchneeForge phase 優先, installer 内部メッセージ非依存)
  - `/nix/receipt.json` の読み取り専用 view
  - 2 段階 Plan UX (root 不要 preflight → 管理者認証 → detailed plan)
  - uninstall 順序保証 (nix-darwin → Nix)
- **MODIFIED `bootstrap-flow` の「Nix 未検出時のメッセージ」**
  - curl|sh 表示から Managed Nix インストールアクションへ切り替え
- **CLI 追加**: `schneeforge nix install / doctor / uninstall`
- **CI 追加**: upstream release 追跡と `bootstrap-manifest.toml` 自動 bump (別 workflow)

非対象 (Phase 2 以降):
- DMG bundle 配布 (LGPL 再配布の legal 設計が別 ADR)
- GUI Tauri IPC 接続 (CLI 安定後)
- offline 配布 (online キャッシュ再利用のみ Phase 1 対応)

## Impact

- **Affected specs**: `managed-nix-bootstrap` (new), `bootstrap-flow` (modified)
- **Affected code**:
  - `crates/core/src/managed_nix/{mod,provider,manifest,download,verify,installer,receipt}.rs` (new module)
  - `crates/cli/src/nix.rs` (new subcommand)
  - `bootstrap-manifest.toml` (repo root, new)
  - `.github/workflows/upstream-nix-installer.yml` (new, release bump CI)
- **Related ADR**: ADR-0001 (provisionally accepted)
- **Dependencies**: `reqwest` (Rust workspace dep 追加)
- **License**: LGPL-2.1 の nix-installer を subprocess 呼出 (link 無)。SchneeForge 側ライセンス (MIT) は維持。
