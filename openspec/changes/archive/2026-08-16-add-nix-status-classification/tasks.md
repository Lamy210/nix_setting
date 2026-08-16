# Tasks

## 1. Core model

- [x] 1.1 `crates/core/src/managed_nix/status.rs` に `NixStatus` enum (Missing / Healthy / Degraded / Broken) を定義
- [x] 1.2 分類 input (`StatusProbe`: marker paths / receipt path / ownership path / store ping 結果) を injectable な引数として `classify()` を実装
- [x] 1.3 `NixStatus::next_action()` — 各状態の次アクション文案を返す
- [x] 1.4 unit test: tempdir で 4 状態を再現 (marker 無し / 完備 / marker のみ / ownership 不一致)
- [x] 1.5 `existing_nix_detected()` は変更しないことの test (fail-closed 維持)

## 2. CLI 表示

- [x] 2.1 `schneeforge nix doctor` の冒頭に `[status]` 欄 (分類 + 次アクション) を追加
- [x] 2.2 既存の receipt / runtime 診断が壊れないことの確認 (表示順は既存維持)

## 3. 仕様・文書

- [x] 3.1 `openspec validate add-nix-status-classification --strict` が通ること
- [x] 3.2 docs/STATUS.md の「次の作業」を更新 (#15 の分類 model が完了した旨)
