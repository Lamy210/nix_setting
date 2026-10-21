# Repository Path Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve OS-native repository path identity across SchneeForge core filesystem/process boundaries while keeping managed flake references textual and existing GUI/Windows/escalation contracts intact.

**Architecture:** Introduce a path-native `RepositoryPath` type and a typed `EffectiveSourceRef` split between local checkout paths and managed textual flake references. Migrate process argv to `OsString`, then convert repository-consuming layers to `&Path`; only presentation boundaries such as Diagnostics convert paths to strings. Privilege escalation remains an explicit UTF-8-only exception and must keep failing closed.

**Tech Stack:** Rust 2021, std `Path`/`PathBuf`/`OsStr`/`OsString`, clap, Tauri 2, OpenSpec, GitHub Actions.

**Spec:** `openspec/changes/refactor-repository-path-identity/design.md` plus delta specs under `openspec/changes/refactor-repository-path-identity/specs/`.

## Global Constraints

- Repository precedence stays: explicit CLI repository > `NIX_SETTING_DIR` > absolute `$HOME/nix_setting` > `.`.
- Local repository identity must not require UTF-8 from resolution through filesystem access and child-process argv construction.
- Managed release flake refs remain validated UTF-8 protocol strings such as `github:<owner>/<repo>/<tag>`.
- `Diagnostics.repo_path` remains serialized as `String`; lossy conversion is display-only and must never feed back into filesystem decisions.
- GUI privilege escalation keeps the current UTF-8 text transport and must reject non-UTF-8 repo paths before elevation without lossy substitution.
- Windows native launcher / WSL2 Unicode/path-validation policy is unchanged; path-native behavior applies to macOS/Linux/WSL Linux execution-side code.
- Do not promise that upstream Nix accepts every arbitrary byte path; SchneeForge guarantees preservation up to the process boundary.
- Do not change managed source semantics, profile selection semantics, command ordering, persistent state schema, or source-kind classification.
- This change intentionally breaks String-based public Rust repository APIs rather than carrying a long-lived compatibility shim.
- Keep the workspace compiling after each task; do not introduce temporary `Path -> String -> Path` operational round-trips.

## Review Focus

- Empty/relative `HOME` and empty CLI/env values must retain the existing fallback behavior while non-UTF-8 absolute inputs remain path-native.
- A managed source must never be treated as a filesystem path; local source and `github:` source must remain distinct at every action builder.
- `PlanResult` display text must not become the operational Nix argv; a lossy display representation can never be reused for execution.
- Process logging may render argv lossily for humans, but the exact `OsString` values passed to `Command` must remain unchanged.
- GUI escalation must continue rejecting non-UTF-8 repo paths before spawning `osascript`/`pkexec`; the general path-native migration must not silently widen this boundary.

---

### Task 1: Add path-native repository identity and resolver

**Files:**
- Modify: `crates/core/src/repo.rs`
- Modify: `crates/core/src/lib.rs`
- Test: `crates/core/src/repo.rs`

**Interfaces:**
- Produces: `RepositoryPath`, `Repo { path: RepositoryPath, exists: bool }`, path-native `RepoResolver`, path-native `resolve_repo` / `resolve_repo_with`, and `current_git_revision(repo: &Path, ...)`.
- Consumed by: Tasks 3-6.

- [ ] **Step 1: Write failing resolver tests for native path identity**

Add tests covering:

```rust
#[cfg(unix)]
#[test]
fn resolve_repo_with_preserves_non_utf8_cli_path() { /* compare OsStr bytes */ }

#[cfg(unix)]
#[test]
fn resolve_repo_with_preserves_non_utf8_env_dir() { /* compare OsStr bytes */ }

#[cfg(unix)]
#[test]
fn resolve_repo_with_preserves_non_utf8_absolute_home() { /* expect <home>/nix_setting */ }
```

Keep existing tests for CLI/env/HOME precedence, empty inputs, relative HOME, and `.` fallback, but make their expectations path-native.

