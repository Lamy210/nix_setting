# core-operations Specification

## Purpose
core ライブラリが提供する apply / rollback / upgrade / plan / sync / verify 操作の契約を定義する。全操作は排他ロックで直列化され、解決済み `Toolchain` を経由してツールを呼び出し、成功時に `State` を永続化する。CLI / desktop はこの core を thin に呼び出す only。
## Requirements
### Requirement: repo-aware 操作

全操作（plan/apply/verify/rollback/update/sync）SHALL は repository
path を明示的に受け取り、CWD に依存しない。

#### Scenario: update が repo を指定する

- **WHEN** 別ディレクトリから update を実行する
- **THEN** 対象 repo のみを更新し、CWD は変更しない

#### Scenario: upgrade が repo を指定する

- **WHEN** 別ディレクトリから upgrade (alias) を実行する
- **THEN** `nix flake update --flake <repo>` を実行し、CWD ではなく repo を更新する

#### Scenario: sync が repo を指定する

- **WHEN** 別ディレクトリから sync を実行する
- **THEN** `git -C <repo>` で操作する

### Requirement: 操作の core 集約
CLI と GUI SHALL は同じ core operation を呼ぶ。実ロジックを CLI/GUI に重複させない。全操作 SHALL は `Toolchain` を受け取り、文字列リテラルによる `Command::new` を使わない。

#### Scenario: CLI と GUI の apply
- **WHEN** CLI と GUI が apply する
- **THEN** 両者とも同じ `core::operations::apply` を呼ぶ
- **AND** 両者とも同じ `Toolchain` の nix の絶対パスを spawn する

#### Scenario: apply が Toolchain を使う
- **WHEN** `core::operations::apply` が nix を実行する
- **THEN** `toolchain.nix.path` を `process::run_stream` / `run_capture` へ `&Path` として渡す

#### Scenario: plan の dry-run が Toolchain を使う
- **WHEN** `core::operations::plan` が `nix build --dry-run` を実行する
- **THEN** `toolchain.nix.path` を使う

#### Scenario: sync が Toolchain の git を使う
- **WHEN** `core::operations::sync` が `git pull` を実行する
- **THEN** `toolchain.git.path` を使う

#### Scenario: verify が Toolchain を使う
- **WHEN** `core::operations::verify` が nix / git の存在を検査する
- **THEN** `which(cmd)` ではなく `Toolchain` の解決済みパスを使う

### Requirement: State 永続化
apply 成功後 SHALL は State（host/revision/applied_at）を core 内で保存する。

#### Scenario: GUI apply 後の State 更新
- **WHEN** GUI から apply が成功する
- **THEN** State が保存され、applied_revision が更新される

#### Scenario: State 保存エラー
- **WHEN** State 保存に失敗する
- **THEN** エラーを返し、成功と偽らない

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

### Requirement: MachineFacts の自動検出

SchneeForge core SHALL は machine 固有情報 (username / home directory / OS / architecture / hostname) を `MachineFacts` として実行環境から自動検出する。利用者に入力させず、configuration repo から読まない。

#### Scenario: 検出は実行環境から行う

- **WHEN** core が MachineFacts を検出する
- **THEN** username は実行 user、home directory は実効 HOME、OS / architecture は実行環境の値を返す
- **AND** configuration repo 内の file から username を読まない

#### Scenario: 検出不能な項目は error にする

- **WHEN** username または home directory が検出できない
- **THEN** error を返し、空文字のまま処理を続けない

### Requirement: machine input の生成と注入

SchneeForge core SHALL は apply / plan の評価時に MachineFacts から `machine.nix` を生成し、flake の `machine` input へ `--override-input` で注入する。評価は pure (builtins.getEnv 不使用) を維持する。

#### Scenario: apply 時に machine input が注入される

- **WHEN** apply が flake 評価を実行する
- **THEN** state dir に生成した machine.nix が `--override-input machine <path>` で渡される
- **AND** flake 内の `inputs.machine` は hosts が参照する username / homeDirectory をその machine の値で解決する

#### Scenario: repo は書き換えられない

