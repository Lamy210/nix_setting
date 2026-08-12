# SchneeForge: Developer Workstation Manager 仕様書

**Status:** Proposal  
**Target:** `nix_setting` を「dotfiles リポジトリ」から「Declarative Developer Workstation Manager」へ発展させる

## 最重要方針

> **Nix をユーザーインターフェースにしない。Nix を再現性エンジンとして利用する。**

## 製品定義

```
Nix GUI でも、dotfiles installer でもない。
"Declarative Developer Workstation Manager"
```

統一 UX:

```
Desired State → Plan → Apply → Verify → Rollback
```

## 現在のギャップ

| 観点 | 現状 | 目標 |
|------|------|------|
| Host/Profile | 具体値が混在 (lamy210, /Users) | 分離 (module → profile → platform → host → user) |
| 設定入力 | Nix 式直接編集 | Product Manifest (TOML) |
| 導入 | clone + bootstrap.sh | installer / CLI / flake |
| 更新 | 手動 | sync / upgrade / self-update |
| 状態管理 | なし | State + Receipt (ownership ledger) |
| 配布 | GitHub Release 0件 | flake apps/templates + release |

## アーキテクチャ (既存維持 + 追加)

```
Presentation (CLI / TUI / GUI)
        ↓
   Product Core (Rust)
        ↓
   Policy Engine
        ↓
  Backend Adapters
  ├── Nix / HM / Darwin
  ├── Homebrew / Cask / MAS
  └── Project (devenv)
```

## フェーズ

### Phase A: 現行の健全化 (今回の P0/P1) — 完了

- LSQuarantine 削除、nh コマンド統一、Rust ownership 整理
- CI permissions/concurrency、template devShell、linux-arm eval

### Phase B: Host/Profile 分離 (次の一手)

- Host と Profile を分離し、username/homeDirectory を generic 化
- Product Manifest (TOML) 導入、JSON Schema 検証
- Homebrew apply の idempotency 修正 (autoUpdate/upgrade/cleanup 見直し)

### Phase C: State + Receipt + Doctor

- State Model (Current/Desired/Remote/Applied)
- Receipt (Ownership Ledger + Transaction Record)
- Discovery Engine (既存 Nix/Homebrew/dotfiles 検出)
- `doctor` / `doctor --fix`

### Phase D: CLI + flake 公開

- flake `templates` export (`nix flake init -t github:Lamy210/nix_setting#rust`)
- flake `apps` (`#doctor`, `#apply`, `#update`)
- standalone Rust CLI (dist で release)

### Phase E: Release / Installer

- GitHub Release + checksum + SBOM + provenance
- macOS DMG (Tauri 2) / Linux CLI
- self-update

## ADR (実装前に決定)

| # | 事項 | 推奨 |
|---|------|------|
| ADR-001 | Nix backend | configurable + existing 優先 |
| ADR-002 | macOS installer | DMG first / PKG enterprise |
| ADR-003 | Manifest format | TOML |
| ADR-004 | Desktop framework | Tauri 2 |
| ADR-005 | Binary cache | Cachix first |

## Release Blocker (v0.1)

- clean install / existing Nix preserve / Homebrew preserve
- dotfile backup / idempotency / failed-build-no-switch
- rollback / uninstall / adopted-Nix-not-deleted
- secret leak / signed / notarized / provenance / doctor

## 実装順序

1. Host/Profile 分離
2. Manifest schema
3. Rust workspace (core/cli)
4. Discovery → State/Receipt → Planner → Adapter
5. flake templates/apps export
6. CLI release
7. (将来) TUI / localhost UI / Tauri Desktop

## 次の設計書 (個別詳細化)

1. `MANIFEST_SCHEMA.md`
2. `STATE_AND_RECEIPT.md`
3. `BACKEND_ADAPTERS.md`
4. `INSTALLER_DESIGN.md`
5. `SECURITY_MODEL.md`
