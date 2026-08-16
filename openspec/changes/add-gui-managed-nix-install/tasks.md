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

## 4. Review 指摘修正 (PR review A-E)

- [x] 4.1 (A) 昇格先を CLI sidecar (Tauri externalBin) に変更: build.rs が workspace CLI binary を `binaries/schneeforge-cli-$TRIPLE` として stage し、runtime は `cli_sidecar_path()` で解決。GUI 自身の binary を昇格しない
- [x] 4.2 (B) `NIX_SETTING_DIR` を昇格先へ明示渡し: osascript は export prefix、pkexec は `env` 経由、root 直接実行は `cmd.env()`
- [x] 4.3 (C) stdout / stderr を別 thread で並行読み取り (pipe buffer 満杯による相互 block の解消)
- [x] 4.4 (D) frontend が `status.repo_exists` で install 案内を gate (未 clone は repo step へ誘導)。backend も `nix_prepare_plan` で manifest 不存在を fail-closed 拒否
- [x] 4.5 (E) pkexec / osascript を絶対 path (`/usr/bin/…`) で呼ぶ
- [x] 4.6 静的回帰 test 追加 (sidecar 参照 / repo gate / NIX_SETTING_DIR) と core・desktop の全 test 再実行
- [x] 4.7 fmt / clippy / openspec validate --strict

## 5. Progress streaming (issue #16 作業項目 4)

- [x] 5.1 `nix_install_escalated` の stderr reader が CLI の JSON Lines を `parse_json_line` で best-effort parse し、`nix-install-progress` event (phase / message) を frontend へ随時 emit する
- [x] 5.2 frontend が event を listen し phase 表示と直近 log (10 行) を随時更新。listener 取得失敗時も install 自体は継続 (完了後一括表示へ退化)
- [x] 5.3 静的回帰 test: backend の emit 名と frontend の listen 名の整合
