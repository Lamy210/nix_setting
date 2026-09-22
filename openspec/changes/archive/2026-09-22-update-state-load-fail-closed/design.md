## Context

`StateStore::load()` は現在:

```rust
pub fn load(&self) -> Option<State> {
    let content = std::fs::read_to_string(&self.path).ok()?;
    serde_json::from_str(&content).ok()
}
```

であり、missing file / permission failure / path type error / malformed JSON をすべて `None` にする。

一方で `None` は「初期 state がまだ無い」という正常状態として広く利用されている。managed source と selected profile は checkout から完全には復元できないため、既存 state が壊れたときに `None` へ落とすと semantic state を失う。

特に現在の call site には次がある。

- apply / rollback: effect 実行後に previous state を読み、profile / managed source を保持する
- update / managed update / source init / profile mutation: `unwrap_or_default()` で破損 state を空 state に置換できる
- source resolution / manifest load: state error を checkout/local path fallback と区別できない
- self-update / Dashboard: state error を stable channel default と区別できない
- diagnostics / status: corrupt state を missing state として表示できる

## Goals / Non-Goals

### Goals

- missing state を正常な `None` として維持する。
- existing state の read / parse failure は structured error にする。
- valid legacy state は互換維持する。
- mutation operation は state error を副作用前に検出する。
- state semantic を使う read path は silent fallback しない。
- diagnostics は可能な範囲で他の診断を維持しつつ state failure を明示する。

### Non-Goals

- state schema versioning。
- automatic repair。
- state corruption の recovery wizard。
- atomic save protocol の変更。

## Decisions

### D1: `StateStore::load() -> Result<Option<State>>`

別名の strict API を追加して旧 `load()` を lossy のまま残す案は採用しない。lossy API が残ると新旧 call site が混在し、同じ silent fallback が再導入されるためである。

意味は次のとおり。

- file not found: `Ok(None)`
- valid JSON: `Ok(Some(state))`
- other read error: `Err(Error::State(...))`
- JSON parse error: `Err(Error::State(...))`

`Error::State(String)` を追加し、既存 state file の read / parse failure を state-specific error として返す。missing file だけは error にしない。state path/context は message に含めるが、state JSON 本文は error へ含めない。

### D2: valid legacy JSON はそのまま受理

`State` の optional field / serde default を維持する。`source` や `profile` が存在しない古い JSON は corruption ではなく互換対象である。

### D3: mutation は effect より先に strict load

state を参照・保持・更新する mutation は lock 取得後、外部 command / checkout / save 等の副作用より前に必要な state を load する。

対象例:

- apply / rollback
- update / managed update
- source init
- profile set / clear

load error の場合は command 実行や state overwrite を開始しない。

### D4: semantic fallback を禁止

state error は以下へ変換しない。

- managed source → local checkout
- selected profile → manifest default
- self-update channel → stable
- existing state → `State::default()`

missing state (`Ok(None)`) のときだけ既存 default/fallback behavior を許可する。

### D5: read-only surfaces は error を可視化

CLI / Desktop の read-only surface は、state corruption を「未設定」と表示しない。

- command 自体が `Result` を返せる status/source/dashboard path は state error を caller へ伝播してよい
- diagnostics のように複数独立checkをまとめる surface は、他checkを失わない形で state error field/check を持たせてもよい

実装時に既存 public data modelへの影響を最小化する。

## Risks / Trade-offs

- `StateStore::load()` の public signature change でcallsiteが広く変わる。
  - Mitigation: repository-wide searchで全callsiteをmigrationし、compilerを網羅性チェックとして使う。
- apply / rollback の state pre-load により処理順が変わる。
  - Mitigation: state load は lock 取得後、effect 前に固定し、既存 lock semantics は維持する。
- diagnostics API を厳格化すると一部情報まで見えなくなる可能性がある。
  - Mitigation: diagnosticsは state-specific error を保持する方式を優先し、完全失敗はstate semanticが不可欠なsurfaceに限定する。
- unreadable file test はpermission挙動がOS/rootで変わる。
  - Mitigation: cross-platform unit testでは「state path がdirectory」等の deterministic read error を使い、permission-specific behaviorは必要ならplatform testへ分離する。

## Migration Plan

1. `StateStore::load()` を strict result API に変更し、missing / malformed / read error / legacy JSON testを追加する。
2. coreのread-only callsiteをmigrationする。
3. mutation callsiteをeffect前 strict loadへ並べ替える。
4. CLI/Desktop adapterをmigrationし、state error表示を明示する。
5. managed source / profile / self-update channel のsilent fallback regression testを追加する。
6. `cargo test --all`, fmt, clippy, OpenSpec strict, existing CIを通す。

## Rollback

runtime migration前ならchange proposalを削除可能。実装後のrollbackはAPI変更とcallsite migrationを同一squash commitでrevertする。on-disk state formatは変更しないためdata migration rollbackは不要。
