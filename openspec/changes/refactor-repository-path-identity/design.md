## Context

`crates/core/src/repo.rs` currently resolves repository locations as `String`. That textual representation propagates through manifest/profile/source resolution, operations/actions, git working directories, Nix argument construction, CLI/Desktop adapters, and diagnostics.

Filesystem paths are not text identifiers. On supported Unix execution environments, an OS-valid path may not be representable as UTF-8. Converting repository paths through `std::env::var`, `to_str`, `format!`, or `String` before filesystem/process use can lose identity or reject a valid path before the underlying operation is attempted.

At the same time, SchneeForge also supports managed release sources represented by canonical textual flake references such as `github:<owner>/<repo>/<tag>`. Those are protocol strings, not filesystem paths, and should remain UTF-8 text.

A third boundary exists in GUI privilege escalation. macOS AppleScript/shell transport and the existing `NIX_SETTING_DIR` textual environment handoff intentionally require UTF-8 and currently reject non-UTF-8 repository paths before elevation. That fail-closed contract is preserved by this change.

## Goals / Non-Goals

### Goals

- Preserve OS-native local repository path identity from resolution through SchneeForge filesystem and process boundaries.
- Make local repository paths and managed textual flake references distinct types/variants.
- Remove repository-path UTF-8 coercion from manifest, profile, source, operations, actions, and git process working-directory paths.
- Permit process argument APIs to carry `OsString` repository arguments without first converting them to `String`.
- Keep Diagnostics JSON/frontend compatibility by converting paths only at the presentation boundary.
- Preserve Windows native launcher / WSL2 delegation behavior.
- Preserve the current fail-closed UTF-8 requirement for GUI privilege-escalation transport.

### Non-Goals

- Changing source-kind semantics, managed release behavior, profile selection, Nix command names, or operation ordering.
- Changing Windows native launcher arguments from their existing Unicode contract.
- Making AppleScript privilege escalation transport binary-safe.
- Guaranteeing that upstream Nix accepts every possible OS-native byte sequence as a flake URL. SchneeForge SHALL preserve the argument it hands to Nix; any additional upstream Nix flake-reference restriction remains an upstream boundary.
- Solving unrelated special-character/URL escaping behavior for local Nix flake references in this change.

## Decisions

### Decision 1: Introduce a repository filesystem type

Local repositories SHALL be represented by a dedicated `RepositoryPath` value backed by `PathBuf` (or an equivalent path-native newtype) rather than `String`.

The type SHALL:

- own the resolved local repository path;
- expose `as_path() -> &Path` / `AsRef<Path>` for filesystem consumers;
- avoid implicit conversion to `String`;
- provide explicit display conversion only for diagnostics/error/UI presentation.

`Repo.path` SHALL use this path-native representation. `RepoResolver` SHALL accept path-native CLI input and resolve `NIX_SETTING_DIR` / `HOME` with `var_os`.

Resolution precedence remains unchanged:

1. explicit CLI repository
2. `NIX_SETTING_DIR`
3. `$HOME/nix_setting` when HOME is absolute
4. `.`

### Decision 2: Separate local checkout paths from managed flake references

The current `effective_ref(repo: &str, ...) -> String` conflates two domains. Replace that internal boundary with an enum equivalent to:

```rust
pub enum EffectiveSourceRef {
    Checkout(RepositoryPath),
    ManagedFlake(String),
}
```

`Checkout` SHALL preserve the original path identity. `ManagedFlake` SHALL contain only the validated canonical textual flake reference produced from managed source state.

Filesystem consumers MUST NOT accept `ManagedFlake`; protocol/Nix reference consumers SHALL explicitly handle both variants.

### Decision 3: Convert repository filesystem APIs to `&Path`

Repository-local operations SHALL use `&Path` rather than `&str`, including:

- manifest loading (`repo.join("schneeforge.toml")`);
- profile/source-file local reads;
- `.git` checks;
- git `current_dir` / `-C` repository arguments;
- source detection;
- core operation local-repository inputs.

Error messages MAY use `Path::display()` because rendering an error is a presentation concern. The rendered text MUST NOT be reused as the filesystem path.

### Decision 4: Make subprocess argv OS-native capable

`process::run_stream` / `run_capture` and internal argument builders SHALL accept arguments that can carry `OsStr` / `OsString`, rather than requiring every argument to be a `String`.

Text-only arguments remain ordinary strings and are converted losslessly to `OsString`. Repository-path arguments are inserted from their native path representation without UTF-8 conversion.

For Nix flake target construction, the repository argument/target builder SHALL preserve the checkout path bytes when appending SchneeForge-owned ASCII syntax such as a flake output fragment. Managed flake references continue to use textual construction.

Tests SHALL verify the exact argv observed by a fake executable rather than assuming upstream Nix accepts all non-UTF-8 flake references.

### Decision 5: Diagnostics keeps its serialized String contract

