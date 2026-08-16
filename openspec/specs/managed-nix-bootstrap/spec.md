# managed-nix-bootstrap Specification

## Purpose
SchneeForge 経由で Nix (NixOS/nix-installer) を install / doctor / uninstall する Managed Nix bootstrap の要件。version + SHA256 pinning、既存 Nix の上書き拒否、ownership record による uninstall safety、2 段階 Plan UX (確認責任は SchneeForge 側) を規定する。
## Requirements
### Requirement: Managed Nix provider の選択
SchneeForge SHALL は NixOS/nix-installer を Managed-Nix の default provider として利用する。SchneeForge Core は nix-installer を外部プロセスとして実行し、コードをリンクしない。

#### Scenario: NixOS/nix-installer を default provider として利用
- **WHEN** SchneeForge が Managed Nix の install/doctor/uninstall を実行する
- **THEN** 指定された version の `nix-installer-{arch}` binary を GitHub Releases から取得して subprocess で実行する
- **AND** SchneeForge 側コードは nix-installer とリンクしない

#### Scenario: x86_64-darwin は未サポート
- **WHEN** `x86_64-darwin` で SchneeForge を実行する
- **THEN** Managed Nix install は `UnsupportedArch` エラーで停止する
- **AND** ユーザーは手動で Nix を導入するか、別 machine を使う旨のメッセージを受け取る

### Requirement: bootstrap-manifest.toml による version + SHA256 pinning
SchneeForge SHALL は `bootstrap-manifest.toml` で nix-installer の version と arch 毎の expected SHA256 を pin する。runtime は manifest の値を权威とし、GitHub Releases 側の最新値に左右されない。

#### Scenario: manifest の SHA256 と一致する download
- **WHEN** install 時に download した binary の SHA256 が manifest と一致する
- **THEN** 検証を通過し install を継続する

#### Scenario: SHA256 不一致
- **WHEN** download した binary の SHA256 が manifest と一致しない
- **THEN** `ManagedNixError::ChecksumMismatch` で即座に停止する
- **AND** download 済みファイルは削除される

#### Scenario: manifest の version は SchneeForge release 毎に固定
- **WHEN** SchneeForge を新 version へ upgrade する
- **THEN** manifest の nix-installer version は SchneeForge release 単位で pin される
- **AND** 利用者 PC で nix-installer の最新版が自動取得されることは無い

### Requirement: installer binary のキャッシュ
SchneeForge SHALL は download した nix-installer binary を実行権限ごとに分かれた cache へ保存し、二回目以降の install は offline で動作させる。

- **root 実行 (Phase 1 CLI の通常経路)**: privileged state dir 配下
  (`/var/lib/schneeforge` (Linux) / `/private/var/db/schneeforge` (macOS))
  の `managed-nix/cache/{version}/nix-installer`。sudo で user の HOME/XDG が
  持ち込まれても user-writable path を root 実行 binary の cache に使わない
- **非 root (将来の prefetch 用)**: `XDG_DATA_HOME/schneeforge/managed-nix/{version}/nix-installer`

#### Scenario: 初回 install は online 必須
- **WHEN** キャッシュが無い状態で `schneeforge nix install` を実行する
- **THEN** binary を download してキャッシュした上で install を実行する

#### Scenario: 二回目以降は offline で動作
- **WHEN** キャッシュ済みの version で `schneeforge nix install` を実行する
- **THEN** ネットワークアクセス無しでキャッシュから binary を取り出して install する
- **AND** キャッシュの SHA256 を manifest と再検証する

### Requirement: SLSA provenance と SHA256SUMS の CI 検証
SchneeForge SHALL は upstream release を取り込む CI で SLSA provenance (`gh attestation verify`) と SHA256SUMS を検証し、`bootstrap-manifest.toml` へ bump する PR を作成する。runtime は manifest の SHA256 のみを検証する。bump PR は SchneeForge の release cycle で評価し、breaking change がある場合は棄却する。

#### Scenario: CI が release を verify して manifest を更新
- **WHEN** upstream が新 release を公開する
- **THEN** SchneeForge の CI は `gh attestation verify` で provenance を確認する
- **AND** SHA256SUMS から各 arch の sha256 を抽出して manifest を更新する PR を作成する

