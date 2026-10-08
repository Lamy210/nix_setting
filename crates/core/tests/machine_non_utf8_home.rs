#![cfg(unix)]

use schneeforge_core::machine::MachineFacts;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

#[test]
fn machine_facts_rejects_non_utf8_home() {
    let original_home = std::env::var_os("HOME");
    let non_utf8_home = OsString::from_vec(b"/tmp/schneeforge-home-\xff".to_vec());
    std::env::set_var("HOME", &non_utf8_home);

    let detected = MachineFacts::detect();

    match original_home {
        Some(home) => std::env::set_var("HOME", home),
        None => std::env::remove_var("HOME"),
    }

    let err = detected.expect_err("non-UTF8 HOME must be rejected before machine.nix generation");
    assert!(
        err.to_string().contains("HOME must be valid UTF-8"),
        "unexpected error: {err}"
    );
}
