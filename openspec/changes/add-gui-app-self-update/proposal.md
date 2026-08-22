# Change: GUI 本体の自己更新 (設計提案 — 実装は方針決定後の別 change)

## Why

CLI 側の自己更新 `schneeforge self-update` は merge 済み (PR #70)。
GUI (SchneeForge.app / DMG) 側は Dashboard が新しい release を検知して
も **notify-only** (`dash-update`: 「GitHub Releases / install.sh で更新
できます」) であり、本体 (.app) の更新は user が手動で DMG を download
して差し替える運用のまま。

rc.6 以降 GUI を使い続ける user にとって、release 毎の手動差し替えは
唯一の手動 step になる。ただし実装方式には鍵管理・release pipeline 変更
を伴う選択肢があり、STATUS.md の指針通り **user の設計判断が必要** と
して本 proposal で選択肢を提示する (2026-08-22 の user 決定により起草)。

## 前提となる事実確認 (2026-08-22)

- Tauri v2 updater plugin (公式 doc 実確認):
  - 署名検証は **mandatory** (無効化不可)。minisign 鍵 pair
    (`tauri signer generate`)。public key は `tauri.conf.json` の
    `plugins.updater.pubkey`、private key は build 時に
    `TAURI_SIGNING_PRIVATE_KEY` env で与える
  - **private key を失うと既存 install への update 配信が不可能**
    (backup・保管方針が必須)
  - updater artifact は `bundle.createUpdaterArtifacts: true` で生成:
    macOS は `.app.tar.gz` + `.sig`、**Linux は AppImage のみ**、
    Windows は NSIS/MSI
  - update manifest (`latest.json` 相当) は `version` +
    `platforms.<os-arch>.{url,signature}` (signature は .sig の中身)
  - `tauri-action` が GitHub Releases hosting 用の latest.json 生成を
    代行可能
- SchneeForge の現状:
  - macOS GUI 配布は **DMG のみ** (release asset)。Linux GUI は
    `nix build` (AppImage 配布なし) → **tauri-plugin-updater は
    Linux では実質使えない**
  - Windows build は未提供
  - release の integrity は CHECKSUMS.txt (SHA256) + SLSA provenance
    attestation (PR #67) で担保 — updater 署名とは別階層
    (TLS 配信前提の hash 突合 vs end-to-end minisign 署名)
  - release workflow の DMG build は host cargo (RC2 事故以降) で
    署名 env は未設定

## 選択肢

### Option A: tauri-plugin-updater 導入 (macOS のみ完全自動更新)

Dashboard の案内からワンクリックで download → 署名検証 → .app 置換 →
relaunch。Tauri v2 標準経路。

- 必要: minisign 鍵 pair (private key は GitHub Actions secret +
    backup)、`createUpdaterArtifacts`、`.app.tar.gz` + `.sig` +
    `latest.json` の 3 asset 追加、release workflow と RELEASE.md
    checklist の拡張
- 利点: end-to-end 署名 (GitHub/CDN 侵害に対しても強い)、UX 完全自動、
    公式 plugin 保守
- 代价: 鍵管理の恒常運用 (喪失時 update 不可・rotation 時は旧 install
    に届かない)、Linux GUI は対象外 (notify-only のまま)、
    asset が release unit に 3 追加

### Option B: notify-only 強化 (半自動 — 開く・拾ってくる)

現状の検知表示に、(1)「GitHub Releases を開く」button (browser で
該当 release へ)、(2) 略 — CLI のみ `schneeforge self-update` を GUI
から起動する command — を足す。GUI 本体は手動差し替えのまま。

- 必要: 小規模な frontend + Tauri command 変更のみ。鍵・pipeline 変更なし
- 利点: 実装が最小、鍵管理不要、release unit 不変
- 代价: .app の手動差し替えが残る (update UX の本丸は解消しない)。
    また GUI は core を直接 call する構造 (Tauri command) で
    standalone CLI を更新しても GUI 内蔵 logic は更新されないため、
    (2) の価値は限定的

### Option C: 独自 updater (download + 検証 + .app 置換を自前実装)

CHECKSUMS.txt 突合 + .app 置換を core に実装し CLI と同じ検証 model で
統一。

- 代价: .app の実行中置換・quarantine attribute (Gatekeeper)・
    relaunch などの macOS 特有の safety を自前で担うことになり、
    tauri-plugin-updater が解決済みの問題群を再発明する。非推奨

### Option D: 現状維持 (notify-only)

rc.6 時点から変えない。GUI user が少数の間は手動差し替えでも運用可能。

## 推奨

**Step 1 として Option B (1) のみ (Releases への link button)** を先行、
**Step 2 として Option A を鍵管理方針の決定後に別 change で実装**。

理由:

- Option A は技術的には正道だが、鍵の喪失が「既存 install を永久に
    update できない」事故に直結する運用 risk を持つ。鍵の保管
    (password manager / CI secret の 2 重保管)、rotation 方針、
    dev build での無署名挙動を user が明示的に決めてから導入するべき
- Linux GUI (nix 配布) が updater 対象外である以上、A を入れても
    platform 毎に案内が分かれる。B(1) はその差を埋める最小の UX 改善
- B(1) は鍵・asset・pipeline を一切変えずリリース可能で、A への
    移行時も無駄にならない (案内 UI の土台になる)

## User の決定が必要な点 (Open Questions)

1. **minisign 鍵の管理方針** (Option A の前提)
   - private key の保管先と backup、CI secret 名、rotation 要否
2. **Linux GUI の扱い**
   - nix 配布のまま updater 対象外でよいか (notify-only 継続)
3. **latest.json の生成方式**
   - `tauri-action` 任せ vs release workflow で自前生成
     (`schneeforge-release.json` との併存・役割分担)
4. **実装タイミング**
   - rc.6 → Final Acceptance → v0.3? (A の asset 追加は release
     pipeline 変更を伴うため、rc 直後の方が混乱が少ない)

## 非対象 (本 change では実装しない)

- **実装そのもの** — 本 change の成果物は本 proposal + design.md
  (意思決定材料) のみ。方針決定後に実装 change (spec delta 付き) を
  起票する
- Windows 向け updater (Windows build 未提供のため)
- CLI 側 self-update の変更 (PR #70 で完了済み)
