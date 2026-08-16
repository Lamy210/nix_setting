# Change: GUI apply / rollback / upgrade の privilege escalation 統合 (デグレ #5)

## Why

GUI (desktop) の `run_apply` / `run_rollback` / `run_upgrade` は core の
`apply()` / `rollback()` / `upgrade()` を **GUI process 内で直接呼ぶ**。
一方これらの操作は root 権限を必要とする:

- **Linux**: `activationPackage/activate` は sudo を要求する
- **macOS**: `nix-darwin` の `darwin-rebuild switch` は sudo を要求する

GUI process には TTY が無いため sudo の password prompt が表示できず、
操作が固まるか失敗する (デグレ #5「GUI apply の sudo/TTY 問題」)。
design.md D4 Phase 2 は privileged-gui-operations での解決を予定して
いたが、issue #16 では `nix install` のみ実装され、apply 系は残った。

Core には既に `escalate_command()` (osascript / pkexec) と
`EscalatedOp`、desktop には CLI sidecar (`cli_sidecar_path()`) と
昇格実行の infra が揃っている (PR #36)。これを apply 系へ拡張する。

## What Changes

- **MODIFIED: `EscalatedOp` へ Apply / Rollback / Upgrade を追加** (core)
  - `schneeforge apply` / `schneeforge rollback` / `schneeforge upgrade`
    を昇格先で実行できるよう `cli_args()` を拡張
- **MODIFIED: GUI の apply 系 command を sidecar 経由に切替** (desktop)
  - `run_apply` / `run_rollback` / `run_upgrade` は core 直接呼び出しを
    やめ、CLI sidecar を昇格 (非 root) または直接 (root) 実行する
  - `NIX_SETTING_DIR` を昇格先へ明示渡し (repo 解決のため)
  - State (`state.json`) は CLI 側で保存される (core と同一 logic)
  - 出力は stdout/stderr を capture して CommandOutput へ返す
- **維持: lock / state / progress の既存挙動**
  - 操作 lock は昇格先 CLI process 内で取得される (跨 process で直列化)
  - wizard の First Run apply は既存の `run_apply` 経由のまま
    (昇格は同じ経路に集約される)

## 非対象 (本 change では実装しない)

- **`schneeforge sync` の昇格** — `git pull` は user 権限で完結する
  (remote 認証が user の credential を使うため、root 実行だとむしろ壊れる)
- **`schneeforge nix uninstall` の GUI 化** — 別 change (GUI uninstall /
  repair) で設計予定
- **macOS authorization (SMJobBless 等)** — osascript で足りている
  範囲。正式な privileged helper framework は必要になった時に検討

## Impact

- **specs**: `gui-operations` に apply 系の昇格要件を追加
- **リスク**: 低-中 — 昇格の仕組み自体は `nix install` で CI/静的 test
  済み。apply 系は既存 CLI command の再利用のため新規の昇格面は無い。
  実機 (osascript / pkexec) の動作確認は macOS Final Acceptance に統合

## Sources

- design.md D4 Phase 2: `openspec/changes/archive/2026-08-14-add-managed-nix-bootstrap/design.md`
- 実装済み escalation: `crates/core/src/managed_nix/escalate.rs` (PR #36)
