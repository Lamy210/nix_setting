## ADDED Requirements

### Requirement: macOS GUI signed self-update

SchneeForge Desktop SHALL provide an in-app update path on supported macOS aarch64 builds using the Tauri updater signature verification model. The updater action MUST NOT be exposed when the build is not configured with an active production updater trust root.

#### Scenario: Signed update is available
- **WHEN** a macOS aarch64 build with updater enabled checks the configured endpoint and a newer signed update is available
- **THEN** Dashboard exposes an app update action with the available version
- **AND** installation occurs only after the user explicitly confirms the update
- **AND** download/install progress is visible

#### Scenario: Update check or install fails
- **WHEN** network, manifest, signature verification, download, or install fails
- **THEN** the currently installed app remains usable and unchanged
- **AND** the GitHub Releases link fallback remains available
- **AND** the error is shown without converting the failure into a successful update state

#### Scenario: Updater is disabled or unsupported
- **WHEN** the app runs on Linux, Windows, or a build without production updater activation
- **THEN** automatic app update controls are not shown
- **AND** the existing release/update guidance remains available

#### Scenario: Update install completes on macOS
- **WHEN** the signed update is installed successfully
- **THEN** the user can restart SchneeForge to run the new version
- **AND** no downgrade is performed automatically

### Requirement: GUI updater IPC contract

Desktop backend SHALL own updater check/install state and expose stable Tauri commands to the frontend. Pending update state MUST NOT be reconstructed from untrusted frontend-supplied URL or signature data.

#### Scenario: Frontend checks for update
- **WHEN** frontend invokes the updater check command
- **THEN** backend returns only serializable update metadata and stores the pending signed update internally

#### Scenario: Frontend installs pending update
- **WHEN** frontend invokes install after confirmation
- **THEN** backend installs only the internally stored pending update
- **AND** frontend does not provide the artifact URL or signature used for verification
