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


### Requirement: wizard は GUI から Managed Nix install を実行できる

First Run Wizard SHALL は Nix 未導入 (Missing) の場合、ターミナルでの CLI 手打ちに頼らず GUI の操作で Managed Nix install を完了できる。install は core の `ManagedNix::prepare_plan()` / `execute_plan()` 経路 (CLI と同一 policy) を使う。

#### Scenario: Nix 未導入時に install ボタンが表示される

- **WHEN** wizard の前提確認で Nix が未導入 (Missing) と判定される
- **AND** repository が既に clone 済みである
- **THEN** Managed Nix を導入する操作 (ボタン) が表示される
- **AND** CLI の手打ち案内が escalation が利用できない環境向け fallback として表示される

#### Scenario: repository 未 clone 時は install を offering しない

- **WHEN** wizard の前提確認で Nix が未導入かつ repository が未 clone である
- **THEN** Managed Nix の導入操作は表示されず repository 設定 step への誘導が表示される
- **AND** install 操作を提供しない理由として repository の clone が必要であることが表示される

#### Scenario: detailed plan 表示から最終確認を経て install する

- **WHEN** ユーザーが Managed Nix の導入操作を実行する
- **THEN** plan 生成後に detailed plan (actions 概要) が表示される
- **AND** ユーザーの最終確認操作を受けた場合のみ install が実行される
- **AND** 確認を取り消した場合 `/nix` は変更されない

#### Scenario: install progress が表示される

- **WHEN** install が実行中
- **THEN** phase (download / verify / plan / install / post-install) が順次表示され UI は応答し続ける
- **AND** 完了時に receipt / ownership の確認結果が表示される

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
