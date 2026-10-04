#![cfg(unix)]

use schneeforge_core::{diagnose, ToolInventory};
use std::process::Command;

const CHILD_CASE: &str = "SCHNEEFORGE_DIAGNOSTICS_HOME_CHILD";

#[test]
fn empty_home_is_reported_as_unavailable() {
    if std::env::var(CHILD_CASE).as_deref() == Ok("empty-home") {
        let inventory = ToolInventory {
            nix: None,
            git: None,
            homebrew: None,
            nh: None,
        };
        let diagnostics = diagnose(&inventory, Some("/definitely/not/a/real/repo"));
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
