# Tasks

## 1. core: source module

- [x] 1.1 `crates/core/src/source.rs` を新設: `SourceKind` enum (ReleaseStable / ReleasePreview / GitTracking / GitPinned / Local) と `Display` (小文字 kebab: `release-stable` 等)
- [x] 1.2 `SourceResolver::detect(repo, git)`: detached HEAD + `describe --tags --exact-match` で tag 取得 → `v` prefix + semver なら Release (prerelease suffix で Stable/Preview を判別)、branch があれば GitTracking、detached で tag 無しは GitPinned、`.git` 無しは Local
- [x] 1.3 `SourceState { kind, ref_, channel }` 型 (serde。channel は Release のみ Some)
- [x] 1.4 unit test: 各 kind の検出 (temp git repo を作成して branch / tag checkout / .git 削除を再現)、prerelease 判別 (`v0.5.0-rc.2` → Preview)

## 2. core: update dispatch

- [x] 2.1 `operations::update(repo, tc, capture)`: source kind を detect して dispatch。`OperationLock` で直列化
- [x] 2.2 Release 更新: `git fetch --tags` → 候補 tag の列挙 (local の tag から channel filter + semver sort) → 最新 tag へ `checkout`。dirty は中止
- [x] 2.3 GitTracking 更新: 既存 `sync_with_lock` の fetch + pull --ff-only 経路を再利用
- [x] 2.4 GitPinned / Local: no-op の案内文字列を返す (error にしない)
- [x] 2.5 update 成功時に State へ source 情報を保存
- [x] 2.6 unit test: dispatch 先の選択 (純関数化した `dispatch_plan(kind) -> UpdateAction`)、tag sort の semver 順、channel filter (stable は prerelease を含まない)

## 3. core: source sync / deps update

- [x] 3.1 `operations::source_sync(repo, tc, capture)`: 従来 sync を Advanced 扱いへ。Tracking 以外は kind を説明する no-op note
- [x] 3.2 `operations::deps_update(repo, tc, capture)`: 従来 upgrade (`nix flake update`) をそのまま中身に。source kind が Release の場合は警告文を先頭に付ける
- [x] 3.3 unit test: Release で警告が付く / Local で付かない、既存 sync の dirty 中止挙動は維持

## 4. core: State 拡張

- [x] 4.1 `State` に `source: Option<SourceState>` を追加 (serde default。従来 JSON の読み込み互換)
- [x] 4.2 unit test: source 無し JSON の読み込み、source 付き roundtrip

## 5. CLI

- [x] 5.1 `Update` subcommand 追加 (help に source kind 毎の挙動説明)
- [x] 5.2 `Source(SourceSub)` subcommand 追加: `status` / `sync` / `deps update`。`source status` は kind / ref / channel / State の整合を表示
- [x] 5.3 旧 `Upgrade` / `Sync` に deprecation note を表示 (実行は継続)
- [x] 5.4 `doctor` の host detection section に source kind を 1 行追加
- [x] 5.5 unit test (bins): subcommand の定義存在と deprecation 出力

## 6. test / CI

- [x] 6.1 `cargo test` 全 green (core / cli)
- [x] 6.2 `nix flake check` green (本 change は flake 変更なし)
- [x] 6.3 bats に update dispatch の smoke を追加 (temp repo で GitPinned no-op を確認)
- [x] 6.4 PR 作成 (base: develop)。required checks green を確認
