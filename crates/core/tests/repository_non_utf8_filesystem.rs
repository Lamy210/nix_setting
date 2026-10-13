#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use schneeforge_core::profile;
use schneeforge_core::source::{SourceKind, SourceResolver};
use schneeforge_core::{load_manifest_for, Manifest, ResolvedTool, StateStore, ToolSource};

const MANIFEST: &str = r#"
schema = 1
[profiles]
default = "developer"
available = ["minimal", "developer"]
[systems]
x86_64-linux = true
"#;

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn non_utf8_repo() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "sf-repository-native-fs-{}-{}",
        std::process::id(),
        TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root.join(OsString::from_vec(b"repo-\xff".to_vec()))
}

fn store_for(repo: &Path) -> StateStore {
    StateStore::new(repo.parent().unwrap().join("state.json"))
}

#[test]
fn manifest_and_profile_preserve_non_utf8_repository_path() {
    let repo = non_utf8_repo();
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("schneeforge.toml"), MANIFEST).unwrap();
    let store = store_for(&repo);

    let manifest = Manifest::load(&repo).unwrap();
    assert_eq!(manifest.profiles.default.as_deref(), Some("developer"));

    let sourced = load_manifest_for(&repo, &store).unwrap();
    assert_eq!(sourced.profiles.default.as_deref(), Some("developer"));

    let (profile_name, selected) = profile::resolve_with(&repo, &store).unwrap();
    assert_eq!(profile_name, "developer");
    assert!(!selected);

    let _ = std::fs::remove_dir_all(repo.parent().unwrap());
}

#[test]
fn source_resolver_checks_non_utf8_repository_without_text_conversion() {
    let repo = non_utf8_repo();
    std::fs::create_dir_all(&repo).unwrap();

    let git = ResolvedTool::new(PathBuf::from("/usr/bin/git"), ToolSource::Path);
    let source = SourceResolver::new().detect(&repo, &git).unwrap();
    assert_eq!(source.kind, SourceKind::Local);

    let _ = std::fs::remove_dir_all(repo.parent().unwrap());
}
