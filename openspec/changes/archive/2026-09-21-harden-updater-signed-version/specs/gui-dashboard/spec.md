## ADDED Requirements

### Requirement: GUI updater binds signed artifact version to announced version

Activated macOS GUI updater builds MUST require the updater signature to carry the app version and MUST reject
an update when the signed version differs from the version announced by the updater endpoint.

#### Scenario: Signed version matches announced version
- **WHEN** updater endpoint announces version N and the downloaded artifact signature is bound to version N
- **THEN** signature/version validation may proceed to the existing update installation flow

#### Scenario: Signed version is missing
- **WHEN** updater signature does not contain a signed app version
- **THEN** the update is rejected
- **AND** the existing app remains unchanged
- **AND** the GitHub Releases fallback remains available

#### Scenario: Signed version differs from announced version
- **WHEN** updater endpoint announces version N but the valid artifact signature is bound to version M
- **THEN** the update is rejected as a signed-version mismatch
- **AND** no downgrade or replacement occurs
