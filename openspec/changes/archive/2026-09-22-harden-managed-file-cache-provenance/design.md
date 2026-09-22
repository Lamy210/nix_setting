## Context

Managed source は local working tree を持たないため、`schneeforge.toml` 等を tag-pinned raw GitHub URL から取得し state dir へ cache する。

従来 layout:

```text
sources/<tag>/<file>
```

この key には repository identity が含まれない。一方 `SourceState.remote` / `SCHNEEFORGE_REPO_URL` は fork を許可する。そのため repository A と B が同じ tag を持つ場合、A の cache を B が読む collision が成立する。

## Goals / Non-Goals

### Goals

- repository を跨いだ cache reuse を防ぐ。
- cache content corruption を検出する。
- verified cache は offline で従来どおり利用できる。
- interrupted cache replacement は fail-closed にする。
- existing content path layout を維持する。
- mutable/non-release ref を indefinite cache boundary に入れない。

### Non-Goals

- cache directory layout の全面 migration。
- Git tag 自体が remote で force-move されないことの検証。
- repository content signing。
- same-user attacker による sidecar/content 同時改竄への耐性。

## Decisions

### D1: content path は維持し provenance sidecar を追加

content path は互換性のため `sources/<tag>/<file>` のままとする。各 content に:

```text
<file>.schneeforge-cache.json
```

を追加し、以下を保存する。

- provenance schema version
- normalized `owner/repo`
- SHA-256(content)

これにより layout の大規模 migration を避けつつ repository collision と accidental corruption を検出できる。

### D2: verified cache だけを hit とする

cache hit 条件は content file の存在ではなく、次をすべて満たすこととする。

1. content + sidecar が読める
2. sidecar schema が対応版
3. repository identity が現在 source と一致
4. sidecar digest が content SHA-256 と一致

いずれか不一致なら cache miss として network fetch へ進む。

### D3: legacy cache は trust しない

sidecar の無い旧 cache は provenance を証明できないため、そのまま offline reuse しない。

- online: tag-pinned URL から再取得し、新 provenance を生成
- offline: verified cache無しとして error

security boundary を弱める「legacyなら無条件trust」は導入しない。

### D4: replacement は provenance-last

repository切替や破損cacheの再取得時:

1. old provenance を削除
2. content を atomic write
3. new provenance を atomic write

とする。

途中停止時に old provenance と new content の組み合わせを valid hit として返さないためである。

### D5: immutable-ref boundary を release tag syntax で enforce

managed repo-file cache は `classify_release_tag(tag)` が受理する `v<SemVer>` のみを対象とする。`main` 等の mutable ref を cache key として受理しない。

これは既存設計の「release tag は cache boundary」という契約をコード境界でも enforce するもので、remote側tag移動検証までは本changeに含めない。

### D6: provenance namespace を予約

content file 名が `.schneeforge-cache.json` suffix を持つ場合、別entryのsidecar pathと衝突し得るため拒否する。`cache_path` は module-private とし、validation を迂回する path construction API を外へ出さない。

## Risks / Trade-offs

- upgrade直後にofflineだとlegacy cacheを使えない。
  - Mitigation: security provenanceを持たないcacheをtrustしないことを優先。online初回で自動再生成する。
- 同一tagでforkを切替えるとcache content/sidecarが置き換わるため、元forkへofflineで戻った際はmissになる。
  - Mitigation: wrong-repository contentを返すよりfail-closedを優先。layout変更を伴うper-repo cacheは将来検討可能。
- Git tagは技術的にはforce-move可能。
  - Mitigation: current specのrelease-tag boundaryを維持。本changeはrepository collision/local integrityを対象とし、commit-SHA pinningは別changeとする。

## Migration Plan

1. provenance sidecar read/writeを追加する。
2. verified cache hit判定へ切り替える。
3. legacy cacheをmiss扱いにする。
4. mutable ref/path traversal/sidecar namespaceをrejectする。
5. fork collision / tamper / offline / legacy migration regression testを追加する。
6. required CIとOpenSpec strict validationを通す。

## Rollback

sidecar追加のみでcontent layoutは変えないため、revertすると従来cache readerへ戻せる。sidecar fileは旧readerから無視される。
