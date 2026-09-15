## Context

`check.yml` の required gate は安全側に倒れているが、`rust-check` は以下を直列実行している。

1. Tauri/GTK system dependency install
2. Nix install/devShell setup
3. `cargo test`
4. `cargo fmt -- --check`
5. `cargo clippy -- -D warnings`
6. CLI release build + smoke
7. CLI debug build + Tauri desktop build smoke

root Cargo workspace は `crates/core` と `crates/cli` のみで、desktop (`apps/desktop/src-tauri`) は workspace member ではない。そのため GTK/Tauri system dependencies は core/CLI quality gate には不要であり、desktop smoke に閉じ込められる。

GitHub Actions では `needs` の依存 job が failure/skipped の場合、通常は dependent job も skipped になる。required aggregator が skipped して成功扱いになる曖昧さを避けるため、aggregator は job-level `if: ${{ always() }}` で必ず評価し、各 `needs.<job>.result` が `success` 以外なら step を明示的に failure にする。

## Goals / Non-Goals

### Goals

- required context 名を維持したまま Rust CI を fan-out する。
- desktop 固有 dependency setup を desktop worker のみに限定する。
- worker failure/cancel/skip を required `rust-check` へ fail-closed で集約する。
- 現行 required 7 checks を集約する `ci-required` candidate を追加する。
- branch protection は変更せず、複数 PR で `ci-required` を観測可能にする。
- before/after の wall-clock と runner duration を測る。

### Non-Goals

- 全 CI job の一括再設計。
- macOS runner 数を増やす最適化。
- path filter による required workflow skip。
- cache provider の変更。
- branch protection required context の即時一本化。

## Decisions

### D1: existing required context `rust-check` を aggregator として残す

worker の job ID/name は新規に `rust-quality`, `rust-cli-smoke`, `rust-desktop-smoke` とする。既存 `rust-check` は実ビルドを持たず、この3 worker の結果だけを集約する。

理由:
- branch protection の server-side context 設定を同時変更しない。
- migration 中も既存 required context の契約を維持できる。
- worker を後から増減しても external contract を固定できる。

### D2: Rust work を3系統へ分離する

#### rust-quality
- Nix setup
- `cargo test`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- Tauri/GTK apt dependency install は行わない

#### rust-cli-smoke
- Nix setup
- `cargo build --release -p schneeforge`
- `schneeforge --version`
- `schneeforge doctor`
- Tauri/GTK apt dependency install は行わない

#### rust-desktop-smoke
- Tauri/GTK apt dependencies
- Nix setup
- `cargo build -p schneeforge` (Tauri sidecar stage source)
- `cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml`

3 worker は依存関係を持たず同時開始できる。

### D3: aggregator は fail-closed

`rust-check` は:

- `needs: [rust-quality, rust-cli-smoke, rust-desktop-smoke]`
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

目標:
- primary: 25%以上短縮
- secondary: +20%以内

目標未達の場合は「高速化済み」と扱わず、worker 分割粒度や cache の必要性を再評価する。

## Failure Modes

- **worker fail なのに aggregator green**: `always()` + explicit non-success check で防止。
- **worker skip で required check が抜ける**: skip も failure とする。
- **required context rename で PR が永久 Pending**: `rust-check` 名を維持し server-side 設定を触らない。
- **parallelization で runner minutes が急増**: before/after 計測し +20% guard を使う。
- **desktop cold build が想定以上に遅い**: 3-way split を2-wayに戻す、または後続で既存 cache strategy を再利用する。
- **path filtering で workflow 自体が起動しない**: 本 change では導入禁止。

## Rollback

`.github/workflows/check.yml` の Rust worker fan-out と aggregators を revert し、従来の単一 `rust-check` job に戻す。branch protection server-side 設定を変更しないため、rollback 時に GitHub settings migration は不要。
