# Change: `schneeforge nix repair` — state-driven 修復

## Why

NixStatus 分類 (PR #33) で 4 状態を機械判定できるようになったが、
Degraded / Broken の次アクションは「`schneeforge nix uninstall` で削除して
から再 install するか、手動で確認」の案内止まりで、SchneeForge 単独では
復旧できない (issue #15 の残件)。

特に以下の gap が実害になる:

1. **Broken (ownership のみ残存)** — uninstall 中断の跡。uninstall は
   receipt を要求するため、`/nix` 配下が既に消えた状態では「receipt not
   found」で失敗し、stale ownership record を削除する手段が CLI に無い。
   この状態が残ると doctor は永久に Broken を表示し続ける。
2. **Degraded (receipt 欠損)** — marker が残るため install は
   `ExistingNixDetected` で拒否され、uninstall も receipt 無しでは
   `--force` を要求される。ユーザーは flags の意味を調べる必要がある。
3. **upstream `nix-installer repair {hooks|sequoia}`** — shell profile 修復
   と Sequoia `_nixbld` 回復の実装が実在するのに SchneeForge から呼ぶ
   経路が無い (spike report 実測)。

## What Changes

- **ADDED: `schneeforge nix repair` subcommand (CLI)**
  - `NixStatus` 分類を入力に取り、状態ごとに修復 action を実行する
    (dry-run 対応: `--dry-run` で実行内容の表示のみ)
  - **Broken**: stale ownership record の削除 (marker が無い = Nix 実態が
    無いため、record を消すだけで Missing へ復帰。receipt が残っていれば
    それも表示して案内)
  - **Degraded**: receipt の有無で分岐
    - receipt がある → 既存 uninstall flow の案内 (repair からの自動実行は
      破壊的操作のため行わない)
    - receipt が無い → `uninstall --force` 相当の手順を表示 (marker のみ
      残存では upstream も revert できないため、手動削除の案内)
  - **Healthy**: 「対応不要」を表示して正常終了
  - **Missing**: install 案内を表示して正常終了
- **ADDED: upstream repair hooks / sequoia の wrapper (core)**
  - `nix-installer repair hooks` / `repair sequoia` (macOS) を subprocess
    呼び出しする function と、CLI からの実行 option (`nix repair --hooks`
    / `--sequoia`)
  - SchneeForge は revert logic を再実装しない (既存の uninstall と同じ
    委譲方針)
- **MODIFIED: doctor の Degraded / Broken 案内**
  - 次アクション文案を `schneeforge nix repair` へ更新 (実行可能な
    command ができたため)

## 非対象 (本 change では実装しない)

- Degraded からの自動 uninstall + 再 install — 破壊的操作を伴うため
  D8 と同様の確認フロー設計が別途必要。本 change は案内と stale record
  の除去 (非破壊) のみ
- GUI (Tauri) からの repair — CLI 安定後に #16 で接続
- upstream `repair sequoia` の自動判定 (Sequoia 乗っ取り検出) — option
  指定での実行のみ

## Impact

- **specs**: `managed-nix-bootstrap` に repair の要件を追加
- **code**: `crates/core/src/managed_nix/` (repair action 判定 / upstream
  repair 呼び出し)、`crates/cli/src/nix_cmd.rs` (`repair` subcommand)
- **test**: unit test (StatusProbe からの repair action 判定)、E2E test
  (Broken 状態からの repair で Missing 復帰)
- **リスク**: 中 — repair 自体は stale file 削除と案内表示のみだが、
  誤った状態判定で ownership record を消すと uninstall 対称性が失われる。
  削除は「marker が一切無い + ownership がある」= Broken のみに限定し、
  dry-run を既定の確認手段にする
