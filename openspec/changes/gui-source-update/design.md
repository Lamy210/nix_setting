# Design: GUI からの configuration source 更新 (run_update)

## Context

- core `update()` (`crates/core/src/operations.rs`) は git と操作 lock
  (`acquire()`) だけを要求し、root 権限を必要としない:
  - `UpdateManagedRef`: `git ls-remote` で channel 最新 tag を解決し state
    のみ更新 (ReleaseMetadata があれば revision を記録)
  - `CheckoutLatestTag`: checkout 表現の Release を最新 tag へ fetch +
    checkout
  - `FastForward`: dirty check の後 `git pull --ff-only`
  - `Local`: NoOp (案内文のみ)
- 一方 `upgrade` (flake.lock 更新) は managed source で fail-closed
  (`DEPS_MANAGED_ERROR`)。GUI の「アップグレード」ボタンは昇格して
  `schneeforge upgrade` を実行するため、managed machine では必ず失敗する。
- gui-operations 仕様の昇格要件は apply / rollback / upgrade (と nix
  repair / uninstall) を対象とし、sync (git pull) は「昇格せず user 権限の
  まま」と明記されている。`update` は sync と同じ性質 (git 操作のみ) の
  ため in-process 実行とする。

## Goals / Non-goals

**Goals**

- GUI から v2 主操作 (`schneeforge update` 相当) を実行できる
- managed machine で必ず失敗する非推奨ボタンを表示しない
- CLI / install.sh / GUI wizard に続き、更新操作も CLI と GUI で経路を
  揃える (core の同一関数を呼ぶ)

**Non-goals**

- app 本体の自己 update (Phase E)
- deps update (flake.lock) の managed 以外での UI 変更 — 従来の昇格経路
  を維持する

## Decisions

### D1: `run_update` は in-process (昇格しない)

core `update()` は root 不要のため、`run_plan` / `run_source_init` と同じ
`spawn_blocking` + core 直接呼び出し pattern にする。昇格経路
(`run_escalated_cli`) を使うと不要な管理者認証を要求することになり、sync
 の前例 (仕様上「昇格せず user 権限のまま」) とも矛盾する。

操作 lock は core `update()` 内の `acquire()` で取得されるため、GUI と CLI
の同時実行直列化も CLI と同一の仕組みで働く。

### D2: capture mode (`capture: true`) で結果を CommandOutput に載せる

`update()` は `capture` flag で出力を `UpdateResult.output` に返す。GUI は
`capture: true` で呼び、成功時の出力 (例: "Already on the latest stable
release" / 移行先 tag) をそのまま output area に表示する。git fetch / pull
の出力も capture されるため、進捗は run helper の「実行中...」表示で足りる
(apply 系のような数分の長期操作ではない)。

### D3: ボタン label は「ソース更新」(既存「更新」と衝突しない)

既存の「更新」ボタン (id: `refresh`) は status 再取得専用。同一 label の
ボタンが 2 つ並ぶのは誤操作の元のため、新ボタン (id: `update`) の label は
「ソース更新」とする。DOM id は CLI 主操作名 (`update`) と一致させる。

### D4: managed source では「アップグレード」を hidden にする

- `disabled` ではなく `hidden`: flake.lock 更新は managed source では
  「存在しない操作」であり、押せないボタンを置いても意味が無い
- 判定は `get_status` の `managed_source` (PR #60 で追加) を使い、
  `refresh()` の再取得のたびに再評価する (update で checkout → managed に
  移行した場合も直ちに反映される)
- checkout 表現 / GitTracking では従来通り表示し、従来の昇格経路を使う

### D5: 実行後は status / dashboard を再取得する

`update` は state の `source` を書き換えるため、成功時に `refresh()` +
`refreshDashboard()` を呼び Dashboard の channel / tag 表示を更新する。
run helper の finally 句が既に `refresh()` を呼ぶため、追加で
`refreshDashboard()` だけ呼ぶ。

## Risks / Trade-offs

- **update 中の network 失敗**: core が error を返すため CommandOutput の
  失敗表示に載る。offline でも managed の現 tag は維持される (state 更新
  は成功時のみ)
- **アップグレードボタンを hidden にする発見性**: managed machine で
  flake.lock を更新したい user は CLI (`schneeforge source deps update`)
  でのみ可能 — 仕様上 fail-closed なので GUI からは提供しない方が正しい

## Migration Plan

- frontend の gate は `managed_source` の有無のみで判定 (後方互換: 古い
  binary + 新 frontend の組合せは無い。Tauri app は bundle 配布のため
  frontend と backend が常に同版)

## Open Questions

- なし (Phase E の app 自己 update は別 change)
