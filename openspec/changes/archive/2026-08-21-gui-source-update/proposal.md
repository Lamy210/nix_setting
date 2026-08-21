# Change: GUI からの configuration source 更新 (run_update)

## Why

v2 で CLI の主操作は `update` (configuration source の更新) に移行し、
`upgrade` (flake.lock 更新) は非推奨の alias になった。しかし GUI (desktop
app) には `update` を呼ぶ経路が無く:

1. **managed source で GUI から更新できない**: PR #59/#60 以降、fresh
   install は managed source (state + flake ref) で初期化されるが、GUI に
   新しい release tag へ移る操作が存在しない (Dashboard に「新しいリリース
   があります」と表示されるだけ)
2. **非推奨ボタンが managed machine で必ず失敗する**: 既存の「アップグレー
   ド」ボタンは deprecated な `upgrade` (flake.lock 更新) を昇格実行するが、
   core は managed source で fail-closed (`DEPS_MANAGED_ERROR`) のため
   managed machine では押すと必ず error になる

CLI と GUI で更新操作の提供が食い違う状態を解消する。

## What Changes

- **ADDED (gui-operations)**: `run_update` Tauri command。core `update()`
  を GUI process 内で実行する (root 権限不要のため昇格なし。sync と同じ
  扱い)。managed は state の tag 更新、checkout 表現の Release は tag
  fetch、GitTracking は pull --ff-only
- **ADDED (gui-operations)**: Ready 画面に「ソース更新」ボタン (id:
  `update`)。実行後に status / dashboard を再取得し、移行先 tag を反映する
- **ADDED (gui-operations)**: managed source の場合、非推奨の「アップグレー
   ド」ボタン (flake.lock 更新) を隠す。checkout 表現 / GitTracking では
  従来通り表示する

## Impact

- `apps/desktop/src-tauri/src/lib.rs` (command 追加 + regression test)
- `apps/desktop/dist/main.js` / `apps/desktop/dist/index.html` (ボタン +
  gate)
- 既存の「更新」ボタン (id: `refresh`, status 再取得のみ) との混同を避ける
  ため、新ボタンの label は「ソース更新」とする

## Non-goals

- GUI app 本体の自己 update (Phase E: 別 change で設計判断が必要)
- flake.lock 更新 (deps update) の GUI 提供 — 非推奨操作のためmanaged 以外
  でも既存の昇格経路をそのまま使う
