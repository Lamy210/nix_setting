# Product Core リファクタリング仕様書

**Status:** Proposal → Applying  
**Target:** 操作ロジックを schneeforge-core に集約し、設計書の核心原則へ回帰する

## 背景

設計書の最重要原則:
> GUI・CLI・TUI それぞれに Nix 実行ロジックを持たせない。Product Core のみが Backend 操作を理解する。

しかし現在、操作ロジックが **4箇所に散在** している。

```
crates/cli/src/main.rs          → apply/rollback/scan/verify (Rust)
apps/desktop/src-tauri/lib.rs   → run_apply/run_rollback (Rust, 重複)
modules/flake-parts/apps.nix    → doctor/apply/status/rollback (シェル, 重複)
bootstrap.sh                    → host 検出 (シェル, 重複)
```

## 問題点

| # | 問題 | 深刻度 |
|---|------|:---:|
| 1 | apply/rollback が4箇所重複 | 高 |
| 2 | `nix run nix-darwin`（unpinned）を3箇所で使用 | 高 |
| 3 | CLI が CWD 前提（config.toml / .#host） | 高 |
| 4 | host 検出が3重実装 | 中 |
| 5 | `detect_host` がコンパイル時定数でテスト不能 | 中 |
| 6 | ロジックが main.rs にありテスト不能 | 中 |
| 7 | flake apps（シェル）と Rust CLI が二重管理 | 中 |
| 8 | `applied_at` が UNIX 秒 | 低 |

## 解決方針

### 1. core::actions モジュール追加

操作ロジックを schneeforge-core に集約:

```
schneeforge-core/
├── discovery.rs   # detect_host (引数化してテスト可能に)
├── manifest.rs
├── state.rs
└── actions.rs     # NEW: apply / rollback / scan / verify
```

### 2. detect_host のテスト可能化

```rust
pub fn detect_host_for(os: &str, arch: &str) -> Host { ... }  // 純関数・テスト可能
pub fn detect_host() -> Host { detect_host_for(consts::OS, consts::ARCH) }
```

### 3. CLI / desktop / apps の委譲

```
CLI     → core::actions を呼ぶだけ
desktop → core::actions を呼ぶだけ
apps.nix → Rust CLI に委譲 (シェル廃止)
```

### 4. nix-darwin を pin に統一

`nix run nix-darwin` → `nh darwin switch`（flake の pin を使用）

## 完了条件

- [ ] apply/rollback/scan/verify が core::actions に集約
- [ ] detect_host が引数化され、OS/arch 全パターンのテストあり
- [ ] CLI / desktop が core::actions を呼ぶだけ
- [ ] apps.nix が Rust CLI に委譲
- [ ] `nix run nix-darwin` を全廃
- [ ] cargo test + nix flake check が green
