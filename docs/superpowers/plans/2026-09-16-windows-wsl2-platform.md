# Windows WSL2 Platform Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an experimental Windows-native SchneeForge CLI launcher that delegates operational commands to a compatible Linux SchneeForge helper in a selected WSL2 distro without treating Windows as a Nix system.

**Architecture:** Keep existing `discovery::Platform` / `ConfigurationTarget` macOS/Linux-only. Add a focused `execution` core module for launcher-host detection, WSL inventory/selection, helper compatibility, argv construction, path validation, and delegated exit mapping. On Windows the CLI branches before repo resolution/tool discovery/state access; native macOS/Linux startup remains unchanged.

**Tech Stack:** Rust 2021, clap, serde/serde_json, std::process, GitHub Actions, Bats/OpenSpec.

**Spec:** `openspec/changes/add-windows-wsl2-platform/design.md` plus delta specs under `openspec/changes/add-windows-wsl2-platform/specs/`.

## Global Constraints

- Windows MUST remain a launcher host only; existing Nix-side `Platform` and Nix `system` strings remain macOS/Linux only.
- WSL selection precedence MUST be `--wsl-distro` > `SCHNEEFORGE_WSL_DISTRO` > WSL default distro.
- Selected distro MUST exist and MUST be WSL2; no WSL1/native-Windows/arbitrary-distro fallback.
- Delegation MUST use process argv (`wsl.exe -d <distro> -- schneeforge ...`) and MUST NOT concatenate user input into shell command strings.
- Helper compatibility MUST fail closed on missing/malformed helper, protocol mismatch, app-version mismatch, non-Linux OS, or unsupported Linux architecture.
- Windows `--repo` accepts only absolute Linux paths; drive-letter/UNC Windows paths are rejected and no implicit `wslpath` conversion is performed.
- Windows self-update, Desktop/Tauri, native Windows Nix/package managers, WSL auto-install, helper auto-install/update, and Windows release assets remain out of scope.
- New Windows CI coverage is non-required and MUST NOT alter the current seven required contexts or release dependencies.
- Every production change follows RED -> minimal GREEN -> focused regression verification before the next task.

---

### Task 1: Host/execution model and platform-safe PATH discovery

**Files:**
- Create: `crates/core/src/execution.rs`
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/core/src/discovery.rs`

**Interfaces:**
- Produces: `HostPlatform::{MacOS, Linux, Windows, Unsupported}`
- Produces: `ExecutionBackend::{Native, Wsl2 { distro: String }}`
- Produces: `detect_host_platform_for(os: &str) -> HostPlatform`
- Existing `discovery::Platform` remains unchanged and Windows remains `Platform::Unsupported`.

- [ ] **Step 1: Write failing host-model and PATH tests**

Add tests in `execution.rs` and `discovery.rs` equivalent to:

```rust
#[test]
fn host_platform_distinguishes_windows_from_nix_platform() {
    assert_eq!(detect_host_platform_for("windows"), HostPlatform::Windows);
    assert_eq!(crate::detect_platform_for("windows"), crate::Platform::Unsupported);
}

#[test]
fn windows_host_selects_no_native_nix_target() {
    assert_eq!(crate::detect_target_for("windows", "x86_64").name(), "unsupported");
}
```

Refactor `which` through a pure helper so Windows PATH semantics can be fixture-tested with `std::env::split_paths` rather than `split(':')`.

- [ ] **Step 2: Run focused tests and verify RED**

Run: `cargo test -p schneeforge-core execution::tests discovery::tests`

Expected: FAIL because `execution` types/functions do not exist and the old PATH implementation does not use platform-safe splitting.

- [ ] **Step 3: Implement minimal host/backend model and PATH fix**

`execution.rs` starts with:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostPlatform {
    MacOS,
    Linux,
    Windows,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionBackend {
    Native,
    Wsl2 { distro: String },
}

pub fn detect_host_platform_for(os: &str) -> HostPlatform {
    match os {
        "macos" => HostPlatform::MacOS,
        "linux" => HostPlatform::Linux,
        "windows" => HostPlatform::Windows,
        _ => HostPlatform::Unsupported,
    }
}
```

Change PATH parsing to `std::env::split_paths` / a pure `find_executable_in_path` helper while preserving the existing executable-resolution contract.

- [ ] **Step 4: Run focused and core tests**

Run: `cargo test -p schneeforge-core execution::tests discovery::tests && cargo test -p schneeforge-core`

