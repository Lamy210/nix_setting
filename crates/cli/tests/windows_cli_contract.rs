use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn backend_info_probe_is_hidden_and_machine_readable() {
    let mut help = Command::cargo_bin("schneeforge").unwrap();
    help.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("__backend-info").not());

    let mut probe = Command::cargo_bin("schneeforge").unwrap();
    probe
        .arg("__backend-info")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{"))
        .stdout(predicate::str::contains("\"protocol_version\":"))
        .stdout(predicate::str::contains(format!(
            "\"app_version\":\"{}\"",
            env!("CARGO_PKG_VERSION")
        )))
        .stdout(predicate::str::contains("\"os\":\"linux\""))
        .stdout(predicate::str::contains("\"arch\":"));
}

#[test]
fn wsl_distro_is_global_but_does_not_change_native_linux_status() {
    let mut help = Command::cargo_bin("schneeforge").unwrap();
    help.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--wsl-distro"));

    let state = std::env::temp_dir().join(format!(
        "schneeforge-wsl-selector-native-{}",
        std::process::id()
    ));
    let mut status = Command::cargo_bin("schneeforge").unwrap();
    status
        .args(["--wsl-distro", "Ubuntu Dev", "status"])
        .env("XDG_STATE_HOME", state)
        .assert()
        .success()
        .stdout(predicate::str::contains("host:"));
}
