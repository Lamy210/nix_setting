#![cfg(unix)]

use schneeforge_core::{nix_health, ResolvedTool, ToolInventory, ToolSource};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const CHILD_CASE: &str = "SCHNEEFORGE_DIAGNOSTICS_XDG_CHILD_CASE";
const CHILD_NIX: &str = "SCHNEEFORGE_DIAGNOSTICS_XDG_CHILD_NIX";

fn inventory(nix: PathBuf) -> ToolInventory {
    ToolInventory {
        nix: Some(ResolvedTool::new(nix, ToolSource::Path)),
        git: None,
        homebrew: None,
        nh: None,
    }
}

fn write_fake_nix(path: &Path) {
    fs::write(
        path,
        r#"#!/bin/sh
if [ "$1" = "store" ] && [ "$2" = "ping" ]; then
  exit 0
fi
if [ "$1" = "config" ] && [ "$2" = "show" ] && [ "$3" = "experimental-features" ]; then
  echo "nix-command flakes"
  exit 0
fi
exit 1
"#,
    )
    .expect("write fake nix");
    let mut permissions = fs::metadata(path).expect("fake nix metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make fake nix executable");
}

fn assert_child_health_has_no_state_path_warning() {
    let nix = PathBuf::from(std::env::var(CHILD_NIX).expect("child fake nix path"));
    let health = nix_health(&inventory(nix));
    assert!(health.installed, "fake nix should be detected");
    assert!(health.store_accessible, "fake nix store ping should succeed");
    assert!(health.flakes_available, "fake nix should report required features");
    assert_eq!(
        health.warning, None,
        "unusable empty state-path environment values must not redirect diagnostics into the process CWD"
    );
}

#[test]
fn empty_xdg_state_home_falls_back_to_non_empty_home() {
    if std::env::var(CHILD_CASE).as_deref() == Ok("empty-xdg") {
        assert_child_health_has_no_state_path_warning();
        return;
    }

    let root = std::env::temp_dir().join(format!(
        "schneeforge-diagnostics-empty-xdg-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let cwd = root.join("cwd");
    let home = root.join("home");
    fs::create_dir_all(&cwd).expect("create isolated cwd");
    fs::create_dir_all(home.join(".local/state/nix/profiles"))
        .expect("create HOME state profile dir");
    let fake_nix = root.join("nix");
    write_fake_nix(&fake_nix);

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "empty_xdg_state_home_falls_back_to_non_empty_home",
            "--nocapture",
        ])
        .current_dir(&cwd)
        .env(CHILD_CASE, "empty-xdg")
        .env(CHILD_NIX, &fake_nix)
        .env("XDG_STATE_HOME", "")
        .env("HOME", &home)
        .status()
        .expect("spawn isolated diagnostics regression test");

    let _ = fs::remove_dir_all(&root);
    assert!(
        status.success(),
        "empty XDG_STATE_HOME should fall back to non-empty HOME"
    );
}

#[test]
fn empty_home_without_xdg_state_home_disables_state_path_warning() {
    if std::env::var(CHILD_CASE).as_deref() == Ok("empty-home") {
        assert_child_health_has_no_state_path_warning();
        return;
    }

    let root = std::env::temp_dir().join(format!(
        "schneeforge-diagnostics-empty-home-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let cwd = root.join("cwd");
    fs::create_dir_all(&cwd).expect("create isolated cwd");
    let fake_nix = root.join("nix");
    write_fake_nix(&fake_nix);

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "empty_home_without_xdg_state_home_disables_state_path_warning",
            "--nocapture",
        ])
        .current_dir(&cwd)
        .env(CHILD_CASE, "empty-home")
        .env(CHILD_NIX, &fake_nix)
        .env_remove("XDG_STATE_HOME")
        .env("HOME", "")
        .status()
        .expect("spawn isolated diagnostics regression test");

    let _ = fs::remove_dir_all(&root);
    assert!(
        status.success(),
        "empty HOME without XDG_STATE_HOME should not derive a relative diagnostics path"
    );
}
