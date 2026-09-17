# Design: Windows host + WSL2 execution backend

## Context

SchneeForge's existing architecture intentionally centralizes operational behavior in `schneeforge-core`; CLI and Desktop are adapters. That architecture works because the host OS is also the execution OS: macOS operations run against Darwin/Nix and Linux operations run against Linux/Home Manager.

Windows breaks that assumption. The Windows process needs to provide a native entry point, but the supported Nix environment is Linux under WSL2. Existing code also has native-environment assumptions that must not be allowed to run on Windows before delegation:

- `detect_platform_for("windows")` is currently unsupported.
- `MachineFacts` maps only macOS/Linux into Nix system strings.
- CLI startup resolves repo/tool inventory before command dispatch.
- tool resolution returns absolute paths meaningful only inside its execution environment.
- default state/repo discovery uses Unix-style environment/layout semantics.
- managed Nix and privileged flows assume Linux/macOS permissions and `sudo`.

The key design constraint is therefore to separate **where the launcher runs** from **where SchneeForge's Nix workflow executes**.

## Goals

1. Provide an experimental Windows-native CLI launcher that delegates SchneeForge operations into WSL2.
2. Preserve the current macOS/Linux execution path and existing Nix semantics.
3. Make WSL2 selection, readiness, and helper compatibility explicit and testable.
4. Keep repository, state, Nix, Git, Unix permissions, and privilege semantics inside the WSL Linux environment.
5. Compile and test the relevant Rust crates on Windows without making the current branch-protection set depend on a new Windows runner.

## Non-Goals

- Windows Desktop/Tauri support.
- Native Windows Nix, winget, or Scoop execution backends.
- Automatic WSL/distro/helper installation.
- WSL1 support.
- Windows release packaging/self-update.
- Implicit Windows↔Linux project-path translation.

## Decision 1: Separate host platform from execution backend

Add a new execution-oriented module in core rather than extending the existing Nix-side `Platform` enum with Windows.

Conceptually:

```rust
pub enum HostPlatform {
    MacOS,
    Linux,
    Windows,
}

pub enum ExecutionBackend {
    Native,
    Wsl2 { distro: String },
}
```

`HostPlatform` answers where the current launcher process is running. `ExecutionBackend` answers where operational commands execute. Existing `discovery::Platform` and `ConfigurationTarget` continue to model the Nix execution side and therefore remain macOS/Linux only in this change.

This avoids producing invalid values such as `x86_64-windows` as a Nix system and minimizes churn in existing native paths.

## Decision 2: Delegate the whole CLI operation, not individual Nix/Git commands

On Windows, delegation happens at the CLI boundary before the current repo resolution, `ToolInventory::discover`, machine/state discovery, or command-specific core calls.

The Windows process:

1. parses enough global arguments to identify the command and optional `--wsl-distro`;
2. handles Windows-local commands such as help/version/backend diagnostics;
3. selects and validates `ExecutionBackend::Wsl2` for execution-requiring commands;
4. checks the Linux helper compatibility handshake;
5. invokes the Linux helper through `wsl.exe` with argv forwarding;
6. inherits/relays stdio and returns the helper's exit status.

The Linux helper then executes the existing normal Linux path. It resolves the repo, tools, state, Nix paths, locks, and privileges within Linux.

This is preferred over wrapping every Nix/Git subprocess from Windows because the latter would create two path/state/permission models inside core and would undermine the current single-source-of-truth execution logic.

## Decision 3: WSL inventory and distro selection fail closed

WSL discovery is a pure/testable core concern plus a thin process adapter.

Selection precedence:

1. global CLI `--wsl-distro <name>`
2. `SCHNEEFORGE_WSL_DISTRO`
3. WSL default distribution marked by `*`

The selected distro MUST exist and MUST report WSL version 2. WSL unavailable, no registered distro, unknown explicit distro, default distro absent, or selected WSL1 distro are structured precondition failures.

No automatic fallback to WSL1, native Windows execution, or a different arbitrary distro is allowed.

### Parsing robustness

`wsl.exe` output can vary in encoding/format across Windows/WSL versions. The implementation should keep output decoding and inventory parsing pure and fixture-driven:

- tolerate UTF-8 and UTF-16LE/NUL/BOM forms;
- use `wsl --list --quiet` as the source of distro names;
- use `wsl --list --verbose` for default marker/state/version;
- avoid assuming distro names contain no spaces;
- identify WSL version from the rightmost version field and associate verbose entries with names from the quiet inventory.

## Decision 4: Use a helper compatibility handshake

A Windows launcher must not blindly dispatch to an arbitrary `schneeforge` binary inside WSL.

Add an internal machine-readable helper probe (name may be implementation-specific, e.g. hidden `__backend-info`) that reports at least:

- bridge protocol version;
- application/package version;
- execution OS;
- execution architecture.

The Windows launcher requires:

- Linux execution OS;
- supported Linux architecture;
- matching bridge protocol version;
- matching application version for the initial implementation.

Missing helper, malformed probe output, protocol mismatch, or application-version mismatch is a hard precondition error with remediation text. Automatic helper installation/update is deliberately deferred.

The protocol version exists because package version alone is not a reliable compatibility boundary during development; the application-version equality requirement additionally prevents obvious mixed-release operation.

## Decision 5: Preserve argv/stdio/exit status; do not insert a shell

Delegation MUST build a process argv equivalent to:

```text
wsl.exe -d <distro> -- schneeforge <forwarded args...>
```

