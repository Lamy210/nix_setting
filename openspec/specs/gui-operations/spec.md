# gui-operations Specification

## Purpose
desktop (Tauri) がユーザー操作 (apply / rollback / upgrade / sync / plan / verify) を core へ伝達する際の IPC 契約と状態遷移を定義する。全ハンドラは `CachedToolchain` (`tauri::State`) を介して単一の解決済み `Toolchain` を使い、長時間操作は非同期 command として実行され stdout/stderr がストリーム表示される。
## Requirements
### Requirement: 非同期操作
plan/apply/verify/rollback/upgrade SHALL は UI スレッドを占有せず非同期で実行する。scan と status は軽量のため同期実行でよい。

#### Scenario: apply 実行中も UI が応答する
- **WHEN** ユーザーが apply を実行する
- **THEN** スピナーが表示され、UI は応答し続ける
- **AND** 完了時に出力が表示される

#### Scenario: plan/verify の非同期実行
- **WHEN** ユーザーが plan または verify を実行する
- **THEN** スピナーが表示され、UI は応答し続ける
- **AND** 完了時に結果が表示される

### Requirement: 実行状態の可視化
操作中 SHALL は Running 状態を明示する。

#### Scenario: 実行中
- **WHEN** scan/apply を実行中
- **THEN** スピナーと進捗表示が表示され、ボタンは disable になる

#### Scenario: 失敗
- **WHEN** 操作が失敗する
- **THEN** エラーが表示され、ボタンは再度有効になる

### Requirement: 状態機械
GUI SHALL は SetupState（NeedsSetup/Ready）と OperationState（Idle/Running/Failed）の 2 軸で状態を持つ。

#### Scenario: NeedsSetup 状態
- **WHEN** repository が存在しない
- **THEN** NeedsSetup 状態になり Setup のみ表示する

#### Scenario: Ready + Running の合成
- **WHEN** Ready 状態で apply を実行する
- **THEN** Running(Apply) 状態になり、他の mutating 操作は disable になる

### Requirement: Tauri API 初期化
GUI SHALL は起動時に Tauri IPC が利用可能か検証する。

#### Scenario: Tauri API が無い場合
- **WHEN** `window.__TAURI__` が利用できない
- **THEN** 分かりやすいエラーを表示し、例外で固まらない

### Requirement: ボタンと IPC の対応
操作ボタン SHALL は DOM ID・表示ラベル・backend command を分離して定義する。

#### Scenario: ボタンクリック
- **WHEN** ユーザーがスキャンボタンを押す
- **THEN** 期待した IPC command（run_scan）が dispatch される

### Requirement: 操作結果の判定
backend の CommandOutput.success SHALL に基づいて成功/失敗を表示する。

#### Scenario: backend が失敗を返す
- **WHEN** CommandOutput.success が false
- **THEN** GUI は失敗としてエラーを表示する

### Requirement: プロセス間操作ロック
mutating 操作 SHALL はプロセス間で共有されるロック（ロックファイルの flock）で直列化する。

#### Scenario: CLI と GUI の同時実行
- **WHEN** GUI で apply 実行中に別 terminal から upgrade を実行する
- **THEN** 後発の操作はロックにより拒否または待機する

### Requirement: セキュリティ設定
GUI SHALL は CSP を null にせず、frontend からの system operation を必要最小限の capability に制限する。

#### Scenario: 未使用 plugin
- **WHEN** opener plugin を使う機能が無い
- **THEN** opener の capability と plugin 初期化を削除する

### Requirement: Ready 画面からの Plan/Verify
Ready 状態の GUI SHALL は Plan（dry-run）と Verify（検証）を実行できる。

#### Scenario: Ready 画面で Plan を実行する
- **WHEN** ユーザーが Ready 画面で Plan ボタンを押す
- **THEN** `run_plan` コマンドが dispatch され、dry-run 結果が表示される

#### Scenario: Ready 画面で Verify を実行する
- **WHEN** ユーザーが Ready 画面で Verify ボタンを押す
- **THEN** `run_verify` コマンドが dispatch され、チェック結果が表示される

### Requirement: GUI install の privilege escalation

GUI process SHALL は自身を root にせず、特権が必要な操作を別 process として昇格実行する。macOS は osascript、Linux は pkexec を使う。

