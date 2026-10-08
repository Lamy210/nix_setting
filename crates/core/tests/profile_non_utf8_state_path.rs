#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

use schneeforge_core::{profile, state::StateStore};

#[test]
fn profile_override_rejects_non_utf8_state_paths() {
    let repo = std::env::temp_dir().join(format!(
        "schneeforge-profile-non-utf8-state-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(
        repo.join("schneeforge.toml"),
        "schema = 1\n[profiles]\ndefault = \"developer\"\navailable = [\"minimal\", \"developer\"]\n",
    )
    .unwrap();
    let store = StateStore::new(repo.join("isolated-state.json"));

    let mut state_home_bytes = format!(
        "/tmp/schneeforge-profile-non-utf8-state-{}-",
        std::process::id()
    )
    .into_bytes();
    state_home_bytes.push(0xff);
    let state_home = PathBuf::from(OsString::from_vec(state_home_bytes));
    let original_xdg_state_home = std::env::var_os("XDG_STATE_HOME");
    std::env::set_var("XDG_STATE_HOME", state_home.as_os_str());

    let result = profile::override_args_with(repo.to_str().unwrap(), &store);

    match original_xdg_state_home {
        Some(value) => std::env::set_var("XDG_STATE_HOME", value),
        None => std::env::remove_var("XDG_STATE_HOME"),
    }
    let _ = std::fs::remove_dir_all(&state_home);
    let _ = std::fs::remove_dir_all(&repo);

    let err = result.expect_err(
        "non-UTF8 state paths must be rejected instead of rewritten in Nix path: overrides",
    );
    assert!(
        err.to_string().contains("UTF-8"),
        "unexpected error: {err}"
    );
}
