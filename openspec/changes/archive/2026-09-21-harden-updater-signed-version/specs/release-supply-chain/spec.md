## ADDED Requirements

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