#### Scenario: SLSA provenance 検証失敗
- **WHEN** CI が upstream release の attestation 検証に失敗する
- **THEN** CI job を fail させ、`gh issue create` で tracked-issue を自動起票して手動対応を促す
- **AND** manifest bump は行われない

#### Scenario: bump PR に breaking change が含まれる場合は棄却
- **WHEN** bump PR の nix-installer が CLI flag 廃止・receipt schema 変更等の breaking change を含む
- **THEN** SchneeForge の release cycle 評価で PR を棄却し、SchneeForge 側で対応を整えてから取り込む

#### Scenario: runtime は gh / cosign を要求しない
- **WHEN** 利用者 PC で `schneeforge nix install` を実行する
- **THEN** `gh` や `cosign` が未導入でも manifest の SHA256 比較で検証を完結する

### Requirement: subprocess 実行と logger stderr parse
SchneeForge SHALL は nix-installer を subprocess 実行し、plan.json を **positional argument** として渡し (`install <plan.json>`)、`--logger json` の stderr を JSON Lines として best-effort parse する。SchneeForge 側で phase (Download / Verify / Privilege / Plan / Install / PostInstall) を管理し、installer 内部のメッセージに直接依存しない。plan と planner-subcommand は upstream 側で排他 (両方渡すと upstream が error。SchneeForge は plan を secure dir 内で生成するため user がこの状態を作ることはできない)。flakes は plan 生成時 (`plan <planner> --enable-flakes`) に plan へ焼き込み、install replay 時には再度指定しない。

#### Scenario: subprocess で install を実行
- **WHEN** `schneeforge nix install` を実行する
- **THEN** `nix-installer install <plan.json> --logger json --no-confirm` を subprocess で起動する (plan は positional)
- **AND** stderr の JSON Lines を SchneeForge 側 phase へ map して progress 表示する

#### Scenario: installer 内部メッセージの schema 変更に耐性がある
- **WHEN** upstream が installer 内部の `Step: *` メッセージを変更する
- **THEN** SchneeForge 側の phase 表示は壊れない (best-effort parse のみ依存)

### Requirement: 2 段階 Plan UX
SchneeForge SHALL は root 不要の preflight と、root 実行後の detailed plan 表示・最終確認に分けて UX を提供する。upstream を `--no-confirm` で呼ぶため、install 前の確認責任は SchneeForge 側にある。

#### Scenario: preflight は root 不要で概要を表示
- **WHEN** ユーザーが `schneeforge nix install` を起動する
- **THEN** `/nix`, daemon/launchd, build users, shell profiles, flakes を変更する旨を root 権限無しで表示する
- **AND** root 未実行の場合は `sudo schneeforge nix install` での再実行を促して終了する

#### Scenario: detailed plan 表示と最終確認
- **WHEN** root で `schneeforge nix install` を実行する
- **THEN** `nix-installer plan <planner> --enable-flakes` を実行し、stdout へ出力された plan JSON を secure dir 内の file へ保存して actions の概要を表示する (upstream 2.35.1 の `plan` に出力先 flag は無く plan JSON は stdout へ出力される)
- **AND** TTY では `y/N` の最終確認を求め、`y`/`yes` 以外なら install せずに終了する
- **AND** 非 TTY (CI 等) では確認を取れないため error で終了する
- **AND** `--yes` 指定時は最終確認を skip する (automation 用)

#### Scenario: install 失敗時
- **WHEN** detailed plan 表示後にユーザーが install を中止する
- **THEN** upstream installer は実行せず、`/nix` は変更しない

### Requirement: 既存 Nix 検出は PATH 以外の known locations も含める
SchneeForge SHALL は既存 Nix の検出に PATH のみでなく `/nix/var/nix/profiles/default/bin` 等 の known locations を含める (sudo 実行時の minimal PATH で PATH-only 検出が false negative にならないようにする)。検出は tool-resolution と同一の resolver を使う。

