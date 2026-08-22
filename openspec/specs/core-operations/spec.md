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

### Requirement: Release Metadata の解釈

core SHALL は release asset `schneeforge-release.json` を ReleaseMetadata
として parse・検証できる。metadata は release の version / channel /
source revision / 最低限必要な schneeforge 版数 / 対応 systems を
machine-readable に表現する (v2 §27)。

#### Scenario: metadata の parse

- **WHEN** schema 1 の `schneeforge-release.json` が与えられる
- **THEN** core は version / channel / source_revision / minimum_schneeforge_version / configuration_schema / systems を持つ ReleaseMetadata を返す

#### Scenario: 未対応 schema

- **WHEN** metadata の `schema` が 1 でない
- **THEN** parse は fail-closed に error を返す

#### Scenario: tag との整合検証

- **WHEN** `validate` が tag (例: `v0.2.0-rc.5`) に対して呼ばれる
- **THEN** version が tag の `v` 接頭辞を除いた部分と一致し、channel が version の prerelease 有無から導出されるものと一致することを検証する
- **AND** 不一致は error を返す

#### Scenario: channel の導出

- **WHEN** version に prerelease suffix (例: `-rc.5`) が含まれる
- **THEN** channel は `preview` と判定される
- **WHEN** prerelease suffix が無い
- **THEN** channel は `stable` と判定される

### Requirement: Release Metadata の取得

CLI SHALL は指定 tag の release asset から ReleaseMetadata を取得して
表示できる。asset が存在しない release (過去 release 等) や network
error は fail-closed に error を返す。

#### Scenario: 存在する tag の metadata 表示

- **WHEN** `schneeforge source metadata <tag>` が metadata asset を持つ tag で実行される
- **THEN** version / channel / source_revision / systems 等が表示される

#### Scenario: asset が無い release

- **WHEN** metadata asset を持たない tag (metadata 導入前の release や存在しない tag) で実行される
- **THEN** error を返し、誤った情報を表示しない

### Requirement: 利用可能 release の解決

core SHALL は指定 channel の最新 release tag を remote の tag 列から
解決できる。tag 列は `git ls-remote --tags` (ToolResolver 経由の解決済み
git) から取得し、channel 毎の最新選択は既存の `latest_tag_for_channel`
(stable は prerelease を含まない、preview は prerelease のみ、semver
降順の先頭) に従う。解決した tag の `ReleaseMetadata` は §27 の fetch
(parse + tag 整合検証) で取得する。

#### Scenario: channel の最新 tag 解決

- **WHEN** tag 列 (`v0.1.0`, `v0.2.0-rc.5`, `v0.2.0-rc.4`, `v0.3.0`) と channel が与えられる
- **THEN** stable は `v0.3.0`、preview は `v0.2.0-rc.5` を返す

#### Scenario: channel に該当 tag が無い

- **WHEN** tag 列に channel に合う release tag が存在しない
- **THEN** 解決は fail-closed に error を返す (誤った available を表示しない)

#### Scenario: metadata 取得失敗

- **WHEN** 解決した tag の metadata asset が存在しない (metadata 導入前の release) か network error が発生する
- **THEN** error 種別と理由を呼び出し元に返す (Dashboard は available を未知として表示できる)

### Requirement: Dashboard snapshot の構築

core SHALL は GUI Dashboard (§28) 表示のための snapshot を構築できる。
snapshot は次で構成する:

- `installed`: 実行 binary version / 実効 profile (state 選択 >
  manifest default) / channel (state の source channel、無ければ
  `stable`) / applied revision / applied_at
- `available`: channel の最新 release の ReleaseMetadata。解決失敗時は
  None と理由 (`available_error`)
- `update_available`: available version が実行 binary version より
  新しい場合に限り true

network access は snapshot 構築から分離し、呼び出し元が解決結果を
差し込めること (hermetic test 可能)。

#### Scenario: available が新しい場合

- **WHEN** 実行 version 0.2.0 に対し available が 0.3.0
- **THEN** `update_available` は true

#### Scenario: available が同等以下の場合

- **WHEN** 実行 version 0.2.0-rc.5 に対し available が 0.2.0-rc.5 以下
- **THEN** `update_available` は false

#### Scenario: offline 時の snapshot

- **WHEN** available 解決が network error で失敗する
- **THEN** snapshot は `available: None` と `available_error` に理由を持ち、installed は値を保持する (snapshot 全体は失敗しない)

#### Scenario: channel の決定

- **WHEN** state に source 情報がある場合はその channel、無い場合は `stable` を channel として使う

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

### Requirement: 本体の自己更新

core SHALL は実行 binary を channel の最新 release へ自己更新できる。
tag 解決は「利用可能 release の解決」と同じ規則 (`git ls-remote --tags`
→ `latest_tag_for_channel`) に従う。binary asset は platform 毎の提供
条件 (darwin は aarch64 のみ / linux は x86_64 のみ) で選択し、
`CHECKSUMS.txt` の sha256 と突合してから置換する。置換は同一
filesystem 上の temp file → rename で atomic に行い、検証失敗時は
実行 binary を一切変更しない (fail-closed)。

#### Scenario: 最新版では no-op

- **WHEN** 実行 version が channel の最新 tag と同等以上
- **THEN** 何も download / 置換せず、最新である旨の結果を返す

#### Scenario: CHECKSUMS 突合による検証

- **WHEN** download した binary の sha256 が `CHECKSUMS.txt` の該当
  asset entry と一致する
- **THEN** 実行 binary を新 binary へ atomic に置換し、移行元 / 移行先
  version と置換 path を結果として返す

#### Scenario: 検証失敗で実行 binary を保護

- **WHEN** sha256 が一致しない、または `CHECKSUMS.txt` に該当 asset の
  entry が存在しない
- **THEN** error を返し、実行 binary は変更しない

#### Scenario: 非対応 platform は download 手前で拒否

- **WHEN** macOS x86_64 または Linux aarch64 で自己更新を実行する
- **THEN** fail-closed に error を返す (install.sh と同一の提供条件)

#### Scenario: 書き込み権限なし

- **WHEN** 実行 binary の置換に必要な directory への書き込み権限が無い
- **THEN** 手動更新 (`sudo` 実行または install.sh) を案内する structured
  error を返す

