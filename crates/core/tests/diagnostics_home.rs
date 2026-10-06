#![cfg(unix)]

use schneeforge_core::{diagnose, ToolInventory};
use std::process::Command;

const CHILD_CASE: &str = "SCHNEEFORGE_DIAGNOSTICS_HOME_CHILD";

fn empty_inventory() -> ToolInventory {
    ToolInventory {
        nix: None,
        git: None,
        homebrew: None,
        nh: None,
    }
}

#[test]
fn empty_home_is_reported_as_unavailable() {
    if std::env::var(CHILD_CASE).as_deref() == Ok("empty-home") {
        let diagnostics = diagnose(&empty_inventory(), Some("/definitely/not/a/real/repo"));
        assert_eq!(
            diagnostics.home, None,
            "an empty HOME value must be reported as unavailable rather than as a real path"
        );
        return;
    }

    let state_root = std::env::temp_dir().join(format!(
        "schneeforge-diagnostics-home-state-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&state_root);
    std::fs::create_dir_all(&state_root).expect("create isolated state root");

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "empty_home_is_reported_as_unavailable",
            "--nocapture",
        ])
        .env(CHILD_CASE, "empty-home")
        .env("HOME", "")
        .env("XDG_STATE_HOME", &state_root)
        .status()
        .expect("spawn isolated diagnostics HOME regression test");

    let _ = std::fs::remove_dir_all(&state_root);
    assert!(status.success(), "empty HOME must be normalized to None");
}

#[test]
fn relative_home_is_reported_as_unavailable() {
    if std::env::var(CHILD_CASE).as_deref() == Ok("relative-home") {
        let diagnostics = diagnose(&empty_inventory(), Some("/definitely/not/a/real/repo"));
        assert_eq!(
            diagnostics.home, None,
            "a relative HOME value must be reported as unavailable rather than as a real home directory"
        );
        return;
    }

    let state_root = std::env::temp_dir().join(format!(
        "schneeforge-diagnostics-relative-home-state-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&state_root);
    std::fs::create_dir_all(&state_root).expect("create isolated state root");

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "relative_home_is_reported_as_unavailable",
            "--nocapture",
        ])
        .env(CHILD_CASE, "relative-home")
        .env("HOME", "relative-home")
        .env("XDG_STATE_HOME", &state_root)
        .status()
        .expect("spawn isolated diagnostics HOME regression test");

    let _ = std::fs::remove_dir_all(&state_root);
    assert!(
        status.success(),
        "relative HOME must be normalized to None in Diagnostics.home"
    );
}
