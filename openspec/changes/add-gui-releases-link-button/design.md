# Design: GUI Dashboard に GitHub Releases への link button を追加する

親 proposal の詳細設計は `add-gui-app-self-update/design.md` §3 (Option B(1))
を参照。本 document は実装の具体をまとめる。

## 1. URL の組み立て (core 純関数)

`ReleaseMetadata::asset_url` と同じ方針で、release page URL は core に
純関数として置く (GUI 以外からも test・再利用可能):

```rust
/// release page の URL。GUI の「GitHub Releases を開く」誘導で使う。
/// repo_url は `DEFAULT_REPO_URL` (`SCHNEEFORGE_REPO_URL` で上書き可)、
/// version は ReleaseMetadata.version (tag から先頭の v を除いたもの)。
pub fn release_page_url(repo_url: &str, version: &str) -> String {
    format!("{}/releases/tag/v{version}", repo_url.trim_end_matches(".git"))
}
```

- tag は常に `v<version>` (`ReleaseMetadata::validate` の規約と同一)
- `DEFAULT_REPO_URL` は `.git` suffix 付きのため、web URL への変換で
  trim する
- fork (`SCHNEEFORGE_REPO_URL` 上書き) 環境ではその fork の releases
  page へ飛ぶ (get_dashboard の available 解決と同じ repo 解決)

## 2. Tauri command `open_release`

```rust
#[tauri::command]
async fn open_release(version: String) -> Result<CommandOutput, String>
```

- frontend から available version (DashboardSnapshot.available.version)
  を受け取る。backend が state を持たないため引数渡しとする
  (get_dashboard の available は毎回解決される cache 無しの値)
- URL は core `release_page_url` で組み立て、
  `tauri_plugin_opener::open_url(url, None::<&str>)` で既定 browser で
  開く (plugin の初期化は `tauri_plugin_opener::init()`)
- 開く操作のみで権限不要・network I/O も伴わないため async のまま
  user 権限で実行する (run_update と同じ区分)
- 失敗 ( opener error ) は CommandOutput の失敗として frontend の
  output 領域に表示する

## 3. frontend

- `index.html`: `dash-update` note の直後に
  `<button id="dash-release-link" hidden>GitHub Releases を開く</button>`
  を置く
- `main.js` `refreshDashboard()`: `d.update_available && d.available` の
  ときのみ `hidden` を外す (available が取れていない場合は表示しない)
- click → `invoke("open_release", { version: d.available.version })`。
  成功時は note を維持、失敗時は error を output 領域へ

## 4. 回帰検証 (rc.3 事故対策と同型)

- `frontend_commands_match_backend` (既存) が `open_release` の
  invoke 名 / generate_handler 登録の一致を検証する
- 新規 test: index.html が `dash-release-link` id を持ち、main.js が
  `open_release` を参照することを静的検証する (serialize key × JS 参照
  × DOM id の 3 層検証の DOM/JS 層)
- core 側: `release_page_url` の unit test (default URL / `.git` 付き /
  上書き URL)

## 5. tauri-plugin-opener 導入

- `apps/desktop/src-tauri/Cargo.toml` に `tauri-plugin-opener = "2"` を
  追加 (Cargo.lock 更新)。Tauri v2 公式 plugin で鍵・asset は不要
- Rust 側からのみ呼ぶため capabilities への permission 追加は不要
  (plugin command を frontend JS から直接 invoke しない。frontend は
  自前 command `open_release` を経由する)
- dev machine は GTK 依存のため desktop workspace を compile できない
  ので、compile 検証は CI (macos-check) に委ねる。local は rustfmt の
  parse 検査のみ実施する
