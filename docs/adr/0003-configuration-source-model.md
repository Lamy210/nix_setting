# ADR-0003: ConfigurationSource モデル (Release / Git / Local)

Date: 2026-08-17
Status: Accepted (2026-08-18, PR #44 にて実装とともに承認)

## Context

SchneeForge v0.2 までの configuration は単一の Git checkout
(`~/nix_setting`, install.sh が release tag を pinned clone) のみを
想定しており、source の種別を表現する model が無い。このため:

1. **update 操作が source の実態と不一致**: `schneeforge upgrade` は
   常に `nix flake update` (dependency 更新) を実行するが、release
   tag に pinned された checkout では flake.lock の変更は
   「CI で検証された release 単位」を壊す (v2 設計 §26:
   Stable では flake.lock 変更禁止)
2. **複数の利用形態を同等に扱えていない**: 一般 user (Stable)、
   RC 検証者 (Preview)、fork user (Git)、SchneeForge 開発者 (Local) は
   同じ core 操作の背後で異なる更新挙動が必要
3. **State に source 情報が無い**: 「source の現在状態」と
   「PC に適用済みの状態」を区別できず、GUI dashboard が
   installed / available / applied を表示できない

v2 設計 (「Easy by default, Git-native when desired」) は
Source Resolver を core の中心に置き、4 種の source を解決する:

```text
Release (Stable)    exact release tag
Release (Preview)   prerelease tag
Git (Tracking)      branch 追従 (fetch → pull --ff-only)
Git (Pinned)        tag / commit 固定
Local               開発中 working tree
```

## Decision

1. **core に `source` module を新設する**。`SourceKind` enum
   (ReleaseStable / ReleasePreview / GitTracking / GitPinned / Local) と
   `SourceResolver` が checkout の実態 (detached HEAD at tag /
   branch / flake 未管理) から kind を検出する
2. **update 操作を source kind で dispatch する**:
   - ReleaseStable / ReleasePreview: 次の release tag へ checkout
     (fetch tags → 移動)。flake.lock は触らない
   - GitTracking: `git fetch` + `git pull --ff-only`
   - GitPinned / Local: no-op (明示的な案内表示)
   - dependency 更新 (`nix flake update`) は update から分離し
     `source deps update` (Advanced) とする
3. **State を拡張する**: `source.kind` / `source.ref` /
   `source.channel` を記録し、applied_revision とは分離する
4. **install.sh の pinned clone は GitPinned 相当**として検出する
   (detached HEAD + release tag は Release source の checkout 表現)

## Consequences

- `schneeforge upgrade` / `schneeforge sync` は v0.3 まで alias として
  残し、新しい `schneeforge update` / `schneeforge source sync` /
  `schneeforge source deps update` へ移行する (deprecation note 表示)
- ReleaseSource の「次の release 検出」は GitHub Releases API への
  依存が発生する。offline 時は現状維持の案内表示に fallback する
- Stable checkout で `nix flake update` 相当を実行する手段は
  `source deps update` に集約し、release 単位の検証 (CHECKSUMS の
  flake.lock hash) から外れる操作であることを help / docs で明示する

## Alternatives Considered

- **単一 Git source のまま update を smart にする**: source 実態と
  操作の不一致が残り、GUI で channel 表示もできない。採用しない
- **Release source で working tree を持たない (Nix Store 直接)**:
  v2 §7 の理想形として実現済み (openspec change
  `add-managed-release-source`)。Release source の実体を flake ref
  `github:<owner>/<repo>/<tag>` とし、nix が直接取得・cache する
  (SchneeForge は source tree を local に持たない)。既存の nix 引数
  (`--inputs-from <repo>` / `--flake <ref>` / `nix build <ref>`) は
  flake ref 文字列をそのまま受け、`--override-input path:` も remote
  flake に有効なため引数構成は不変。tag が不変のため取得結果・
  repo file cache (`raw.githubusercontent.com` tag-pinned 取得を
  state dir へ無期限 cache) も不変。移行は `schneeforge source init`
  の明示実行のみで、install.sh 由来の pinned checkout (checkout 表現)
  との 2 表現佷存。Phase 1 で pinned checkout を仮表現としたのは、
  apply/rollback の引数経路が repo path 前提だったため
