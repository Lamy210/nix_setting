#![cfg(target_os = "linux")]

use schneeforge_core::managed_nix::{cache_path, is_root};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const CHILD_MODE: &str = "SCHNEEFORGE_MANAGED_NIX_CACHE_PATH_CHILD";

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "schneeforge-managed-nix-cache-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn spawn_child(test_name: &str, mode: &str, cwd: &PathBuf, home: &str) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .current_dir(cwd)
        .env(CHILD_MODE, mode)
        .env("HOME", home)
        .env_remove("XDG_DATA_HOME")
        .output()
        .unwrap()
}

#[test]
fn relative_home_is_rejected_for_managed_nix_cache() {
    if std::env::var(CHILD_MODE).as_deref() == Ok("relative-home") {
        assert!(!is_root(), "test requires the non-root Managed Nix path");

        let result = cache_path("9.9.9");
        assert!(
            result.is_err(),
            "relative HOME must not resolve a Managed Nix cache path: {result:?}"
        );
        return;
    }

    if is_root() {
        return;
    }

    let root = temp_dir("relative-home");
    let cwd = root.join("cwd");
    fs::create_dir_all(&cwd).unwrap();

    let output = spawn_child(
        "relative_home_is_rejected_for_managed_nix_cache",
        "relative-home",
        &cwd,
        "relative-home",
    );

    assert!(
        output.status.success(),
        "child failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(root);
}
