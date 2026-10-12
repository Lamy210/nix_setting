#![cfg(all(unix, not(target_os = "macos")))]

use schneeforge_core::{ToolResolver, ToolSource};
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const CASE_ENV: &str = "SCHNEEFORGE_TOOL_NON_UTF8_TEST_CASE";
const EXPECTED_PATH_ENV: &str = "SCHNEEFORGE_TOOL_NON_UTF8_EXPECTED_PATH";

fn non_utf8_temp_dir(label: &str) -> PathBuf {
    let mut name = format!("sf-{label}-{}-", std::process::id()).into_bytes();
    name.push(0xff);
    std::env::temp_dir().join(OsString::from_vec(name))
}

fn write_executable(dir: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, b"#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn expected_path() -> PathBuf {
    std::env::var_os(EXPECTED_PATH_ENV)
        .map(PathBuf::from)
        .expect("child must receive expected path")
}

fn run_child(test_name: &str, case: &str, expected: &Path, configure: impl FnOnce(&mut Command)) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env(CASE_ENV, case)
        .env(EXPECTED_PATH_ENV, expected);
    configure(&mut command);

    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn resolver_preserves_non_utf8_env_override_path() {
    if std::env::var(CASE_ENV).as_deref() == Ok("env-override") {
        let resolved = ToolResolver::with_known_paths(vec![])
            .resolve_tool("mytool-non-utf8-env")
            .expect("non-UTF-8 env override must remain a usable OS path");
        assert_eq!(resolved.source, ToolSource::EnvOverride);
        assert_eq!(
            resolved.path,
            std::fs::canonicalize(expected_path()).unwrap()
        );
        return;
    }

    let dir = non_utf8_temp_dir("tool-env");
    let tool = write_executable(&dir, "mytool-non-utf8-env");
    run_child(
        "resolver_preserves_non_utf8_env_override_path",
        "env-override",
        &tool,
        |command| {
            command.env("SCHNEEFORGE_MYTOOL_NON_UTF8_ENV_BIN", &tool);
        },
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn resolver_preserves_non_utf8_path_directory() {
    if std::env::var(CASE_ENV).as_deref() == Ok("path") {
        let resolved = ToolResolver::with_known_paths(vec![])
            .resolve_tool("mytool-non-utf8-path")
            .expect("non-UTF-8 PATH directory must remain searchable");
        assert_eq!(resolved.source, ToolSource::Path);
        assert_eq!(
            resolved.path,
            std::fs::canonicalize(expected_path()).unwrap()
        );
        return;
    }

    let dir = non_utf8_temp_dir("tool-path");
    let tool = write_executable(&dir, "mytool-non-utf8-path");
    let path = std::env::join_paths([&dir]).unwrap();
    run_child(
        "resolver_preserves_non_utf8_path_directory",
        "path",
        &tool,
        |command| {
            command.env("PATH", path);
        },
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn resolver_preserves_non_utf8_xdg_state_home_known_path() {
    if std::env::var(CASE_ENV).as_deref() == Ok("xdg-state-home") {
        let resolved = ToolResolver::new()
            .resolve_tool("mytool-non-utf8-xdg")
            .expect("non-UTF-8 XDG_STATE_HOME known path must remain searchable");
        assert_eq!(resolved.source, ToolSource::XdgStateProfile);
        assert_eq!(
            resolved.path,
            std::fs::canonicalize(expected_path()).unwrap()
        );
        return;
    }

    let state_home = non_utf8_temp_dir("tool-xdg-state-home");
    let tool = write_executable(
        &state_home.join("nix/profile/bin"),
        "mytool-non-utf8-xdg",
    );
    run_child(
        "resolver_preserves_non_utf8_xdg_state_home_known_path",
        "xdg-state-home",
        &tool,
        |command| {
            command.env("XDG_STATE_HOME", &state_home);
        },
    );
    let _ = std::fs::remove_dir_all(&state_home);
}

#[test]
fn resolver_preserves_non_utf8_nix_profile_known_path() {
    if std::env::var(CASE_ENV).as_deref() == Ok("nix-profile") {
        let resolved = ToolResolver::new()
            .resolve_tool("mytool-non-utf8-nix-profile")
            .expect("non-UTF-8 NIX_PROFILE known path must remain searchable");
        assert_eq!(resolved.source, ToolSource::NixProfileEnv);
        assert_eq!(
            resolved.path,
            std::fs::canonicalize(expected_path()).unwrap()
        );
        return;
    }

    let profile = non_utf8_temp_dir("tool-nix-profile");
    let tool = write_executable(&profile.join("bin"), "mytool-non-utf8-nix-profile");
    run_child(
        "resolver_preserves_non_utf8_nix_profile_known_path",
        "nix-profile",
        &tool,
        |command| {
            command.env("NIX_PROFILE", &profile);
        },
    );
    let _ = std::fs::remove_dir_all(&profile);
}

#[test]
fn resolver_preserves_non_utf8_home_nix_profile_known_path() {
    if std::env::var(CASE_ENV).as_deref() == Ok("home") {
        let resolved = ToolResolver::new()
            .resolve_tool("mytool-non-utf8-home")
            .expect("non-UTF-8 HOME known path must remain searchable");
        assert_eq!(resolved.source, ToolSource::NixProfileHome);
        assert_eq!(
            resolved.path,
            std::fs::canonicalize(expected_path()).unwrap()
        );
        return;
    }

    let home = non_utf8_temp_dir("tool-home");
    let tool = write_executable(&home.join(".nix-profile/bin"), "mytool-non-utf8-home");
    run_child(
        "resolver_preserves_non_utf8_home_nix_profile_known_path",
        "home",
        &tool,
        |command| {
            command.env("HOME", &home);
            command.env_remove("XDG_STATE_HOME");
            command.env_remove("NIX_PROFILE");
        },
    );
    let _ = std::fs::remove_dir_all(&home);
}
