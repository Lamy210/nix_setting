# Change: GUI (Tauri) からの Managed Nix install — privilege escalation 付き

## Why

Managed Nix install は CLI (`sudo schneeforge nix install`) のみで、GUI
(First Run Wizard) は `sudo schneeforge nix install` をターミナルで手打ち
するよう案内するだけ (issue #16 / issue #5)。GUI から完結できないため:

1. **wizard が Nix 未導入で止まる** — stepPrereq は NG 表示と案内のみで、
   「次へ」に進めない。ユーザーはターミナルを開いて sudo を打ち、戻って
   「再確認」を押す必要がある
2. **issue #5 の GUI apply sudo/TTY 問題と同根** — GUI process から
   TTY を要求する sudo は使えない。design.md D4 Phase 2 は
   privileged-gui-operations での osascript / pkexec 統合を予定していた

Core 側の API (`ManagedNix::prepare_plan()` / `execute_plan()`) と
D8 の 2 段階 Plan UX は Phase 1 (PR #13) で集約済みで、GUI はこれを
呼ぶだけでよい状態にある。

## What Changes

- **ADDED: GUI 用 privilege escalation helper (core)**
  - `escalate_command()`: macOS は `osascript -e 'do shell script "…" with administrator privileges'`、Linux は `pkexec env NIX_SETTING_DIR=… DISPLAY=… XAUTHORITY=… <cmd>` を構築する
  - 対象 command は SchneeForge の **CLI** binary (`schneeforge nix install --yes`) に限定。GUI binary は CLI 引数を解釈しないため昇格先には使えない。shell 文字列を組み立てる際は引数の escape を helper が担う
  - 昇格先には `NIX_SETTING_DIR` (repo 位置) を明示渡しする (root 環境では HOME が変わり repo が解決できなくなるため)
  - escalation 先でも D8 の確認責任は維持する: GUI が detailed plan を表示してユーザーの [Install] 操作を受けたことを「確認済み」とみなし、`--yes` を付けて再実行する (GUI が確認 UI の一部であるため)
- **ADDED: GUI install flow (desktop / Tauri)**
  - wizard stepPrereq の Nix 未導入時、「SchneeForge で導入」ボタンを追加
  - 押下 → (root なら直接 / 非 root なら escalation helper で) `prepare_plan` 相当の plan 生成 → detailed plan 表示 (D8 の GUI 版) → 最終確認 → `execute_plan` 相当の install 実行
  - progress は stderr JSON Lines の phase (Download / Verify / Plan / Install / PostInstall) を表示
  - install 完了後は ownership record / receipt / post-install gate を CLI と同じ基準で検証
- **MODIFIED: wizard の Managed Nix 案内**
  - CLI の手打ち案内から GUI ボタンへ。CLI 案内は escalation helper が
    利用できない環境 (pkexec 未導入等) の fallback として残す

## 実装方針

- GUI process 自身は root にならず、**root で再実行した別 process** の
  結果を待つ (GUI が root で動くと $HOME / XDG が root のものになり
  cache / plan dir の所有者が崩れるため)
- 再実行先は `schneeforge nix install --yes` (CLI の既存 flow をそのまま
  使う。policy / plan 生成 / ownership 記録 / post-install gate の
  二重実装を避ける)。GUI は plan preview のために root 不要の
  `prepare_plan` を自 process で呼び、確認後に root 再実行へ移行する
- 昇格先の CLI binary は Tauri の externalBin (sidecar) として GUI bundle
  に同梱する — build script が workspace の CLI binary を
  `binaries/schneeforge-cli-$TARGET_TRIPLE` へ stage し、runtime は
  main binary と同じ directory から解決する (Nix-less 環境で PATH 解決に
  頼れないため)
- 進捗の可視化: root process の stderr JSON Lines を GUI が読めるよう、
  再実行時は stderr を pipe で受け取り phase 行を event で frontend へ
  流す

## 非対象 (本 change では実装しない)

- GUI からの uninstall / repair — install flow が安定したら別 change で
  追加 (破壊的操作のため確認 UX の設計が別途必要)
- macOS の STAuthorizationTool / SMJobBless 等の native authorization —
  osascript で足りる範囲をまず提供する (design.md D4 と同じ段階的判断)
- pkexec 未導入環境への polkit 設定 file の同梱 — fallback は CLI 案内

## Impact

- **specs**: `gui-operations` (GUI install flow / escalation の要件)、
  `managed-nix-bootstrap` (escalation helper の要件) に追加
- **code**: `crates/core/src/managed_nix/escalate.rs` (新規 helper)、
  `apps/desktop/src-tauri/src/lib.rs` (Tauri command)、
  `apps/desktop/src-tauri/build.rs` + `tauri.conf.json` (CLI sidecar の
  stage / bundle 設定)、
  `apps/desktop/dist/main.js` (wizard UI)
- **test**: escalation helper の unit test (args 構築の escape 検証)、
  desktop 静的回帰 test (frontend / backend の IPC 整合・CLI fallback
  案内の維持)、Docker E2E への GUI command 経由検証は Linux pkexec
  環境の制約から本 change では省略 (CI の desktop test で静的検証)
- **リスク**: 高 — 特権昇格を伴う。緩和策: (a) 実行する command は
  SchneeForge 自身の `nix install --yes` に固定し shell 文字列の
  任意実行を許さない (b) 引数は helper が escape する (c) escalation
  失敗時は CLI 案内へ fallback し部分実行状態を作らない
