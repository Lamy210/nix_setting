## Context

`check.yml` の required gate は安全側に倒れているが、変更前の `rust-check` は以下を直列実行していた。

1. Tauri/GTK system dependency install
2. Nix install/devShell setup
3. `cargo test`
4. `cargo fmt -- --check`
5. `cargo clippy -- -D warnings`
6. CLI release build + smoke
7. CLI debug build + Tauri desktop build smoke

root Cargo workspace は `crates/core` と `crates/cli` のみで、desktop (`apps/desktop/src-tauri`) は workspace member ではない。そのため GTK/Tauri system dependencies は core/CLI quality gate には不要であり、desktop compile gate を持つ worker のみに閉じ込められる。

GitHub Actions では `needs` の依存 job が failure/skipped の場合、通常は dependent job も skipped になる。required aggregator が skipped して成功扱いになる曖昧さを避けるため、aggregator は job-level `if: ${{ always() }}` で必ず評価し、各 `needs.<job>.result` が `success` 以外なら step を明示的に failure にする。

初期の3-way fan-out (`rust-quality` / `rust-cli-smoke` / `rust-desktop-smoke`) は実装して実測した。negative path では `rust-quality=failure` が `rust-check=failure`、さらに `ci-required=failure` へ伝播し fail-closed 性を確認できた。一方 positive-path 計測では `rust-quality=117s`、`rust-cli-smoke=123s` に加え、`rust-desktop-smoke` が334sを超えても完了しておらず、合計はその時点で574s超となった。変更前 `rust-check=448s` の +20% 上限537.6sを完了前に超過したため、この3-way構成は棄却した。

cold desktop full build の主因は、旧単一jobでは `cargo test` / CLI build が生成した Cargo target state を後続 desktop build が再利用していたのに対し、独立runnerへ分離したことでそのwarm stateを失ったことにある。新しい設計では Linux desktop gate を compile smoke に縮小し、実 full build は既に required `release-artifact-check` が release workflow と同一の macOS DMG build script で保証している事実を利用して重複を除く。

2-way Rust構成の成功 run #393 では Rust runner total は426sまで収まったが、required critical path は387sで baseline 479s から19.2%短縮に留まり、25%目標 (359s以下) を満たさなかった。step timing を分解すると `flake-check` の `build linux home-manager` が約349sを占め、その中で default `developer` profile に含まれる Terraform の source build が約283sを占めていた。

Terraform は `profiles/developer.nix` → `modules/packages/containers.nix` から意図的に導入されており、product default profile から削除するのは CI optimization の責務を逸脱する。そこで required Linux flake gate は default `developer` を derivation evaluation まで維持し、actual realization smoke は既存 supported `minimal` profile を明示 override して行う。これにより product semantics を変えずに upstream package source build を毎PRのcritical pathから外す。

## Goals / Non-Goals

### Goals

- required context 名を維持したまま Rust CI を2-way fan-outする。
- desktop 固有 dependency setup を build-smoke worker のみに限定する。
- Linux desktop Rust/Tauri code の compile coverage を維持しつつ、full artifact build の重複を除く。
- worker failure/cancel/skip を required `rust-check` へ fail-closed で集約する。
- 現行 required 7 checks を集約する `ci-required` candidate を追加する。
- branch protection は変更せず、複数 PR で `ci-required` を観測可能にする。
- product default `developer` profile の評価 coverage をrequired Linux gateに残し、actual realization smoke は `minimal` profile で実行する。
- before/after の wall-clock と runner duration を測り、明示した性能基準を満たす。

### Non-Goals

- 全 CI job の一括再設計。
- `flake-check` の job fan-outや新cache provider導入。
- product profile (`developer` / `minimal`) の内容やdefault変更。
- `release-artifact-check` / release workflow の build semantics 変更。
- macOS runner 数を増やす最適化。
- path filter による required workflow skip。
- branch protection required context の即時一本化。

## Decisions

### D1: existing required context `rust-check` を aggregator として残す

worker の job ID/name は `rust-quality`, `rust-build-smoke` とする。既存 `rust-check` は実ビルドを持たず、この2 worker の結果だけを集約する。

理由:
- branch protection の server-side context 設定を同時変更しない。
- migration 中も既存 required context の契約を維持できる。
- worker を後から増減しても external contract を固定できる。

### D2: Rust work を2系統へ分離する

#### rust-quality
- Nix setup
- `cargo test -- --test-threads=1`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- Tauri/GTK apt dependency install は行わない

`--test-threads=1` は PR 検証中に露見した既存 operations global-lock test の競合をCI内で封じる暫定措置であり、production lock semantics は変更しない。根本修正は issue #91 で追跡する。

#### rust-build-smoke
- Tauri/GTK apt dependencies
- Nix setup
- `cargo build --release -p schneeforge`
- `schneeforge --version`
- `schneeforge doctor`
- `cargo check --release --manifest-path apps/desktop/src-tauri/Cargo.toml`

CLI release smoke と desktop compile smoke を同じrunnerに置き、setup重複を避ける。Linux desktop は compile gate とし、link/bundleを含む実 artifact gate は required `release-artifact-check` の macOS DMG full build が担う。

Tauri desktop の `build.rs` は build profile に対応する `target/<profile>/schneeforge` sidecar source を要求する。実CIで debug `cargo check` と release CLI build の組合せが `target/debug/schneeforge` 不在で失敗したため、compile gate も `--release` に揃える。これにより同じrunnerで作成済みの `target/release/schneeforge` をsidecar sourceとして利用する。

