#![cfg(target_os = "macos")]

use assert_cmd::Command;
use predicates::prelude::*;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

#[test]
fn non_utf8_absolute_home_still_blocks_uninstall_when_nix_darwin_marker_exists() {
    let mut name = format!(
        "schneeforge-non-utf8-home-{}-",
        std::process::id()
    )
    .into_bytes();
    name.push(0xff);
    let home = std::env::temp_dir().join(OsString::from_vec(name));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(home.join(".nix-darwin")).unwrap();

    let receipt = home.join("receipt.json");
    std::fs::write(&receipt, "{}").unwrap();

    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("nix")
        .arg("uninstall")
        .arg("--receipt")
        .arg(&receipt)
        .arg("--force")
        .env("HOME", home.as_os_str())
        .assert()
        .failure()
        .stderr(predicate::str::contains("nix-darwin detected"))
        .stderr(predicate::str::contains("not running as root").not());

    let _ = std::fs::remove_dir_all(&home);
}
