## MODIFIED Requirements

### Requirement: repo file の tag-pinned 取得

core SHALL は managed source について、repo file (`schneeforge.toml` /
`bootstrap-manifest.toml` 等) を release tag pinned で取得できる。取得は
`raw.githubusercontent.com/<owner>/<repo>/<tag>/<file>` とする。

cache content は state dir の `sources/<tag>/<file>` に原子保存し、各
entry は repository identity と content SHA-256 を持つ provenance
sidecar で検証する。repository / digest / provenance schema のすべてが
一致する verified cache のみを cache hit として扱う。verified cache は
2 回目以降 network を行わず offline でも利用できる。

provenance を持たない legacy cache、repository 不一致、digest 不一致、
未対応 provenance schema は cache miss として扱い、network から再取得
できなければ fail-closed に error を返す。

managed repo-file の ref は valid `v<SemVer>` release tag に限定し、
mutable ref や path traversal に利用できる tag/file component は拒否する。
path source (checkout / Local) の file 読み取りは従来どおり local
filesystem を使う。

#### Scenario: 初回取得と verified cache

- **WHEN** managed source の `schneeforge.toml` に verified cache が無い状態で読み取られる
- **THEN** release tag pinned URL から取得する
- **AND** content と repository/digest provenance を state dir へ保存する
- **WHEN** 同じ repository / tag / file を再度読み取る
- **THEN** verified cache から返し network には行かない

#### Scenario: verified cache の offline 利用

- **WHEN** repository identity と content digest が一致する verified cache が存在する
- **AND** network が利用できない
- **THEN** cache から内容を返す

#### Scenario: 同じ tag を持つ別 repository

- **WHEN** repository A の tag `vX.Y.Z` cache が存在する
- **AND** source が repository B の同じ tag へ切り替わる
- **THEN** repository A の cache を repository B の content として返さない
- **AND** repository B を fetch できなければ error を返す

#### Scenario: cache content の tamper / corruption

- **WHEN** cached content の SHA-256 が provenance と一致しない
- **THEN** cache hit として返さない
- **AND** network から再取得できなければ error を返す

#### Scenario: legacy cache

- **WHEN** content file は存在するが provenance sidecar が存在しない
- **THEN** legacy cache を untrusted miss として扱う
- **AND** network が利用可能なら再取得して verified cache へ移行する
- **AND** network が利用できなければ error を返す

#### Scenario: mutable ref を cache boundary にしない

- **WHEN** managed source ref が `main` 等の valid release SemVer tag ではない
- **THEN** repo-file read は fetch/cache lookup の前に error を返す

#### Scenario: unsafe path component

- **WHEN** tag/file に traversal、path separator、percent-encoded traversal、または provenance sidecar namespace と衝突する file name が指定される
- **THEN** raw URL / cache path を使用する前に error を返す

#### Scenario: path source

- **WHEN** source が checkout または Local で managed release ではない
- **THEN** repo file は従来どおり local filesystem から読み取る