`Diagnostics.repo_path` remains `String` so existing CLI/Desktop/frontend JSON consumers do not change schema.

The diagnostics implementation SHALL:

1. resolve and retain the native repository path;
2. perform `is_dir`, manifest/profile checks, etc. using that native path;
3. derive `repo_path` only for serialized/display output, using a clearly presentation-only conversion such as `to_string_lossy`.

Therefore lossy display text cannot influence `repo_exists` or filesystem behavior.

### Decision 6: Preserve privilege-escalation UTF-8 rejection

`managed_nix::escalate_command` currently transports repository identity through text (`NIX_SETTING_DIR`; additionally AppleScript/shell on macOS). This change SHALL NOT replace that transport.

If the repository path is not representable as UTF-8, escalation MUST fail before executing an elevated command and MUST NOT substitute a lossy representation.

This is an explicit exception to normal core path handling, not permission for other filesystem/core APIs to coerce repository paths to text.

### Decision 7: Preserve Windows launcher policy

The Windows native launcher remains a control-plane boundary with Unicode arguments. Explicit Windows `--repo` continues to follow the existing WSL path validation policy. If `--repo` is omitted, repository resolution continues inside the selected WSL2 Linux helper.

The path-native migration therefore applies to the macOS/Linux/WSL Linux execution side and does not redefine Windows host path translation.

## Migration Plan

The migration SHALL keep the workspace buildable at each implementation step and SHALL avoid temporary `Path -> String -> Path` round-trips.

1. Add `RepositoryPath` and `EffectiveSourceRef` with unit tests.
2. Convert repository environment/CLI resolution to path-native values.
3. Convert manifest, source-files, profile, and source resolver local-path APIs to `&Path`.
4. Convert operations/current-git-revision boundaries to `&Path`.
5. Generalize process/action argv construction to `OsString`-capable arguments and migrate Nix/git call sites.
6. Migrate CLI and Desktop adapters.
7. Keep Diagnostics schema stable and move string conversion to the presentation boundary.
8. Remove obsolete String-based repository APIs once all in-repo callers are migrated.
9. Add/complete Unix non-UTF-8 regression coverage and run the full quality/CI matrix.

The Rust API migration is intentionally breaking for String-based `schneeforge-core` repository callers. This is explicit in the proposal rather than hidden behind a long-lived compatibility shim.

## Testing Strategy

### Unit tests

- resolution precedence with `OsString` / `PathBuf` inputs;
- empty and relative HOME fallback behavior remains unchanged;
- `RepositoryPath` never requires UTF-8 for filesystem operations;
- `EffectiveSourceRef` keeps checkout and managed reference variants distinct;
- diagnostics display conversion does not feed filesystem checks.

### Unix regression tests

On Unix environments that permit constructing/using the target byte sequence on the filesystem (Linux CI in particular):

- create a repository directory containing a non-UTF-8 path component;
- load `schneeforge.toml` from it;
- detect `.git` / source state from it;
- invoke a fake git/Nix-style process and assert exact argv/current-dir bytes are preserved.

macOS tests MUST NOT assume APFS/filesystem APIs will accept arbitrary invalid UTF-8 filenames. macOS coverage should validate path-native API construction/propagation without requiring creation of an invalid-UTF-8 filesystem entry.

### Existing contract tests

The following behavior must stay green:

- UTF-8 repository operations across CLI and Desktop;
- managed source resolution and cached manifest loading;
- Diagnostics field names and frontend expectations;
- Windows WSL2 contract tests;
- `managed_nix_escalation_non_utf8_paths` continues to reject non-UTF-8 escalation paths;
- existing non-UTF-8 HOME/XDG/tool-path regression tests.

## Risks / Trade-offs

- **Cross-cutting API churn**: many core functions currently use `&str`. Mitigation: migrate by layer and keep compilation/tests green after each task.
- **Nix flake parser constraints**: SchneeForge can preserve argv bytes but cannot promise upstream Nix accepts every OS-native path. Mitigation: specify preservation, not universal upstream acceptance; use fake-process argv tests for SchneeForge's contract.
- **Presentation ambiguity**: lossy diagnostics may render replacement characters. Mitigation: presentation text is informational only; filesystem decisions use the retained native path.
- **Privilege escalation remains narrower than normal execution**: non-UTF-8 repo paths still cannot cross current GUI escalation transport. Mitigation: keep this explicit and fail-closed; a binary-safe escalation redesign is a separate change.
- **Breaking Rust API**: external `schneeforge-core` callers using repository `String` APIs must migrate. Mitigation: document the break in proposal/release notes and perform the workspace migration atomically within this OpenSpec change.

## Rollback

Before merge, rollback is branch deletion/revert. After merge, a revert of the implementation PR restores String-based repository APIs. No persisted state/schema migration is introduced, so rollback does not require state conversion.

## Open Questions

None for this change. Binary-safe GUI privilege escalation and stronger local-flake URL encoding are explicitly deferred to separate changes.