- [ ] **Step 2: Run repository unit tests and verify the new tests fail**

Run: `cargo test -p schneeforge-core repo::tests -- --nocapture`

Expected: FAIL because the current resolver accepts/returns `&str`/`String` and uses `std::env::var`.

- [ ] **Step 3: Implement `RepositoryPath` and path-native resolution**

Implement in `crates/core/src/repo.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryPath(PathBuf);

impl RepositoryPath {
    pub fn new(path: impl Into<PathBuf>) -> Self;
    pub fn as_path(&self) -> &Path;
    pub fn into_path_buf(self) -> PathBuf;
}

impl AsRef<Path> for RepositoryPath;

pub struct Repo {
    pub path: RepositoryPath,
    pub exists: bool,
}

impl RepoResolver {
    pub fn resolve(&self, cli_repo: Option<&Path>) -> Repo;
}

pub fn resolve_repo(cli_repo: Option<&Path>) -> RepositoryPath;

pub fn resolve_repo_with(
    cli_repo: Option<&Path>,
    env_dir: Option<&OsStr>,
    home: Option<&OsStr>,
) -> RepositoryPath;

pub fn current_git_revision(repo: &Path, git_bin: &Path) -> Option<String>;
```

Use `std::env::var_os("NIX_SETTING_DIR")` and `std::env::var_os("HOME")`. Treat empty `OsStr` as unset; only append `nix_setting` when HOME is absolute.

- [ ] **Step 4: Add a non-UTF-8 current-directory regression for git revision lookup**

On Unix, create a fake git executable and a non-UTF-8 directory path; assert `current_git_revision(&repo, &fake_git)` reaches that directory without UTF-8 conversion and still rejects blank successful output.

- [ ] **Step 5: Export the new type from `crates/core/src/lib.rs`**

Update the existing repo re-export to include `RepositoryPath` and keep the public names aligned with the new signatures.

- [ ] **Step 6: Verify Task 1**

Run:

```bash
cargo test -p schneeforge-core repo::tests -- --nocapture
cargo check -p schneeforge-core
```

Expected: PASS.

- [ ] **Step 7: Commit Task 1**

```bash
git add crates/core/src/repo.rs crates/core/src/lib.rs
git commit -m "refactor: make repository resolution path-native"
```

---

### Task 2: Make subprocess argv OS-native capable

**Files:**
- Modify: `crates/core/src/process.rs`
- Modify: all direct `command_succeeds`, `run_stream`, and `run_capture` call sites under `crates/core/src/` required for compilation
- Test: `crates/core/src/process.rs`

**Interfaces:**
- Consumes: no repository-specific API.
- Produces: process helpers accepting `&[OsString]`, which Tasks 3-5 use to carry repository bytes without UTF-8 conversion.

- [ ] **Step 1: Write a failing Unix argv-byte preservation test**

Add a test that runs a fake executable receiving a non-UTF-8 `OsString` argument and writes the raw argv bytes to a temp file; assert the captured bytes equal the original bytes.

- [ ] **Step 2: Run process tests and verify the new test fails to compile/pass with `&[String]`**

Run: `cargo test -p schneeforge-core process::tests -- --nocapture`

Expected: FAIL because process helpers currently require `String` arguments.

- [ ] **Step 3: Change process helper signatures**

Implement:

```rust
pub fn command_succeeds(cmd: &Path, args: &[OsString]) -> bool;
pub fn run_stream(cmd: &Path, args: &[OsString]) -> Result<()>;
pub fn run_capture(cmd: &Path, args: &[OsString]) -> Result<String>;
```

Pass `args` directly to `Command::args`. For human-readable logging only, render each argument with `to_string_lossy`; do not use that rendering to execute the command.

- [ ] **Step 4: Migrate textual process callers mechanically to `OsString`**

