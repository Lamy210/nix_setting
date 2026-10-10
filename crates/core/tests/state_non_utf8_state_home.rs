#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use std::process::Command;

use schneeforge_core::state::State;

const CHILD_CASE: &str = "SCHNEEFORGE_STATE_NON_UTF8_CHILD";

#[test]
fn state_default_path_preserves_non_utf8_xdg_state_home() {
    if std::env::var(CHILD_CASE).as_deref() == Ok("non-utf8-xdg") {
        let state_home = PathBuf::from(std::env::var_os("XDG_STATE_HOME").expect("XDG_STATE_HOME"));
        assert!(
            state_home.to_str().is_none(),
            "test XDG_STATE_HOME must be non-UTF8"
        );
        assert_eq!(
            State::default_path(),
            state_home.join("schneeforge/state.json"),
            "State::default_path must preserve the exact XDG_STATE_HOME filesystem path"
        );
        return;
    }

    let mut bytes = format!(
        "/tmp/schneeforge-state-non-utf8-xdg-{}-",
        std::process::id()
    )
    .into_bytes();
    bytes.push(0xff);
    let state_home = PathBuf::from(OsString::from_vec(bytes));
    let fallback_home = format!(
        "/tmp/schneeforge-state-fallback-home-{}",
        std::process::id()
    );

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "state_default_path_preserves_non_utf8_xdg_state_home",
            "--nocapture",
        ])
        .env(CHILD_CASE, "non-utf8-xdg")
        .env("XDG_STATE_HOME", state_home.as_os_str())
        .env("HOME", fallback_home)
        .status()
        .expect("spawn isolated state path regression test");

    assert!(
        status.success(),
        "non-UTF8 XDG_STATE_HOME must not be treated as unset or rewritten"
    );
}
