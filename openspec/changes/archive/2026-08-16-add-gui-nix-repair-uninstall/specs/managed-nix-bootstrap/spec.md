## MODIFIED Requirements

### Requirement: privilege escalation の明示

SchneeForge SHALL は install / uninstall 時の privilege escalation を明示的に扱う。Phase 1 (CLI) では SchneeForge 側で自前 `sudo` 呼び出しを行わず、root 未実行時は `sudo schneeforge nix install ...` での再実行を促す。Phase 2 (GUI) では TTY 非依存の osascript (macOS) / pkexec (Linux) を別 change で統合する。

#### Scenario: Phase 1 CLI で root 未実行時は再実行を促す

- **WHEN** root 権限を持たずに `schneeforge nix install` を実行した場合
- **THEN** SchneeForge は「sudo で再実行してください」のメッセージを出して停止する (自前で sudo 呼び出しはしない)

#### Scenario: Phase 1 CLI で root 実行時はそのまま続行

- **WHEN** root 権限で `schneeforge nix install` を実行した場合
- **THEN** そのまま plan → install の phase を実行する

#### Scenario: Phase 2 GUI では TTY 非依存の認証を要求

- **WHEN** Tauri GUI から install を実行する (Phase 2 以降)
- **THEN** TTY に依存せず、osascript (macOS) / pkexec (Linux) 等で認証を要求する

#### Scenario: GUI から repair を昇格実行する

- **WHEN** Tauri GUI から repair を実行する
- **THEN** CLI sidecar (`schneeforge nix repair`) が osascript / pkexec 経由で実行される
- **AND** stale ownership record の削除 (`/nix/schneeforge-managed.json`, root 所有) が昇格先で完結する

#### Scenario: GUI から uninstall を昇格実行する

- **WHEN** Tauri GUI から uninstall を実行する (確認 dialog 済み)
- **THEN** CLI sidecar (`schneeforge nix uninstall`) が osascript / pkexec 経由で実行される
- **AND** upstream `nix-installer uninstall` の root 検査は昇格先 process で満たされる
- **AND** `--force` は GUI から付与されない
