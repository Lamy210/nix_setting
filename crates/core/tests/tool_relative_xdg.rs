#![cfg(unix)]

use schneeforge_core::tool::{ToolResolver, ToolSource};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn make_executable(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fake tool parent");
    }
    fs::write(path, b"#!/bin/sh\nexit 0\n").expect("write fake tool");
    let mut permissions = fs::metadata(path).expect("fake tool metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make fake tool executable");
}

#[test]
fn relative_xdg_state_home_is_ignored_for_tool_resolution() {
    let root = std::env::temp_dir().join(format!(
        "schneeforge-tool-relative-xdg-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let cwd = root.join("cwd");
    let home = root.join("home");
    fs::create_dir_all(&cwd).expect("create isolated cwd");
    fs::create_dir_all(&home).expect("create isolated home");

    let tool = "sf-relative-xdg-probe";
    let cwd_probe = cwd.join("relative-state/nix/profile/bin").join(tool);
    let home_probe = home.join(".local/state/nix/profile/bin").join(tool);
    make_executable(&cwd_probe);
    make_executable(&home_probe);

    std::env::set_current_dir(&cwd).expect("enter isolated cwd");
    std::env::set_var("XDG_STATE_HOME", "relative-state");
    std::env::set_var("HOME", &home);
    std::env::set_var("NIX_PROFILE", "");
    std::env::set_var("USER", "");
    std::env::set_var("PATH", "");
    std::env::remove_var("SCHNEEFORGE_SF_RELATIVE_XDG_PROBE_BIN");

    let resolved = ToolResolver::new()
        .resolve_tool(tool)
        .expect("absolute HOME fallback should resolve the probe");

    assert_eq!(resolved.source, ToolSource::XdgStateProfile);
    assert_eq!(
        resolved.path,
        fs::canonicalize(&home_probe).expect("canonical HOME probe"),
        "relative XDG_STATE_HOME must not redirect tool discovery into the process CWD"
    );
}
