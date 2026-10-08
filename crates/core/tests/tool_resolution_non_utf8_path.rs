#![cfg(target_os = "linux")]

use schneeforge_core::tool::ToolResolver;
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
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

fn create_executable(dir: &Path, name: &str) {
    fs::create_dir_all(dir).expect("create test directory");
    let path = dir.join(name);
    fs::write(&path, b"#!/bin/sh\nexit 0\n").expect("write executable");
    let mut permissions = fs::metadata(&path).expect("read metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("set executable bit");
}

#[test]
fn tool_resolver_rejects_non_utf8_path_entries() {
    let tool = "sf-non-utf8-path";
    let root = unique_temp_dir("tool-resolver-non-utf8");

    let mut raw_dir = root.as_os_str().as_bytes().to_vec();
    raw_dir.extend_from_slice(b"-\xff");
    let non_utf8_dir = PathBuf::from(OsString::from_vec(raw_dir));
    assert!(
        non_utf8_dir.to_str().is_none(),
        "test path must be non-UTF8"
    );

    let lossy_dir = PathBuf::from(non_utf8_dir.to_string_lossy().into_owned());
    create_executable(&lossy_dir, tool);

    let original_path = std::env::var_os("PATH");
    std::env::set_var("PATH", &non_utf8_dir);

    let resolver = ToolResolver::with_known_paths(Vec::new());
    let resolved = resolver.resolve_tool(tool);

    match original_path {
        Some(path) => std::env::set_var("PATH", path),
        None => std::env::remove_var("PATH"),
    }
    let _ = fs::remove_dir_all(&lossy_dir);

    assert!(
        resolved.is_none(),
        "a non-UTF8 PATH entry must not be rewritten to a different UTF-8 directory"
    );
}