- **WHEN** apply / plan が実行される
- **THEN** configuration repo 内の file (config.toml 含む) は作成・変更されない
- **AND** machine.nix は repo 外の state dir に生成される

#### Scenario: clone 直後の repo も評価できる

- **WHEN** machine.nix 未生成の状態で `nix flake check` 等が repo で実行される
- **THEN** repo 同梱の placeholder (`defaults/machine.nix`) により評価が失敗しない

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

### Requirement: Distribution Manifest の読み込み

core SHALL は repository root の `schneeforge.toml` を distribution
manifest として読み込む。machine 情報を持たず、repository が何を提供
するか (distribution 名 / profiles / 対応 systems) のみを記述する。

#### Scenario: schneeforge.toml の読み込み

- **WHEN** repository に `schneeforge.toml` が存在する
- **THEN** core はこれを parse し distribution 名と profiles を返す

#### Scenario: schneeforge.toml が無い場合

- **WHEN** repository に `schneeforge.toml` が存在しない
- **THEN** manifest 読み込みは構造化 error を返し、`manifest_found` は false になる

#### Scenario: 旧 config.toml は読み込まない

- **WHEN** repository に v1 形式の `config.toml` のみが存在する
- **THEN** core は `config.toml` を読み込まず schneeforge.toml 無しと同じ扱いにする

### Requirement: Distribution Manifest の検証

manifest SHALL は読み込み後に実行時検証される。検証は schema 版数、
default profile の妥当性、実行 system の対応を含む。

#### Scenario: schema 不一致

- **WHEN** `schneeforge.toml` の `schema` が 1 でない
- **THEN** validation error を返し有効とみなさない

#### Scenario: default profile が available に無い

- **WHEN** `[profiles]` の `default` が `available` に含まれない
- **THEN** validation error を返す

#### Scenario: 実行 system が未対応

- **WHEN** 実行中の system が `[systems]` に含まれない
- **THEN** validation error (warning ではなく error) を返す

#### Scenario: 有効な manifest

- **WHEN** schema = 1 / default ∈ available / 実行 system が systems に含まれる
- **THEN** validation は valid になる

### Requirement: profile の選択と注入

SchneeForge core SHALL は user が選択した profile を state に保持し、
apply / plan の評価時に flake の `profile` input へ
`--override-input` で注入する。選択が無い場合は manifest の
`profiles.default` を用いる。repo は書き換えない。

#### Scenario: 選択 profile の注入

- **WHEN** state に profile が保存されており apply / plan が実行される
- **THEN** state dir に生成した profile input が `--override-input profile <path>` で渡される
- **AND** flake 内の hosts は `profiles/<選択名>.nix` を import する

#### Scenario: 未選択時は manifest default

- **WHEN** state に profile が保存されていない
- **THEN** manifest の `profiles.default` が選択されたものとして注入される

#### Scenario: 利用不可能な profile の選択

- **WHEN** 保存済み profile が manifest の `profiles.available` に含まれない
- **THEN** core は fail-closed に error を返し、評価を実行しない

#### Scenario: repo 同梱の placeholder で評価できる

- **WHEN** profile input 未生成の状態で repo で `nix flake check` 等が実行される
- **THEN** repo 同梱の placeholder (`defaults/profile.nix` = null) により manifest default 相当の profile で評価が失敗しない

### Requirement: profile 選択の CLI 操作

CLI SHALL は profile の一覧表示・選択・確認を提供する。選択は
manifest 検証後に state へ保存され、repo は書き換えない。

#### Scenario: profile の一覧

- **WHEN** `schneeforge profile list` が実行される
- **THEN** manifest の `profiles.available` と現在の選択 / default が表示される

#### Scenario: profile の選択

- **WHEN** `schneeforge profile set <name>` が manifest の available 内の name で実行される
- **THEN** 選択が state に保存され、以降の apply で反映される

#### Scenario: 不正な profile の選択

- **WHEN** `schneeforge profile set <name>` が available に無い name で実行される
- **THEN** error を返し state は変更されない

#### Scenario: 現在の選択確認

- **WHEN** `schneeforge profile show` が実行される
- **THEN** 解決済み profile (state または manifest default) と出典が表示される

