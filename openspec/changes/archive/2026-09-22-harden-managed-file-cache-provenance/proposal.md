# Change: Managed repo-file cache を repository provenance に束縛する

## Why

Managed Release Source の repo-file cache は現在 `sources/<tag>/<file>` だけを key とする。SchneeForge は `SCHNEEFORGE_REPO_URL` で fork を利用できるため、異なる repository が同じ release tag を持つと、先に保存された別 repository の cache を誤って返し得る。

また cache content 自体の integrity metadata が無く、local cache の破損を検出できない。tag-pinned cache の offline 性を維持しつつ、repository identity と content digest を検証できる cache entry だけを trust する。

## What Changes

- managed repo-file cache entry に provenance sidecar を追加する。
- provenance は schema version / normalized GitHub owner+repo / content SHA-256 を保持する。
- cache hit 時に repository identity と content digest を検証し、不一致は cache miss とする。
- provenance を持たない legacy cache は untrusted miss とし、network 利用可能なら再取得して migration する。
- offline かつ verified cache が無い場合は従来どおり fail-closed に error とする。
- managed repo-file ref は valid `v<SemVer>` release tag に限定し、mutable ref (`main` 等) を indefinite cache に使わない。
- cache/URL path component を allow-list 検証し、traversal / separator / percent-encoded traversal / provenance sidecar namespace collision を拒否する。
- cache path helper は module-private とし、validation を迂回する public path builder を提供しない。

## Non-Goals

- cache directory layout の repository slug 化。
- remote Git tag の cryptographic immutability 検証。
- ReleaseMetadata の `source_revision` を raw content URL に使用する変更。
- legacy cache を offline のまま trust する migration。
- local state directory を malicious same-user actor から保護する sandbox。

## Impact

- Affected specs: `core-operations`
- Affected code: `crates/core/src/source_files.rs`
- Cache layout: content path `sources/<tag>/<file>` は維持。sidecar file を追加
- Compatibility: provenance 無しの旧 cache は初回再取得が必要。offline の旧 cache は trust せず error
