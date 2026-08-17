## ADDED Requirements

### Requirement: source 種別の解決

core SHALL は repository checkout の実態から `SourceKind`
(`ReleaseStable` / `ReleasePreview` / `GitTracking` / `GitPinned` /
`Local`) を検出する。検出は git の状態のみから行い、network access
を必要としない。

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

`schneeforge update` SHALL は解決された source kind に応じて
更新挙動を切り替える。flake.lock はいずれの経路でも更新しない。

#### Scenario: Release Stable の更新

- **WHEN** `ReleaseStable` の checkout で update を実行する
- **THEN** 同 channel (prerelease を含まない) の最新 tag へ
  checkout する
- **AND** flake.lock は変更しない

#### Scenario: Release Preview の更新

- **WHEN** `ReleasePreview` の checkout で update を実行する
- **THEN** prerelease tag のみを候補に最新へ checkout する

#### Scenario: Git Tracking の更新

- **WHEN** `GitTracking` の checkout で update を実行する
- **THEN** `git fetch` の後 `git pull --ff-only` する
- **AND** dirty working tree は処理を中止する

#### Scenario: Git Pinned / Local は no-op

- **WHEN** `GitPinned` または `Local` の checkout で update を実行する
- **THEN** エラーにせず、更新方法の案内を表示して終了する

### Requirement: State への source 情報の記録

State SHALL は source の現在状態 (`kind` / `ref` / `channel`) を
applied 情報とは独立した field として保持する。従来の state.json
(当該 field 無し) は欠損 field を空として読み込める。

#### Scenario: update 後の source 記録

- **WHEN** update が checkout を新しい tag へ移動させる
- **THEN** State の source.ref が新 tag を指す

#### Scenario: 従来 state との互換

- **WHEN** source 情報を含まない state.json を読み込む
- **THEN** エラーにせず source は None として扱う

### Requirement: dependency 更新の分離

`nix flake update` 相当の操作 SHALL は `schneeforge source deps
update` として update 操作から分離される。Release channel で
実行した場合は release 検証単位から外れる警告を表示する。

#### Scenario: Stable での dependency 更新

- **WHEN** `ReleaseStable` で `source deps update` を実行する
- **THEN** 警告を表示した上で `nix flake update` を実行する

#### Scenario: Local での dependency 更新

- **WHEN** `Local` で `source deps update` を実行する
- **THEN** 警告なしで `nix flake update` を実行する

## MODIFIED Requirements

### Requirement: repo-aware 操作

全操作（plan/apply/verify/rollback/update/sync）SHALL は repository
path を明示的に受け取り、CWD に依存しない。

#### Scenario: update が repo を指定する

- **WHEN** 別ディレクトリから update を実行する
- **THEN** 対象 repo のみを更新し、CWD は変更しない

#### Scenario: sync が repo を指定する

- **WHEN** 別ディレクトリから sync を実行する
- **THEN** `git -C <repo>` で操作する

### Requirement: 同期の安全性

sync SHALL は dirty working tree を検出して競合を防ぐ。
`schneeforge sync` は `source sync` への alias として v0.3 まで
動作し、deprecation note を表示する。

#### Scenario: dirty な repo

- **WHEN** repo に未コミット変更がある状態で sync する
- **THEN** 処理を中止し、先にローカル変更の解決を促す

#### Scenario: 更新の反映

- **WHEN** リモートに更新がある
- **THEN** `--ff-only` で fast-forward のみ反映する

#### Scenario: 旧 command の deprecation 案内

- **WHEN** `schneeforge upgrade` または `schneeforge sync` を
  実行する
- **THEN** 実行はするが、`source deps update` / `source sync` への
  移行を促す note を表示する