#### Scenario: 非 root で install 操作を実行する

- **WHEN** GUI が root 以外で動作しておりユーザーが install を確認した
- **THEN** GUI bundle に同梱された SchneeForge CLI sidecar (`schneeforge nix install --yes`) が管理者権限で再実行される
- **AND** GUI process 自身は root 権限を取得しない
- **AND** 昇格先の process に `NIX_SETTING_DIR` (repo 位置) が引き継がれる

#### Scenario: 昇格が拒否された場合は fallback 案内を出す

- **WHEN** ユーザーが昇格の認証をキャンセルする、または osascript / pkexec が利用できない
- **THEN** install を実行せずエラーを表示する
- **AND** CLI (`sudo schneeforge nix install`) での実行案内を表示する

### Requirement: GUI install の確認責任

GUI SHALL は upstream installer を `--no-confirm` 相当で呼ぶ場合、detailed plan 表示とユーザーの明示的な最終確認を確認 gate とする (D8 の GUI 版)。

#### Scenario: 確認操作なしに install が始まらない

- **WHEN** detailed plan が表示されている
- **THEN** ユーザーの確認操作を受け取るまで install phase へ遷移しない

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

### Requirement: nix repair / uninstall の昇格実行

GUI SHALL は `schneeforge nix repair` / `schneeforge nix uninstall` を GUI process 内で直接実行せず、CLI sidecar を昇格実行する (apply 系と同一の仕組み)。

#### Scenario: wizard から repair を実行する

- **WHEN** NixStatus が `Degraded` または `Broken` の状態でユーザーが wizard の「修復を試みる」を選択する
- **THEN** CLI sidecar (`schneeforge nix repair`) が管理者権限で実行される
- **AND** 結果 (実行した action または案内文案) が表示され、再確認へ戻れる

#### Scenario: repair は確認 dialog なしで実行できる

- **WHEN** ユーザーが wizard の「修復を試みる」を選択する
- **THEN** repair は非破壊の状態 (Healthy / Missing / 案内のみ) を含むため確認なしで実行する
- **AND** 唯一の破壊操作 (stale ownership record 削除) の内容は CLI 側の dry-run 同様の案内を含む

#### Scenario: Ready 画面から uninstall を確認付きで実行する

- **WHEN** ユーザーが Ready 画面の「Nix を削除」ボタンを押す
- **THEN** 確認 dialog (Nix と `/nix` 配下が削除される旨) を表示する
- **AND** 確認後にのみ CLI sidecar (`schneeforge nix uninstall`) が管理者権限で実行される
- **AND** `--force` は付与しない (ownership record 無しの uninstall は CLI の明示指定に限定)

#### Scenario: uninstall の確認をキャンセルする

- **WHEN** ユーザーが確認 dialog でキャンセルする
- **THEN** 何も実行せず元の画面に戻る

#### Scenario: repair / uninstall の失敗は CLI fallback 案内を出す

- **WHEN** 昇格が拒否された、または CLI が非 zero exit で失敗した
- **THEN** エラーと stdout/stderr の末尾を表示する
- **AND** CLI (`sudo schneeforge nix repair` / `sudo schneeforge nix uninstall`) での実行案内を表示する

#### Scenario: repair 実行後の状態は get_status に反映される

- **WHEN** repair の実行が完了する
- **THEN** frontend は `get_status` を呼び直し `nix_status` が更新される (例: `Broken` → `Missing`)

### Requirement: GUI からの profile 切替

desktop SHALL は async command `get_profiles` / `set_profile(name)` /
`clear_profile` を提供する。`set_profile` は manifest の
`profiles.available` 検証を行い、検証を通った場合のみ state へ保存する
(fail-closed)。manifest 取得を伴うため UI thread を占有しない。

#### Scenario: profile 一覧の取得

- **WHEN** frontend が `get_profiles` を invoke する
- **THEN** manifest の available / default と state の selected を持つ `ProfileList` が返る

#### Scenario: 不正 profile の拒否

- **WHEN** available に無い名前で `set_profile` を invoke する
- **THEN** command は error を返し、state の選択は変更されない

#### Scenario: 選択の解除

