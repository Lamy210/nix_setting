# Proposal: Managed Release Source — Release checkout の working tree-less 化 (v2 §7)

## Why

現在 Release source (Stable/Preview) の表現は `~/nix_setting` の pinned
git checkout (install.sh が `git clone --branch <tag> --depth 1`) であり、
mutable な working tree であることに起因する問題がある:

- **検証単位の不整合**: release は「1 release = 1 source tree = 1
  checksum set」の検証単位だが、working tree は user が自由に改変できる。
  dirty tree は update を中止させるが、commit 済みの改変や flake.lock
  変更は検知できない
- **update の複雑さ**: fetch --tags → checkout の working tree 操作は
  dirty check・detach 状態・branch 混在などの失敗 mode を持つ
- **表現の過剰**: Release を消費するだけの user に git履歴 や branch
  semantics は不要 (v2「Easy by default, Git-native when desired」)

ADR-0003 は「Release source で working tree を持たない (Nix Store
直接)」を v2 §7 の理想形とし、Phase 1 では pinned checkout を Release の
表現として使うことを明記していた。本 change がその後続である。

## What Changes

- **core: managed release source の導入** (`source.rs` 拡張)
  - Release source の新表現「managed」: working tree を持たず、flake
    ref `github:<owner>/<repo>/<tag>` として nix が直接取得・cache
    (Nix Store) する。tag が不変なので取得結果も不変
  - `SourceState` に managed 表現の flag を追加 (serde default で旧
    state.json と互換)
  - `SourceResolver::detect`: state が managed Release を示す場合は
    それを返す (checkout 実態を見ない)。checkout 由来の検出
    (install.sh の pin checkout) は従来どおり Release 検出とする
  - **rev 検証**: managed source の設定・更新時に §27 の
    `ReleaseMetadata.fetch(tag)` を取得し `source_revision` を記録。
    tag → commit の不変性保証を state に残す (metadata asset が無い
    旧 tag は警告付きで skip)
- **core: repo file の tag-pinned 取得**
  - `schneeforge.toml` / `bootstrap-manifest.toml` など repo file は、
    source が managed の場合 `raw.githubusercontent.com` から tag pinned
    で取得し state dir (`sources/<tag>/`) へ cache する。tag 不変のため
    cache は無期限で正しい (offline でも profile 解決・manifest 表示可)
  - path source は従来どおり local file 読み取り
- **core: 操作の flake ref 対応**
  - `plan` / `apply` / `rollback` は `repo` 文字列を flake ref として
    nix に渡す (path も `github:` ref もそのまま有効なため引数構成は
    不変)。`override_args` (machine / profile input) の manifest 読み
    取りを上記 tag-pinned 取得へ切り替え
  - `update` dispatch: managed Release は新 tag の state 更新のみ
    (checkout 操作なし)。checkout 表現の Release は従来動作 + managed
    移行の案内表示
  - `sync` / dirty check など git 前提の処理は managed では skip /
    案内 (git 実態が無いため)
- **CLI**: `schneeforge source init [--channel stable|preview] [--tag
  <tag>]` (managed source の設定。既存 checkout が同 tag pin なら移行
  表示)、`source status` に managed 状態 (ref / channel / rev 検証 /
  cache) を追加、`update` の managed 分岐
- **test**: HTTP fetch は差し込み可能にし hermetic に (既存の
  dashboard.rs と同じ分離 pattern)

## Impact

- **既存 user への影響**: install.sh 由来の checkout は現状どおり動作
  (detect / update 従来動作)。managed への移行は `source init` の明示
  実行のみ。state.json は旧形式とも互換
- **offline**: managed source の初回 nix 評価は network 必要 (nix が
  取得後は store / lookup cache で原則 offline 動作)。manifest 等 repo
  file は state cache のため offline で可
- **Specs**: `core-operations` の「source 種別の解決」「update の source
  kind dispatch」を MODIFIED、managed release source / repo file 取得の
  requirement を ADDED
- **ADR-0003**: Alternatives の「working tree-less は後続 change で検討」
  を実現形 (github flake ref) へ更新
- **CLI**: `Source` subcommand に `init` を追加 (破壊なし)

## Scope / Non-goals

- **install.sh / bootstrap-flow の変更は別 change**: install.sh の
  pinned clone と `bootstrap.sh` 実行は本 change では不改変
  (bootstrap-flow spec の clone requirement を維持)。install 完了後の
  managed 移行は `source init` で明示的に行う
- **checkout 表現 Release の廃止** (install.sh 切替と同時) は後続
  change。本 change では 2 表現の併存と managed への移行経路の提供のみ
- **GUI の source 選択 UI / wizard 変更**は対象外 (`get_status` の
  source 表示が壊れないことのみ保証)
- GitHub API 依存の tag 検索は使わない (git ls-remote + 既存
  `latest_tag_for_channel` のみ)
