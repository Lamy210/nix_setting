## ADDED Requirements

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
SchneeForge SHALL は upstream release を取り込む CI で SLSA provenance (`gh attestation verify`) と SHA256SUMS を検証し、`bootstrap-manifest.toml` へ bump する PR を作成する。runtime は manifest の SHA256 のみを検証する。

#### Scenario: CI が release を verify して manifest を更新
- **WHEN** upstream が新 release を公開する
- **THEN** SchneeForge の CI は `gh attestation verify` で provenance を確認する
- **AND** SHA256SUMS から各 arch の sha256 を抽出して manifest を更新する PR を作成する

#### Scenario: SLSA provenance 検証失敗
- **WHEN** CI が upstream release の attestation 検証に失敗する
- **THEN** manifest bump を行わず、alert を出す

#### Scenario: runtime は gh / cosign を要求しない
- **WHEN** 利用者 PC で `schneeforge nix install` を実行する
- **THEN** `gh` や `cosign` が未導入でも manifest の SHA256 比較で検証を完結する

### Requirement: subprocess 実行と logger stderr parse
SchneeForge SHALL は nix-installer を `--logger json` 付きで subprocess 実行し、stderr を JSON Lines として best-effort parse する。SchneeForge 側で phase (Download / Verify / Privilege / Plan / Install / PostInstall) を管理し、installer 内部のメッセージに直接依存しない。

#### Scenario: subprocess で install を実行
- **WHEN** `schneeforge nix install` を実行する
- **THEN** `nix-installer install --logger json --enable-flakes --no-confirm` を subprocess で起動する
- **AND** stderr の JSON Lines を SchneeForge 側 phase へ map して progress 表示する

#### Scenario: installer 内部メッセージの schema 変更に耐性がある
- **WHEN** upstream が installer 内部の `Step: *` メッセージを変更する
- **THEN** SchneeForge 側の phase 表示は壊れない (best-effort parse のみ依存)

### Requirement: 2 段階 Plan UX
SchneeForge SHALL は root 不要の preflight と、管理者認証後の detailed plan に分けて UX を提供する。

#### Scenario: preflight は root 不要で概要を表示
- **WHEN** ユーザーが `schneeforge nix install` を起動する
- **THEN** `/nix`, daemon/launchd, build users, shell profiles, flakes を変更する旨を root 権限無しで表示する
- **AND** ユーザーが Continue を選ぶまで install を開始しない

#### Scenario: 管理者認証後に detailed plan を取得
- **WHEN** ユーザーが Continue を選ぶ
- **THEN** 管理者認証を要求した上で `nix-installer plan --out-file plan.json` を実行し、detailed plan を表示する
- **AND** Install の最終確認を再度ユーザーに求める

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
SchneeForge SHALL は uninstall 時に nix-darwin を先に外してから nix-installer の uninstall を実行し、SSL cert 破損を防止する。

#### Scenario: nix-darwin 残留時は先に外す
- **WHEN** nix-darwin が検出される状態で `schneeforge nix uninstall` を実行する
- **THEN** nix-darwin を先に外す手順を表示/実行し、その後に nix-installer の uninstall を呼ぶ
- **AND** SchneeForge は SSL cert 破損リスクを事前に警告する

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
SchneeForge SHALL は install / uninstall 時の privilege escalation を明示的に行い、GUI 起動時も TTY 非依存で認証を要求する。

#### Scenario: GUI 起動でも認証を要求
- **WHEN** Tauri GUI から install を実行する (Phase 2 以降)
- **THEN** TTY に依存せず、osascript (macOS) / pkexec (Linux) 等で認証を要求する

#### Scenario: CLI 実行時の root 昇格
- **WHEN** CLI から install を実行する
- **THEN** 必要に応じて sudo 経由で自身を再実行する旨をユーザーに明示する