2 worker は依存関係を持たず同時開始できる。

### D3: aggregator は fail-closed

`rust-check` は:

- `needs: [rust-quality, rust-build-smoke]`
- `if: ${{ always() }}`
- 各 `needs.*.result` が `success` 以外なら `exit 1`

とする。failure だけでなく `cancelled` / `skipped` も non-success として扱う。

### D4: `ci-required` を shadow aggregator として追加する

`ci-required` は以下を `needs` に持つ。

- `openspec-check`
- `flake-check`
- `rust-check`
- `lint`
- `bootstrap-test`
- `managed-nix-e2e`
- `release-artifact-check`

`if: ${{ always() }}` で実行し、1件でも non-success なら failure とする。

本 change では branch protection の required context に `ci-required` を登録しない。まず shadow gate として複数 PR の実績を作る。

### D5: required workflow へ path filter を入れない

GitHub の required workflow が path/branch/commit skip で trigger 自体されない場合、check が Pending のまま残り merge を阻害できる。そのため required contexts が現行 workflow に紐づく migration 中は workflow-level `paths` / `paths-ignore` を導入しない。

将来 path-aware CI を導入する場合は、required aggregator 自体が常に起動し、安全に worker skip を判定できる構造へ別 change で設計する。

### D6: performance gate は wall-clock と resource cost の2軸

比較対象は変更前の成功 run と変更後 PR run とする。

- primary: required critical path elapsed time
- secondary: Rust worker + aggregator の elapsed runner time 合計

baseline (successful run #378):
- required execution critical path: 479s (`15:50:16Z` の最初の required start → `15:58:15Z` の最後の required completion)
- `rust-check`: 448s

目標:
- primary: 25%以上短縮 (359s以下)
- secondary: Rust runner total +20%以内 (537.6s以下)

計測履歴:
- 3-way experiment: runner total が completion 前に574s超となりsecondary guard違反で棄却。
- 2-way run #393: Rust total 426sでsecondaryは合格。ただし required critical path 387sでprimaryは19.2%短縮に留まり不合格。
- final measured run #397: current required は `17:42:51Z` の最初の required startから `17:48:18Z` の `rust-check` completionまで327s。479s→327sで31.7%短縮。Rust runner total は `rust-quality=119s` + `rust-build-smoke=320s` + `rust-check=4s` =443sで、baseline 448s以下。shadow `ci-required` completionまで含めても332s。

したがって final 2-way + flake realization split は primary / secondary 両方の目標を満たす。

### D7: Linux flake gate は default evaluation と lightweight realization を分離する

required `flake-check` は以下を行う。

1. `nix flake check --allow-import-from-derivation`
2. default `developer` Linux activation package の `drvPath` を `nix eval`
3. `tests/fixtures/profile-minimal.nix` を `--override-input profile` で指定し、`minimal` Linux activation package を `nix build`
4. Linux ARM activation package の `drvPath` を `nix eval`

product default (`schneeforge.toml` の `developer`) は変更しない。fixture はCIのrealization targetを明示するためだけに使う。

trade-off:
- required Linux gate は default `developer` の module/config/derivation construction error を検出する。
- required Linux gate は developer-only upstream packages の build-time failureを毎PRではrealizeしない。
- supported `minimal` profileについては actual Home Manager realization を維持する。
- default `developer` の actual realizationは非required `macos-check` が継続し、release artifact gateは別途macOS DMG full buildを継続する。

これは単純なcoverage削除ではなく、PR merge gateのcritical pathとfull-environment verificationの責務分離である。

## Failure Modes

- **worker fail なのに aggregator green**: `always()` + explicit non-success check で防止。
- **worker skip で required check が抜ける**: skip も failure とする。
- **required context rename で PR が永久 Pending**: `rust-check` 名を維持し server-side 設定を触らない。
- **parallelization で runner minutes が急増**: before/after 計測し +20% guard を使う。3-way案はこのguardで実際に棄却済み。
- **desktop profile mismatch で sidecar source が見つからない**: CLI release build と desktop `cargo check --release` のprofileを一致させ、contract testで固定する。
- **desktop full build の重複で cold runner が遅い**: Linux は release-profile `cargo check` compile gate、macOS required artifact job は full build と責務を分離する。
- **Linux linker固有の問題を compile gate が見逃す**: Linux GUI artifact は現行 release workflow の配布対象ではない。Linux full GUI artifactを配布対象にする際は専用release gateを追加する。
- **developer-only packageのbuild-time regressionをrequired Linux gateが見逃す**: default `developer` はderivation evaluationをrequiredで維持し、actual developer realizationは非required `macos-check` で継続する。必要になれば別changeでbinary cacheまたは専用scheduled realization gateを設計する。
- **CI fixtureがproduct defaultを書き換える**: `--override-input` は当該 build invocation のみに限定し、`schneeforge.toml` / `defaults/profile.nix` は変更しない。
- **path filtering で workflow 自体が起動しない**: 本 change では導入禁止。

## Rollback

性能回帰またはcoverage問題が判明した場合は段階的に戻す。

1. Linux flake realization split をrevertし、default `developer` Linux activation package のfull buildへ戻す。
2. Rust worker fan-outに問題がある場合は `.github/workflows/check.yml` の2-way workers/aggregatorをrevertし、従来の単一 `rust-check` jobに戻す。
3. branch protection server-side 設定は本changeで変更しないため、rollback 時にGitHub settings migrationは不要。
