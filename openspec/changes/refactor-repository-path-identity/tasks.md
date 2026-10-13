## 1. Path model and resolver

- [ ] 1.1 Add path-native `RepositoryPath` representation and `EffectiveSourceRef` separation with unit tests first.
- [ ] 1.2 Convert repository resolution inputs (`--repo`, `NIX_SETTING_DIR`, `HOME`) to `OsString` / `PathBuf` semantics while preserving precedence, empty-value handling, and relative-HOME fallback behavior.
- [ ] 1.3 Migrate `Repo` / `RepoResolver` and public core exports away from String-based repository identity.

## 2. Filesystem consumers

- [ ] 2.1 Convert `Manifest::load` and source-file local checkout reads to `&Path` + `Path::join`.
- [ ] 2.2 Convert profile resolution/selection APIs that consume local repository paths to `&Path`.
- [ ] 2.3 Convert `SourceResolver`, git revision inspection, `.git` checks, and repository-local git operations to path-native inputs.
- [ ] 2.4 Add Linux/Unix regression tests proving non-UTF-8 repository filesystem identity is preserved through local reads and git/fake-process boundaries.

## 3. Operations and process boundaries

- [ ] 3.1 Convert core operations to receive local repository paths as `&Path` / `RepositoryPath` without changing operation semantics.
- [ ] 3.2 Replace String-returning effective-source plumbing with typed `EffectiveSourceRef` handling for checkout vs managed flake references.
- [ ] 3.3 Generalize process/action argv handling so repository arguments can be supplied as `OsStr` / `OsString` without UTF-8 coercion.
- [ ] 3.4 Add fake executable tests that assert repository argv/current-directory bytes are preserved by SchneeForge.
- [ ] 3.5 Preserve existing managed `github:` flake reference behavior and textual arguments.

## 4. Adapters and presentation boundary

- [ ] 4.1 Migrate native macOS/Linux/WSL Linux CLI repository arguments and dispatch to the path-native core API.
- [ ] 4.2 Migrate Desktop/Tauri repository resolution and core calls without duplicating repository logic outside core.
- [ ] 4.3 Keep `Diagnostics.repo_path` serialized as `String`, deriving it only at the display boundary while `repo_exists` / manifest/profile checks use the native path.
- [ ] 4.4 Keep Windows native launcher / WSL2 delegation and explicit Windows repo Unicode/path policy unchanged.

## 5. Explicit text-only exception

- [ ] 5.1 Keep GUI privilege escalation fail-closed for non-UTF-8 CLI/repository paths; do not introduce lossy substitution into `NIX_SETTING_DIR`, pkexec, or AppleScript transport.
- [ ] 5.2 Keep `managed_nix_escalation_non_utf8_paths` and related escalation tests green.

## 6. Validation

- [ ] 6.1 Run `openspec validate refactor-repository-path-identity --strict`.
- [ ] 6.2 Run `openspec validate --all --strict`.
- [ ] 6.3 Run `cargo test --all` and targeted non-UTF-8 repository tests.
- [ ] 6.4 Run `cargo clippy -- -D warnings` and `cargo fmt -- --check`.
- [ ] 6.5 Run `nix flake check`.
- [ ] 6.6 Verify required CI contexts plus `ci-required`, stable macOS lanes, Windows lane, CodeQL, and release-artifact checks on the final PR head.
- [ ] 6.7 Update `docs/STATUS.md` after implementation to record the repository path identity migration and any deliberately deferred upstream Nix/escalation limitations.