Update direct callers in core modules (including bootstrap, nix feature checks, diagnostics/tool health, dashboard/release helpers, managed-nix helpers, and tests) so ordinary textual arguments become `OsString::from(...)`. Do not alter command semantics in this task.

- [ ] **Step 5: Verify no process helper still requires repository text coercion**

Run:

```bash
rg 'run_(capture|stream)\(' crates/core/src crates/core/tests
rg 'command_succeeds\(' crates/core/src crates/core/tests
cargo test -p schneeforge-core process::tests -- --nocapture
cargo check -p schneeforge-core
```

Expected: all call sites compile and process tests PASS.

- [ ] **Step 6: Commit Task 2**

```bash
git add crates/core/src crates/core/tests
git commit -m "refactor: support OS-native process arguments"
```

---

### Task 3: Convert filesystem repository consumers and source resolution to `&Path`

**Files:**
- Modify: `crates/core/src/manifest.rs`
- Modify: `crates/core/src/source_files.rs`
- Modify: `crates/core/src/profile.rs`
- Modify: `crates/core/src/source.rs`
- Test: existing module tests plus a new Unix integration test `crates/core/tests/repository_non_utf8_filesystem.rs`

**Interfaces:**
- Consumes: `RepositoryPath` from Task 1 and `OsString` process APIs from Task 2.
- Produces: path-native manifest/profile/source APIs and `EffectiveSourceRef`.

- [ ] **Step 1: Write failing non-UTF-8 filesystem tests**

On Unix/Linux-compatible filesystems, create a repository directory with a non-UTF-8 component containing `schneeforge.toml` and optionally `.git`. Assert:

- `Manifest::load(&repo)` reads the manifest;
- `source_files::load_manifest_for(&repo, &store)` uses the same local path when source is unmanaged;
- profile resolution reads the manifest through the native path;
- `SourceResolver::detect(&repo, &git)` can inspect `.git` without converting the repo to UTF-8.

- [ ] **Step 2: Run focused tests and verify failure**

Run:

```bash
cargo test -p schneeforge-core --test repository_non_utf8_filesystem -- --nocapture
cargo test -p schneeforge-core manifest::tests profile::tests source::tests -- --nocapture
```

Expected: new test FAILS/does not compile while APIs are `&str`.

- [ ] **Step 3: Convert manifest/source-files/profile repository parameters to `&Path`**

Use these canonical signatures:

```rust
impl Manifest {
    pub fn load(repo: &Path) -> Result<Self>;
}

pub fn load_manifest_for(repo: &Path, store: &StateStore) -> Result<Manifest>;
pub fn load_manifest_for_with(
    repo: &Path,
    store: &StateStore,
    cache_base: &Path,
    fetch: &dyn Fn(&str) -> std::result::Result<String, String>,
) -> Result<Manifest>;

pub fn profile::resolve(repo: &Path) -> Result<(String, bool)>;
pub fn profile::resolve_with(repo: &Path, store: &StateStore) -> Result<(String, bool)>;
pub fn profile::list(repo: &Path) -> Result<ProfileList>;
pub fn profile::list_with(repo: &Path, store: &StateStore) -> Result<ProfileList>;
pub fn profile::set_selection(repo: &Path, name: &str) -> Result<()>;
pub fn profile::set_selection_with(repo: &Path, store: &StateStore, name: &str) -> Result<()>;
pub fn profile::override_args(repo: &Path) -> Result<Vec<String>>;
pub fn profile::override_args_with(repo: &Path, store: &StateStore) -> Result<Vec<String>>;
```

`profile::override_args` stays textual because its generated `path:<abs>` inputs intentionally require valid UTF-8; this is not the repository-path transport itself.

- [ ] **Step 4: Add typed `EffectiveSourceRef` and path-native source detection**

