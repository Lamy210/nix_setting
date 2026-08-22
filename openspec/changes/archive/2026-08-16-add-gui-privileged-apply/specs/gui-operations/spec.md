## MODIFIED Requirements

### Requirement: apply / rollback / upgrade の昇格実行

GUI SHALL は apply / rollback / upgrade を GUI process 内で直接実行せず、root 権限が必要な操作として別 process で昇格実行する。macOS は osascript、Linux は pkexec を使う (nix install と同一の仕組み)。

#### Scenario: 非 root で apply を実行する

- **WHEN** GUI が root 以外で動作しておりユーザーが apply を実行する
- **THEN** GUI bundle に同梱された SchneeForge CLI sidecar (`schneeforge apply`) が管理者権限で実行される
- **AND** GUI process 自身は root 権限を取得しない
- **AND** 昇格先の process に `NIX_SETTING_DIR` (repo 位置) が引き継がれる

#### Scenario: root で起動した GUI は昇格せず直接実行する

- **WHEN** GUI が root 権限で既に動作している
- **THEN** CLI sidecar を昇格なしで直接実行する (env のみ明示渡しする)

#### Scenario: 昇格が拒否された場合は fallback 案内を出す

- **WHEN** ユーザーが昇格の認証をキャンセルする、または osascript / pkexec が利用できない
- **THEN** 操作を実行せずエラーを表示する
- **AND** CLI (`sudo schneeforge apply` 等) での実行案内を表示する

#### Scenario: rollback / upgrade も同一経路で昇格される

- **WHEN** ユーザーが rollback または upgrade を実行する
- **THEN** apply と同じ sidecar 昇格の経路で実行される
- **AND** sync (git pull) は昇格せず user 権限のまま実行される

#### Scenario: 操作 lock と state は CLI 側で機能する

- **WHEN** 昇格先の CLI process が apply を実行する
- **THEN** 操作 lock (flock) を取得して直列化し、成功時に state (`state.json`) を保存する
- **AND** GUI 側で別の mutating 操作を開始しても lock により拒否される
