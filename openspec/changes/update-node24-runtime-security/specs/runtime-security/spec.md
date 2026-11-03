## ADDED Requirements

### Requirement: Secure and consistent Node.js 24 development runtimes

All supported SchneeForge Nix development entry points that provide Node.js 24 **SHALL** use a reproducibly pinned Node.js 24 build at or above the approved security floor and **MUST** independently verify the security-critical Brotli and c-ares runtime closure before release.

#### Scenario: Root Nix devShell provides patched Node

- **WHEN** a supported host enters the root Node devShell
- **THEN** the executed `node` binary SHALL meet the approved Node 24 version floor and its verified Brotli/c-ares security closure SHALL be recorded

#### Scenario: Standalone templates resolve independently

- **WHEN** a supported host enters either the standalone Node or devenv template shell
- **THEN** that template's own committed immutable inputs (lockfile or exact NAR-hash pin) SHALL resolve to the same security floor and SHALL pass actual executable / dependency verification independently of root `flake.lock`

#### Scenario: Unlocked devenv template or skipped execution gate

- **WHEN** the standalone devenv template lacks an immutable input pin or its CI runs only flake metadata without executing `node`
- **THEN** the security acceptance gate SHALL fail and the release SHALL remain blocked

#### Scenario: A version-only or unverified closure update

- **WHEN** a package version string meets the approved floor but a runtime dependency is unpatched or unverified
- **THEN** CI SHALL fail the security acceptance gate and the release SHALL remain blocked, including when a statically bundled library is not attributable to a reviewed patched source/patchset

### Requirement: Node security remediation remains scoped and reversible

The project **MUST** keep security remediation changes isolated from broad unverified Nixpkgs upgrades and **SHALL NOT** assert that an existing host is patched before the installed/active runtime and migration outcome are verified.

#### Scenario: Narrow source pin introduced

- **WHEN** a new Node-only Nixpkgs input or package overlay is proposed
- **THEN** its immutable source SHA/hash, closure and ABI risk, and rollback procedure SHALL be reviewed before implementation is accepted

#### Scenario: Host has a separate Node installation

- **WHEN** a host uses an active Node binary from `mise` or another source outside the updated Nix shell
- **THEN** the project SHALL report the host's remediation state as unknown until that specific runtime is inventoried and successfully activated with rollback evidence