Implement in `source.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectiveSourceRef {
    Checkout(RepositoryPath),
    ManagedFlake(String),
}

impl EffectiveSourceRef {
    pub fn display_lossy(&self) -> String;
    pub fn as_nix_arg(&self) -> OsString;
    pub fn with_fragment(&self, fragment: &str) -> OsString;
}

pub fn effective_ref(repo: &Path, store: &StateStore) -> Result<EffectiveSourceRef>;

impl SourceResolver {
    pub fn detect(&self, repo: &Path, git: &ResolvedTool) -> Result<SourceState>;
    pub fn resolve(
        &self,
        repo: &Path,
        git: &ResolvedTool,
        stored: Option<&SourceState>,
    ) -> Result<SourceState>;
}
```

Convert `current_branch`, `exact_tag`, `head_revision`, and `git_output` to `&Path`; construct git `-C` argv as `OsString` so repository bytes are preserved.

- [ ] **Step 5: Add source-ref variant tests**

Assert unmanaged/no-state returns `EffectiveSourceRef::Checkout` with the exact path, while a valid managed release returns `EffectiveSourceRef::ManagedFlake("github:...")` and never exposes that value to filesystem APIs.

- [ ] **Step 6: Verify Task 3**

Run:

```bash
cargo test -p schneeforge-core --test repository_non_utf8_filesystem -- --nocapture
cargo test -p schneeforge-core source::tests profile::tests manifest::tests -- --nocapture
cargo check -p schneeforge-core
```

Expected: PASS.

- [ ] **Step 7: Commit Task 3**

```bash
git add crates/core/src/manifest.rs crates/core/src/source_files.rs crates/core/src/profile.rs crates/core/src/source.rs crates/core/tests/repository_non_utf8_filesystem.rs
git commit -m "refactor: separate repository paths from source refs"
```

---

### Task 4: Preserve repository identity through Nix/git action construction and core operations

**Files:**
- Modify: `crates/core/src/discovery.rs`
- Modify: `crates/core/src/actions.rs`
- Modify: `crates/core/src/operations.rs`
- Modify: `crates/core/src/bootstrap.rs`
- Modify: `crates/core/src/lib.rs`
- Test: module tests plus new Unix argv integration test `crates/core/tests/repository_non_utf8_process_args.rs`

**Interfaces:**
- Consumes: `RepositoryPath`, `EffectiveSourceRef`, path-native source/profile APIs, and `OsString` process helpers.
- Produces: path-native public core operations and native Nix/git argv while retaining display-compatible plan output.

- [ ] **Step 1: Write failing fake-process tests for exact repository argv**

Use fake git/Nix-style executables to record argv bytes and assert a non-UTF-8 checkout reaches:

- `git -C <repo> ...` unchanged;
- `nix ... --inputs-from <repo> ...` unchanged;
- Nix flake target `<repo>#<ASCII fragment>` with the original repo bytes followed by the expected fragment bytes.

Do not require real Nix to accept the path.

- [ ] **Step 2: Split configuration fragments from source identity**

Replace repository-concatenating `ConfigurationTarget::switch_ref` / `build_ref` with fragment-only methods:

```rust
pub fn switch_fragment(&self) -> String;
pub fn build_fragment(&self) -> String;
```

The returned strings begin with `#` and contain only SchneeForge-owned ASCII output syntax. `EffectiveSourceRef::with_fragment` owns the byte-preserving concatenation.

- [ ] **Step 3: Convert action builders to `EffectiveSourceRef` + `OsString`**

Change apply/rollback/build/upgrade action builders so repository-bearing arguments accept `&EffectiveSourceRef` and return `Vec<OsString>`. Convert profile override strings to `OsString` only when inserting them into the argv vector.

Keep temporary activation-link handling path-native (`PathBuf`) until its path is passed to a child process; do not introduce new lossy operational conversion.

- [ ] **Step 4: Convert operation APIs to `&Path`**

Migrate repository-bearing operations in `operations.rs`, including apply, rollback, upgrade/deps update aliases, plan, verify, sync/source sync, source initialization/update helpers, and repository checks. `source_for_persistence` and `current_git_revision` must receive `&Path`.

