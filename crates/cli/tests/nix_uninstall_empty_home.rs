#[cfg(target_os = "macos")]
use assert_cmd::Command;
#[cfg(target_os = "macos")]
use predicates::prelude::*;

#[cfg(target_os = "macos")]
fn isolated_dir(name: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("schneeforge-{name}-{}-{nonce}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(target_os = "macos")]
#[test]
fn empty_home_does_not_treat_cwd_nix_darwin_marker_as_home_marker() {
    let dir = isolated_dir("empty-home-nix-darwin-marker");
    std::fs::create_dir_all(dir.join(".nix-darwin")).unwrap();
    let receipt = dir.join("receipt.json");
    std::fs::write(&receipt, "{}").unwrap();

    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.current_dir(&dir)
        .arg("nix")
        .arg("uninstall")
        .arg("--receipt")
        .arg(&receipt)
        .arg("--force")
        .env("HOME", "")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not running as root"))
        .stderr(predicate::str::contains("nix-darwin detected").not());

    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(target_os = "macos")]
#[test]
fn relative_home_does_not_treat_cwd_relative_nix_darwin_marker_as_home_marker() {
    let dir = isolated_dir("relative-home-nix-darwin-marker");
    let relative_home = "relative-home";
    std::fs::create_dir_all(dir.join(relative_home).join(".nix-darwin")).unwrap();
    let receipt = dir.join("receipt.json");
    std::fs::write(&receipt, "{}").unwrap();

    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.current_dir(&dir)
        .arg("nix")
        .arg("uninstall")
        .arg("--receipt")
        .arg(&receipt)
        .arg("--force")
        .env("HOME", relative_home)
        .assert()
        .failure()
        .stderr(predicate::str::contains("not running as root"))
        .stderr(predicate::str::contains("nix-darwin detected").not());

    let _ = std::fs::remove_dir_all(&dir);
}
