## ADDED Requirements

### Requirement: read-only Nix upgrade inspection

SchneeForge SHALL provide a `schneeforge nix upgrade inspect` CLI command whose initial implementation performs **only read-only observations** and SHALL NOT run `nix upgrade-nix`, `nix-darwin switch`, `nix uninstall`, `nix install`, daemon restart or any destructive cleanup.

#### Scenario: user requests inspection
- **WHEN** a user invokes `schneeforge nix upgrade inspect` on a supported host
- **THEN** SchneeForge reports observed installation state, resolved Nix client version, daemon reachability, independently verified daemon version if available, ownership, and unknowns
- **AND** no Nix profile, receipt, ownership record, daemon, or system configuration is modified
- **AND** the command does not require elevated privileges or network access

#### Scenario: JSON output
- **WHEN** a user invokes `schneeforge nix upgrade inspect --json`
- **THEN** the CLI emits a schema-versioned machine-readable result with explicit uncertainty, safe display-only metadata and decision reasons
- **AND** it SHALL NOT serialize secrets, process environment, or full receipt payload

### Requirement: independently classified Nix client, daemon and installer versions

SchneeForge SHALL distinguish `nix-installer` provider version from installed `nix` CLI and running daemon versions, and MUST NOT infer one from another. Any daemon version without a verified probe SHALL be reported as unknown.

#### Scenario: provider bump does not prove runtime upgrade
- **WHEN** the embedded `bootstrap-manifest.toml` specifies nix-installer 2.35.3 but the host has an existing Nix installation
- **THEN** inspection MUST NOT report Nix 2.35.3, security remediation or daemon upgrade solely on the basis of the embedded installer version

#### Scenario: daemon is accessible but version cannot be verified
- **WHEN** a resolved Nix CLI successfully probes the daemon but the output format has not been verified as a stable daemon version source
- **THEN** daemon reachability is reported separately
- **AND** daemon version remains unknown rather than being copied from the CLI version

### Requirement: upgrade ownership and uncertainty remain explicit

SchneeForge SHALL record whether installer receipt and SchneeForge ownership are valid, missing, or unreadable; it SHALL NOT infer effective nix-darwin ownership from a repository declaration alone. Unknown, foreign, or mixed managers SHALL NOT be treated as eligible for automated upgrades.

#### Scenario: malformed receipt with ownership record
- **WHEN** an existing receipt path is present but unreadable or invalid
- **THEN** inspection classifies the receipt as invalid/unknown and does not classify the host as safe to update or as simply not installed
- **AND** ownership data remains unchanged

#### Scenario: nix-darwin is declared in source
- **WHEN** a checked-out repository has `nix.enable = true` but active host ownership cannot be independently verified
- **THEN** inspection treats the declaration as configuration evidence only
- **AND** does not claim the host is safe to mutate using `nix upgrade-nix`

#### Scenario: unresolved or conflicting owner
- **WHEN** the host's actual Nix manager is not reliably identifiable, or ownership and receipt conflict
- **THEN** inspection returns an explicit unknown/manual-review route
- **AND** no privileged upgrade is attempted

### Requirement: inspect failures are bounded and fail closed

SchneeForge SHALL run observational subprocesses through the OS-native resolved binary path with fixed arguments, a finite timeout and bounded output. A failed observation MUST NOT be silently converted into a healthy or patched state.

#### Scenario: tool resolver cannot find Nix
- **WHEN** no resolved Nix binary is available but Nix installation markers remain
- **THEN** inspection records an inconsistent or unknown state rather than declaring a fully absent installation

#### Scenario: daemon query times out or errors
- **WHEN** the daemon probe times out, exits non-zero, or returns an unrecognized format
- **THEN** inspection includes a diagnostic error/unknown reason
- **AND** does not report the daemon as patched or approve an automatic update
