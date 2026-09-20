# Change: GUI self-update Step 2 (macOS updater, staged activation)

## Why

SchneeForge Desktop は update_available を検知し GitHub Releases への link を表示できるが、
SchneeForge.app 自体の更新は手動 DMG 差し替えのままである。

2026-08-23 に以下は決定済み:
- Step 2 は Tauri updater を採用
- macOS aarch64 のみ auto-update 対象
- Linux GUI は notify-only / Releases link を維持
- updater signing key は GitHub Actions secret + user offline backup の二重保管
- latest.json は release workflow で生成
- production activation は v0.3 / macOS Final Acceptance 後

現時点では macOS Final Acceptance の manual GUI / install.sh / full bootstrap gate と
production signing key の provision が未完了であるため、コード準備と本番 activation を
分離して実装する。

## What Changes

- **ADDED: macOS desktop updater backend**
  - `tauri-plugin-updater` を Rust backend から利用
  - `fetch_app_update` / `install_app_update` Tauri command
  - pending update は backend state で保持
  - download/install progress を event で frontend へ通知
  - install success 後は app restart を user confirmation 後に行う
- **MODIFIED: Dashboard update UX**
  - macOS では auto-update capability が有効な build のみ「アプリを更新」button を表示
  - Linux / unsupported platform / updater disabled build では既存 Releases link を維持
  - check/download/install failure は既存 app を保持し link fallback を表示
- **ADDED: updater manifest generator**
  - release tag + updater artifact URL + .sig content から static `latest.json` を生成
  - target key は `darwin-aarch64`
  - generator は pure script として PR CI で fixture test
- **MODIFIED: release pipeline contract**
  - tag release で updater artifact 作成を有効化する場合のみ signing secret を要求
  - updater artifact / .sig / latest.json を CHECKSUMS / provenance 対象へ追加
  - missing signing key / signature / manifest field は fail-closed
- **ADDED: staged activation gate**
  - production updater endpoint / public key / updater artifacts を shipping path に有効化する前に
    macOS Final Acceptance PASS と production key provision を必須とする
  - activation 前の develop / PR build は updater disabled として build/test 可能

## Non-goals

- Linux GUI の in-place updater
- Windows desktop distribution/updater
- CLI `self-update` の置換
- rollback / downgrade UI
- signing private key の repository 保存
- production key pair の自動生成
- Final Acceptance 未完了状態での shipping activation

## Impact

- specs: `gui-dashboard`, `release-supply-chain`
- desktop: updater dependency / backend commands / Dashboard UI
- CI: latest.json generator contract, release signing gate
- security: end-to-end updater signature verification is mandatory; verification failure never replaces the app
