# Design: GUI 本体の自己更新 (選択肢比較と実装方式の素材)

目的は実装ではなく **user の設計判断の材料** を揃えること。各 option の
実装形態・影響範囲・risk を比較する。

## 1. 現状の data flow (notify-only)

```
core ReleaseMetadata (§27, PR #50)
  └─ release channel の最新 tag を ls-remote / schneeforge-release.json から解決
       └─ Tauri command (dashboard) → dist/main.js
            └─ dash-update: 「新しいリリース vX があります — GitHub Releases / install.sh で更新できます」
```

検知 (update_available) は既に core test あり。本 change の論点は
「検知したあと GUI が何をするか」のみ。

## 2. Option A: tauri-plugin-updater (macOS 完全自動) の実体

### 2.1 必要な変更

| 対象 | 変更 |
|---|---|
| `tauri.conf.json` | `bundle.createUpdaterArtifacts: true`、`plugins.updater.pubkey`、`plugins.updater.endpoints` |
| capabilities | `updater:default` (+ relaunch 用 `process:allow-restart`) |
| frontend | `check()` → 確認 dialog → `downloadAndInstall()` (progress 表示) → relaunch |
| release workflow | `TAURI_SIGNING_PRIVATE_KEY` (+password) secret、`.app.tar.gz` / `.sig` / `latest.json` の upload |
| RELEASE.md | checklist に鍵 confirm・asset 検証 step 追加 |
| 鍵運用 | 鍵 pair 生成・backup・保管 (下記 2.3) |

endpoint は静的 JSON 1 つで成立:
`https://github.com/Lamy210/nix_setting/releases/latest/download/latest.json`
(production では HTTPS 強制。`{{target}}`/`{{arch}}` template も可だが
単一 URL + platforms map で足りる)

### 2.2 update manifest (latest.json) の中身

```json
{
  "version": "0.2.0-rc.6",
  "notes": "...",
  "pub_date": "2026-08-22T00:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "url": "https://github.com/Lamy210/nix_setting/releases/download/v0.2.0-rc.6/SchneeForge.app.tar.gz",
      "signature": "<.sig の中身>"
    }
  }
}
```

Tauri は **file 全体を検証してから** version 比較するため、対応
platform entry が不完全だと全 platform で fail する (fail-closed)。
SchneeForge は darwin-aarch64 のみ entry にする (他 platform は
配布していないため。entry 無し platform は plugin が対象外扱い)。

### 2.3 鍵管理の論点 (Option A 最大の decision point)

- **喪失事故**: private key を失うと「その key で build された全
  install」への update 配信が不可能。DMG を再 install してもらう
  以外の回復手段がない
- **保管案**: (a) GitHub Actions secret (1 鍵) + password manager
  等 user 私有の場所に offline backup、(b) 複数鍵の併用 — minisign
  的には可能だが tauri plugin の support 範囲を確認要
- **rotation**: pubkey は app の build 時に焼き込まれる。新鍵への
  交代は「version N に新 pubkey を compile (旧鍵で署名) → user が
  N に更新した後、N+1 から新鍵で署名」の 2 release にまたがる手順
  になる。運用 complexity が高いので **初回鍵は長期運用前提** とし
  rotation は破洩時のみ、が現実的
- **dev build**: 署名 env が無い build では updater artifact が
  作られない。CI の PR preview build では `createUpdaterArtifacts`
  を条件付きにする等の調整が必要

### 2.4 security 上の位置づけ

| 層 | 現行 (CHECKSUMS.txt) | Option A (minisign) |
|---|---|---|
| 通信 | TLS (GitHub CDN) | TLS |
| 改ざん検出 | download 後 sha256 突合 | **end-to-end 署名** (CDN/GitHub 侵害時も成立) |
| 鍵 | なし (hash 直値) | minisign 鍵 pair |
| 対象 | 全 asset | updater artifact のみ |

fail-closed 設計 (検証失敗時は置換しない・例外なし) は CLI
self-update (PR #70) と同じ原則。`latest.json` 自体の真贋は
「署名対象の .tar.gz が正しいこと」で間接的に担保される (manifest
改竄 → 署名 mismatch → install 中止)。

### 2.5 release unit への影響

- asset が 3 個追加 (`.app.tar.gz` / `.sig` / `latest.json`)。
  「1 release = 1 source tree = 1 checksum set」原則は維持可能
  (CHECKSUMS.txt に 3 asset を含める。provenance attestation の
  subject は asset 全てなので自動的に coverage)
- DMG (手動 install 用) と updater artifact (自動更新用) が同一
  source tree から生成されることは RC2 事故 (CI binary ≠ release
  binary) の教訓と整合 — build script 共通化を維持する

## 3. Option B(1): Releases link button の実体

- frontend: `dash-update` 表示時に button を出す。click で
  `https://github.com/Lamy210/nix_setting/releases/tag/<tag>` を開く
  (Tauri opener plugin / `open_url` command)
- backend: URL は core が解決済みの tag から組み立て (test 可能な
  純関数)
- 鍵・asset・pipeline 変更なし。Step 2 (Option A) 導入後は
  「自動更新できない platform 向けの fallback 案内」として残る

## 4. Platform 別の挙動まとめ

| Platform | 配布 | Option A | Option B(1) |
|---|---|---|---|
| macOS aarch64 | DMG + updater artifact | 自動更新可 | link 案内 |
| Linux x86_64/aarch64 | `nix build` (GUI) | **不可** (AppImage のみ対応のため) | link 案内 (nix flake 更新案内に読み替え) |
| Windows | 未提供 | — | — |

Linux は nix で導入した GUI 自体も `nix flake update` 相当で更新される
前提のため、updater 対象外でも運用上の穴にはならない (案内文の調整
のみ)。

## 5. 推奨の段階実装

1. **Step 1 (Option B(1))**: link button。鍵議論と独立に merge 可能。
   実装 change は spec delta 付きで小規模
2. **Step 2 (Option A)**: Open Questions 1-3 の user 決定後に実装
   change を起票。rc 系 release ではなく minor release (v0.3) での
   導入が無難 (鍵運用開始のタイミングとして)
3. Step 2 の検証は macOS 実機での update e2e (rc N → rc N+1 の
   実際の置換) を Final Acceptance 的な手順書 item として追加

## 6. 検証戦略 (実装時)

- manifest 生成: release workflow の step を純関数化 (tag → json)
  し unit test (asset 名・signature 埋め込み・platform key)
- frontend gating: updater 有効/無効 (platform 判定) の分岐を
  serialize key × DOM id の regression test (rc.3 事故対策と同型)
- macOS 実機 e2e: 興味のある経路は ①正常置換+relaunch ②署名 mismatch
  (改竄 .tar.gz) で中止 ③network 断で「後で」fallback
