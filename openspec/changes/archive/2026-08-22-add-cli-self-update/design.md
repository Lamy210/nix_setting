# Design: CLI 自己更新

## Context / 現状

- tag 解決・version 比較・metadata 取得の primitives は v2 §27/§28 で
  実装済み (`remote_tags` / `latest_tag_for_channel` / `compare_versions`)。
- binary asset の download + SHA256 検証の手指りは install.sh
  (`fetch_schneeforge_binary`) に実績があるが、shell 側のみ。
- core には `managed_nix::download::{download, download_text}` (reqwest
  blocking) があり再利用可能。
- GUI Dashboard は既に notify-only (`dash-update` に案内文表示)。

## Goals / Non-Goals

- **Goal**: CLI が自力で最新 release へ更新できること。検証は既存の
  供給網モデル (CHECKSUMS.txt の sha256) に乗せること。
- **Non-Goal**: GUI 内自己更新、`.app` bundle 置換、自動昇格、
  (downgrade を含む) 任意 version 指定、release channel 以外の配布元。

## 決定事項

1. **command 形式は top-level `schneeforge self-update`**。
   `update` は configuration source の更新 (v2 主操作)、`upgrade`/`sync`
   は deprecated alias 化の教訓から、目的が違う操作を既存 verb に
   overload しない。
2. **tag 解決は ls-remote のみ** (`remote_tags` → `latest_tag_for_channel`)。
   `ReleaseMetadata::fetch` は使わない。binary の検証は CHECKSUMS 突合で
   完結し、metadata と tag の一致性は release 時の generate 検証
   (release-artifact-check) が担保するため、network 依存を増やさない。
3. **platform gating は install.sh と同一**: darwin は aarch64 のみ、
   linux は x86_64 のみ。対応 asset が存在しない組み合わせは
   download 手前で fail-closed (`UnsupportedPlatform`)。
4. **検証は runtime SHA256 比較のみ** (ADR-0001 の 2 層供給網と同じ
   考え方: CI で attest、runtime は pin/checksum 比較)。CHECKSUMS.txt は
   sha256sum 形式 (`<64hex>  <path>/schneeforge-<arch>-<os>`) を仮定し、
   該当 asset の entry が無ければ fail-closed。
5. **置換は same-dir temp + rename で atomic に**: temp file に write →
   fsync → chmod (元 binary の mode を踏襲) → rename。検証失敗時は
   temp のみ削除し、実行 binary は一切変更しない。unix の inode 置換に
   より稼働中 process は旧 binary のまま動作し、次回起動から新 binary。
6. **権限なしは structured error で手動案内** (`sudo schneeforge
   self-update` または install.sh)。v1 では GUI 昇格 (osascript/pkexec)
   の仕組みを流用しない — 昇格が必要な操作は Managed Nix install のみ
   という現行の線を維持する。
7. **channel は state の source channel** (`channel_of(state)`、未初期化
   なら stable)。Dashboard の available 解決と同じ規則。
8. **URL 構築は `repo_url()` 規約** (`SCHNEEFORGE_REPO_URL` >
   `DEFAULT_REPO_URL` + `github_slug`)。install.sh の fork 規約と同じ。
9. **no-op guard**: 実行 version が channel 最新以上なら何も download せず
   `UpToDate` を返す (`compare_versions` 再利用)。

## 実装構成

```
crates/core/src/self_update.rs
  pub enum SelfUpdateStatus { UpToDate { version }, Updated { from, to, exe } }
  pub fn platform_asset(os: &str, arch: &str) -> Result<&'static str>      // 純関数
  pub fn expected_sha256(checksums: &str, asset: &str) -> Result<String>   // 純関数
  pub fn release_asset_url(tag: &str, asset: &str) -> Result<String>       // repo_url 規約
  pub struct SelfUpdatePlan { tag, version, asset, checksums_url, asset_url, exe, temp_path, mode }
  pub fn plan(tags: &[String], channel: &str, current_version: &str) -> Result<Plan-or-UpToDate>  // 純関数
  pub fn run(plan, git: &ResolvedTool) -> Result<SelfUpdateStatus>         // network + fs
```

- 純関数 (asset 選択 / checksums parse / plan / URL) と effect (download /
  verify / replace) を分離し、test は純関数 + fs 置換 (temp dir) に限定
  する。dashboard の snapshot 分離と同じ方針。
- `run` は `remote_tags` (network) → `plan` (純) → download 2 件 →
  verify → replace の順。失敗は temp 削除後に伝播。
- error は `Error::SelfUpdate(String)` を追加 (`UnsupportedPlatform` は
  既存 variant を再利用)。

## Risks / Trade-offs

- CHECKSUMS.txt の形式が release workflow で変わると parse が落ちる
  → fail-closed なので誤更新ではなく「更新できない」側に倒れる。
  release-artifact-check が形式を検証しているわけではないため、
  parse test で現行形式を固定する。
- 稼働中の GUI process が旧 binary を掴み続ける → 仕様 (次回起動で
  新 binary)。CLI は process が終わるので影響なし。
- GitHub release の redirect (asset は S3 へ 302) → reqwest default で
  redirect追従するため対応不要 (install.sh の curl -L と同じ)。

## Test Plan

- unit (hermetic): platform_asset の全組み合わせ (darwin/aarch64, linux/
  x86_64, 例外 2 種)、expected_sha256 (正常 / entry 無し / 形式不正)、
  plan (update あり / UpToDate / tag 無し)、release_asset_url (env 上書き)。
- unit (fs): temp file write → verify 成功 → rename で exe が置換される
  こと、verify 失敗で exe が無変更であること (temp dir 上で実施)。
- cli test: `self-update` の引数 wiring と fail-closed (git 未検出) 経路。
  network が必要な成功経路は CI の実 network に依存しないため対象外。
