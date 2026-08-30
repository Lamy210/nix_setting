## ADDED Requirements

### Requirement: release asset への cosign attestation bundle 生成

release workflow は配布 artifact (CLI binary / DMG) ごとに、keyless (OIDC) の cosign 署名 bundle を `<asset 名>.sig.bundle` として生成し、release asset に添付しなければならない (MUST)。

署名は release workflow 自身の OIDC identity (`https://github.com/Lamy210/nix_setting/.github/workflows/release.yml@`) で行わなければならない (MUST)。

#### Scenario: tag push で署名 bundle が添付される

- **WHEN** release tag が push され release job が成功する
- **THEN** 各配布 artifact と同じ名前の `.sig.bundle` asset が release に含まれる

#### Scenario: 署名の identity は release workflow に pin される

- **WHEN** 生成した bundle を platform と同じ identity pin (`^https://github.com/Lamy210/nix_setting/.github/workflows/release\.yml@`) と OIDC issuer (`https://token.actions.githubusercontent.com`) で検証する
- **THEN** 検証は成功する (release job 内の自己検証 gate が通る)

#### Scenario: 署名に失敗した場合は release を作らない

- **WHEN** cosign 署名または自己検証が失敗する
- **THEN** release job は失敗し、release は作成されない

### Requirement: SLSA provenance bundle の生成

release workflow は各配布 artifact について SLSA v0.2 predicate による provenance bundle を `<asset 名>.provenance.bundle` として生成しなければならない (MUST)。

predicate は (MUST) builder identity が release workflow の ref であり、materials の sha1 が tag の commit SHA と一致すること。subject は cosign が blob から設定する (name = asset 名 / digest = asset の sha256)。

#### Scenario: predicate が source revision を含む

- **WHEN** `slsa_predicate.py <TAG> <SHA>` で predicate を生成する
- **THEN** `predicate.builder.id` は release workflow の ref を含み、`predicate.materials[0].digest.sha1` は `<SHA>` に一致する

#### Scenario: provenance の subject digest は artifact と一致する

- **WHEN** 生成した provenance bundle の subject を確認する
- **THEN** subject の sha256 は当該 artifact の sha256 と一致する

### Requirement: artifact ごとの SPDX SBOM 生成

release workflow は CLI binary ごとに syft で SPDX SBOM を `<asset 名>.spdx.json` として生成して release asset に添付しなければならない (MUST)。

DMG は対象外とする (SHOULD: scan 手段が確立したら追加する)。

#### Scenario: binary に SBOM が添付される

- **WHEN** release job が成功する
- **THEN** 各 CLI binary と同じ名前の `.spdx.json` asset が release に含まれる

### Requirement: PR CI での生成 logic 検証

predicate 生成 logic は PR CI (bats) で dummy 値による構造検証を行わなければならない (MUST)。

cosign 署名は OIDC 依存のため PR CI では実行しない。

#### Scenario: dummy 値で predicate が生成できる

- **WHEN** bats test が dummy tag / SHA で `slsa_predicate.py` を実行する
- **THEN** 出力は正当な JSON で、builder id / materials sha1 が入力と一致する
