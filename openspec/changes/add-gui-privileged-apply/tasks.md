# Tasks

## 1. openspec

- [x] 1.1 `openspec/changes/add-gui-privileged-apply/` に proposal / specs / tasks を作成
- [x] 1.2 `openspec validate add-gui-privileged-apply --strict` が通ること

## 2. core: EscalatedOp の拡張

- [x] 2.1 `EscalatedOp` に `Apply` / `Rollback` / `Upgrade` を追加し、`cli_args()` が `schneeforge apply` / `rollback` / `upgrade` を生成すること
- [x] 2.2 unit test: 各 op の引数列・osascript / pkexec 引数 shape への反映

## 3. desktop: apply 系 command の sidecar 経由化

- [x] 3.1 `run_apply` / `run_rollback` / `run_upgrade` を core 直接呼び出しから CLI sidecar 昇格実行へ切替 (nix_install_escalated と同じ構造を共通化)
- [x] 3.2 root 実行時は昇格なし直接実行 + `NIX_SETTING_DIR` 明示渡し
- [x] 3.3 昇格失敗時の CLI fallback 案内 (`sudo schneeforge apply` 等)
- [x] 3.4 sync は昇格しない (user 権限で git pull)

## 4. test

- [x] 4.1 desktop 静的 test: apply 系が sidecar / escalation 経由であること、sync が昇格対象外であること
- [x] 4.2 既存 test (frontend_commands_match_backend 等) が壊れないこと

## 5. CI / docs

- [ ] 5.1 `cargo test` / `clippy` / `fmt` / desktop build が green
- [ ] 5.2 STATUS.md のデグレ #5 を解消済みへ更新 (実機確認は macOS Final Acceptance に統合)
