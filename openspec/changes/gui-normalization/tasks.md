## 1. ToolResolver (core)

- [ ] 1.1 `ToolStatus { available, path, version }` を core に定義
- [ ] 1.2 `ToolResolver::nix/nh/git/brew` を実装（PATH → 既知パス順に解決）
- [ ] 1.3 `resolve_repo` を ToolResolver と統合してツール解決を一本化
- [ ] 1.4 ToolResolver の unit test（PATH 無し・既知パス有りのケース）

## 2. nh 循環依存の解消 (core)

- [ ] 2.1 `actions::apply` を `nh darwin switch` → `nix run nix-darwin -- switch` に変更
- [ ] 2.2 `actions::apply` Linux を `nh home switch` → `homeConfigurations.*.activationPackage` build + activate に変更
- [ ] 2.3 `actions::rollback` を `nh` 非依存に変更
- [ ] 2.4 `bootstrap.sh` の macOS 適用を `nh` 非依存パスに統一
- [ ] 2.5 actions の switch_command テストを更新

## 3. Status 診断 API (core + desktop)

- [ ] 3.1 `Status` struct を `ToolStatus` + repo/manifest/state 存在フラグに拡張
- [ ] 3.2 `get_status` command を診断情報（repo_path/repo_exists/manifest_found/manifest_error）対応に
- [ ] 3.3 Status の unit test（repo 無し・manifest 無しのケース）

## 4. GUI 診断型 UI (desktop)

- [ ] 4.1 index.html を診断型表示（System/Configuration/Runtime/State セクション）に
- [ ] 4.2 repo 無し時に「Repository not configured」を表示
- [ ] 4.3 `window.__TAURI__` の IPC guard を追加
- [ ] 4.4 `devUrl` を整理（static frontend のみ）

## 5. 非同期 Tauri command (desktop)

- [ ] 5.1 `run_apply/run_rollback/run_upgrade/run_verify` を `spawn_blocking` で非同期化
- [ ] 5.2 実行中 spinner + ボタン disable の確認

## 6. First Run Wizard (desktop)

- [ ] 6.1 state machine（Booting/NeedsSetup/Ready/Applying/Failed）を実装
- [ ] 6.2 NeedsSetup 時に Setup フロー（clone → username → config.toml 生成）を実装
- [ ] 6.3 Setup 完了後に Ready 状態へ遷移

## 7. GUI E2E テスト

- [ ] 7.1 App boot smoke test
- [ ] 7.2 Status が host を返す test
- [ ] 7.3 Missing repo 警告 test
- [ ] 7.4 Action spinner/disable test
