#![cfg(target_os = "linux")]

use schneeforge_core::{is_root, secure_plan_dir};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const CHILD_MODE: &str = "SCHNEEFORGE_MANAGED_NIX_PLAN_PATH_CHILD";

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "schneeforge-managed-nix-plan-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn spawn_child(test_name: &str, mode: &str, cwd: &Path, home: &str) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .current_dir(cwd)
        .env(CHILD_MODE, mode)
        .env("HOME", home)
        .env_remove("XDG_STATE_HOME")
        .env_remove("XDG_DATA_HOME")
        .output()
        .unwrap()
}

#[test]
fn relative_home_is_rejected_without_creating_cwd_relative_plan_dir() {
    if std::env::var(CHILD_MODE).as_deref() == Ok("relative-home") {
        assert!(!is_root(), "test requires the non-root Managed Nix path");

        let cwd = std::env::current_dir().unwrap();
        let result = secure_plan_dir();

        assert!(
            result.is_err(),
            "relative HOME must fail closed: {result:?}"
        );
        assert!(
            !cwd.join("relative-home/.local/state/schneeforge/managed-nix/plans")
                .exists(),
            "relative HOME must not create Managed Nix plan state below the current directory"
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
        "relative_home_is_rejected_without_creating_cwd_relative_plan_dir",
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
    assert!(
        !cwd.join("relative-home/.local/state/schneeforge/managed-nix/plans")
            .exists(),
        "child must not leave a CWD-relative Managed Nix plan directory"
    );

    let _ = fs::remove_dir_all(root);
}
