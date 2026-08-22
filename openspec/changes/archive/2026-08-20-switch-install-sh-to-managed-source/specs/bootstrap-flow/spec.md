## ADDED Requirements

### Requirement: install.sh の fresh install は managed source を使う

`install.sh` SHALL は repository checkout が存在しない場合、clone せずに
managed source で初期化する。手順は:

1. CLI binary を release asset から CHECKSUMS 検証付きで取得
2. Nix 未検出の場合は Managed Nix を install (embedded manifest で動作)
3. 既存 dotfile の backup
4. `schneeforge source init --tag <install.sh の pin tag>` で managed
   source を state に設定
5. `schneeforge apply` (flake ref から build。macOS は darwin-rebuild が
   内部で権限を要求)

#### Scenario: fresh machine で clone が発生しない

- **WHEN** repository が存在しない状態で install.sh を実行する
- **THEN** `git clone` は実行されず、managed source (flake ref) で apply まで到達する

#### Scenario: source init の tag は binary と同一 release

- **WHEN** install.sh が `source init` を実行する
- **THEN** pin されている release tag (`SCHNEEFORGE_BOOTSTRAP_REF`) が指定される (binary の pin と一致)

#### Scenario: 既存 checkout は従来 flow を維持

- **WHEN** repository に `.git` が存在する状態で install.sh を実行する
- **THEN** clone は行わず、既存 checkout を使った従来 flow (`bootstrap.sh`) で適用する

#### Scenario: 適用は user 権限で実行される

- **WHEN** fresh install の `schneeforge apply` を実行する
- **THEN** apply は user 権限で実行され (state dir が user 側に作られる)、macOS では darwin-rebuild が必要とする権限昇格を内部経由で処理する

#### Scenario: dotfile backup を初回適用前に行う

- **WHEN** fresh install で初回の apply を実行する前
- **THEN** home-manager が管理する既存 dotfile が timestamp 付き backup dir へ退避される
