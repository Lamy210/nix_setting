# Design: add-release-attestation-bundles

## Context

配信 platform (SchneeSoftwareDistribution) は release asset の命名規約
(`<artifact>.sig.bundle` / `<artifact>.provenance.bundle` / `<artifact>.spdx.json`)
で attestation を取り出し、identity pin
`https://github.com/Lamy210/nix_setting/.github/workflows/release.yml@` 付きで
cosign 検証する。規約 asset は platform 側で配布 artifact から除外される
(change: exclude-attestation-assets)。本 change は producer 側の生成を担う。

## 決定事項

### D1: keyless (OIDC) 署名は release job で行う

cosign の証明書 SAN identity は署名した GitHub Actions workflow の ref になる。
platform の pin は `.github/workflows/release.yml@` prefix なので、署名は必ず
release.yml の release job (`id-token: write` 付き。rc.6 事故で既に付与済み)
で行う。PR CI や local で署名しない (identity が変わるため)。

### D2: provenance は cosign attest-blob + SLSA v0.2 predicate

slsa-github-generator の reusable workflow を使うと証明書 identity が
`slsa-framework/slsa-github-generator/...` になり platform の pin に一致しない。
よって `cosign attest-blob --type slsaprovenance --predicate <json> --bundle`
で自 workflow から attest する。predicate は python script
(`slsa_predicate.py`) で生成し、以下を保証する:

- `builder.id`: `https://github.com/Lamy210/nix_setting/.github/workflows/release.yml@refs/tags/<TAG>`
- `materials[].digest.sha1`: tag の commit SHA (platform は release の
  sourceRevision との一致を検証する)
- subject (name / sha256) は cosign が blob から自動設定する
  (name = asset 名と同一のため platform の subject 突合に一致)

### D3: SBOM は binary のみ syft (SPDX) で生成

配布 platform の規約は SPDX (`<artifact>.spdx.json`)。既存の sbomnix CycloneDX
(`sbom.cdx.json`) は nix closure の SBOM として通常 asset のまま維持する。
syft は UDIF (DMG) を scan できないため、DMG の SBOM は「未提供」とする
(platform は未検証として正直に表示。署名 + provenance は添付する)。

### D4: 生成直後の自己検証 gate

`cosign verify-blob` / `cosign verify-blob-attestation` を platform と同じ
`--certificate-identity-regexp '^https://github.com/Lamy210/nix_setting/.github/workflows/release\.yml@'`
+ `--certificate-oidc-issuer https://token.actions.githubusercontent.com` で
実行する。失敗したら release を作らない (誤った identity で署名した場合の
事故防止。platform 側検証は次回 catalog sync なので、release 時点で止める)。

### D5: bundle は CHECKSUMS.txt に含めない

CHECKSUMS.txt は配布 asset の digest 一覧 (install script / 人手検証用)。
bundle は証明書 + Rekor entry を含む自己検証可能な形式で、digest の仲介は
不要。platform も bundle を checksum 突合対象にしない (規約 asset は catalog
外のため)。

### D6: PR CI での検証 (drift 防止)

cosign 署名は OIDC 依存で PR CI では実行できないため、純粋 logic である
predicate 生成 (python) を bats で dummy 値検証する。shell script 全体は
shellcheck + actionlint (workflow 変更) で静的検証する。これは
generate-release-metadata.sh (release 専用だが生成部は PR CI で dummy 検証)
と同じ構図。

## リスク / トレードオフ

- cosign / syft は `nix run nixpkgs#cosign` / `nixpkgs#syft` (unstable channel)
  で version drift がある。sbomnix と同じ運用で許容
- Rekor / Fulcio への依存が増える (keyless 署名の本質)。cosign 異常時は
  release job が失敗し release が出ない (fail-fast を選ぶ)
