# Tasks

## 1. OpenSpec

- [x] 1.1 proposal / design / delta specs / tasks を作成
- [ ] 1.2 `openspec validate add-gui-self-update-step2 --strict`
- [ ] 1.3 `openspec validate --all --strict`

## 2. Manifest generator (secret-free)

- [ ] 2.1 RED: latest.json generator contract test を追加
- [ ] 2.2 tag/version + URL + signature から static latest.json を生成
- [ ] 2.3 darwin-aarch64 only / HTTPS / non-empty signature / deterministic output を test
- [ ] 2.4 release workflow が generator を利用する contract を追加

## 3. Desktop backend

- [ ] 3.1 `tauri-plugin-updater` dependency を追加
- [ ] 3.2 pending update state + `fetch_app_update` command
- [ ] 3.3 `install_app_update` + progress event
- [ ] 3.4 signature/install failure を fail-closed で返す
- [ ] 3.5 unsupported/updater-disabled build の capability response

## 4. Frontend UX

- [ ] 4.1 macOS updater enabled build だけ auto-update button を表示
- [ ] 4.2 progress / result / restart UX
- [ ] 4.3 Releases link fallback を全 failure path で維持
- [ ] 4.4 Linux/Windows で auto-update control を表示しない
- [ ] 4.5 frontend/backend command mapping regression

## 5. Release pipeline preparation

- [ ] 5.1 tag release のみ updater artifact creation を有効化できる構造
- [ ] 5.2 updater artifact / sig / latest.json を CHECKSUMS / provenance 対象へ含める
- [ ] 5.3 signing secret 不足時は activation release を fail-closed
- [ ] 5.4 RELEASE.md に key backup / verification / rotation / E2E 手順を追加
- [ ] 5.5 PR CI は production signing secret を要求しない

## 6. Activation gate (merge後も未完了可)

- [ ] 6.1 macOS manual Final Acceptance PASS
- [ ] 6.2 production key pair を user が生成
- [ ] 6.3 private key + password を GitHub Actions secret に登録
- [ ] 6.4 public key を review して production config に固定
- [ ] 6.5 signed N -> N+1 updater E2E PASS
- [ ] 6.6 tampered artifact signature mismatch E2E PASS

## 7. Verification / lifecycle

- [ ] 7.1 cargo test / fmt / clippy / desktop build
- [ ] 7.2 actionlint / bats / release artifact gate / required checks
- [ ] 7.3 implementation PR merge
- [ ] 7.4 separate archive PR after implementation scope is complete
