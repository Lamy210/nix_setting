#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use std::process::Command;

use schneeforge_core::{verify, ToolInventory};

const CHILD_CASE: &str = "SCHNEEFORGE_VERIFY_NON_UTF8_HOME_CHILD";

#[test]
fn verify_preserves_non_utf8_home_for_dotfile_checks() {
    if std::env::var_os(CHILD_CASE).is_some() {
        let home = PathBuf::from(std::env::var_os("HOME").expect("HOME"));
        assert!(
            home.to_str().is_none(),
            "test HOME must contain non-UTF-8 bytes"
        );

        let report = verify(".", &ToolInventory::default());
        for name in [".zshrc", ".gitconfig", "starship.toml"] {
            let check = report
                .checks
                .iter()
                .find(|check| check.name == name)
                .unwrap_or_else(|| panic!("missing verify check: {name}"));
            assert!(
                check.ok,
                "verify must preserve absolute non-UTF-8 HOME for {name}"
            );
        }
        return;
    }

    let mut home_bytes = format!(
        "/tmp/schneeforge-verify-non-utf8-home-{}-",
        std::process::id()
    )
    .into_bytes();
    home_bytes.push(0xff);
    let home = PathBuf::from(OsString::from_vec(home_bytes));
    let state_root = std::env::temp_dir().join(format!(
        "schneeforge-verify-non-utf8-state-{}",
        std::process::id()
    ));

    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&state_root);
    std::fs::create_dir_all(home.join(".config")).expect("create non-UTF-8 HOME");
    std::fs::create_dir_all(&state_root).expect("create isolated state root");
    std::fs::write(home.join(".zshrc"), "# non-UTF-8 HOME\n").expect("write .zshrc");
    std::fs::write(home.join(".gitconfig"), "[user]\n\tname = test\n").expect("write .gitconfig");
    std::fs::write(home.join(".config/starship.toml"), "# test\n").expect("write starship.toml");

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "verify_preserves_non_utf8_home_for_dotfile_checks",
            "--nocapture",
        ])
        .env(CHILD_CASE, "1")
        .env("HOME", home.as_os_str())
        .env("XDG_STATE_HOME", &state_root)
        .status()
        .expect("run isolated verify child");

    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&state_root);
    assert!(
        status.success(),
        "absolute non-UTF-8 HOME must remain usable for verify dotfile checks"
    );
}