Keep URL parameters such as `repo_url: &str` textual; do not convert remote repository URLs to filesystem types.

- [ ] **Step 5: Keep `PlanResult.flake_target` display-compatible without using it for execution**

Retain:

```rust
pub struct PlanResult {
    pub host: String,
    pub flake_target: String,
    pub output: Option<String>,
    flake_target_arg: OsString,
}
```

Construct `flake_target` from `EffectiveSourceRef::display_lossy()` plus the fragment for display only. Construct `flake_target_arg` from `EffectiveSourceRef::with_fragment(...)` from the same resolved source. `plan()` must pass `flake_target_arg` to Nix and must never execute `flake_target`.

- [ ] **Step 6: Convert bootstrap/core facade repository parameters**

Change `bootstrap::setup` and repository-bearing facade functions/re-exports in `lib.rs` to `&Path`. Keep `generate_config`'s unused compatibility `_repo` parameter textual unless its call graph requires removal; it does not perform repository filesystem I/O.

- [ ] **Step 7: Verify Task 4**

Run:

```bash
cargo test -p schneeforge-core --test repository_non_utf8_process_args -- --nocapture
cargo test -p schneeforge-core
cargo check -p schneeforge-core
```

Expected: PASS.

- [ ] **Step 8: Commit Task 4**

```bash
git add crates/core/src/discovery.rs crates/core/src/actions.rs crates/core/src/operations.rs crates/core/src/bootstrap.rs crates/core/src/lib.rs crates/core/tests/repository_non_utf8_process_args.rs
git commit -m "refactor: preserve repository paths through operations"
```

---

### Task 5: Migrate CLI/Desktop adapters and keep Diagnostics/Windows/escalation boundaries explicit

