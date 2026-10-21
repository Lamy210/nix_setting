#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command;
use predicates::prelude::*;

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn fixture(label: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "schneeforge-cli-native-repo-{label}-{}-{}",
        std::process::id(),
        TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
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
    let state = root.join("state");
    (root, repo, state)
}

#[test]
fn explicit_repo_argument_accepts_non_utf8_filesystem_path() {
    let (root, repo, state) = fixture("explicit");

    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(repo.as_os_str())
        .arg("status")
        .env("XDG_STATE_HOME", &state)
        .assert()
        .success()
        .stdout(predicate::str::contains("profile: developer (default)"));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn omitted_repo_preserves_non_utf8_nix_setting_dir() {
    let (root, repo, state) = fixture("env");

    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("status")
        .env("NIX_SETTING_DIR", repo.as_os_str())
        .env("XDG_STATE_HOME", &state)
        .assert()
        .success()
        .stdout(predicate::str::contains("profile: developer (default)"));

    let _ = std::fs::remove_dir_all(root);
}
