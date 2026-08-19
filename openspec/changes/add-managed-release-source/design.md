# Design: Managed Release Source (v2 §7)

## Context

ADR-0003 は Phase 1 で pinned checkout を Release 表現とし、
working tree-less を後続 change とした。本 design はその実現形を
決定する。

## Decision 1: 表現は github flake ref (ローカル tree の展開はしない)

Release source の実体を `github:<owner>/<repo>/<tag>` flake ref とし、
nix に取得・cache (Nix Store) させる。候補とした代替:

1. **github flake ref (採用)**
   - SchneeForge 側に実体を持たない (= working tree-less の直接表現)
   - 既存の nix 引数 (`--inputs-from <repo>` / `--flake <ref>` /
     `nix build <repo>#target`) は flake ref 文字列をそのまま受け、
     path も `github:` ref も引数構成を変えずに通る
   - `--override-input machine/profile path:<abs>` は remote flake に
     も有効 (PR #48 の path: URL 形式をそのまま使用)
   - tag が不変 → nix の取得結果も不変。cache の TTL / invalidation
     問題が原理的に発生しない
2. **state dir への release tree 展開 (git archive / tarball)**
   - flake 評価は local path のまま動くが、SchneeForge が実体管理
     (展開・世代・削除) を負い、working tree-less の趣旨 (管理する
     実体を減らす) に反する。不採用
3. **bare repo + fetch のみで rev を解決し flake は local checkout 継続**
   - working tree が残るため §7 の解決にならない。不採用

## Decision 2: repo file 読み取りは tag-pinned HTTP + 無期限 cache

nix 評価以外で repo file を読む箇所 (`schneeforge.toml`: profile 解決 /
manifest / dashboard、`bootstrap-manifest.toml`: nix install plan) は、
managed source の場合 `raw.githubusercontent.com/<owner>/<repo>/<tag>/<file>`
を取得し `~/.local/state/schneeforge/sources/<tag>/<file>` へ保存する。

- tag が不変なので一度取得した cache は永遠に正しい (TTL 無し)。
  offline でも 2 回目以降は cache から読める
- 失敗時 (offline 初回 / 404) は fail-closed に error。ただし cache
  があれば network に行かない
- 取得関数は差し込み可能 (引数に fetch closure) にし hermetic test を
  維持する (dashboard.rs と同じ分離 pattern)

## Decision 3: 2 表現の併存と移行

- **checkout 表現** (従来): install.sh の pin checkout。detect /
  update (`fetch --tags` + `checkout`) は現状維持。install.sh と
  bootstrap-flow は本 change で不改変のため必ず併存する
- **managed 表現** (新): state の `source` が managed を示す。repo
  解決は flake ref を返し、update は tag の state 更新のみ
- 移行は `schneeforge source init` の明示実行のみ。既存 checkout が
  release tag pin なら「同 tag を managed に移行した」旨を表示
  (checkout dir は削除しない。user が自由に退避できる)
- checkout 表現の廃止 (install.sh の managed 化) は bootstrap-flow
  spec 変更を伴う後続 change とする

`SourceState` は `managed: bool` を `#[serde(default)]` で追加し、旧
state.json はそのまま読める (managed=false = checkout 表現)。

## Decision 4: rev 検証は設定・更新時に 1 回

`source init` / `update` (managed) の際に `ReleaseMetadata::fetch(tag)`
(§27) を取得し `source_revision` を state に記録する:

- metadata asset を持つ tag では「tag → commit SHA」が記録され、
  tag の不変性が state から確認できる (apply 時の再検証は不要。
  nix は ref を rev に解決して cache する)
- metadata asset が無い tag (rc.5 以前) は警告付きで検証 skip
  (fail-closed にすると旧 tag への init が全滅するため)

## Decision 5: update dispatch の分岐

| source 状態 | update の挙動 |
|---|---|
| Release (managed) | ls-remote → `latest_tag_for_channel` → state の tag 更新 + rev 検証。checkout 操作なし |
| Release (checkout 表現) | 従来どおり fetch --tags + checkout。実行後に managed 移行の案内を 1 行表示 |
| GitTracking | 従来どおり (fetch + pull --ff-only) |
| GitPinned / Local | 従来どおり no-op 案内 |

sync / dirty check など git 実態を前提とする処理は、managed では
「git working tree が無い」旨の案内に切り替える (error にしない。

## よくある疑問

- **offline で apply できるか**: 初回の nix 評価は network 必要
  (github ref の tag → rev lookup + tarball 取得)。nix は取得結果を
  cache するため 2 回目以降は原則 offline で動く。repo file
  (schneeforge.toml 等) は state cache のため offline で可
- **fork では**: `SCHNEEFORGE_REPO_URL` (既存 env) から owner/repo を
  解決し `github:<owner>/<repo>/<tag>` を生成する。raw URL も同様
- **`github:` ref の lock**: repo 自体の flake.lock は tag pinned で
  不変 ([nix-path-input-lock-gotcha] の path input 問題は
  machine/profile input 側で既に対処済み)
