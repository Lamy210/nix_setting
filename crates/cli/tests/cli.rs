use assert_cmd::Command;
use predicates::prelude::*;

/// このテストファイル内の "nix 必須" テストは環境に nix が無い場合は skip する
fn nix_available() -> bool {
    std::process::Command::new("nix")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

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
    if !nix_available() {
        eprintln!("skipping: nix not installed");
        return;
    }
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("[system]").and(predicate::str::contains("host")));
}

/// status は Toolchain 解決を必要としないので nix 無しでも動く
#[test]
fn status_runs_without_nix() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("host:"));
}

#[test]
fn scan_runs() {
    if !nix_available() {
        eprintln!("skipping: nix not installed");
        return;
    }
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
    if !nix_available() {
        eprintln!("skipping: nix not installed");
        return;
    }
    // --repo を明示すると target 表示までは進む (nix build は未実行)
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("plan")
        .assert()
        .stdout(predicate::str::contains("target:"));
}

/// uninstall は info 系なので Toolchain 無しで動く
#[test]
fn uninstall_runs_without_nix() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("uninstall")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== uninstall ==="));
}

/// doctor を nix 無し環境で実行すると、toolchain 解決エラーで非ゼロ終了する
#[test]
fn doctor_fails_gracefully_without_nix() {
    if nix_available() {
        eprintln!("skipping: nix is installed (test is for no-nix env)");
        return;
    }
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("doctor").assert().failure().stderr(
        predicate::str::contains("nix not found").or(predicate::str::contains("not found")),
    );
}
