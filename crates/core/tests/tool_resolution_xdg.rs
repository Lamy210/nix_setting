use schneeforge_core::{ToolResolver, ToolSource};
use std::{env, fs, path::Path, process};

fn make_executable(path: &Path) {
    fs::create_dir_all(path.parent().expect("tool parent")).unwrap();
    fs::write(path, b"#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }
}

#[test]
fn relative_xdg_state_home_is_ignored_for_tool_resolution() {
    let root = env::temp_dir().join(format!("schneeforge-tool-relative-xdg-{}", process::id()));
    let _ = fs::remove_dir_all(&root);

    let work = root.join("work");
    let home = root.join("home");
    let tool_name = "schneeforge-relative-xdg-tool-test";
    let relative_tool = work
        .join("relative-state")
        .join("nix/profile/bin")
        .join(tool_name);
    let home_tool = home.join(".local/state/nix/profile/bin").join(tool_name);
    make_executable(&relative_tool);
    make_executable(&home_tool);

    let original_dir = env::current_dir().unwrap();
    let original_xdg = env::var_os("XDG_STATE_HOME");
    let original_home = env::var_os("HOME");

    env::set_current_dir(&work).unwrap();
    env::set_var("XDG_STATE_HOME", "relative-state");
    env::set_var("HOME", &home);

    let resolved = ToolResolver::new()
        .resolve_tool(tool_name)
        .expect("HOME fallback tool should resolve");

    env::set_current_dir(original_dir).unwrap();
    match original_xdg {
        Some(value) => env::set_var("XDG_STATE_HOME", value),
        None => env::remove_var("XDG_STATE_HOME"),
    }
    match original_home {
        Some(value) => env::set_var("HOME", value),
        None => env::remove_var("HOME"),
    }

    assert_eq!(resolved.source, ToolSource::XdgStateProfile);
    assert_eq!(resolved.path, fs::canonicalize(&home_tool).unwrap());

    let _ = fs::remove_dir_all(root);
}