- **WHEN** `clear_profile` を invoke する
- **THEN** state の選択が解除され、以降の解決は manifest default を使う

### Requirement: Dashboard での profile 切替 UI

frontend SHALL は Dashboard に profile 切替 UI (manifest available からの
選択 + 適用) を表示する。切替は state のみを変更するため、**次回の apply
から反映される**旨を UI に表示する。

#### Scenario: 切替の反映案内

- **WHEN** `set_profile` が成功する
- **THEN** UI は次回の apply から反映される旨を表示し、status / dashboard の表示を更新する

#### Scenario: manifest が解決できない場合

- **WHEN** `get_profiles` が manifest 不在などの error を返す
- **THEN** 切替 UI は使用できない旨を表示し、Dashboard の他の表示は通常通り行う

### Requirement: frontend と backend の契約一致 (profile 切替)

`ProfileList` の serialize key (available / default / selected) への
frontend 参照と、切替 UI の DOM id SHALL は backend の定義と一致する。
desktop の unit test が serialize key + frontend 参照 + DOM id の対応を
検証する。

#### Scenario: key 参照の regression 検証

- **WHEN** desktop の test suite が実行される
- **THEN** `ProfileList` の serialize key と `main.js` の当該 key 参照・切替 UI の DOM id が検証される

### Requirement: wizard による managed source の初期化

wizard の source 設定 step SHALL は managed source の初期化
(core `source_init` と同等: channel stable の最新 tag 解決 + ReleaseMetadata
検証) を提供する。git clone は fork / 開発者向けの選択肢として維持する。

#### Scenario: fresh machine で clone せず初期化する

- **WHEN** repository も state も無い状態で wizard の source 設定 step で
  managed source (既定) を選ぶ
- **THEN** `run_source_init` が呼ばれ、git clone は発生せず、state に
  managed source が設定される

#### Scenario: clone は選択肢として残る

- **WHEN** source 設定 step で git clone を選ぶ
- **THEN** 従来の `run_clone_repo` (URL 入力付き) が使える

### Requirement: 起動時の setup 表示条件

desktop app 起動時の setup wizard 表示 SHALL は「source が未初期化」
(repository checkout 無し かつ managed source 無し) の場合のみ行う。

#### Scenario: managed source 初期化済みなら setup を表示しない

- **WHEN** managed source のみ初期化済み (repo checkout 無し) の状態で
  app を起動する
- **THEN** setup wizard は表示されず main UI (Dashboard) が表示される

#### Scenario: 未初期化なら従来通り setup

- **WHEN** repository も managed source も無い状態で app を起動する
- **THEN** 従来通り setup wizard を表示する

### Requirement: wizard は GUI から Managed Nix install を実行できる (repository 非依存)

First Run Wizard SHALL は Nix 未導入 (Missing) の場合、ターミナルでの CLI 手打ちに頼らず GUI の操作で Managed Nix install を完了できる。install は core の `ManagedNix::prepare_plan()` / `execute_plan()` 経路 (CLI と同一 policy) を使う。repository checkout は前提としない (escalated CLI sidecar の embedded manifest で動作)。

#### Scenario: Nix 未導入時に install ボタンが表示される

- **WHEN** wizard の前提確認で Nix が未導入 (Missing) と判定される
- **THEN** Managed Nix を導入する操作 (ボタン) が表示される
- **AND** CLI の手打ち案内が escalation が利用できない環境向け fallback として表示される

#### Scenario: repository 未 clone でも install を提供する

- **WHEN** wizard の前提確認で Nix が未導入かつ repository が未 clone である
- **THEN** Managed Nix の導入操作が表示され、そのまま install を開始できる

#### Scenario: detailed plan 表示から最終確認を経て install する

- **WHEN** ユーザーが Managed Nix の導入操作を実行する
- **THEN** plan 生成後に detailed plan (actions 概要) が表示される
- **AND** ユーザーの最終確認操作を受けた場合のみ install が実行される
- **AND** 確認を取り消した場合 `/nix` は変更されない

#### Scenario: install progress が表示される

- **WHEN** install が実行中
- **THEN** phase (download / verify / plan / install / post-install) が順次表示され UI は応答し続ける
- **AND** 完了時に receipt / ownership の確認結果が表示される

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

