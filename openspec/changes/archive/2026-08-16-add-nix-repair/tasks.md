# Tasks

## 1. Core (repair action 判定)

- [x] 1.1 `crates/core/src/managed_nix/status.rs` に `RepairAction` (RemoveStaleOwnership / SuggestUninstall / SuggestManualCleanup / NoActionNeeded / SuggestInstall) を定義し、`StatusProbe` から `repair_action()` を返す
- [x] 1.2 `crates/core/src/managed_nix/installer.rs` に upstream `repair hooks` / `repair sequoia` の args builder と実行 function を追加
- [x] 1.3 unit test: 4 状態 + receipt 有無の Degraded 分岐で RepairAction が正しく決まること
- [x] 1.4 `NixStatus::next_action()` を repair 案内へ更新 (Degraded / Broken)

## 2. CLI

- [x] 2.1 `schneeforge nix repair` subcommand (`--dry-run` / `--hooks` / `--sequoia`) を追加
- [x] 2.2 Broken で stale ownership record 削除 (dry-run は表示のみ)。削除は marker が一切無い場合のみ
- [x] 2.3 Degraded / Healthy / Missing の案内表示
- [x] 2.4 root 実行を要求しない (stale record 削除は ownership file の権限次第。upstream repair 呼び出し時のみ必要な権限を upstream が要求する)

## 3. Test / 文書

- [x] 3.1 E2E: uninstall 中断を模した Broken 状態 (ownership のみ残存) から repair で Missing 復帰、dry-run では維持されること
- [x] 3.2 doctor の案内更新の回帰 (unit test で next_action が `nix repair` を含むこと)
- [x] 3.3 `openspec validate add-nix-repair --strict` が通ること
- [x] 3.4 docs/STATUS.md 更新
