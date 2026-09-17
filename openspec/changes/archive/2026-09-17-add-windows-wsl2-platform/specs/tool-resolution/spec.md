## ADDED Requirements

### Requirement: Tool resolution occurs inside the selected execution environment

When SchneeForge uses a non-native execution backend, platform tool resolution SHALL occur inside that execution environment rather than resolving host-native paths that cannot be executed by the backend. For the Windows/WSL2 backend, the Windows launcher MUST delegate before `ToolInventory::discover` or equivalent Nix/Git tool resolution, and the Linux helper SHALL perform the existing Linux tool-resolution contract inside WSL.

#### Scenario: Windows launcher does not resolve Linux tools from Windows PATH

- **WHEN** an execution-requiring SchneeForge command runs on a Windows host with a valid WSL2 backend
- **THEN** the Windows launcher does not resolve Nix/Git/Home Manager absolute paths from the Windows environment
- **AND** the delegated Linux helper performs existing tool discovery inside WSL

#### Scenario: native hosts retain existing absolute-path resolution

- **WHEN** SchneeForge runs with the native backend on macOS or Linux
- **THEN** the existing `ToolInventory`/`Toolchain` resolution order and absolute-path execution contract remains unchanged

### Requirement: PATH enumeration uses platform-aware path-list parsing

Shared host-side tool lookup code SHALL parse PATH using platform-aware path-list semantics rather than hard-coding the POSIX `:` separator. Windows compilation/tests MUST cover this behavior.

#### Scenario: Windows-style PATH is not split on drive-letter colon

- **WHEN** tool lookup runs against a Windows-style PATH containing drive-letter paths
- **THEN** path entries are enumerated with the platform path-list parser
- **AND** the `C:` drive prefix is not treated as a POSIX PATH separator
