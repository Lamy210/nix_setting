# Change: GUI への NixStatus 分類の提供

## Why

issue #16 (GUI Managed Nix 統合) のうち、privilege escalation
(osascript / pkexec) は managed-nix-bootstrap spec が「別 change で設計」と
位置づけており、GUI install 実行そのものはまだ接続できない。

一方、PR #33 で `NixStatus` 4 状態分類 (Missing / Healthy / Degraded /
Broken) が core に入った。GUI は現在 `NixHealth` しか持たず、部分削除された
degraded install を「Nix あり」としか表示できない。また wizard の
Nix 未導入時の案内は未だ legacy `curl -L https://nixos.org/nix/install | sh`
のままで、SchneeForge 自身の Managed Nix install を案内できていない。

分類の表示と案内の修正は root 権限不要・破壊的操作なしで完了するため、
GUI 統合の第一段階として先に切り出す。

## What Changes

- **MODIFIED: `Diagnostics` (core) に `nix_status` (StatusReport) を追加**
  - `diagnose()` が `classify_current()` の結果を含める
  - GUI / CLI で同一の分類を共有 (doctor の [status] 欄と同じ値)
- **MODIFIED: wizard の Nix 未導入時の案内 (desktop)**
  - legacy `curl | sh` の案内を `sudo schneeforge nix install` へ変更
  - stepPrereq が `nix_status.status` を表示 (Missing / Degraded / Broken の
    区別 + next action)
- **非対象**: GUI からの install 実行 (privilege escalation を含む design は
  privileged-gui-operations として別 change)、`nix repair` (#15 残り)

## Impact

- **specs**: `gui-diagnostics` に NixStatus 提供の要件を追加
- **code**: `crates/core/src/diagnostics.rs` (Diagnostics に field 追加)、
  `apps/desktop/dist/main.js` (stepPrereq の表示)
- **test**: Diagnostics の serialize に nix_status が含まれることの unit
  test、wizard が legacy curl 案内を出さないことの回帰 test
- **リスク**: 低 — field 追加は serialize の後方互換 (既存 frontend は
  新 field を無視)、案内文言の変更のみで操作の挙動は変わらない
