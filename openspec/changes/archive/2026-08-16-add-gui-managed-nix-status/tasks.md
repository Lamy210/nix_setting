# Tasks

## 1. Core (Diagnostics 拡張)

- [x] 1.1 `Diagnostics` に `nix_status: StatusReport` field を追加 (`diagnose()` 内で `classify_current()` を呼ぶ)
- [x] 1.2 unit test: Diagnostics の serialize に `nix_status` (status + next action) が含まれること
- [x] 1.3 既有の `nix_health` / `tools` field が壊れないことの test 維持

## 2. Desktop (wizard 表示)

- [x] 2.1 `stepPrereq` が `nix_status.status` / next action を表示
- [x] 2.2 Nix 未導入時の案内を legacy `curl | sh` から `sudo schneeforge nix install` へ変更
- [x] 2.3 回帰 test: wizard が legacy curl 案内を出さないこと (main.js の静的 test)

## 3. 仕様・文書

- [x] 3.1 `openspec validate add-gui-managed-nix-status --strict` が通ること
- [x] 3.2 docs/STATUS.md の進行中 table に反映
