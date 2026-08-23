# Tasks

## 1. 事実確認 (本 proposal 起草時に完了)

- [x] 1.1 Tauri v2 updater plugin の要件を公式 doc で確認 (署名 mandatory / createUpdaterArtifacts / manifest 形式 / platform 対応)
- [x] 1.2 SchneeForge 側の現状確認 (DMG のみ配布・Linux GUI は nix build・release workflow に署名 env なし)
- [x] 1.3 CLI self-update (PR #70) との検証 model の差整理 (CHECKSUMS 突合 vs minisign end-to-end)
- [x] 1.4 proposal.md / design.md として選択肢と推奨を文書化

## 2. User の設計判断 (Open Questions)

- [x] 2.1 方針の決定: **Step 1 = B(1) link button 先行・Step 2 = Option A を別 change で** (2026-08-23 決定。Step 1 は #81 で実装済み)
- [x] 2.2 minisign 鍵管理方針: **2 重保管・長期鍵** — GitHub Actions secret (`TAURI_SIGNING_PRIVATE_KEY` + password) と user 私有 (password manager 等) の offline backup。初回鍵は長期運用前提、rotation は破洩時のみ (2 release にまたがる手順) (2026-08-23 決定)
- [x] 2.3 Linux GUI は **updater 対象外 (notify-only / link 案内継続)**。nix 配布のため flake 更新 (`schneeforge update`) で更新される (2026-08-23 決定)
- [x] 2.4 latest.json は **release workflow 自前生成** — core に tag → json の純関数 + unit test、`schneeforge-release.json` と同じ生成経路で一貫性を制御 (2026-08-23 決定)
- [x] 2.5 実装タイミング: **v0.3 で導入** (Final Acceptance PASS 後。rc 系 release と鍵運用開始を分ける) (2026-08-23 決定)

## 3. 実装 (方針決定後に別 change で起票 — この section は参考)

- [x] 3.1 Step 1: Releases link button — `add-gui-releases-link-button` として実装・merge・archive 済み (PR #81 / #82)
- [ ] 3.2 Step 2: tauri-plugin-updater 導入 (鍵生成・release workflow 拡張・capabilities・frontend・RELEASE.md 更新) — v0.3 で別 change
- [ ] 3.3 Step 2 の macOS 実機 e2e 手順書への組込み — 3.2 と同じ change で
