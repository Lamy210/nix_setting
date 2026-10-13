#!/usr/bin/env python3
from pathlib import Path

LIB = Path("crates/core/src/lib.rs")
DIAG = Path("crates/core/src/diagnostics.rs")
lib = LIB.read_text()
diag = DIAG.read_text()


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one occurrence, got {count}")
    return text.replace(old, new, 1)


# Public core facade: keep URL/text parameters textual, repository filesystem parameters native.
lib = replace_once(
    lib,
    '''    pub fn source_init(
        repo: &str,
        store: &crate::state::StateStore,
        git: &crate::tool::ResolvedTool,
        channel: Option<String>,
        tag: Option<String>,
    ) -> crate::error::Result<crate::operations_impl::SourceInitResult> {''',
    '''    pub fn source_init<P: AsRef<std::path::Path> + ?Sized>(
        repo: &P,
        store: &crate::state::StateStore,
        git: &crate::tool::ResolvedTool,
        channel: Option<String>,
        tag: Option<String>,
    ) -> crate::error::Result<crate::operations_impl::SourceInitResult> {''',
    "lib source_init",
)
lib = replace_once(
    lib,
    '''    pub fn deps_update(
        repo: &str,
        tc: &crate::tool::ToolInventory,
        capture: bool,
    ) -> crate::error::Result<Option<String>> {''',
    '''    pub fn deps_update<P: AsRef<std::path::Path> + ?Sized>(
        repo: &P,
        tc: &crate::tool::ToolInventory,
        capture: bool,
    ) -> crate::error::Result<Option<String>> {''',
    "lib deps_update",
)
lib = replace_once(
    lib,
    '''        if !managed && std::path::Path::new(repo).join(".git").exists() {
            tc.require_git()?;
        }
''',
    '''        if !managed && repo.as_ref().join(".git").exists() {
            tc.require_git()?;
        }
''',
    "lib deps_update path check",
)
lib = replace_once(
    lib,
    '''    pub fn upgrade(
        repo: &str,
        tc: &crate::tool::ToolInventory,
        capture: bool,
    ) -> crate::error::Result<Option<String>> {''',
    '''    pub fn upgrade<P: AsRef<std::path::Path> + ?Sized>(
        repo: &P,
        tc: &crate::tool::ToolInventory,
        capture: bool,
    ) -> crate::error::Result<Option<String>> {''',
    "lib upgrade",
)
lib = replace_once(
    lib,
    '''    pub fn verify(
        repo: &str,
        tc: &crate::tool::ToolInventory,
    ) -> crate::operations_impl::VerifyReport {''',
    '''    pub fn verify<P: AsRef<std::path::Path> + ?Sized>(
        repo: &P,
        tc: &crate::tool::ToolInventory,
    ) -> crate::operations_impl::VerifyReport {''',
    "lib verify",
)
lib = replace_once(
    lib,
    'pub use repo::{current_git_revision, resolve_repo, Repo, RepoResolver};',
    '''pub use repo::{
    current_git_revision, current_git_revision_path, resolve_repo, resolve_repo_path, Repo,
    RepoResolver, RepositoryPath,
};''',
    "repo reexports",
)
lib = replace_once(
    lib,
    '''pub use source::{
    classify_release_tag, effective_ref, github_slug, latest_tag_for_channel, repo_url, SourceKind,
    SourceResolver, SourceState,
};''',
    '''pub use source::{
    classify_release_tag, effective_ref, effective_source_ref, github_slug, latest_tag_for_channel,
    repo_url, EffectiveSourceRef, SourceKind, SourceResolver, SourceState,
};''',
    "source reexports",
)

