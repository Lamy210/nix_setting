## ADDED Requirements

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
