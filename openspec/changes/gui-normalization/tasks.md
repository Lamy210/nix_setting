## 0. Specification reconciliation

- [x] 0.1 P0 hotfix requirements を gui-operations spec へ追加（Tauri API init / button dispatch / failure rendering）
- [x] 0.2 Plan / Verify の仕様差異を解消（proposal と tasks の齟齬を統一）
- [x] 0.3 Platform と ConfigurationTarget を分離して仕様化
- [x] 0.4 privilege escalation strategy を仕様化
- [x] 0.5 rollback semantics を仕様化（Generation Rollback / Configuration Revert / Restore Pre-install）
- [x] 0.6 state persistence（apply 後の State 保存）を core 責務として仕様化
- [x] 0.7 repo-aware operation（upgrade/sync が CWD 非依存）を仕様化
- [x] 0.8 `openspec validate` を通す

## 1. Core environment model

- [x] 1.1 Platform / Architecture / ConfigurationTarget を分離
- [x] 1.2 `Manifest::validate` を追加（schema == 1, username != "", 実行ユーザー一致チェック）
- [x] 1.3 `ToolStatus` / `ToolResolver` を追加（PATH → 既知パス、実行可能フラグ）
- [x] 1.4 `RepoResolver` を独立実装（ToolResolver とは別責務）
- [x] 1.5 structured error model を追加
- [x] 1.6 unit tests

## 2. State and operation safety

- [x] 2.1 State save を core へ移動（CLI/GUI 共通の ApplyResult に内包）
- [x] 2.2 atomic StateStore（temp → fsync → rename）
- [x] 2.3 process-wide operation lock を追加（CLI/GUI 共通）
- [x] 2.4 apply/rollback 後の state 更新
- [x] 2.5 state tests

## 3. Core operations（repo-aware）

- [x] 3.1 `plan(repo)`
- [x] 3.2 `apply(repo)`（state 保存込み）
- [x] 3.3 `verify(repo)`
- [x] 3.4 `rollback(repo)`（世代ロールバック）
- [x] 3.5 `upgrade(repo)` — `nix flake update --flake <repo>`
- [x] 3.6 `sync(repo)` — dirty check + `git -C <repo>` + `--ff-only`
- [x] 3.7 CLI を core delegation のみへ整理（doctor/setup/enable_flakes/plan/verify/sync/uninstall を core へ）

## 4. Bootstrap

- [ ] 4.1 prerequisite preflight（Nix/Git/flakes が実際に動くか）
- [ ] 4.2 pinned nix-darwin bootstrap（`--inputs-from <repo> nix-darwin#darwin-rebuild`）
- [ ] 4.3 Linux Home Manager bootstrap（locked HM input 利用を比較検討）
- [ ] 4.4 privilege handling（sudo / GUI privileged helper の分離）
- [ ] 4.5 backup/restore 設計
- [ ] 4.6 fresh-install integration tests（ケースB: repo/config/state/nh なし）

## 5. Diagnostics API

- [ ] 5.1 expanded Status（repo_path/repo_exists/manifest_found/manifest_error/ToolStatus）
- [ ] 5.2 system user と config user の不一致検出
- [ ] 5.3 repo/manifest/state の診断
- [ ] 5.4 tool path/version 診断

## 6. Desktop operations

- [ ] 6.1 async Tauri commands（spawn_blocking）
- [ ] 6.2 backend operation lock の handling
- [ ] 6.3 IPC guard
- [ ] 6.4 devUrl 削除（static frontend のみ）
- [ ] 6.5 CSP 設定（null を解消）
- [ ] 6.6 未使用の opener plugin / capability を削除

## 7. First Run Wizard

- [ ] 7.1 NeedsSetup UI
- [ ] 7.2 prerequisite step（Nix/Git 検出）
- [ ] 7.3 config generation（username/HOME を OS から取得）
- [ ] 7.4 plan step
- [ ] 7.5 explicit confirmation（自動 apply しない）
- [ ] 7.6 apply
- [ ] 7.7 verify
- [ ] 7.8 resume after failure

## 8. GUI E2E

- [ ] 8.1 boot
- [ ] 8.2 host/status 表示
- [ ] 8.3 missing repo
- [ ] 8.4 action mapping（button → IPC）
- [ ] 8.5 backend failure
- [ ] 8.6 spinner/disable
- [ ] 8.7 successful apply の state refresh

## 9. Release gate

- [ ] 9.1 OpenSpec validation を CI に追加
- [ ] 9.2 desktop build smoke
- [ ] 9.3 CLI artifact smoke
- [ ] 9.4 DMG smoke
- [ ] 9.5 Homebrew tap を `Lamy210/homebrew-tap` に分離（`brew install Lamy210/tap/schneeforge`）
- [ ] 9.6 README と実 Release を同期（v0.2.0-rc.1）