User-provided arguments are passed as process arguments rather than concatenated into `sh -c`, PowerShell, or another command string. This prevents quoting/injection problems and preserves arguments containing whitespace or shell metacharacters.

The launcher inherits or transparently relays stdin/stdout/stderr and returns the delegated process exit status. Spawn/transport errors remain launcher errors.

The Windows-only `--wsl-distro` selector is consumed by the launcher and is not forwarded to the Linux helper.

## Decision 6: Keep project/state ownership in WSL

For Windows-hosted execution:

- when `--repo` is omitted, the Linux helper performs existing default repo resolution inside WSL;
- when `--repo` is explicit, the initial implementation accepts an absolute Linux path intended for the WSL filesystem;
- drive-letter paths (`C:\...`), UNC paths, and other clearly Windows-native paths are rejected with guidance;
- no automatic `wslpath` conversion is performed;
- the launcher does not inject Windows `HOME`, Windows PATH, or Windows SchneeForge state into Linux execution.

The intent is to keep Linux build repositories/state on the WSL filesystem instead of creating a cross-filesystem default under `/mnt/c`.

## Decision 7: Windows doctor is host-aware; self-update is initially unsupported

`doctor` on a Windows host must be useful even when delegation cannot start. It reports at minimum:

- host platform;
- WSL availability/status;
- discovered distros and WSL versions;
- selected distro and selection source;
- helper presence/compatibility when probeable;
- actionable backend readiness errors.

When the backend is ready, doctor may also include delegated Linux diagnostics, but failure to reach the helper must still leave host-side diagnostic output available.

`self-update` on Windows is explicitly unsupported in this change. Delegating Linux self-update would update only the helper and could create launcher/helper skew, while no Windows release asset is being introduced. The command must fail with clear guidance rather than silently updating the wrong component.

## Decision 8: Windows portability fixes are limited to shared core/CLI

Initial Windows support compiles/tests `schneeforge-core` and CLI only. Desktop/Tauri Windows build is out of scope.

Required portability audit includes:

- replace manual PATH `:` splitting with `std::env::split_paths` / platform-safe APIs;
- ensure Unix-only permission/metadata code is correctly cfg-gated;
- keep Nix/sudo/Unix state semantics behind the Linux helper path;
- ensure tests use platform-neutral temporary paths and executable assumptions where applicable;
- ensure command construction does not depend on POSIX shells on the Windows side.

Unrelated Unix code does not need to be generalized if Windows never executes it and it is correctly cfg-gated.

## Decision 9: Add Windows CI as non-required coverage first

Add `windows-check` pinned to `windows-2025` to the PR workflow. It validates:

- Windows compilation for core/CLI;
- Windows-compatible unit tests;
- native launcher `--version`/basic smoke;
- hermetic WSL output-decoding, inventory, selection, helper-handshake, argv-forwarding, path-rejection, and fail-closed tests.

Do not add `windows-check` to existing branch protection in this change. Existing seven required contexts remain unchanged.

A separate real-WSL observation workflow/job may run on `develop`, schedule, or manual dispatch. It is non-required and MUST NOT be a release dependency. The canary must distinguish WSL package presence from a usable registered WSL2 distro; it may report/skip environmental absence rather than converting hosted-runner image variability into a PR blocker.

## Error handling

Backend failures should map to existing structured error concepts rather than panics/string-only control flow. Errors should identify the failed layer, e.g.:

- WSL executable unavailable;
- no distro registered;
- selected distro not found;
- selected distro is WSL1;
- helper not found;
- helper probe invalid;
- protocol/application version mismatch;
- explicit Windows path unsupported;
- delegated process spawn/transport failure.

A nonzero exit code from a successfully started Linux helper is the delegated command result, not a backend-discovery error.

## Testing strategy

### Pure/unit tests

- host detection;
- UTF-8/UTF-16LE WSL output decoding;
- quiet/verbose distro inventory correlation, including names with spaces;
- selector precedence and explicit invalid selectors;
- WSL1 rejection;
- helper handshake compatibility;
- Windows-path rejection and Linux-path acceptance;
- forwarded argv construction and stripping `--wsl-distro`;
- delegated exit-code mapping.

### Existing native regression

- existing Linux/macOS test suites and required CI remain green;
- native backend selection does not route through WSL.

### Windows CI

- `windows-2025` core/CLI compile/test;
- static workflow contract tests ensuring Windows coverage is non-required and isolated from release jobs;
- optional real WSL2 canary outside PR merge gates.

## Rollout

1. Land the host/backend model and pure tests.
2. Add WSL inventory/selection and helper handshake.
3. Add early Windows CLI dispatch and diagnostics.
4. Fix shared-code Windows compilation issues.
5. Add non-required Windows CI and observational WSL canary.
6. Verify no required context/release path changed and measure Windows job stability.

A later change can add helper bootstrap/distribution and Windows release assets once the backend contract is proven.

## Risks and mitigations

- **WSL output format variability** → separate decoding/parsing, fixture tests, avoid fixed-column/name token assumptions.
- **Argument injection/quoting bugs** → process argv only; no shell command concatenation.
- **Windows/WSL state divergence** → execution-side repo/state/tool discovery only.
- **helper skew** → explicit protocol + app-version handshake; fail closed.
- **cross-filesystem performance** → WSL filesystem is the default/recommended execution-side location; no implicit `/mnt/c` mapping.
- **CI runner WSL variability** → Windows compile/contracts are PR coverage; real WSL canary remains non-required.
- **scope explosion into full Windows product support** → Desktop, package managers, WSL bootstrap, release packaging, and self-update remain follow-ups.
