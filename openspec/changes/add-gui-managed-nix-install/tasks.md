# Tasks

## 1. Core (escalation helper)

- [x] 1.1 `crates/core/src/managed_nix/escalate.rs` を新規作成し、macOS (osascript) / Linux (pkexec) の昇格 command 構築 function を実装する
- [x] 1.2 shell 文字列の escape (quote / 特殊文字) を helper 内に実装し、実行対象を SchneeForge binary + `nix install --yes` 固定に限定する
- [x] 1.3 unit test: 構築される引数列の形式・escape 動作・env 引き継ぎ (DISPLAY / XAUTHORITY / WAYLAND_DISPLAY) を検証

## 2. Desktop (Tauri)

- [x] 2.1 root 不要の plan preview command (`nix_prepare_plan`) を追加: `ManagedNix::prepare_plan()` を呼び detailed plan 行を返す
- [x] 2.2 昇格付き install command (`nix_install_escalated`) を追加: escalation helper で `schneeforge nix install --yes` を再実行し、stderr JSON Lines の phase を progress として返す
- [x] 2.3 wizard stepPrereq に「SchneeForge で導入」flow を追加 (plan preview → 最終確認 → install → 結果表示)。CLI 案内は fallback として維持
- [x] 2.4 静的回帰 test: frontend invoke 名と generate_handler の整合 / escalation 失敗時の CLI fallback 案内の存在

## 3. Test / 文書

- [x] 3.1 core / desktop の全 unit test pass (Docker: GTK deps install 済み container)
- [x] 3.2 `openspec validate add-gui-managed-nix-install --strict` が通ること
- [x] 3.3 docs/STATUS.md 更新
