## Purpose

Define the Windows launcher and WSL2 Linux execution boundary for SchneeForge so Windows host detection, deterministic WSL2 selection, helper compatibility checks, argv-preserving delegation, and execution-side repository/tool/state ownership remain explicit and fail-closed without treating Windows as a Nix system.

## ADDED Requirements

### Requirement: Host platform and execution backend are separate concerns

SchneeForge SHALL model the launcher host separately from the environment in which operational commands execute. The host model SHALL distinguish macOS, Linux, and Windows. The execution backend SHALL distinguish native execution from WSL2 execution with a selected distribution. Existing Nix-side platform/configuration concepts SHALL remain macOS/Linux concepts; Windows MUST NOT be emitted as a Nix `system` value.

#### Scenario: macOS and Linux keep native execution

- **WHEN** SchneeForge runs on macOS or Linux
- **THEN** the selected execution backend is native
- **AND** existing native repository, tool, state, and Nix behavior remains unchanged

#### Scenario: Windows selects a WSL2 backend

- **WHEN** SchneeForge runs on Windows for an execution-requiring command
- **THEN** the host platform is Windows
- **AND** execution occurs through a selected WSL2 Linux distribution
- **AND** Windows is not converted into a Nix platform/system string

### Requirement: WSL2 distribution selection is deterministic and fail-closed

On Windows, SchneeForge MUST select the WSL distribution using the precedence `--wsl-distro` > `SCHNEEFORGE_WSL_DISTRO` > WSL default distribution. The selected distribution MUST exist and MUST use WSL version 2. WSL unavailable, no registered distribution, an unknown explicit distribution, no usable default, or a WSL1-only selected distribution SHALL be a structured precondition failure. SchneeForge MUST NOT silently fall back to WSL1, native Windows execution, or a different arbitrary distribution.

#### Scenario: CLI selector overrides all other sources

- **WHEN** `--wsl-distro Ubuntu` is supplied and an environment/default distro also exists
- **THEN** `Ubuntu` is selected
- **AND** the other selection sources do not override it

#### Scenario: environment selector is used without CLI override

- **WHEN** `--wsl-distro` is absent
- **AND** `SCHNEEFORGE_WSL_DISTRO=Debian` is set
- **THEN** `Debian` is selected if it is a registered WSL2 distro

#### Scenario: WSL default is used as the final selection source

- **WHEN** neither the CLI nor environment selects a distro
- **AND** WSL reports a default WSL2 distro
- **THEN** that default distro is selected

#### Scenario: WSL1 is rejected

- **WHEN** the selected registered distribution reports WSL version 1
- **THEN** backend selection fails with an actionable precondition error
- **AND** the command is not delegated

### Requirement: WSL inventory parsing tolerates Windows output forms

SchneeForge MUST decode and parse WSL inventory output without depending on a single text encoding or fixed whitespace columns. The parser SHALL support UTF-8 and UTF-16LE/NUL/BOM forms used by `wsl.exe`, SHALL use quiet inventory names to preserve distro names containing spaces, and SHALL correlate verbose output to determine default marker and WSL version.

#### Scenario: UTF-16LE inventory is decoded

- **WHEN** `wsl.exe` returns a UTF-16LE/NUL-delimited distro listing
- **THEN** SchneeForge decodes the distro names and version metadata correctly

#### Scenario: distro name contains spaces

- **WHEN** a registered distro name contains spaces
- **THEN** inventory parsing preserves the complete distro name
- **AND** selection does not depend on naïvely splitting the distro name by whitespace

### Requirement: Windows delegates before execution-side discovery

For an execution-requiring Windows command, SchneeForge MUST choose/validate the WSL2 backend and delegate before Windows-side repository resolution, tool inventory discovery, machine/state discovery, Nix discovery, or operation-specific core execution. The delegated Linux SchneeForge process SHALL perform those existing operations inside WSL.

#### Scenario: Windows apply does not resolve Nix on Windows

- **WHEN** a Windows user runs an execution-requiring command such as `apply`
- **THEN** the Windows launcher validates the WSL2 backend first
- **AND** it does not attempt to resolve a native Windows Nix executable or Linux state path before delegation
- **AND** the Linux helper performs normal Linux repo/tool/state/Nix resolution

