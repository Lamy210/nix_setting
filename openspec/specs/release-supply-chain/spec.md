# release-supply-chain Specification

## Purpose
release workflow が配布 artifact ごとに cosign 署名 / SLSA provenance / SPDX SBOM の attestation bundle を生成し、release workflow 自身の OIDC identity に pin した自己検証 gate を通して release asset に添付する生産者側の規約。Schnee Software Distribution platform の検証規約 (docs/security/attestations.md) と対になる。

## Requirements

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

### Requirement: Tauri updater artifact signing for activated macOS releases

When macOS GUI updater activation is enabled for a release, the release pipeline MUST produce a Tauri updater artifact signed by the configured long-lived updater key and MUST publish a static latest.json that references the signature content.

#### Scenario: Activated release has signing material
- **WHEN** a tag release is configured to ship the macOS updater
- **THEN** the build uses the configured Tauri signing private key
- **AND** the updater artifact and its signature are produced from the same source/release unit as the DMG
- **AND** latest.json contains a complete `darwin-aarch64` entry

#### Scenario: Activated release lacks signing material
- **WHEN** updater activation is enabled but required signing material is unavailable
- **THEN** the release fails before publishing an updater manifest
- **AND** no unsigned updater artifact is advertised

### Requirement: Static updater manifest generation

The repository SHALL generate latest.json from validated release inputs rather than hand-editing JSON in the release workflow.

#### Scenario: Manifest is generated
- **WHEN** generator receives a valid version, HTTPS updater artifact URL, and signature content
- **THEN** it emits deterministic JSON containing `version` and `platforms.darwin-aarch64.{url,signature}`

#### Scenario: Invalid updater manifest input
- **WHEN** version, URL, or signature is invalid or missing
- **THEN** generator exits non-zero and no valid latest.json is produced

### Requirement: Production updater activation gate

Production updater trust-root activation MUST NOT occur before macOS Final Acceptance and production key provisioning are complete.

#### Scenario: Implementation is merged before activation prerequisites
- **WHEN** updater code/generator exists but Final Acceptance or production key provisioning is incomplete
- **THEN** ordinary PR/develop builds remain updater-disabled
- **AND** the release pipeline does not advertise a production updater endpoint using placeholder/test signing material

### Requirement: Version-bound Tauri updater signatures

Activated macOS updater artifacts MUST be generated with a Tauri toolchain that records the application version
inside the updater signature trusted comment. Release generation MUST NOT use a pre-version-binding Tauri CLI.

#### Scenario: Updater artifact is produced for activated release
- **WHEN** release workflow builds a signed macOS updater artifact
- **THEN** the pinned Tauri CLI is version 2.11.5 or newer approved version with signed-version support
- **AND** the generated signature binds the release app version to the artifact
- **AND** latest.json announces the same release version

#### Scenario: Build tooling regresses below signed-version support
- **WHEN** release build pin is changed to a Tauri CLI that does not produce version-bound updater signatures
- **THEN** repository contract tests fail before release
- **AND** production updater activation is blocked
