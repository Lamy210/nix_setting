use schneeforge_core::tool::ToolResolver;
use std::fs;
use std::path::Path;

fn make_executable(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
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
fn empty_home_does_not_enable_cwd_relative_nix_profile_discovery() {
    let dir = std::env::temp_dir().join(format!(
        "schneeforge-tool-empty-home-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let probe = dir.join(".nix-profile/bin/sf-empty-home-probe");
    make_executable(&probe);

    std::env::set_current_dir(&dir).unwrap();
    std::env::set_var("HOME", "");
    std::env::set_var("XDG_STATE_HOME", "");
    std::env::set_var("NIX_PROFILE", "");
    std::env::set_var("USER", "");
    std::env::set_var("PATH", "");
    std::env::remove_var("SCHNEEFORGE_SF_EMPTY_HOME_PROBE_BIN");

    let resolved = ToolResolver::new().resolve_tool("sf-empty-home-probe");

    assert!(
        resolved.is_none(),
        "empty HOME must not make ~/.nix-profile resolve relative to the process CWD: {resolved:?}"
    );
}