# Diagnostics: operational path remains native; repo_path String is presentation-only.
diag = replace_once(diag, "use serde::Serialize;", "use std::path::Path;\n\nuse serde::Serialize;", "diag Path import")
diag = replace_once(diag, "use crate::repo::resolve_repo;", "use crate::repo::resolve_repo_path;", "diag resolver import")
diag = replace_once(
    diag,
    '''pub fn diagnose(tc: &ToolInventory, cli_repo: Option<&str>) -> Diagnostics {
    diagnose_with_store(tc, cli_repo, &StateStore::default())
}

fn diagnose_with_store(
    tc: &ToolInventory,
    cli_repo: Option<&str>,
    store: &StateStore,
) -> Diagnostics {''',
    '''pub fn diagnose(tc: &ToolInventory, cli_repo: Option<&Path>) -> Diagnostics {
    diagnose_with_store(tc, cli_repo, &StateStore::default())
}

fn diagnose_with_store(
    tc: &ToolInventory,
    cli_repo: Option<&Path>,
    store: &StateStore,
) -> Diagnostics {''',
    "diagnose signatures",
)
diag = replace_once(
    diag,
    '''    let target = detect_target();
    let repo_path = resolve_repo(cli_repo);
    let repo_exists = std::path::Path::new(&repo_path).is_dir();

    let (manifest_found, manifest_error, manifest_default, validation) =
        manifest_diagnostics(&repo_path, target.name(), store);''',
    '''    let target = detect_target();
    let native_repo = resolve_repo_path(cli_repo);
    let repo_path = native_repo.as_path();
    let repo_exists = repo_path.is_dir();

    let (manifest_found, manifest_error, manifest_default, validation) =
        manifest_diagnostics(repo_path, target.name(), store);''',
    "diagnose native repo",
)
diag = replace_once(
    diag,
    '''        match crate::profile::resolve_with(&repo_path, store) {''',
    '''        match crate::profile::resolve_with(repo_path, store) {''',
    "diagnose profile path",
)
diag = replace_once(
    diag,
    '''        repo_path,
        repo_exists,''',
    '''        repo_path: repo_path.to_string_lossy().into_owned(),
        repo_exists,''',
    "diagnostics presentation conversion",
)
diag = replace_once(
    diag,
    '''fn manifest_diagnostics(
    repo_path: &str,
    system: &str,
    store: &StateStore,
) -> (bool, Option<String>, Option<String>, Option<Validation>) {''',
    '''fn manifest_diagnostics(
    repo_path: &Path,
    system: &str,
    store: &StateStore,
) -> (bool, Option<String>, Option<String>, Option<Validation>) {''',
    "manifest diagnostics path",
)

# Existing diagnostics tests use fixed textual fixture paths; make the conversion explicit at test call sites.
diag = diag.replace(
    'Some("/definitely/not/a/real/repo")',
    'Some(Path::new("/definitely/not/a/real/repo"))',
)

marker = '''    #[test]
    fn diagnose_nonexistent_repo() {
'''
if diag.count(marker) != 1:
    raise SystemExit("diagnostics test insertion marker changed")
new_test = r'''    #[cfg(target_os = "linux")]
    #[test]
    fn diagnose_uses_native_repository_path_before_display_conversion() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let tc = dummy_tc();
        let root = std::env::temp_dir().join(format!(
            "schneeforge-diagnostics-native-repo-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let repo = root.join(OsString::from_vec(b"repository-\xff".to_vec()));
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            repo.join("schneeforge.toml"),
            "schema = 1\n[profiles]\ndefault = \"developer\"\navailable = [\"developer\"]\n[systems]\nx86_64-linux = true\n",
        )
        .unwrap();
        let store = StateStore::new(root.join("state.json"));

        let d = diagnose_with_store(&tc, Some(&repo), &store);
        assert!(d.repo_exists);
        assert!(d.manifest_found);
        assert_eq!(d.repo_path, repo.to_string_lossy());
        let json = serde_json::to_value(&d).unwrap();
        assert!(json.get("repo_path").is_some_and(|value| value.is_string()));

        let _ = std::fs::remove_dir_all(root);
    }

'''
diag = diag.replace(marker, new_test + marker, 1)

prod_diag = diag.split("\n#[cfg(test)]\nmod tests {", 1)[0]
for bad in ["resolve_repo(", "repo_path: &str", "Path::new(&repo_path)"]:
    if bad in prod_diag:
        raise SystemExit(f"diagnostics operational String boundary remains: {bad}")

LIB.write_text(lib)
DIAG.write_text(diag)