#### Scenario: PATH に無い既存 Nix を検出する
- **WHEN** Nix が `/nix/var/nix/profiles/default/bin/nix` に存在するが PATH に無い状態 (sudo の minimal PATH 等) で `schneeforge nix install` を実行する
- **THEN** 既存 Nix として検出し、`ExistingNixDetected` で install を中止する

### Requirement: post-install verification
SchneeForge SHALL は install 完了後に nix binary の解決・`nix store ping`・flakes 有効化を確認してから成功を宣言する。upstream installer の self-test 失敗は warning に留まるため、SchneeForge 側で最終 gate を持つ。検証失敗時も install 済み Nix の自動 rollback は行わない。

#### Scenario: 検証成功時のみ成功表示
- **WHEN** install 完了後の検証で nix binary / store / flakes が全て OK
- **THEN** `Managed Nix install 完了` を表示して正常終了する

#### Scenario: 検証失敗時は non-zero で終了
- **WHEN** install 完了後の検証で `nix store ping` 等に失敗する
- **THEN** 失敗項目を表示し、non-zero exit で終了する
- **AND** SchneeForge は install 済み Nix の自動 rollback を行わず、`schneeforge nix doctor` を案内する

### Requirement: ownership record による uninstall safety
SchneeForge SHALL は install 成功時に `/nix/schneeforge-managed.json` へ ownership record (provider・installer version・**installer SHA256**・upstream receipt path) を書き込み、uninstall 時の信頼根拠とする。

#### Scenario: uninstall 時の cached installer 再検証
- **WHEN** `/nix/nix-installer` が無く、cached binary を uninstall に使う場合
- **THEN** ownership record が保存した installer SHA256 と cached binary の再計算 hash を比較し、一致しなければ abort する

#### Scenario: custom receipt は ownership record と一致しなければ拒否
- **WHEN** `--receipt` で ownership record の `upstream_receipt` と異なる path が指定された場合
- **THEN** 既定では error で停止する (valid な ownership を別 receipt への root 実行に転用させない)

### Requirement: root 実行時の privileged state は root 管理下に置く
SchneeForge SHALL は root 実行時の installer cache と plan file を privileged state dir (Linux: `/var/lib/schneeforge`、macOS: `/private/var/db/schneeforge`) 配下に置き、sudo で持ち込まれた user の HOME/XDG 変数に依存した user-writable path を root 実行 binary の保存先に使わない。macOS では `/var` が `/private/var` への symlink であるため、symlink を含まない実 path を使う。

#### Scenario: root 実行時の cache path
- **WHEN** root で `schneeforge nix install` を実行する
- **THEN** installer binary は privileged state dir 配下の `managed-nix/cache/{version}/nix-installer` に保存される
- **AND** macOS では `/private/var/db/schneeforge`、Linux では `/var/lib/schneeforge` を使う

#### Scenario: download の temp file は既存 file や symlink を open しない
- **WHEN** installer binary を download する
- **THEN** temp file は random suffix + 排他作成 (`O_CREAT|O_EXCL`) で作成され、atomic rename で確定する

### Requirement: manifest 値の検証
SchneeForge SHALL は `bootstrap-manifest.toml` を load 時に検証する (version は `X.Y.Z` 数値形式、sha256 は 64 文字の hex)。

#### Scenario: 不正な manifest は load 時に拒否
- **WHEN** version が `v2.35` 等の形式、または sha256 が 64 hex でない manifest を load する
- **THEN** `ManifestParse` エラーで停止する

### Requirement: receipt は `/nix/receipt.json` を source of truth とする
SchneeForge SHALL は `/nix/receipt.json` を source of truth とし、独自の receipt を複製しない。

#### Scenario: doctor は receipt を読んで診断する
- **WHEN** `schneeforge nix doctor` を実行する
- **THEN** `/nix/receipt.json` を読み、`version / actions / planner` を表示する
- **AND** SchneeForge は別の場所へ receipt を複製しない

#### Scenario: doctor は receipt が無くても診断を継続する
- **WHEN** `/nix/receipt.json` が存在しない状態で doctor を実行する
- **THEN** 「receipt not found」を状態として報告し、Managed Nix 未 install の可能性を案内した上で他項目の診断を継続する (doctor は fresh machine でも動く診断コマンドであるため、正常終了する)

