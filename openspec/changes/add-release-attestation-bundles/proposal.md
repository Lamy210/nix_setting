# Proposal: add-release-attestation-bundles

## Why

Schnee 配信 platform (SchneeSoftwareDistribution) が artifact ごとの供給保証を
検証・表示するために、命名規約に従った attestation asset の添付を producer 側
に求めている (docs/security/attestations.md §2。platfrom change:
exclude-attestation-assets で規約 asset は配布 artifact から除外される):

| attestation | asset 名 | 生成 |
|---|---|---|
| cosign 署名 | `<artifact>.sig.bundle` | `cosign sign-blob --bundle` |
| SLSA provenance | `<artifact>.provenance.bundle` | `cosign attest-blob --bundle` |
| SBOM | `<artifact>.spdx.json` | syft (spdx-json) |

現状の release は GitHub native attestation (`actions/attest-build-provenance`)
のみで、cosign bundle 形式の asset は無い。そのため platform 側の identity pin
付き検証 (`https://github.com/Lamy210/nix_setting/.github/workflows/release.yml@`)
も download page の署名 / Provenance / SBOM 表示も「未検証」のままになる。

## What Changes

- release job で配布 artifact (CLI binary ×2 / DMG) ごとに:
  - keyless cosign 署名 bundle (`.sig.bundle`) を生成
  - SLSA v0.2 predicate (subject = 当該 artifact / materials = source commit /
    builder = release workflow) による provenance bundle (`.provenance.bundle`)
  - syft による SPDX SBOM (`.spdx.json`) — binary のみ (DMG は署名 + provenance のみ)
- 生成直後に platform と同じ identity pin + oidc issuer で自己検証する gate を
  かける (失敗時は release 作成中止)
- 生成 logic は `scripts/ci/` に script 化 (shellcheck 対象)。predicate 生成は
  python に分離し PR CI (bats) で dummy 値の生成検証を行う
- release asset として添付 (`attestations/*` を files に追加)。CHECKSUMS.txt の
  対象は従来どおり配布 asset のみ (bundle は自己検証可能な形式のため)

## 非対象

- GitHub native attestation (`actions/attest-build-provenance`) の変更。人手検証
  (`gh attestation verify`) 用にそのまま維持する
- 過去 release への遡及生成 (1 release = 1 source tree 原則)
- macOS notarization (別要件)

## Impact

- `.github/workflows/release.yml` (release job に生成 step 追加)
- `.github/workflows/check.yml` (shellcheck 対象 list に script 追加)
- `scripts/ci/attest-release-assets.sh` / `scripts/ci/slsa_predicate.py` (新規)
- `tests/ci-scripts.bats` (predicate 生成の検証)
- `RELEASE.md` (checklist に attestation bundle 項目)
