#![cfg(unix)]

use schneeforge_core::{nix_health, ResolvedTool, ToolInventory, ToolSource};
#[cfg(not(target_os = "macos"))]
use std::ffi::OsString;
use std::fs;
#[cfg(not(target_os = "macos"))]
use std::os::unix::ffi::OsStringExt;
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
    assert!(
        health.store_accessible,
        "fake nix store ping should succeed"
    );
    assert!(
        health.flakes_available,
        "fake nix should report required features"
    );
    assert_eq!(
        health.warning, None,
        "empty or relative state-path environment values must not redirect diagnostics into the process CWD"
    );
}

#[cfg(not(target_os = "macos"))]
fn assert_child_health_has_state_path_warning() {
    let nix = PathBuf::from(std::env::var(CHILD_NIX).expect("child fake nix path"));
    let health = nix_health(&inventory(nix));
    assert!(health.installed, "fake nix should be detected");
    assert!(
        health.store_accessible,
        "fake nix store ping should succeed"
    );
    assert!(
        health.flakes_available,
        "fake nix should report required features"
    );
    assert!(
        health
            .warning
            .as_deref()
            .is_some_and(|warning| warning.contains(".local/state/nix/profiles")),
        "a missing profile directory under a valid non-UTF-8 HOME must still be diagnosed"
    );
}

#[cfg(not(target_os = "macos"))]
fn non_utf8_temp_dir(label: &str) -> PathBuf {
    let mut name = format!("schneeforge-diagnostics-{label}-{}-", std::process::id()).into_bytes();
    name.push(0xff);
    std::env::temp_dir().join(OsString::from_vec(name))
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
fn relative_xdg_state_home_falls_back_to_absolute_home() {
    if std::env::var(CHILD_CASE).as_deref() == Ok("relative-xdg") {
        assert_child_health_has_no_state_path_warning();
        return;
    }

    let root = std::env::temp_dir().join(format!(
        "schneeforge-diagnostics-relative-xdg-{}",
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
            "relative_xdg_state_home_falls_back_to_absolute_home",
            "--nocapture",
        ])
        .current_dir(&cwd)
        .env(CHILD_CASE, "relative-xdg")
        .env(CHILD_NIX, &fake_nix)
        .env("XDG_STATE_HOME", "relative-state")
        .env("HOME", &home)
        .status()
        .expect("spawn isolated diagnostics regression test");

    let _ = fs::remove_dir_all(&root);
    assert!(
        status.success(),
        "relative XDG_STATE_HOME should fall back to absolute HOME"
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

#[test]
fn relative_home_without_xdg_state_home_disables_state_path_warning() {
    if std::env::var(CHILD_CASE).as_deref() == Ok("relative-home") {
        assert_child_health_has_no_state_path_warning();
        return;
    }

    let root = std::env::temp_dir().join(format!(
        "schneeforge-diagnostics-relative-home-{}",
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
            "relative_home_without_xdg_state_home_disables_state_path_warning",
            "--nocapture",
        ])
        .current_dir(&cwd)
        .env(CHILD_CASE, "relative-home")
        .env(CHILD_NIX, &fake_nix)
        .env_remove("XDG_STATE_HOME")
        .env("HOME", "relative-home")
        .status()
        .expect("spawn isolated diagnostics regression test");

    let _ = fs::remove_dir_all(&root);
    assert!(
        status.success(),
        "relative HOME without XDG_STATE_HOME should not derive a diagnostics path from the process CWD"
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn non_utf8_xdg_state_home_is_used_for_profile_diagnostics() {
    if std::env::var(CHILD_CASE).as_deref() == Ok("non-utf8-xdg") {
        assert_child_health_has_no_state_path_warning();
        return;
    }

    let root = std::env::temp_dir().join(format!(
        "schneeforge-diagnostics-non-utf8-xdg-root-{}",
        std::process::id()
    ));
    let state_home = non_utf8_temp_dir("non-utf8-xdg");
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&state_home);
    let cwd = root.join("cwd");
    let home = root.join("home");
    fs::create_dir_all(&cwd).expect("create isolated cwd");
    fs::create_dir_all(&home).expect("create fallback HOME");
    fs::create_dir_all(state_home.join("nix/profiles"))
        .expect("create non-UTF-8 XDG profile dir");
    let fake_nix = root.join("nix");
    write_fake_nix(&fake_nix);

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "non_utf8_xdg_state_home_is_used_for_profile_diagnostics",
            "--nocapture",
        ])
        .current_dir(&cwd)
        .env(CHILD_CASE, "non-utf8-xdg")
        .env(CHILD_NIX, &fake_nix)
        .env("XDG_STATE_HOME", &state_home)
        .env("HOME", &home)
        .status()
        .expect("spawn isolated non-UTF-8 XDG diagnostics regression test");

    let _ = fs::remove_dir_all(&state_home);
    let _ = fs::remove_dir_all(&root);
    assert!(
        status.success(),
        "valid non-UTF-8 XDG_STATE_HOME must remain usable for profile diagnostics"
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn non_utf8_home_is_used_for_profile_diagnostics() {
    if std::env::var(CHILD_CASE).as_deref() == Ok("non-utf8-home") {
        assert_child_health_has_state_path_warning();
        return;
    }

    let root = std::env::temp_dir().join(format!(
        "schneeforge-diagnostics-non-utf8-home-root-{}",
        std::process::id()
    ));
    let home = non_utf8_temp_dir("non-utf8-home");
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&home);
    let cwd = root.join("cwd");
    fs::create_dir_all(&cwd).expect("create isolated cwd");
    fs::create_dir_all(&home).expect("create non-UTF-8 HOME");
    let fake_nix = root.join("nix");
    write_fake_nix(&fake_nix);

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "non_utf8_home_is_used_for_profile_diagnostics",
            "--nocapture",
        ])
        .current_dir(&cwd)
        .env(CHILD_CASE, "non-utf8-home")
        .env(CHILD_NIX, &fake_nix)
        .env_remove("XDG_STATE_HOME")
        .env("HOME", &home)
        .status()
        .expect("spawn isolated non-UTF-8 HOME diagnostics regression test");

    let _ = fs::remove_dir_all(&home);
    let _ = fs::remove_dir_all(&root);
    assert!(
        status.success(),
        "valid non-UTF-8 HOME must remain usable for profile diagnostics"
    );
}
