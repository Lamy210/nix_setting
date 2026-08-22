## ADDED Requirements

### Requirement: Managed Release Source (working tree-less)

core SHALL は Release source (Stable / Preview) の表現として、git
working tree を持たない「managed」表現をサポートする (v2 §7)。managed
source の実体は flake ref `github:<owner>/<repo>/<tag>` であり、nix が
直接取得・cache する。SchneeForge は source tree を local に持たない。

- managed は State の source 情報 (`managed` flag) で表現し、旧
  state.json は managed 無し (checkout 表現) として読み込める
- managed source の設定・更新時、§27 の ReleaseMetadata を取得して
  `source_revision` を State に記録する (metadata asset が無い tag は
  警告付きで検証を skip)
- fork は既存の `SCHNEEFORGE_REPO_URL` から owner / repo を解決する

#### Scenario: managed source の flake ref

- **WHEN** State が managed な ReleaseStable (`ref: v0.2.0`) を示す
- **THEN** core は操作 (plan / apply / rollback) の nix 引数に
  `github:<owner>/<repo>/v0.2.0` flake ref を使う
- **AND** machine / profile input の `--override-input path:` は
  そのまま機能する

#### Scenario: 旧 state との互換

- **WHEN** `managed` field を持たない state.json を読み込む
- **THEN** エラーにせず checkout 表現 (managed=false) として扱う

#### Scenario: rev 検証の記録

- **WHEN** managed source を tag に設定・更新する
- **THEN** ReleaseMetadata の `source_revision` を State に記録する
- **AND** metadata asset を持たない tag では警告付きで検証を skip する

### Requirement: repo file の tag-pinned 取得

core SHALL は managed source について、repo file (`schneeforge.toml` /
`bootstrap-manifest.toml` 等) を tag pinned で取得できる。取得は
`raw.githubusercontent.com/<owner>/<repo>/<tag>/<file>` とし、結果は
state dir (`sources/<tag>/`) へ原子保存する。tag は不変のため一度
保存した cache は無期限に有効で、2 回目以降の読み取りは network を
行わない。path source (checkout / Local) の file 読み取りは従来どおり
local filesystem を使う。

#### Scenario: 初回取得と cache

- **WHEN** managed source の `schneeforge.toml` が未 cache の状態で
  読み取られる
- **THEN** tag pinned で取得して state dir へ保存し、内容を返す
- **WHEN** 同 tag で再度読み取られる
- **THEN** cache から返し network には行かない

#### Scenario: offline

- **WHEN** cache が存在する状態で offline で読み取られる
- **THEN** cache から返す (error にしない)

#### Scenario: 取得失敗

- **WHEN** cache が無く取得に失敗する (offline 初回 / 404)
- **THEN** fail-closed に error を返す

## MODIFIED Requirements

### Requirement: source 種別の解決

core SHALL は source の種別を解決する。State が managed な Release
(Stable / Preview) を示す場合はそれをそのまま返し、checkout の実態は
参照しない。それ以外 (managed=false) は repository checkout の実態から
`SourceKind` (`ReleaseStable` / `ReleasePreview` / `GitTracking` /
`GitPinned` / `Local`) を検出する。検出は git の状態のみから行い、
network access を必要としない。

#### Scenario: managed source の解決

- **WHEN** State の source が managed な ReleaseStable を示す
- **THEN** checkout 実態によらず State の source を返す

#### Scenario: release tag への pinned checkout

- **WHEN** checkout が detached HEAD であり HEAD が `vX.Y.Z` 形式の
  release tag を指している
- **THEN** `ReleaseStable` として解決する

#### Scenario: prerelease tag への pinned checkout

- **WHEN** HEAD が `vX.Y.Z-rc.N` 等 prerelease suffix を持つ tag を
  指している
- **THEN** `ReleasePreview` として解決する

#### Scenario: branch への checkout

- **WHEN** checkout が branch に紐付いている
- **THEN** `GitTracking` として解決する

#### Scenario: tag / commit への固定 (release 形式以外)

- **WHEN** detached HEAD だが `v` prefix の release tag が無い
- **THEN** `GitPinned` として解決する

#### Scenario: git 管理外の directory

- **WHEN** repo path に `.git` が存在しない
- **THEN** `Local` として解決する

### Requirement: update の source kind dispatch

`schneeforge update` SHALL は解決された source 種別と表現に応じて
更新挙動を切り替える。flake.lock はいずれの経路でも更新しない。

#### Scenario: Managed Release の更新

- **WHEN** managed な `ReleaseStable` / `ReleasePreview` で update を
  実行する
- **THEN** 同 channel の最新 tag を解決し、State の source を新 tag へ
  更新する (checkout 操作は行わない)
- **AND** ReleaseMetadata の `source_revision` を記録する

#### Scenario: Managed Release で最新が無い場合

- **WHEN** managed な Release source で同 channel に newer tag が
  存在しない
- **THEN** 現状維持の案内を表示して終了する

#### Scenario: Release Stable の更新

- **WHEN** checkout 表現の `ReleaseStable` の checkout で update を
  実行する
- **THEN** 同 channel (prerelease を含まない) の最新 tag へ
  checkout する
- **AND** 実行後に managed への移行 (`schneeforge source init`) の
  案内を表示する

#### Scenario: Release Preview の更新

- **WHEN** `ReleasePreview` の checkout で update を実行する
- **THEN** prerelease tag のみを候補に最新へ checkout する

#### Scenario: Git Tracking の更新

- **WHEN** `GitTracking` の checkout で update を実行する
- **THEN** `git fetch` の後 `git pull --ff-only` する
- **AND** dirty working tree は処理を中止する

#### Scenario: Managed Release での sync

- **WHEN** managed な Release source で sync を実行する
- **THEN** git working tree が存在しない旨を案内して終了する
  (error にしない)

#### Scenario: Git Pinned / Local は no-op

- **WHEN** `GitPinned` または `Local` の checkout で update を実行する
- **THEN** エラーにせず、更新方法の案内を表示して終了する
