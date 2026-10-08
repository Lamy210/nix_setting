#![cfg(unix)]

use schneeforge_core::{ToolResolver, ToolSource};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output};

const CHILD_CASE: &str = "SCHNEEFORGE_TOOL_ABSOLUTE_NIX_PROFILE_CHILD";
const CHILD_TOOL: &str = "SCHNEEFORGE_TOOL_ABSOLUTE_NIX_PROFILE_NAME";
const CHILD_PROFILE: &str = "SCHNEEFORGE_TOOL_ABSOLUTE_NIX_PROFILE_PATH";

fn make_executable(path: &Path) {
    fs::create_dir_all(path.parent().expect("tool parent")).unwrap();
    fs::write(path, b"#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn fixture(case: &str) -> (PathBuf, PathBuf, PathBuf, String) {
    let root = std::env::temp_dir().join(format!(
        "schneeforge-tool-absolute-nix-profile-{}-{case}",
        process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let work = root.join("work");
    let profile = root.join("profile");
    let tool_name = format!(
        "schneeforge-absolute-nix-profile-tool-{}-{case}",
        process::id()
    );
    make_executable(&profile.join("bin").join(&tool_name));
    fs::create_dir_all(&work).unwrap();
    (root, work, profile, tool_name)
}

fn run_rust_isolated(case: &str) {
    let (root, work, profile, tool_name) = fixture(case);

    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("isolated_absolute_nix_profile_tool_resolution_child")
        .arg("--nocapture")
        .current_dir(&work)
        .env(CHILD_CASE, case)
        .env(CHILD_TOOL, &tool_name)
        .env(CHILD_PROFILE, &profile)
        .env("HOME", "")
        .env("XDG_STATE_HOME", "")
        .env("NIX_PROFILE", &profile)
        .env("USER", "")
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("run isolated absolute-NIX_PROFILE tool-resolution test");

    let _ = fs::remove_dir_all(&root);
    assert!(
        output.status.success(),
        "isolated absolute-NIX_PROFILE tool-resolution check failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_shell_resolver(case: &str, inline_install: bool) -> (Output, PathBuf, PathBuf) {
    let (root, work, profile, tool_name) = fixture(case);
    let expected = profile.join("bin").join(&tool_name);
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let shared_resolver = repo_root.join("scripts/resolve-tools.sh");
    let install_script = repo_root.join("install.sh");

    let shell = if inline_install {
        r#"
eval "$(sed -n '1,/^# --- end inline resolver ---$/p' "$INSTALL_SCRIPT")"
resolve_tool "$TOOL_NAME" || exit $?
upper="$(printf '%s' "$TOOL_NAME" | tr '[:lower:]-' '[:upper:]_')"
out_var="${upper}_BIN"
printf '%s' "${!out_var}"
"#
    } else {
        r#"
. "$RESOLVER_SCRIPT"
resolve_tool "$TOOL_NAME" || exit $?
upper="$(printf '%s' "$TOOL_NAME" | tr '[:lower:]-' '[:upper:]_')"
out_var="${upper}_BIN"
printf '%s' "${!out_var}"
"#
    };

    let output = Command::new("bash")
        .arg("-c")
        .arg(shell)
        .current_dir(&work)
        .env("RESOLVER_SCRIPT", &shared_resolver)
        .env("INSTALL_SCRIPT", &install_script)
        .env("TOOL_NAME", &tool_name)
        .env("HOME", "")
        .env("XDG_STATE_HOME", "")
        .env("NIX_PROFILE", &profile)
        .env("USER", "")
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("run shell absolute-NIX_PROFILE tool-resolution test");

    (output, expected, root)
}

fn assert_shell_uses_absolute_nix_profile(case: &str, inline_install: bool) {
    let (output, expected, root) = run_shell_resolver(case, inline_install);
    assert!(
        output.status.success(),
        "shell resolver rejected an absolute NIX_PROFILE:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected.to_string_lossy(),
        "absolute NIX_PROFILE must resolve the profile-local executable"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn absolute_nix_profile_is_used_by_rust_resolver() {
    run_rust_isolated("rust");
}

#[test]
fn shared_shell_resolver_uses_absolute_nix_profile() {
    assert_shell_uses_absolute_nix_profile("shared", false);
}

#[test]
fn install_inline_resolver_uses_absolute_nix_profile() {
    assert_shell_uses_absolute_nix_profile("inline", true);
}

#[test]
fn isolated_absolute_nix_profile_tool_resolution_child() {
    if std::env::var(CHILD_CASE).is_err() {
        return;
    }
    let tool_name = std::env::var(CHILD_TOOL).expect("child tool name");
    let profile = PathBuf::from(std::env::var_os(CHILD_PROFILE).expect("child profile path"));
    let expected = profile.join("bin").join(&tool_name);
    let resolved = ToolResolver::new()
        .resolve_tool(&tool_name)
        .expect("absolute NIX_PROFILE candidate must resolve");
    assert_eq!(resolved.path, expected);
    assert_eq!(resolved.source, ToolSource::NixProfileEnv);
}
