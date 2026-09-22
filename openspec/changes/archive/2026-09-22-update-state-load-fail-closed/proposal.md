# Change: State 読み込みを fail-closed にする

## Why

Issue #124 のとおり、現在の `StateStore::load()` は state file の不存在、read error、JSON parse error をすべて `None` に畳み込む。call site は `None` を「未初期化」と解釈するため、破損した managed source / profile state が local checkout や default profile / stable channel へ静かに fallback し得る。

valid な legacy state JSON の後方互換は維持しつつ、「state が存在しない」と「state が存在するが信用できない」を区別し、状態依存操作を fail-closed にする。

## What Changes

- **BREAKING (core API)**: `StateStore::load()` を `Result<Option<State>>` に変更する。
- state file が存在しない場合のみ `Ok(None)` を返す。
- existing file の read error / malformed JSON は dedicated `Error::State` を返し、未初期化扱いしない。
- `source` / `profile` 等の field を持たない valid legacy JSON は従来どおり deserialize して `Ok(Some(State))` を返す。
- apply / rollback / update / source init / profile mutation 等、state を保持・更新する操作は destructive/effectful action より前に strict load し、load error 時は副作用を開始しない。
- source resolution / manifest loading / self-update channel selection は corrupt state を checkout/default/stable へ fallback しない。
- read-only status / diagnostics / Dashboard は missing state と corrupt state を明確に区別し、corrupt state を「未設定」と表示しない。
- state error を再現する regression tests を追加する。

## Non-Goals

- State JSON schema version の導入。
- valid legacy state field の削除や migration。
- state file の自動修復・自動削除。
- corrupt state の backup / recovery UI。
- cache provenance や Release Metadata format の変更。

## Impact

- Affected specs: `core-operations`
- Affected code: `crates/core/src/state.rs`, `operations.rs`, `source.rs`, `source_files.rs`, `profile.rs`, `diagnostics.rs`, CLI / Desktop adapters
- Compatibility: on-disk valid legacy JSON は互換。Rust API の `StateStore::load()` 呼び出しは migration が必要
- Related: issue #124