### Requirement: Delegation preserves process arguments and status without a shell

The Windows launcher MUST invoke `wsl.exe` with process arguments equivalent to `-d <distro> -- schneeforge <args...>`. User-provided arguments MUST be forwarded as argv entries rather than concatenated into `sh -c`, PowerShell, `cmd /c`, or another command string. The launcher SHALL inherit or transparently relay stdin/stdout/stderr and SHALL return the delegated process exit status. The launcher-only `--wsl-distro` selector MUST NOT be forwarded to the Linux helper.

#### Scenario: shell metacharacters remain literal arguments

- **WHEN** a forwarded argument contains whitespace or shell metacharacters
- **THEN** that value is passed as one process argument to the Linux helper
- **AND** no intermediate shell interprets the value

#### Scenario: helper nonzero status is preserved

- **WHEN** the Linux helper starts successfully and exits nonzero
- **THEN** the Windows launcher exits with the helper's command result status
- **AND** the result is not rewritten as a backend-discovery error

### Requirement: WSL helper compatibility is verified before operations

The Linux helper used by the Windows launcher MUST expose a machine-readable compatibility probe containing at least bridge protocol version, application version, execution OS, and architecture. Before an execution-requiring command is delegated, the Windows launcher MUST verify that the helper is present, reports Linux on a supported architecture, has the same bridge protocol version, and has the same application version. Missing, malformed, or incompatible helpers SHALL fail closed with remediation guidance. This change SHALL NOT automatically install or update the helper.

#### Scenario: compatible helper is accepted

- **WHEN** the selected WSL2 distro contains a helper with matching protocol/application versions and supported Linux identity
- **THEN** the backend is considered ready for delegation

#### Scenario: helper version mismatch is rejected

- **WHEN** the helper's bridge protocol or application version differs from the Windows launcher
- **THEN** SchneeForge reports an incompatibility precondition error
- **AND** the requested operation does not execute

### Requirement: Windows-hosted repositories and state stay execution-side

For WSL2 execution, omitted repository selection SHALL be resolved by the Linux helper inside WSL. An explicit Windows-launcher `--repo` path SHALL be accepted only when it is an absolute Linux execution-side path in the initial implementation. Clearly Windows-native drive-letter or UNC paths MUST be rejected with actionable guidance. SchneeForge MUST NOT implicitly translate a Windows project path into `/mnt/<drive>` or inject Windows `HOME`, PATH, or SchneeForge state as the Linux execution defaults.

#### Scenario: default repository is resolved in WSL

- **WHEN** a Windows user omits `--repo`
- **THEN** the Linux helper performs the existing default repository resolution inside the selected WSL distro

#### Scenario: Windows drive path is rejected

- **WHEN** a Windows user supplies `--repo C:\\Users\\alice\\project`
- **THEN** the launcher rejects the path before delegation
- **AND** explains that the initial WSL2 backend expects a Linux path in the WSL filesystem

### Requirement: Windows doctor exposes backend readiness

On Windows, `doctor` SHALL provide host-side WSL backend diagnostics even if normal delegation is not ready. It SHALL report WSL availability, discovered distros and WSL versions, selected distro/selection source, and helper compatibility when probeable. A backend failure SHALL remain visible as actionable diagnostics rather than only returning the previous generic unsupported-platform behavior.

#### Scenario: doctor works when helper is missing

- **WHEN** WSL2 is available but the Linux SchneeForge helper is missing
- **THEN** `doctor` reports the Windows/WSL state and the missing-helper remediation
- **AND** host diagnostics remain available without requiring a successful delegated helper invocation

### Requirement: Windows self-update is explicitly unsupported in the initial backend

Until a Windows release artifact and coordinated launcher/helper update mechanism exist, Windows `self-update` MUST fail with an explicit unsupported/precondition message. It MUST NOT delegate Linux self-update in a way that updates only the WSL helper.

#### Scenario: Windows self-update does not update only the helper

- **WHEN** a Windows user invokes `self-update`
- **THEN** SchneeForge does not delegate the existing Linux self-update operation
- **AND** explains that coordinated Windows launcher/helper distribution is a follow-up capability
