## ADDED Requirements

### Requirement: ResolvedTool / Toolchain / ToolSource データモデル
core SHALL はツール解決結果を `ResolvedTool`（path + source + version）として返し、全操作は `Toolchain` を経由してツールへアクセスする。

#### Scenario: macOS GUI で PATH に無い Nix を解決する
- **WHEN** macOS の .app から起動し PATH に nix が無いが `/nix/var/nix/profiles/default/bin/nix` が存在する
- **THEN** `Toolchain` の `nix.path` は `/nix/var/nix/profiles/default/bin/nix` になる
- **AND** `nix.source` は `SystemProfile` になる

#### Scenario: canonicalize で symlink 解決
- **WHEN** `/usr/local/bin/nix` が `/nix/var/nix/profiles/default/bin/nix` へ symlink している
- **THEN** `ResolvedTool.path` は realpath の `/nix/var/nix/profiles/default/bin/nix` を保持する

#### Scenario: env override が最優先
- **WHEN** `SCHNEEFORGE_NIX_BIN=/custom/nix` が設定されている
- **THEN** `Toolchain` の `nix.path` は `/custom/nix` になる
- **AND** `nix.source` は `EnvOverride` になる

### Requirement: macOS GUI 起動時の PATH 補正
SchneeForge desktop SHALL は [tauri-apps/fix-path-env-rs](https://github.com/tauri-apps/fix-path-env-rs) を `main()` の冒頭で呼び出し、launchd 起由の PATH 欠損（`/etc/paths` / `/etc/paths.d/*` が読まれない問題）を補正する。

#### Scenario: fix-path-env-rs が main の最初に呼ばれる
- **WHEN** macOS の .app から Tauri desktop が起動する
- **THEN** `fix_path_env::fix()` が `tauri::Builder` より前に呼ばれる
- **AND** PATH に `/usr/local/bin` / `/opt/homebrew/bin` 等の login shell 相当が含まれる

#### Scenario: Linux では noop
- **WHEN** Linux から Tauri desktop を起動する
- **THEN** `fix_path_env::fix()` は影響を与えず、元の PATH が維持される

### Requirement: 探索優先度の明文化
`ToolResolver` SHALL は次の順序でツールを探索する: (1) `SCHNEEFORGE_<NAME>_BIN` env (2) `PATH` (3) `$XDG_STATE_HOME/nix/profile/bin` or `~/.local/state/nix/profile/bin` (4) `$NIX_PROFILE/bin` (5) `~/.nix-profile/bin` (6) `/etc/profiles/per-user/$USER/bin` (7) `/nix/var/nix/profiles/default/bin` (8) `/opt/homebrew/bin` (9) `/usr/local/bin`。

#### Scenario: XDG state profile を探索
- **WHEN** `~/.local/state/nix/profile/bin/nix` のみに nix が存在する
- **THEN** `ResolvedTool.path` は `~/.local/state/nix/profile/bin/nix` になる
- **AND** `source` は `XdgStateProfile` になる

#### Scenario: NIX_PROFILE env を尊重
- **WHEN** `NIX_PROFILE=/custom/profile` 設定下で `/custom/profile/bin/nix` が存在する
- **THEN** `ResolvedTool.path` は `/custom/profile/bin/nix` になる

#### Scenario: XDG state フォルダ欠如を検出
- **WHEN** Nix installer が `~/.local/state/nix/profiles` を作成しておらず、かつ PATH / system profile にも nix が無い
- **THEN** `NixHealth.installed == false` になる
- **AND** `NixHealth.error` に「`mkdir -p ~/.local/state/nix/profiles` を試すか Nix を再インストール」という案内が入る

#### Scenario: PATH を最優先（env override 除く）
- **WHEN** PATH 上と `/nix/var/nix/profiles/default/bin` の両方に nix が存在する
- **THEN** PATH 上の nix が選ばれる

### Requirement: 文字列リテラル spawn の禁止
`tool.rs` 内の resolver 実装を除き、core / CLI / desktop SHALL は `Command::new("nix")` / `run_capture("nix", ...)` / `run_stream("nix", ...)` / `command_succeeds("nix", ...)` 等の文字列リテラルによる spawn を行わない。

#### Scenario: actions が Toolchain を使う
- **WHEN** `actions::apply` が nix を spawn する
- **THEN** `toolchain.nix.path` を `&Path` として `process::run_stream` へ渡す

#### Scenario: CI が文字列リテラル spawn を検出
- **WHEN** PR に `Command::new("nix"` が含まれる（`tool.rs` 以外）
- **THEN** CI ジョブが fail する

### Requirement: shell / Rust 間で探索ルールの一致
`install.sh` / `bootstrap.sh` / Rust `ToolResolver` SHALL は同じ探索順序と同じ優先度を使う。

#### Scenario: 共通の resolve-tools.sh
- **WHEN** `install.sh` と `bootstrap.sh` が nix を解決する
- **THEN** 両者とも `scripts/resolve-tools.sh` の `resolve_nix` 関数を経由する

#### Scenario: Rust 側と shell 側で同じ nix が選ばれる
- **WHEN** 同一環境で GUI と `bootstrap.sh` を実行する
- **THEN** 両者が解決する nix の絶対パスが一致する

## MODIFIED Requirements

### Requirement: PATH 非依存のツール解決
ツール解決 SHALL は PATH だけでなく既知パスも探索する。macOS GUI は Terminal と異なる PATH を持つため。

#### Scenario: PATH に無いが既知パスにある
- **WHEN** ツールが PATH に無いが `/nix/var/nix/profiles/default/bin` に存在する
- **THEN** 解決結果は available: true とその path を返す
- **AND** `ResolvedTool.source` は `SystemProfile` になる

#### Scenario: 解決順序
- **WHEN** ツールが複数箇所に存在する
- **THEN** `SCHNEEFORGE_<NAME>_BIN` → PATH → `$XDG_STATE_HOME/nix/profile/bin` → `$NIX_PROFILE/bin` → `~/.nix-profile/bin` → `/etc/profiles/per-user/$USER/bin` → `/nix/var/nix/profiles/default/bin` → `/opt/homebrew/bin` → `/usr/local/bin` の順で解決する

### Requirement: 実行時の解決済みパス利用
コマンド実行 SHALL は解決済みの絶対パスを使う。

#### Scenario: nh が未解決でも nix-darwin 適用できる
- **WHEN** `nh` が未インストールの fresh machine で apply する
- **THEN** core は `nh` に依存せず `Toolchain.nix.path` で適用する

#### Scenario: Diagnostics と Apply で同じ nix を使う
- **WHEN** GUI が Diagnostics を表示した後に Apply を実行する
- **THEN** Diagnostics の `nix.path` と Apply が spawn する nix の絶対パスが一致する
