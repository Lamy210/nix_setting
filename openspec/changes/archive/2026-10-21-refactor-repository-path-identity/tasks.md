## 1. Path model and resolver

- [x] 1.1 Add path-native `RepositoryPath` representation and `EffectiveSourceRef` separation with unit tests first.
- [x] 1.2 Convert repository resolution inputs (`--repo`, `NIX_SETTING_DIR`, `HOME`) to `OsString` / `PathBuf` semantics while preserving precedence, empty-value handling, and relative-HOME fallback behavior.
- [x] 1.3 Migrate `Repo` / `RepoResolver` and public core exports away from String-based repository identity.

## 2. Filesystem consumers

- [x] 2.1 Convert `Manifest::load` and source-file local checkout reads to `&Path` + `Path::join`.
- [x] 2.2 Convert profile resolution/selection APIs that consume local repository paths to `&Path`.
- [x] 2.3 Convert `SourceResolver`, git revision inspection, `.git` checks, and repository-local git operations to path-native inputs.
- [x] 2.4 Add Linux/Unix regression tests proving non-UTF-8 repository filesystem identity is preserved through local reads and git/fake-process boundaries.

## 3. Operations and process boundaries

- [x] 3.1 Convert core operations to receive local repository paths as `&Path` / `RepositoryPath` without changing operation semantics.
- [x] 3.2 Replace String-returning effective-source plumbing with typed `EffectiveSourceRef` handling for checkout vs managed flake references.
- [x] 3.3 Generalize process/action argv handling so repository arguments can be supplied as `OsStr` / `OsString` without UTF-8 coercion.
- [x] 3.4 Add fake executable tests that assert repository argv/current-directory bytes are preserved by SchneeForge.
- [x] 3.5 Preserve existing managed `github:` flake reference behavior and textual arguments.

## 4. Adapters and presentation boundary

- [x] 4.1 Migrate native macOS/Linux/WSL Linux CLI repository arguments and dispatch to the path-native core API.
- [x] 4.2 Migrate Desktop/Tauri repository resolution and core calls without duplicating repository logic outside core.
- [x] 4.3 Keep `Diagnostics.repo_path` serialized as `String`, deriving it only at the display boundary while `repo_exists` / manifest/profile checks use the native path.
- [x] 4.4 Keep Windows native launcher / WSL2 delegation and explicit Windows repo Unicode/path policy unchanged.

## 5. Explicit text-only exception

- [x] 5.1 Keep GUI privilege escalation fail-closed for non-UTF-8 CLI/repository paths; do not introduce lossy substitution into `NIX_SETTING_DIR`, pkexec, or AppleScript transport.
- [x] 5.2 Keep `managed_nix_escalation_non_utf8_paths` and related escalation tests green.

## 6. Validation

- [x] 6.1 Run `openspec validate refactor-repository-path-identity --strict`.
- [x] 6.2 Run `openspec validate --all --strict`.
- [x] 6.3 Run `cargo test --all` and targeted non-UTF-8 repository tests.
- [x] 6.4 Run `cargo clippy -- -D warnings` and `cargo fmt -- --check`.
- [x] 6.5 Run `nix flake check`.
- [x] 6.6 Verify required CI contexts plus `ci-required`, stable macOS lanes, Windows lane, CodeQL, and release-artifact checks on the final PR head.
- [x] 6.7 Update `docs/STATUS.md` after implementation to record the repository path identity migration and any deliberately deferred upstream Nix/escalation limitations.

## Completion evidence (2026-10-21)

- Implementation squash-merged as PR #238 (`2ffcf1e`), with strict OpenSpec, Rust tests/clippy/fmt, Nix flake checks, macOS 15/26, Windows, release artifact, CodeQL and required CI all successful on final HEAD `505b968` (Actions run `39519506446`).
- Follow-up removes the obsolete textual repository compatibility entry points and migrates their tests to OS-native `PathBuf` / typed `EffectiveSourceRef` contracts before archive.
- Nix's upstream flake path restrictions and the intentional non-UTF-8 GUI privilege-escalation rejection remain out of scope as specified in design.md.
