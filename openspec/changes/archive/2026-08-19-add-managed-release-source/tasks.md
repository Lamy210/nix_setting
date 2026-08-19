# Tasks

## 1. core: SourceState の managed 表現

- [x] 1.1 `source.rs`: `SourceState` に `managed: bool` (`#[serde(default)]`) を追加。`flake_ref()` (managed かつ Release kind なら `github:<owner>/<repo>/<tag>`) と owner/repo 解析 (`DEFAULT_REPO_URL` / `SCHNEEFORGE_REPO_URL` から) を実装
- [x] 1.2 `SourceResolver::detect`: state が managed Release を示す場合は checkout を見ずにそれを返す分岐を追加 (state store 注入可能な形)
- [x] 1.3 unit test: 旧 state.json (managed 無し) 互換 / flake_ref 生成 (owner-repo 解析・tag 形式) / managed 時の detect

## 2. core: repo file の tag-pinned 取得

- [x] 2.1 `source.rs` (または `source_files.rs`): managed source の repo file 読み取り — raw.githubusercontent から tag pinned 取得し `state_dir/sources/<tag>/<file>` へ原子保存、2 回目以降は cache 優先 (network 不要)。fetch 関数は差し込み (hermetic test)
- [x] 2.2 `manifest::load` の呼び出し側 (`profile::resolve_with` / `dashboard::snapshot` の manifest 取得 / desktop `load_manifest`) を source 解決経由へ: path なら従来の fs 読み取り、managed なら 2.1
- [ ] 3.x と共通の unit test: cache hit で fetch が呼ばれない / fetch 失敗は fail-closed / path source は従来どおり

## 3. core: 操作の managed 対応

- [x] 3.1 `operations.rs` update dispatch: managed Release 分岐 (ls-remote → `latest_tag_for_channel` → state 更新 + `ReleaseMetadata` による rev 記録・検証)。checkout 表現 Release の update 後に managed 移行の案内 1 行を追加
- [x] 3.2 `operations.rs`: sync / dirty check の managed 分岐 (git 実態が無い旨の案内。error にしない)
- [x] 3.3 `actions.rs` / `operations.rs` の plan / apply / rollback: repo 解決が managed なら flake ref 文字列を nix 引数に使う (引数構成自体は不変の確認 test)
- [x] 3.4 unit test: update 分岐 (managed 最新なし=現状維持 / 新 tag で state 更新 / metadata 無し tag は警告付き skip) / sync の managed 案内

## 4. CLI

- [x] 4.1 `source init [--channel stable|preview] [--tag <tag>]`: managed source を state に設定 (tag 未指定なら channel 最新を解決)。既存 checkout が同 tag pin なら移行表示
- [x] 4.2 `source status` に managed 状態 (表現 / ref / channel / rev 検証 / cache 有無) を追加
- [x] 4.3 CLI integration test (crates/cli/tests): init → status → update の流れ、旧 state 互換

## 5. 保守 / docs

- [x] 5.1 ADR-0003: Alternatives の working tree-less 項を実現形 (github flake ref) へ更新
- [x] 5.2 `docs/STATUS.md` に §7 の状況を追記

## 6. test / CI

- [x] 6.1 `cargo test` 全 green (core / cli は local、desktop は CI)
- [x] 6.2 `cargo fmt` / `cargo clippy -D warnings` green
- [ ] 6.3 `nix flake check` green (CI)
- [x] 6.4 openspec validate green / PR 作成 (base: develop)
