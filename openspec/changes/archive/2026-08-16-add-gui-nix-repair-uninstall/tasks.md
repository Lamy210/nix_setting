# Tasks

## 1. core: EscalatedOp 拡張

- [x] 1.1 `EscalatedOp` に `NixRepair` (`nix repair`) / `NixUninstall` (`nix uninstall`) を追加し `cli_args()` を拡張
- [x] 1.2 unit test: repair / uninstall の args が昇格 command に埋め込まれること (`--force` を含まないこと)

## 2. desktop: backend command

- [x] 2.1 `nix_repair_escalated` command を追加 (`run_escalated_cli` 経由, dev では dry-run guard)
- [x] 2.2 `nix_uninstall_escalated` command を追加 (`run_escalated_cli` 経由, dry-run guard は無し)
- [x] 2.3 `generate_handler` へ登録
- [x] 2.4 静的 test: repair / uninstall が sidecar 昇格 marker 経由であること / core 直接呼び出しでないこと / CLI fallback 案内を含むこと

## 3. desktop: frontend

- [x] 3.1 wizard stepPrereq の Degraded / Broken 表示に「修復を試みる」ボタンを追加 (repair 実行 → 結果表示 → 再確認)
- [x] 3.2 Ready 画面に「Nix を削除」ボタンを追加 (confirm dialog → uninstall 実行)
- [x] 3.3 repair / uninstall 実行後 `get_status` を呼び直して表示を更新

## 4. 品質 gate

- [x] 4.1 `cargo test -p schneeforge-core` / `-p schneeforge-desktop` 全 pass
- [x] 4.2 `cargo clippy -- -D warnings` / `cargo fmt --check` pass
- [x] 4.3 `openspec validate add-gui-nix-repair-uninstall --strict` pass
- [x] 4.4 CI (check.yml) 全 job green
