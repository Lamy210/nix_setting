use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_commands() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--help").assert().success().stdout(
        predicate::str::contains("Declarative Developer Workstation Manager")
            .and(predicate::str::contains("doctor"))
            .and(predicate::str::contains("apply"))
            .and(predicate::str::contains("rollback")),
    );
}

#[test]
fn version_prints_semver() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"schneeforge \d+\.\d+\.\d+").unwrap());
}

#[test]
fn doctor_runs() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("[system]").and(predicate::str::contains("host")));
}

#[test]
fn status_runs() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("host:"));
}

#[test]
fn scan_runs() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("scan")
        .assert()
        .success()
        .stdout(predicate::str::contains("OS:").and(predicate::str::contains("arch:")));
}

#[test]
fn status_respects_repo_flag() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg("/nonexistent")
        .arg("status")
        .assert()
        .success();
}

#[test]
fn plan_shows_target_with_repo() {
    // --repo を明示すると target 表示までは進む (nix build は未実行)
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("plan")
        .assert()
        .stdout(predicate::str::contains("target:"));
}
