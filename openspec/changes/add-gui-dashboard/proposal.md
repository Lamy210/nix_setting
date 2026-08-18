# Proposal: GUI Dashboard (v2 §28)

## Why

v2 §28 が定義する Dashboard「Installed / Available」表示の data 供給源は
全て merge 済み (§14/15 Distribution Manifest、§17 profile 選択、§27
Release Metadata) だが、GUI にはこれらを統合した view が無い。user は

- 自分の schneeforge が何 (version / channel / profile / applied revision) か
- 最新 release が何で、update が必要か

を別々のコマンド実行なしに確認できない。§28 の Dashboard はこれを
1 画面で示し、update 案内まで含めることを求めている。

## What Changes

- **core**: `crates/core/src/dashboard.rs` — `DashboardSnapshot` の構築
  - `installed`: 実行 binary の version (`CARGO_PKG_VERSION`) / 実効
    profile (state 選択 > manifest default) / channel (state の source
    channel、無ければ `stable`) / applied revision・applied_at (State)
  - `available`: channel の最新 release の `ReleaseMetadata`。
    解決は `git ls-remote --tags` → `latest_tag_for_channel` (既存の
    純関数) → `ReleaseMetadata::fetch(tag)`。network error や asset
    無しは snapshot 全体を失敗させず `available: None` + `available_error`
    に理由を格納 (offline でも Installed は表示する)
  - `update_available`: available version が実行 version より新しい場合
    true (比較は semver 準拠の純関数を新規実装: 正式版 > 同一 core の
    prerelease、`rc.10` > `rc.9` の数値比較。tag 選択には既存
    `latest_tag_for_channel` をそのまま使用)
- **desktop**: async command `get_dashboard` — ls-remote + fetch を
  blocking で実行し `DashboardSnapshot` を返す (UI thread を占有しない)。
  frontend (`main.js` / `index.html`) に Dashboard 表示を追加: Installed
  (version / profile / channel / applied) と Available (version /
  channel / systems / 取得失敗時は理由) と update 案内
- **test**: core は純関数 (tag 列からの available 解決・snapshot 組み
  立て・version 比較) を hermetic に検証。desktop は serialize key と
  frontend 参照の regression test (既存 pattern 踏襲)

## Impact

- 既存の `get_status` / wizard / operation は変更しない。Dashboard は
  読み取り専用の追加 view
- available 解決の network fetch は user 操作 (画面表示) の都度発生。
  失敗しても他機能に影響しない
- specs: `core-operations` に「利用可能 release の解決」「Dashboard
  snapshot の構築」を追加、新 capability `gui-dashboard` を作成
- CLI (`schneeforge dashboard`) は本 change の scope 外。core 関数は
  再利用可能な形にする

## Scope / 非対象

- profile 切替 UI (§17 の GUI 編) は別 change
- Release checkout の working tree-less 化 (§7) は別 change
- Dashboard からの update 実行 (本体自己 update) は別 change。本 change
  は案内の表示のみ
