## ADDED Requirements

### Requirement: GUI からの configuration source 更新

desktop SHALL は async command `run_update` を提供し、core `update()`
(CLI `schneeforge update` と同一経路) を GUI process 内で実行する。
`update` は git 操作と state 更新のみで root 権限を要求しないため、
昇格 (osascript / pkexec) は使わず sync と同じ user 権限のまま実行する。
操作 lock (flock) は core `update()` 内で取得され、CLI との同時実行を
直列化する。

#### Scenario: Ready 画面でソース更新を実行する

- **WHEN** ユーザーが Ready 画面の「ソース更新」ボタンを押す
- **THEN** `run_update` command が dispatch され、core `update()` が
  非同期 (UI thread を占有せず) 実行される
- **AND** 実行中はボタンが disable になりスピナーが表示される
- **AND** 完了時に出力 (移行先 tag / Already on the latest 等) が表示される

#### Scenario: managed source で新しい release へ移る

- **WHEN** managed source の machine で channel に新しい release tag が
  存在するときに「ソース更新」を実行する
- **THEN** state の source ref が新しい tag に更新される
- **AND** frontend は status / dashboard を再取得し、表示中の tag が
  更新される

#### Scenario: 昇格を要求しない

- **WHEN** ユーザーが「ソース更新」を実行する
- **THEN** GUI process 内で実行され、管理者認証 (osascript / pkexec) は
  要求されない

#### Scenario: 失敗時は error を表示する

- **WHEN** network 不通や dirty checkout などで core `update()` が失敗する
- **THEN** CommandOutput の失敗として error が表示され、ボタンは再度
  有効になる

### Requirement: managed source での非推奨 upgrade の隠蔽

frontend SHALL は configuration source が managed (state の
`managed_source` が non-null) の場合、非推奨の flake.lock 更新ボタン
(「アップグレード」) を隠す。core が managed source で deps 更新を
fail-closed で拒否するため、押しても必ず失敗する操作を表示しない。
checkout 表現 / GitTracking では従来通り表示し、従来の昇格経路で実行する。

#### Scenario: managed source ではアップグレードを隠す

- **WHEN** `get_status` の `managed_source` が non-null の状態で status を
  再取得する
- **THEN** 「アップグレード」ボタンは非表示になる

#### Scenario: checkout 表現では従来通り表示する

- **WHEN** `get_status` の `managed_source` が null (repo checkout /
  GitTracking / 未初期化) の状態で status を再取得する
- **THEN** 「アップグレード」ボタンは表示されたままになる

### Requirement: frontend と backend の契約一致 (ソース更新)

`run_update` への frontend 参照と、ソース更新 UI の DOM id SHALL は
backend の定義と一致する。desktop の unit test が command 参照 + DOM id
の対応を検証する。

#### Scenario: key 参照の regression 検証

- **WHEN** desktop の test suite が実行される
- **THEN** `main.js` の `run_update` invoke 参照と「ソース更新」ボタン・
  「アップグレード」ボタンの DOM id が検証される
