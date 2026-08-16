# Change: Nix 状態分類 (NixStatus) と doctor 診断の強化

## Why

`existing_nix_detected()` は `/nix/store` 等の installation marker で
fail-closed に「Nix が存在する」ことしか判定できない。部分削除された
degraded install (例: `/nix/store` のみ残存、receipt 欠損) からは
SchneeForge 単独で復旧手段を案内できず、install も拒否されるため
ユーザーは手動での調査を強いられる (issue #15)。

ADR-0001 の運用に入った今、install 済み環境の状態を分類して
次のアクション (何もしない / repair / 手動対応) を機械的に決定できる
ことが doctor の責務になる。

## What Changes

- **ADDED: `NixStatus` model** (`crates/core/src/managed_nix/status.rs`)
  - 4 状態へ分類: `Missing` / `Healthy` / `Degraded` / `Broken`
  - 分類 input は injectable (marker paths / receipt path / store ping 結果)
    にして unit test は実 `/nix` に依存しない
- **MODIFIED: `schneeforge nix doctor` の表示**
  - 既存の receipt / runtime 診断に加え、冒頭に NixStatus 分類と
    次アクション案内を表示
- **`existing_nix_detected()` は維持** (install 拒否の fail-closed 挙動は
  変更しない。NixStatus はその上位概念)

## 非対象 (本 change では実装しない)

- `schneeforge nix repair` の実装 — 本 change は分類と案内のみ。
  repair は root 実行 + 破壊的操作を伴うため、D8 と同様の確認フローを
  含めて別 change で設計・実装する
- GUI (Tauri) 表示 — CLI 表示を安定させてから #16 で接続

## Impact

- **specs**: `managed-nix-bootstrap` に NixStatus 分類の要件を追加
- **code**: `crates/core/src/managed_nix/status.rs` (新規)、
  `crates/core/src/managed_nix/mod.rs` (export 追加)、CLI doctor 表示
- **test**: NixStatus 分類の unit test (tempdir で marker/receipt を偽装)
- **リスク**: 低 — 既存挙動 (`existing_nix_detected` / install 拒否) は
  変更せず、doctor の表示が増えるのみ
