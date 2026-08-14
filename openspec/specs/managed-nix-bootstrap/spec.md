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

### Requirement: アプリデータ配下へのキャッシュ
SchneeForge SHALL は download した nix-installer binary を `XDG_DATA_HOME/schneeforge/managed-nix/{version}/nix-installer` へキャッシュし、二回目以降の install は offline で動作させる。

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
SchneeForge SHALL は nix-installer を subprocess 実行し、plan.json を **positional argument** として渡し (`install <plan.json>`)、`--logger json` の stderr を JSON Lines として best-effort parse する。SchneeForge 側で phase (Download / Verify / Privilege / Plan / Install / PostInstall) を管理し、installer 内部のメッセージに直接依存しない。plan と planner-subcommand は排他 (両方渡すと upstream が error)。flakes は plan 生成時 (`plan <planner> --enable-flakes`) に plan へ焼き込み、install replay 時には再度指定しない。

#### Scenario: subprocess で install を実行
- **WHEN** `schneeforge nix install` を実行する
- **THEN** `nix-installer install <plan.json> --logger json --no-confirm` を subprocess で起動する (plan は positional)
- **AND** stderr の JSON Lines を SchneeForge 側 phase へ map して progress 表示する

#### Scenario: plan と planner-subcommand の同時指定は不可
- **WHEN** 何らかの理由で plan と planner-subcommand を同時に指定した場合
- **THEN** nix-installer 側で error となり、SchneeForge は `ManagedNixError::PlannerConflict` として報告する

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
- **THEN** `nix-installer plan <planner> --out-file plan.json --enable-flakes` で detailed plan を生成し、actions の概要を表示する
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
SchneeForge SHALL は root 実行時の installer cache と plan file を `/var/lib/schneeforge` 配下に置き、sudo で持ち込まれた user の HOME/XDG 変数に依存した user-writable path を root 実行 binary の保存先に使わない。

#### Scenario: root 実行時の cache path
- **WHEN** root で `schneeforge nix install` を実行する
- **THEN** installer binary は `/var/lib/schneeforge/managed-nix/cache/{version}/nix-installer` に保存される

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

#### Scenario: receipt が存在しない
- **WHEN** `/nix/receipt.json` が存在しない状態で doctor / uninstall を実行する
- **THEN** `ReceiptNotFound` エラーで停止し、手動対応を案内する

### Requirement: uninstall の順序保証
SchneeForge SHALL は uninstall 時に nix-darwin の残留を検出し、残留時は nix-installer の uninstall を呼ぶ前に SSL cert 破損リスクを警告する。Phase 1 では nix-darwin の自動取り外しは行わず、ユーザーへ手動対応を促す (nix-darwin の安全な取り外し手順は ADR-0001 Open Question 4。別 change で設計後に自動化へ昇格)。

#### Scenario: nix-darwin 残留時は警告して abort
- **WHEN** nix-darwin が検出される状態で `schneeforge nix uninstall` を実行する
- **THEN** SSL cert 破損リスクを警告し、先に nix-darwin を手動で外すよう案内して abort する
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

