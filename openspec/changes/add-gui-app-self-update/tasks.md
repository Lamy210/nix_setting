# Tasks

## 1. 事実確認 (本 proposal 起草時に完了)

- [x] 1.1 Tauri v2 updater plugin の要件を公式 doc で確認 (署名 mandatory / createUpdaterArtifacts / manifest 形式 / platform 対応)
- [x] 1.2 SchneeForge 側の現状確認 (DMG のみ配布・Linux GUI は nix build・release workflow に署名 env なし)
- [x] 1.3 CLI self-update (PR #70) との検証 model の差整理 (CHECKSUMS 突合 vs minisign end-to-end)
- [x] 1.4 proposal.md / design.md として選択肢と推奨を文書化

## 2. User の設計判断 (Open Questions)

- [ ] 2.1 方針の決定 (推奨: Step 1 = B(1) link button を先行、Step 2 = Option A を別 change で)
- [ ] 2.2 Option A を採用する場合の minisign 鍵管理方針 (保管先・backup・rotation 要否)
- [ ] 2.3 Linux GUI を updater 対象外 (notify-only 継続) でよいか
- [ ] 2.4 latest.json の生成方式 (tauri-action vs release workflow 自前)
- [ ] 2.5 実装タイミング (v0.3 想定)

## 3. 実装 (方針決定後に別 change で起票 — この section は参考)

- [ ] 3.1 Step 1: Releases link button (spec delta 付き小 change)
- [ ] 3.2 Step 2: tauri-plugin-updater 導入 (鍵生成・release workflow 拡張・capabilities・frontend・RELEASE.md 更新)
- [ ] 3.3 Step 2 の macOS 実機 e2e 手順書への組込み
