#![cfg(all(unix, not(target_os = "macos")))]

use schneeforge_core::{nix_health, ResolvedTool, ToolInventory, ToolSource};
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const CHILD_CASE: &str = "SCHNEEFORGE_DIAGNOSTICS_NON_UTF8_XDG_CHILD";
const CHILD_NIX: &str = "SCHNEEFORGE_DIAGNOSTICS_NON_UTF8_XDG_NIX";

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

fn non_utf8_state_home(root: &Path) -> PathBuf {
    let mut name = b"state-".to_vec();
    name.push(0xff);
    root.join(OsString::from_vec(name))
}

#[test]
fn non_utf8_xdg_state_home_uses_existing_profile_dir_without_warning() {
    if std::env::var(CHILD_CASE).as_deref() == Ok("run") {
        let nix = std::env::var_os(CHILD_NIX)
            .map(PathBuf::from)
            .expect("child fake nix path");
        let health = nix_health(&inventory(nix));
        assert!(health.installed, "fake nix should be detected");
        assert!(health.store_accessible, "fake nix store ping should succeed");
        assert!(
            health.flakes_available,
            "fake nix should report required features"
        );
        assert_eq!(
            health.warning, None,
            "an existing profile directory under non-UTF-8 XDG_STATE_HOME must suppress the missing-state warning"
        );
        return;
    }

    let root = std::env::temp_dir().join(format!(
        "schneeforge-diagnostics-non-utf8-xdg-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let cwd = root.join("cwd");
    let home = root.join("home");
    let state_home = non_utf8_state_home(&root);
    fs::create_dir_all(&cwd).expect("create isolated cwd");
    fs::create_dir_all(&home).expect("create fallback HOME");
    fs::create_dir_all(state_home.join("nix/profiles")).expect("create XDG state profile dir");
    let fake_nix = root.join("nix");
    write_fake_nix(&fake_nix);

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "non_utf8_xdg_state_home_uses_existing_profile_dir_without_warning",
            "--nocapture",
        ])
        .current_dir(&cwd)
        .env(CHILD_CASE, "run")
        .env(CHILD_NIX, &fake_nix)
        .env("XDG_STATE_HOME", &state_home)
        .env("HOME", &home)
        .status()
        .expect("spawn isolated diagnostics regression test");

    let _ = fs::remove_dir_all(&root);
    assert!(
        status.success(),
        "diagnostics must preserve the non-UTF-8 XDG_STATE_HOME filesystem path"
    );
}