#### Scenario: uninstall は receipt が無ければ停止する
- **WHEN** `/nix/receipt.json` が存在しない状態で uninstall を実行する
- **THEN** `ReceiptNotFound` エラーで停止し、手動対応を案内する

### Requirement: uninstall の順序保証
SchneeForge SHALL は uninstall 時に nix-darwin の残留を検出し、残留時は nix-installer の uninstall を呼ぶ前に SSL cert 破損リスクを警告する。Phase 1 では nix-darwin の自動取り外しは行わず、公式 uninstaller (`nix-darwin#darwin-uninstaller`) の実行を案内する (SchneeForge からの自動呼び出しは別 change で設計後に自動化へ昇格)。

#### Scenario: nix-darwin 残留時は警告して abort
- **WHEN** nix-darwin が検出される状態で `schneeforge nix uninstall` を実行する
- **THEN** SSL cert 破損リスクを警告し、公式 uninstaller (`nix-darwin#darwin-uninstaller`) の実行を案内して abort する
- **AND** SchneeForge は自動的な nix-darwin 削除を実行しない (Phase 1)

#### Scenario: nix-darwin 非残留時はそのまま uninstall
- **WHEN** nix-darwin が検出されない状態で `schneeforge nix uninstall` を実行する
- **THEN** `/nix/nix-installer uninstall --no-confirm` を subprocess 呼び出しする
- **AND** cleanup 確認 (build users・/nix の削除) を表示する

#### Scenario: SchneeForge は revert logic を再実装しない
- **WHEN** uninstall を実行する
- **THEN** SchneeForge 側で独自の revert 処理を実装せず、`nix-installer uninstall --no-confirm` を subprocess 呼び出しする

### Requirement: flakes を default 有効化
SchneeForge SHALL は nix-installer へ `--enable-flakes` を default で渡す。

#### Scenario: install 後に flakes が有効
- **WHEN** `schneeforge nix install` を完了する
- **THEN** `nix config show experimental-features` に `flakes` と `nix-command` が含まれる

### Requirement: Nix version を UI から直接指定させない
SchneeForge SHALL は nix-installer tag で間接的に Nix version を指定し、ユーザー UI から直接 Nix version を指定させない。

#### Scenario: installer が選択する Nix version を利用
- **WHEN** SchneeForge が install を実行する
- **THEN** `--nix-package-url` 等の上書きを行わず、installer tag に紐づく Nix version を利用する
- **AND** CLI / GUI から Nix version を直接指定するオプションを提供しない

### Requirement: offline 初回起動時の明示的エラー
SchneeForge SHALL は offline 環境でキャッシュが無い場合、install を開始せずに network 必要を明示する。

#### Scenario: offline かつキャッシュ無し
- **WHEN** network に接続せず、かつキャッシュも存在しない状態で `schneeforge nix install` を実行する
- **THEN** `NetworkRequired` エラーで停止する
- **AND** ユーザーへ online になることを案内する

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

### Requirement: Nix 状態分類 (NixStatus)

SchneeForge SHALL は install 済み環境を `Missing` / `Healthy` / `Degraded` / `Broken` の 4 状態に分類する `NixStatus` model を持つ。分類は installation marker (`/nix/store`, `/nix/var/nix`, `/nix/receipt.json`)、receipt の内容、ownership record、runtime 検証 (`nix store ping`) の組合せで決定する。

#### Scenario: Missing — Nix 未導入

- **WHEN** installation marker が一切存在しない
- **THEN** `NixStatus::Missing` に分類する
- **AND** 次アクションとして `schneeforge nix install` を案内する

#### Scenario: Healthy — 完全に稼働する install

- **WHEN** marker が存在し、receipt が読め、`nix store ping` が成功する
- **THEN** `NixStatus::Healthy` に分類する
- **AND** 次アクションとして「対応不要」を表示する

#### Scenario: Degraded — marker 残存だが不完全

