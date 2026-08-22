# Change: CLI 自己更新 (`schneeforge self-update`)

## Why

Phase E の残件「self-update」が未実装。現在 Dashboard (§28) は
`update_available` を通知するのみで、本体 binary の更新は install.sh の
再実行か手動 download に頼っている。managed source は tag pin で
state と整合するのに、本体だけが古いバージョンのまま取り残される
構図になる (GUI の通知文も「GitHub Releases / install.sh で更新できます」
と手動誘導)。

## What Changes

- core に自己更新操作を追加する:
  - channel の最新 tag 解決 (既存 `remote_tags` + `latest_tag_for_channel`)
  - platform asset 選択 (install.sh と同一の gating)
  - `CHECKSUMS.txt` との sha256 突合
  - 同一 filesystem 上の temp file → rename による atomic 自己置換
- CLI に top-level `schneeforge self-update` を追加する (`update` は
  configuration source の更新なので混同を避ける)。
- GUI は現行の notify-only のまま (scope 外。後述)。

## 代替案と採用しない理由

- **tauri-plugin-updater (GUI 内自己更新)**: minisign 鍵 pair の永続
  管理と `latest.json` asset が必須で、既存の SHA256SUMS + provenance
  の供給網モデルと別系統の署名基盤が増える。また macOS の `.app`
  bundle は稼働中 self-replace が困難 (translocation / quarantine)。
  鍵管理と配布形態は設計判断を要するため、本 change では CLI のみとし
  GUI は別 change で判断する。
- **install.sh 再実行で十分**: Managed Nix 導入済み環境で install.sh は
  staging からやり直しであり、日常更新としては重い。また install.sh は
  Nix 未導入の fresh machine を主対象にしている。

## Capability Impact

| Capability | Impact |
|---|---|
| core-operations | Requirement「本体の自己更新」を追加 |

GUI 側の仕様変更なし (gui-operations / gui-dashboard は現行維持)。
