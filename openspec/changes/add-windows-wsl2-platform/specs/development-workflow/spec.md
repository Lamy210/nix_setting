## ADDED Requirements

### Requirement: Windows core and CLI portability have a pinned non-required CI gate

The PR workflow SHALL include Windows portability coverage pinned to the explicit `windows-2025` runner label. The Windows gate SHALL compile/test the supported Windows scope (core and CLI), run native launcher smoke coverage, and exercise hermetic WSL backend contract tests. This change MUST NOT add the Windows gate to the current server-side required status-check set.

#### Scenario: Windows code is validated on an explicit runner

- **WHEN** a pull request changes the repository
- **THEN** the Windows portability job runs on `windows-2025`
- **AND** it validates core/CLI Windows compilation and contract tests
- **AND** it does not depend on `windows-latest`

#### Scenario: existing branch protection remains unchanged

- **WHEN** the Windows portability job is introduced
- **THEN** the existing required contexts remain `openspec-check`, `flake-check`, `rust-check`, `lint`, `bootstrap-test`, `managed-nix-e2e`, and `release-artifact-check`
- **AND** Windows coverage is not a required context until a separate migration proves stability and is explicitly approved

### Requirement: Windows CI does not require a registered WSL distro for PR correctness

PR-gating Windows tests SHALL be hermetic with respect to WSL inventory/selection/helper behavior and MUST NOT assume that the hosted runner always has a registered usable Linux distribution. Real WSL2 distro execution MAY be observed in a separate canary, but such a canary MUST remain outside the current required aggregate and release dependencies.

#### Scenario: PR runner has no registered distro

- **WHEN** `windows-2025` provides WSL components but no usable registered WSL2 distro
- **THEN** the core/CLI Windows compile and hermetic backend contract tests can still pass or fail on repository behavior deterministically
- **AND** the PR is not blocked solely by absence of an externally registered distro

#### Scenario: real WSL canary fails

- **WHEN** a scheduled, `develop`, or manually dispatched real-WSL canary cannot provision or reach a WSL2 distro
- **THEN** the result is visible for compatibility tracking
- **AND** it does not fail the existing required `ci-required` aggregate or a release artifact gate

### Requirement: Windows support does not expand release scope in this change

The Windows/WSL2 platform change MUST NOT add Windows release artifacts, Windows Desktop bundles, Windows self-update assets, or a Windows shipping dependency to the existing release workflow. Release behavior for currently supported artifacts SHALL remain unchanged.

#### Scenario: tag release remains unchanged for Windows

- **WHEN** a release tag is processed after this change
- **THEN** no Windows artifact is required or published by this change
- **AND** the existing Linux/macOS release checks remain the shipping contract