Expected: PASS.

- [ ] **Step 5: Commit**

Commit message: `refactor(core): separate host and execution platform`

---

### Task 2: WSL output decoding, inventory, and deterministic WSL2 selection

**Files:**
- Modify: `crates/core/src/execution.rs`
- Modify: `crates/core/src/lib.rs`
- Test: inline unit tests in `execution.rs`

**Interfaces:**
- Produces: `WslDistro { name: String, is_default: bool, state: Option<String>, version: Option<u8> }`
- Produces: `WslInventory { distros: Vec<WslDistro> }`
- Produces: `WslSelectionSource::{Cli, Environment, Default}`
- Produces: `SelectedWsl { distro: String, source: WslSelectionSource }`
- Produces: `decode_wsl_output(bytes: &[u8]) -> Result<String>`
- Produces: `parse_wsl_inventory(quiet: &str, verbose: &str) -> Result<WslInventory>`
- Produces: `select_wsl2(inventory: &WslInventory, cli: Option<&str>, env: Option<&str>) -> Result<SelectedWsl>`

- [ ] **Step 1: Write failing decoding/inventory/selection tests**

Cover at minimum:

```rust
#[test]
fn decodes_utf16le_with_bom_and_nuls() { /* Ubuntu output */ }

#[test]
fn parses_default_wsl2_and_name_with_spaces() { /* quiet + verbose fixtures */ }

#[test]
fn selection_precedence_is_cli_then_env_then_default() { /* exact assertions */ }

#[test]
fn explicit_unknown_distro_fails_closed() { /* Error::Precondition */ }

#[test]
fn selected_wsl1_fails_closed() { /* version == 1 */ }

#[test]
fn missing_default_without_explicit_selector_fails_closed() { /* no '*' */ }
```

- [ ] **Step 2: Run focused tests and verify RED**

Run: `cargo test -p schneeforge-core execution::tests`

Expected: FAIL on missing APIs.

- [ ] **Step 3: Implement decoder and parsers as pure functions**

Decoder behavior:
- detect UTF-16LE BOM or NUL-heavy byte streams and decode 16-bit little-endian units;
- otherwise decode UTF-8 losslessly when valid and return a structured precondition/command error for invalid output;
- normalize CRLF but preserve distro names.

Inventory parser behavior:
- `quiet` is authoritative for distro names;
- `verbose` supplies default marker/state/rightmost numeric WSL version;
- never tokenize a distro name by assuming a single word.

Selection behavior:
- choose CLI selector first, then env selector, then default marker;
- chosen distro must exist and have `version == Some(2)`.

- [ ] **Step 4: Run execution tests and full core regression**

Run: `cargo test -p schneeforge-core execution::tests && cargo test -p schneeforge-core`

Expected: PASS.

- [ ] **Step 5: Commit**

Commit message: `feat(core): add deterministic WSL2 selection`

---

### Task 3: Helper handshake, repo validation, and shell-free argv construction

**Files:**
- Modify: `crates/core/src/execution.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Produces: `BACKEND_PROTOCOL_VERSION: u32 = 1`
- Produces: serde `BackendInfo { protocol_version: u32, app_version: String, os: String, arch: String }`
- Produces: `validate_backend_info(info: &BackendInfo, expected_app_version: &str) -> Result<()>`
- Produces: `validate_wsl_repo_path(repo: &str) -> Result<()>`
- Produces: `build_wsl_argv(distro: &str, forwarded: &[String]) -> Vec<String>`
- Produces: `DelegatedExit::{Success, Code(i32)}` or equivalent explicit mapping used by CLI.

- [ ] **Step 1: Write failing contract tests**

Tests must prove:

```rust
#[test]
fn helper_requires_protocol_and_app_version_match() { /* mismatch => Precondition */ }

#[test]
fn helper_requires_linux_and_supported_arch() { /* windows/riscv64 => failure */ }

#[test]
fn repo_accepts_absolute_linux_path_only() {
    assert!(validate_wsl_repo_path("/home/alice/nix_setting").is_ok());
    assert!(validate_wsl_repo_path(r"C:\\src\\nix_setting").is_err());
    assert!(validate_wsl_repo_path(r"\\server\\share").is_err());
    assert!(validate_wsl_repo_path("relative/path").is_err());
}

