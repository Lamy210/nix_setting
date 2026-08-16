# Change: GUI からの nix repair / uninstall (issue #16 残作業)

## Why

CLI 側の `schneeforge nix repair` (PR #35) と `schneeforge nix uninstall` は
実装済みだが、GUI (desktop) からの実行経路が無い。GUI の wizard は
NixStatus が `Degraded` / `Broken` の場合「修復が必要」の案内を表示する
だけで、その場で修復を実行できない (spec: wizard は修復案内を出す —
gui-diagnostics)。利用者はターミナルへ移動して CLI を打つ必要があり、
First Run Wizard で完結するという GUI の目的を果たせていない。

desktop には既に昇格実行の infra (CLI sidecar + osascript / pkexec +
`run_escalated_cli()`) が揃っている (PR #36 / #37)。これを nix repair /
uninstall へ拡張する。

権限面の設計:

- **repair**: 唯一の破壊操作は stale ownership record
  (`/nix/schneeforge-managed.json`, root 所有) の削除のため **root が必要**。
  案内表示のみの状態 (Healthy / Missing / SuggestUninstall 等) でも
  同一の昇格経路で実行して問題ない (何も変更しない)
- **uninstall**: upstream `nix-installer uninstall` は root 専用
  (`nix-installer` が root 検査を行う)。破壊的操作のため確認 dialog を
  挟む。`--force` は GUI からは渡さない (fail-closed の確認を bypass する
  操作は CLI の明示指定に限定する)

## What Changes

- **MODIFIED: `EscalatedOp` へ NixRepair / NixUninstall を追加** (core)
  - `schneeforge nix repair` / `schneeforge nix uninstall` を昇格先で
    実行できるよう `cli_args()` を拡張
- **MODIFIED: GUI の nix 系 command を追加** (desktop)
  - `nix_repair_escalated`: repair を昇格実行し結果を返す
  - `nix_uninstall_escalated`: uninstall を昇格実行する (frontend の確認
    dialog を前提とする。確認責任は D8 と同じく GUI 側)
  - 両者とも `run_escalated_cli()` の共通 runner を使う (lock / progress /
    stdout stderr capture は既存挙動)
- **MODIFIED: frontend (wizard) に修復 / 削除の操作を追加** (desktop)
  - stepPrereq の `Degraded` / `Broken` 表示に「修復を試みる」ボタンを
    追加 (repair 実行 → 結果表示 → 再確認)
  - uninstall はwizard からは出さず、Ready 画面の操作ボタンに含める
    (setup 中に Nix を削除する場面は無い)
- **維持: 破壊操作の確認責任は GUI 側**
  - uninstall は confirm dialog を経てのみ実行する
  - repair は冪等かつ非破壊 (表示のみ) の状態があるため確認なしで実行可

## 非対象 (本 change では実装しない)

- **`--hooks` / `--sequoia` (upstream repair) の GUI 化** — 案内は CLI
  command の表示で足りている。必要になったら option 付きで追加する
- **`--force` / `--receipt` の GUI 指定** — fail-closed を突破する操作は
  CLI の明示指定に限定する (repair の案内文案に CLI command を出す)
- **`nix doctor` の GUI ボタン化** — get_status (`nix_status`) で同等の
  分類情報が取れるため重複する

## Impact

- **specs**: `gui-operations` に nix repair / uninstall の昇格実行要件を
  追加。`managed-nix-bootstrap` の privilege escalation 要件の scenario に
  repair / uninstall を追加
- **リスク**: 低 — 昇格の仕組み・sidecar 実行・確認 UX は PR #36 / #37
  で確立済み。CLI command の再利用のため新規の昇格面は無い