**Files:**
- Modify: `crates/cli/src/legacy_main.rs`
- Modify: `crates/cli/src/main.rs`
- Modify only as needed for unchanged contract tests: `crates/cli/src/windows_backend.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `crates/core/src/diagnostics.rs`
- Test: existing CLI/Desktop/core diagnostics and Windows contract tests

**Interfaces:**
- Consumes: path-native public core APIs from Task 4.
- Produces: native CLI/Desktop repository flow with unchanged serialized Diagnostics and unchanged Windows launcher text protocol.

- [ ] **Step 1: Add/adjust adapter tests before code changes**

Pin these behaviors:

- native clap `--repo` uses `PathBuf` on the execution side;
- omitted native `--repo` still resolves via `NIX_SETTING_DIR` / HOME;
- Windows `LauncherArgs.repo` remains `Option<String>` and explicit Windows repo validation is unchanged;
- Diagnostics returns the same JSON field/type for `repo_path` and calculates `repo_exists` from the native path first.

- [ ] **Step 2: Change native CLI repository parsing to `PathBuf`**

In `legacy_main.rs`:

```rust
#[arg(long, global = true)]
repo: Option<PathBuf>,
```

Pass `cli.repo.as_deref()` into path-native resolution and pass `repo.as_path()` into core functions. Do not change the Windows launcher's pre-delegation Unicode parsing/validation path.

- [ ] **Step 3: Migrate Desktop Tauri commands**

Remove `resolve_repo(None) -> String -> PathBuf` round-trips. Resolve once as `RepositoryPath`/`PathBuf`, retain the native path for core calls, and convert only where a serialized/display string is required.

- [ ] **Step 4: Make Diagnostics conversion presentation-only**

Convert internal helpers such as `manifest_diagnostics` to `&Path`. Resolve repository natively, compute `repo_exists`/manifest/profile state using that path, then assign:

```rust
repo_path: native_repo.as_path().to_string_lossy().into_owned()
```

only when constructing `Diagnostics`. Do not reuse `repo_path: String` for filesystem checks.

- [ ] **Step 5: Preserve escalation fail-closed behavior**

Do not loosen `managed_nix::escalate_command` UTF-8 validation. Run the existing `managed_nix_escalation_non_utf8_paths` test and verify it still rejects non-UTF-8 repository paths before elevation.

- [ ] **Step 6: Verify Windows launcher policy remains unchanged**

Run the Windows/backend contract tests available on the current platform and ensure code changes do not alter `validate_wsl_repo_path`, `LauncherArgs.repo: Option<String>`, or omitted-repo delegation semantics.

- [ ] **Step 7: Verify Task 5**

Run:

```bash
cargo test -p schneeforge-core diagnostics -- --nocapture
cargo test -p schneeforge-core --test managed_nix_escalation_non_utf8_paths -- --nocapture
cargo test -p schneeforge-core --test windows_backend_contract -- --nocapture
cargo test -p schneeforge-cli
cargo check --workspace
```

Expected: PASS on applicable targets; Unix-gated tests are skipped where unsupported.

- [ ] **Step 8: Commit Task 5**

```bash
git add crates/cli apps/desktop/src-tauri crates/core/src/diagnostics.rs crates/core/tests
git commit -m "refactor: migrate repository path adapters"
```

---

### Task 6: Full regression audit, OpenSpec task completion, and implementation PR

**Files:**
- Modify: `openspec/changes/refactor-repository-path-identity/tasks.md`
- Modify if implementation status reporting is required by repository convention: `docs/STATUS.md`
- No archive changes in this PR.

**Interfaces:**
- Consumes: completed Tasks 1-5.
- Produces: verified implementation ready for review/merge; archive remains a separate PR per repository policy.

- [ ] **Step 1: Audit remaining repository-string boundaries**

Run:

```bash
rg 'repo: &str|repo: Option<String>|resolve_repo\(|Path::new\(&repo|to_str\(\)\.unwrap\(\)' crates/core crates/cli apps/desktop/src-tauri
```

Classify every hit. Convert filesystem repository boundaries; retain intentional text boundaries such as remote URLs, Windows launcher protocol, and escalation validation. Document any intentional remaining `&str` hit in code comments where ambiguity would recur.

- [ ] **Step 2: Run targeted non-UTF-8 regressions**

Run all new repository-path tests plus existing non-UTF-8 HOME/XDG/tool-path tests. Confirm Linux/Unix byte-preservation coverage does not assume macOS can create arbitrary invalid-UTF-8 filenames.

- [ ] **Step 3: Run OpenSpec validation**

Run:

```bash
openspec validate refactor-repository-path-identity --strict
openspec validate --all --strict
```

Expected: PASS.

- [ ] **Step 4: Run full Rust quality gates**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 5: Run repository-required non-Rust checks available locally/CI**

Run the repository's documented flake/lint/bootstrap checks where available; at minimum ensure the implementation branch triggers required contexts:

- `openspec-check`
- `flake-check`
- `rust-check`
- `lint`
- `bootstrap-test`
- `managed-nix-e2e`
- `release-artifact-check`

- [ ] **Step 6: Update OpenSpec task checkboxes only after evidence exists**

Mark implementation tasks complete in `openspec/changes/refactor-repository-path-identity/tasks.md`; do not archive the change in the implementation PR.

- [ ] **Step 7: Commit verification/status updates**

```bash
git add openspec/changes/refactor-repository-path-identity/tasks.md docs/STATUS.md
git commit -m "docs: record repository path identity implementation"
```

Only include `docs/STATUS.md` if it is updated with directly relevant implementation status.

- [ ] **Step 8: Open the implementation PR to `develop`**

Use a single concern PR from `refactor/repository-path-identity-implementation` to `develop`. Include the breaking Rust API note, preserved GUI/Windows/escalation contracts, targeted non-UTF-8 evidence, and full CI status in the PR body.

- [ ] **Step 9: Review and merge only after all required checks pass**

Use squash merge for the topic branch. After implementation merge, create a separate `chore/archive-*` branch/PR to archive the OpenSpec change and sync main specs, as required by `openspec/project.md`.