#[test]
fn wsl_argv_preserves_metacharacters_as_arguments() {
    let args = build_wsl_argv("Ubuntu Dev", &["apply".into(), "a b;$(x)".into()]);
    assert_eq!(args, vec!["-d", "Ubuntu Dev", "--", "schneeforge", "apply", "a b;$(x)"]);
}
```

- [ ] **Step 2: Run focused tests and verify RED**

Run: `cargo test -p schneeforge-core execution::tests`

Expected: FAIL on missing handshake/path/argv APIs.

- [ ] **Step 3: Implement minimal contract functions**

Use serde derives already available in core. `validate_backend_info` must require protocol equality, package-version equality, `os == "linux"`, and `arch` in `{x86_64, aarch64}`. `build_wsl_argv` returns argument elements only and never a command string.

- [ ] **Step 4: Run tests**

Run: `cargo test -p schneeforge-core execution::tests && cargo test -p schneeforge-core`

Expected: PASS.

- [ ] **Step 5: Commit**

Commit message: `feat(core): define WSL helper bridge contract`

---

### Task 4: CLI early Windows dispatch, backend info probe, and Windows doctor

**Files:**
- Create: `crates/cli/src/windows_backend.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/Cargo.toml` only if a test-only dependency is demonstrably required; prefer existing dependencies.
- Test: CLI unit/integration tests using `assert_cmd` and a fake `wsl.exe` executable/script fixture where platform permits.

**Interfaces:**
- CLI global option: `--wsl-distro <name>`.
- Hidden helper command: `__backend-info` emitting one JSON object matching `BackendInfo`.
- `windows_backend::dispatch_if_needed(...) -> Option<Result<i32, String>>` (or an equivalent interface that makes early return explicit).
- Windows local `doctor` prints host/WSL inventory/selection/helper readiness even if helper delegation is unavailable.

- [ ] **Step 1: Add failing CLI tests first**

Cover:
- hidden `__backend-info` emits protocol/app version plus Linux OS/arch on Linux CI;
- Windows dispatch decision occurs before `resolve_repo`/`ToolInventory::discover` (factor decision logic into pure/testable function);
- `--wsl-distro` is consumed locally and absent from forwarded helper args;
- `self-update` on Windows returns explicit unsupported guidance;
- fake helper nonzero exit code is propagated exactly;
- helper probe mismatch fails before operational command execution;
- `doctor` can report missing WSL/helper without trying native Nix discovery.

- [ ] **Step 2: Run focused CLI tests and verify RED**

Run: `cargo test -p schneeforge --tests`

Expected: FAIL because Windows backend dispatch/probe command does not exist.

- [ ] **Step 3: Implement hidden backend probe and early dispatch boundary**

Restructure startup conceptually to:

```rust
fn main() {
    let cli = Cli::parse();

    if let Some(exit) = windows_backend::dispatch_if_needed(&cli, std::env::args_os()) {
        finish(exit);
        return;
    }

    let repo = schneeforge_core::resolve_repo(cli.repo.as_deref());
    run_native(cli, repo);
}
```

The actual implementation should avoid reparsing lossy strings and should keep macOS/Linux behavior byte-for-byte equivalent at the command-contract level.

For Windows delegation, use `std::process::Command::new("wsl.exe").args(build_wsl_argv(...))`; inherit stdin/stdout/stderr for the delegated command. Helper probing may capture stdout for JSON parsing.

- [ ] **Step 4: Run CLI/core regression**

Run: `cargo test -p schneeforge --tests && cargo test -p schneeforge-core && cargo test --workspace`

Expected: PASS on the current Linux CI host.

- [ ] **Step 5: Commit**

Commit message: `feat(cli): delegate Windows commands through WSL2`

---

### Task 5: Windows compile portability audit

**Files:**
- Modify only files that fail Windows compilation under `crates/core/src/**` and `crates/cli/src/**`.
- Do NOT modify `apps/desktop/**` for Windows support.
- Likely inspect: `crates/core/src/tool.rs`, `self_update.rs`, `managed_nix/**`, tests using Unix permissions/signals, and CLI test helpers.

**Interfaces:**
- Shared core/CLI compile on `x86_64-pc-windows-msvc` without exposing Unix-only symbols unguarded.
- Existing Unix behavior remains unchanged.

- [ ] **Step 1: Add a non-production Windows compile check locally where possible or rely on the first Windows CI RED**

Preferred command when target/toolchain is available:

`cargo check --workspace --exclude schneeforge-desktop --target x86_64-pc-windows-msvc`

Expected initial result: RED on any real portability defect; do not pre-emptively rewrite cfg-gated Unix code that already compiles.

- [ ] **Step 2: Fix only observed compile/test failures**

Typical allowed fixes:
- `#[cfg(unix)]` / `#[cfg(windows)]` around platform APIs;
- platform-neutral path/environment APIs;
- Windows executable suffix handling in test/tool helpers;
- test gating where the behavior itself is Unix-only.

Do not add native Windows implementations for Nix, sudo, managed-nix install, self-update, or Desktop.

- [ ] **Step 3: Re-run Linux workspace regression**

Run: `cargo test --workspace && cargo fmt -- --check && cargo clippy -- -D warnings`

Expected: PASS.

- [ ] **Step 4: Commit**

Commit message: `fix(core): make shared CLI code Windows portable`

---

### Task 6: Non-required Windows CI and WSL canary contract

**Files:**
- Modify: `.github/workflows/check.yml`
- Optionally create: `.github/workflows/windows-wsl-canary.yml` if a real hosted-runner WSL probe is stable enough to be useful.
- Modify: `tests/ci-scripts.bats`
- Modify: `openspec/changes/add-windows-wsl2-platform/tasks.md`

**Interfaces:**
- PR job `windows-check` uses pinned `windows-2025`.
- It compiles/tests core/CLI and runs hermetic bridge contract tests.
- `windows-check` is NOT in current required-context aggregation and is NOT a release dependency.
- Any real-WSL canary is limited to `push` on `develop`, `schedule`, or `workflow_dispatch`; environmental absence may report/skip, never silently claim backend success.

- [ ] **Step 1: Write failing Bats workflow-contract assertions**

Add tests that assert:
- `windows-check` exists and `runs-on: windows-2025`;
- it invokes core/CLI Windows compile/test commands;
- current `ci-required` needs/aggregation does not include `windows-check`;
- `release-artifact-check` and release workflow do not depend on `windows-check` or a WSL canary;
- if canary exists, it is not triggered by PR and is not required.

- [ ] **Step 2: Run Bats and verify RED**

Run: `bats tests/ci-scripts.bats`

Expected: FAIL because `windows-check` is not yet defined.

- [ ] **Step 3: Add minimal Windows job**

Use `windows-2025`, checkout pinned action SHA consistent with repository policy, install stable Rust, and run the smallest reliable set that proves Windows core/CLI compilation plus unit/contract behavior. Do not install Nix on Windows and do not add this job to `ci-required`.

- [ ] **Step 4: Run static gates**

Run: `bats tests/ci-scripts.bats` and `openspec validate add-windows-wsl2-platform --strict`.

Expected: PASS.

- [ ] **Step 5: Push and use actual GitHub Windows runner as GREEN gate**

Required evidence:
- `windows-check` completes successfully on exact PR head;
- existing seven required contexts remain present and green;
- existing macOS stable lanes / release-artifact check remain unaffected;
- any real WSL canary result is reported separately from PR merge eligibility.

- [ ] **Step 6: Commit**

Commit message: `ci: add non-required Windows portability gate`

---

### Task 7: Final documentation, review, and merge readiness

**Files:**
- Modify: `AGENTS.md`
- Modify: `docs/STATUS.md`
- Modify: `openspec/changes/add-windows-wsl2-platform/tasks.md`
- Update PR #96 body with exact implementation and CI evidence.

**Interfaces:**
- Status docs describe Windows/WSL2 as experimental CLI-only support and preserve all non-goals.
- OpenSpec tasks reflect actual completed evidence only.

- [ ] **Step 1: Run complete verification on exact head**

Verify GitHub Actions latest-head results rather than relying on local claims. Confirm all existing required contexts plus `ci-required`, stable macOS lanes, `macos-check`, release artifact, and new non-required `windows-check` are green.

- [ ] **Step 2: Review final PR diff**

Check changed filenames, full patch, review submissions/threads, no shell-interpolated user input, no Windows Nix-system addition, no branch-protection/release drift, and no unrelated refactor.

- [ ] **Step 3: Update docs/tasks and rerun latest-head CI**

Do not mark implementation complete until the exact documentation head is green.

- [ ] **Step 4: Merge implementation PR**

Squash merge PR #96 to `develop` using expected-head SHA after final verification.

- [ ] **Step 5: Archive separately**

Create `chore/archive-add-windows-wsl2-platform` from the new `develop`, archive/sync OpenSpec in a separate PR, validate strictly, review diff, and squash merge only after its CI is green.