- **WHEN** installation marker は存在するが receipt が読めない、または `nix store ping` が失敗する
- **THEN** `NixStatus::Degraded` に分類する
- **AND** 次アクションとして修復手段 (現時点では `schneeforge nix uninstall` + 手動確認、将来は `nix repair`) を案内する
- **AND** install は `ExistingNixDetected` で拒否する (fail-closed を維持)

#### Scenario: Broken — ownership と実態の不一致

- **WHEN** ownership record が存在するが `/nix` 配下の実態が削除されている (またはその逆)
- **THEN** `NixStatus::Broken` に分類する
- **AND** 手動での調査を要する旨と、不一致の内容 (どちら側が残っているか) を表示する

### Requirement: NixStatus の分類 input は injectable である

SchneeForge SHALL は NixStatus の分類 input (marker path 群・receipt path・ownership path・ping 結果) を引数で差し替え可能にする。実環境の `/nix` に依存した test は書かない。

#### Scenario: unit test は実 /nix に依存しない

- **WHEN** NixStatus の unit test を実行する
- **THEN** tempdir 上に marker / receipt を配置して分類を検証する
- **AND** test の成败が実行環境の Nix 有無に影響されない

### Requirement: doctor は NixStatus を表示する

`schneeforge nix doctor` SHALL は診断の冒頭に NixStatus 分類と次アクションを表示する。既存の receipt / runtime 診断項目は維持する。

#### Scenario: doctor が分類を冒頭に表示

- **WHEN** `schneeforge nix doctor` を実行する
- **THEN** `[status]` 欄に 4 状態のいずれかと次アクション案内が表示される
- **AND** 既存の receipt / runtime 診断が引き続き出力される

#### Scenario: doctor はどの状態でも正常終了する

- **WHEN** いずれの状態 (Missing を含む) で `schneeforge nix doctor` を実行する
- **THEN** doctor は非 zero exit で異常終了しない (診断コマンドであるため)


### Requirement: GUI 向け privilege escalation helper

SchneeForge SHALL は GUI から特権操作を委譲するための escalation helper を core に持つ。helper は macOS では osascript、Linux では pkexec を使う command を構築し、実行する command は SchneeForge の CLI binary (GUI bundle に同梱された sidecar) に限定する。昇格先には `NIX_SETTING_DIR` (repo 位置) を環境変数として明示渡しする — root 環境では HOME が変わり user の repo が解決できなくなるため。

#### Scenario: macOS で osascript 経由の command を構築する

- **WHEN** macOS で SchneeForge CLI を管理者権限で再実行する command を構築する
- **THEN** `osascript -e 'do shell script "…" with administrator privileges'` 形式の引数列が構築される
- **AND** 実行する文字列に含まれる quote 等が escape される
- **AND** 実行する文字列の先頭に `NIX_SETTING_DIR` の export が置かれる

#### Scenario: Linux で pkexec 経由の command を構築する

- **WHEN** Linux で SchneeForge CLI を管理者権限で再実行する command を構築する
- **THEN** `pkexec env <env-assignments…> <schneeforge-binary> nix install --yes` 形式の引数列が構築される
- **AND** GUI 表示に必要な環境変数 (DISPLAY / XAUTHORITY / WAYLAND_DISPLAY) が引き継がれる
- **AND** env-assignments に `NIX_SETTING_DIR` が含まれる

#### Scenario: 任意の command は実行しない

- **WHEN** helper に SchneeForge binary 以外の実行対象を渡す要求がある
- **THEN** 構築を拒否する、または SchneeForge の subcommand 引数として安全に escape された形式のみを受け付ける

### Requirement: GUI から昇格再実行する install は CLI と同一 policy に従う

GUI 経由で昇格実行される `schneeforge nix install --yes` SHALL は CLI 実行と同一の policy (既存 Nix 拒否 / plan 生成 / ownership 記録 / post-install gate) に従う。GUI 経由であることを理由に確認や検証を省略しない。

#### Scenario: GUI 経由の install も既存 Nix を上書きしない

- **WHEN** GUI から昇格実行された install が既存 Nix を検出する
- **THEN** install は ExistingNixDetected で失敗する
- **AND** GUI は失敗を表示し `/nix` は変更されない

