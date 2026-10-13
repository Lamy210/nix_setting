## ADDED Requirements

### Requirement: privilege escalation の textual repository path boundary

GUI privilege escalation SHALL treat repository path transport as a UTF-8 textual boundary while the existing osascript/pkexec + `NIX_SETTING_DIR` handoff is in use. A repository path that cannot be represented losslessly as UTF-8 MUST be rejected before executing the elevated command and MUST NOT be replaced with a lossy display value.

#### Scenario: non-UTF-8 repository path を escalation へ渡す

- **WHEN** GUI privilege escalation が valid UTF-8 に変換できない repository path を受け取る
- **THEN** escalation command construction は fail-closed error を返す
- **AND** elevated command は実行されない
- **AND** `NIX_SETTING_DIR` に lossy path を設定しない

#### Scenario: UTF-8 repository path の既存 escalation

- **WHEN** GUI privilege escalation が absolute valid-UTF-8 repository path を受け取る
- **THEN** 既存の macOS osascript / Linux pkexec transport はその repository identity を `NIX_SETTING_DIR` で引き継ぐ
- **AND** repository path identity refactor は既存の privilege escalation command policy を変更しない
