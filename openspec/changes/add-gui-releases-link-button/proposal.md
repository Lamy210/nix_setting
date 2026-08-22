# Proposal: GUI Dashboard に GitHub Releases への link button を追加する

## Why

GUI 自己更新の設計提案 (`add-gui-app-self-update`, PR #74 merge 済み) が
推奨する **Step 1 (Option B(1))** を実装する。現状 Dashboard は
update がある場合に「GitHub Releases / install.sh で更新できます」と
案内するだけで、user が自ら browser を開いて URL を打つ必要がある。
Step 2 (tauri-plugin-updater) は minisign 鍵管理方針などの Open Questions
が user 判断待ちのため、鍵・asset・pipeline を一切変えない B(1) のみを
先行して merge できる (STATUS.md「Step 1 は鍵管理と独立に実装可」)。

## What Changes

- **core**: release page URL を組み立てる純関数 `release_page_url` を
  `release_metadata.rs` に追加する (unit test 可能)
- **desktop backend**: async command `open_release` を追加する。
  available version を受け取り、core の純関数で
  `https://github.com/Lamy210/nix_setting/releases/tag/v<version>` を
  組み立て、tauri-plugin-opener で既定 browser で開く
- **desktop frontend**: Dashboard の update 案内 (`dash-update`) に
  「GitHub Releases を開く」button を追加する。`update_available` が
  true のときのみ表示する
- 依存に `tauri-plugin-opener` (Tauri 公式 plugin) を追加する。
  鍵・release asset・pipeline の変更は無し

## 影響範囲

- Linux GUI (nix 配布) でも button は表示される (updater 対象外の
  platform への fallback 案内として設計上の役割が残る。design.md §4)
- Step 2 (Option A) 導入後もこの button は「自動更新できない platform
  向けの案内」として残る (`add-gui-app-self-update` design.md §3)

## 非対象

- GUI 本体 (.app) の自動更新 — Step 2 として Open Questions 1-3 の
  user 決定後に別 change で起票する
- wizard (setup view) への button 追加 — Dashboard のみ
