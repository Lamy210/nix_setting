# Change: add Windows host support through a WSL2 execution backend

## Why

SchneeForge currently treats the process OS and the Nix execution environment as the same platform. `Platform` and `MachineFacts` support macOS/Linux only, the CLI resolves the repository and tool inventory before dispatch, and multiple core paths assume Linux/macOS semantics such as `HOME`, Unix permissions, `sudo`, and Nix absolute paths. Adding `Platform::Windows` directly would therefore make Windows appear to be a Nix system even though the intended execution environment is Linux under WSL2.

Windows should instead be a host/control-plane platform while the existing Linux SchneeForge runtime remains the execution environment. Microsoft supports invoking Linux commands from Windows through `wsl.exe`, exposes WSL1/WSL2 state through `wsl --list --verbose`, and recommends keeping Linux-build repositories in the WSL filesystem rather than `/mnt/c` for performance. GitHub's current `windows-2025` runner image includes the WSL components needed for Windows portability validation, but a registered Linux distribution is not treated as a stable PR-runner contract.

## What Changes

- Introduce a host/execution split:
  - `HostPlatform::{MacOS, Linux, Windows}` describes where the launcher runs.
  - `ExecutionBackend::{Native, Wsl2 { distro }}` describes where SchneeForge operations execute.
  - Existing Nix-side `Platform`/`ConfigurationTarget` remain macOS/Linux concepts; Windows MUST NOT become a Nix system string.
- Keep macOS/Linux behavior native and unchanged.
- On Windows, select and validate a WSL2 distribution before any repository resolution, tool discovery, state access, or Nix operation occurs on the Windows host.
- Delegate execution-requiring CLI commands as an argv-preserving `wsl.exe -d <distro> -- schneeforge ...` invocation to a Linux SchneeForge helper inside WSL2.
- Add a helper compatibility handshake so the Windows launcher fails closed when the Linux helper is missing or incompatible.
- Add explicit Windows backend diagnostics. WSL unavailable, no distro, WSL1-only distro, helper missing, helper protocol/version mismatch, or Windows-host repository paths produce actionable precondition errors rather than a native fallback.
- Add `--wsl-distro` plus `SCHNEEFORGE_WSL_DISTRO`, with precedence `CLI > env > WSL default`.
- Keep explicit repositories on the execution side: initial Windows support accepts absolute Linux paths inside WSL and rejects Windows drive/UNC paths instead of automatically translating them.
- Make platform-neutral code compile on Windows, including replacing hard-coded PATH `:` splitting with `std::env::split_paths` and auditing Unix-only assumptions.
- Add a non-required `windows-check` on pinned `windows-2025` for Windows native core/CLI compile/tests and hermetic WSL contract tests. Keep any real WSL2 distro canary outside current required contexts and release dependencies.

## Non-Goals

- Windows Desktop/Tauri GUI support.
- Native Windows package-manager backends such as winget or Scoop.
- Treating Windows as a Nix `system` or running Nix directly on the Windows filesystem.
- Automatically installing WSL, a Linux distribution, or the Linux SchneeForge helper.
- Supporting WSL1.
- Automatically translating `C:\...`, UNC, or `/mnt/c/...` repository locations.
- Publishing a Windows release asset or implementing Windows self-update in this change.
- Managing multiple WSL distributions concurrently in one invocation.

## Impact

- **New capability**: `platform-execution` — host detection, execution-backend selection, WSL2 inventory/selection, helper compatibility, and command delegation.
- **Modified capability**: `tool-resolution` — Windows/WSL2 delegation happens before native repository/tool discovery; tool resolution remains inside the Linux execution environment.
- **Modified capability**: `development-workflow` — add Windows compile/contract coverage without changing the current required branch-protection contexts.
- Likely implementation areas: `crates/core/src/execution.rs` (new), `crates/core/src/discovery.rs`, `crates/cli/src/main.rs`, CLI argument definitions/tests, `.github/workflows/check.yml`, optional Windows WSL canary workflow, and `tests/ci-scripts.bats`.

## External Evidence

- Microsoft WSL interop: Windows can invoke Linux commands through `wsl.exe`, including a selected distribution.
- Microsoft WSL basic commands: `wsl --list --verbose` reports registered distributions and WSL version.
- Microsoft filesystem guidance: Linux build repositories should live in the WSL filesystem for best performance.
- GitHub runner-images: `windows-2025` is the current GA x64 Windows image and includes WSL components; PR validation MUST NOT assume a registered Linux distro is always present.

## Success Criteria

- macOS/Linux behavior remains unchanged and existing required checks stay green.
- The core and CLI compile/test on `windows-2025`.
- A Windows launcher can deterministically choose a WSL2 backend and reject invalid/missing WSL states with structured errors.
- Execution-requiring commands are delegated before Windows-side repo/tool/state logic and preserve argv, stdio, and exit status without shell interpolation.
- The Linux helper compatibility contract is checked before delegated operations.
- No Windows host path is silently converted into an execution-side project path.
- Existing branch-protection required contexts remain unchanged until a separate migration is explicitly approved.
