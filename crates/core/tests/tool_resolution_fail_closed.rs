#![cfg(target_os = "linux")]

use schneeforge_core::tool::{ToolResolver, ToolSource};
use std::fs;
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "schneeforge-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn create_executable(dir: &Path, name: &str) -> PathBuf {
    fs::create_dir_all(dir).expect("create test directory");
    let path = dir.join(name);
    fs::write(&path, b"#!/bin/sh\nexit 0\n").expect("write executable");
    let mut permissions = fs::metadata(&path).expect("read metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("set executable bit");
    path
}

#[test]
fn env_override_canonicalization_failure_is_fail_closed() {
    let tool = "sf-canonicalize-fail-closed";
    let env_key = "SCHNEEFORGE_SF_CANONICALIZE_FAIL_CLOSED_BIN";

    let source_dir = unique_temp_dir("canonicalize-source");
    let source_path = create_executable(&source_dir, tool);
    let held_file = fs::File::open(&source_path).expect("hold executable open");
    fs::remove_file(&source_path).expect("unlink held executable");

    // Linux keeps an unlinked-but-open file executable through /proc/self/fd, while
    // realpath/canonicalize fails because the symlink target is marked "(deleted)".
    // This gives us a deterministic TOCTOU-like candidate without a timing race.
    let uncanonicalizable = PathBuf::from(format!("/proc/self/fd/{}", held_file.as_raw_fd()));
    assert!(
        uncanonicalizable.is_file(),
        "fd path should still be a file"
    );
    assert!(
        fs::canonicalize(&uncanonicalizable).is_err(),
        "test precondition: fd path must not have a canonical real path"
    );

    // A lower-priority candidate proves canonicalization failure must terminate
    // resolution instead of silently falling through to another installation.
    let fallback_dir = unique_temp_dir("canonicalize-fallback");
    let _fallback = create_executable(&fallback_dir, tool);
    let resolver =
        ToolResolver::with_known_paths(vec![(fallback_dir.clone(), ToolSource::SystemProfile)]);

    std::env::set_var(env_key, &uncanonicalizable);
    let resolved = resolver.resolve_tool(tool);
    std::env::remove_var(env_key);

    assert!(
        resolved.is_none(),
        "an executable env override whose canonicalization fails must fail closed"
    );

    drop(held_file);
    let _ = fs::remove_dir_all(source_dir);
    let _ = fs::remove_dir_all(fallback_dir);
}