#### Scenario: GUI 経由の install も ownership record を記録する

- **WHEN** GUI 経由の install が成功する
- **THEN** CLI と同一の ownership record が書き込まれ uninstall 対称性が保たれる

### Requirement: nix repair は NixStatus に基づいて修復 action を決定する

SchneeForge SHALL は `schneeforge nix repair` command を持ち、`NixStatus` 分類を入力として状態ごとの修復 action を決定する。repair は SchneeForge 単独で安全に実行できる操作 (stale record の削除・案内表示) のみを行い、破壊的な uninstall / 再 install の自動実行は行わない。

#### Scenario: Broken 状態で stale ownership record を削除する

- **WHEN** ownership record は存在するが installation marker が一切存在しない (Broken) 状態で `schneeforge nix repair` を実行する
- **THEN** stale ownership record を削除する
- **AND** 削除後に `schneeforge nix doctor` が `Missing` を表示する状態へ復帰する

#### Scenario: Degraded 状態で receipt 有りは uninstall を案内する

- **WHEN** marker は存在し receipt が読めるが store ping が失敗する (Degraded) 状態で `schneeforge nix repair` を実行する
- **THEN** `schneeforge nix uninstall` による削除と再 install を案内する
- **AND** uninstall を自動実行しない

#### Scenario: Degraded 状態で receipt 無しは手動手順を案内する

- **WHEN** marker のみ残存し receipt が読めない (Degraded) 状態で `schneeforge nix repair` を実行する
- **THEN** upstream が revert できない旨と `sudo schneeforge nix uninstall --force` (build users の手動削除を含む手順) を表示する
- **AND** `/nix` 配下や build users の削除を自動実行しない

#### Scenario: Healthy / Missing は対応不要を表示して正常終了する

- **WHEN** Healthy または Missing 状態で `schneeforge nix repair` を実行する
- **THEN** Healthy は「対応不要」、Missing は install 案内を表示する
- **AND** いずれも file system を変更せず正常終了する

### Requirement: nix repair は dry-run を持つ

`schneeforge nix repair` SHALL は `--dry-run` で実行予定の action を表示するのみで file system を変更しない。

#### Scenario: dry-run は stale record を削除しない

- **WHEN** Broken 状態で `schneeforge nix repair --dry-run` を実行する
- **THEN** 削除対象の ownership record path と実行内容を表示する
- **AND** ownership record は削除されず Broken 状態が維持される

### Requirement: upstream repair hooks / sequoia を wrap する

SchneeForge SHALL は upstream `nix-installer repair hooks` (shell profile 修復) と `repair sequoia` (macOS Sequoia の `_nixbld` 回復) を subprocess 呼び出しする option (`--hooks` / `--sequoia`) を持つ。SchneeForge 側で修復 logic を再実装しない (uninstall と同じ委譲方針)。

#### Scenario: repair hooks は upstream を呼び出す

- **WHEN** `schneeforge nix repair --hooks` を実行する
- **THEN** `nix-installer repair hooks` 相当の upstream command を `/nix/nix-installer` (または cached binary) 経由で subprocess 実行する
- **AND** upstream の stderr を利用者に表示する

#### Scenario: sequoia は明示指定のみで実行する

- **WHEN** `schneeforge nix repair` を option 無しで実行する
- **THEN** `repair sequoia` を自動実行しない (Sequoia 乗っ取りの検出・判定は行わない)
- **AND** macOS 15 環境向けの手順として `--sequoia` の存在を案内に含める場合のみ表示する

### Requirement: doctor の次アクションは repair を案内する

`schneeforge nix doctor` SHALL は Degraded / Broken の次アクション文案として `schneeforge nix repair` を含める。

#### Scenario: Degraded の案内が repair を指す

- **WHEN** Degraded 状態で `schneeforge nix doctor` を実行する
- **THEN** 次アクションに `schneeforge nix repair` が含まれる

#### Scenario: Broken の案内が repair を指す

- **WHEN** Broken 状態で `schneeforge nix doctor` を実行する
- **THEN** 次アクションに `schneeforge nix repair` が含まれる
