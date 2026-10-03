use assert_cmd::Command;
use predicates::prelude::*;

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

#[test]
fn deprecated_upgrade_rejects_managed_source_like_deps_update() {
    let dir = isolated_dir("upgrade-managed-source");
    let repo = dir.join("repo");
    std::fs::create_dir_all(&repo).unwrap();

    let state_dir = dir.join("schneeforge");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(
        state_dir.join("state.json"),
        r#"{"source":{"kind":"release-stable","ref":"v0.2.0","channel":"stable","managed":true,"remote":"https://github.com/Lamy210/nix_setting.git"}}"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(&repo)
        .arg("upgrade")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "flake.lock of a managed (github flake ref) source cannot be updated locally",
        ));

    let _ = std::fs::remove_dir_all(&dir);
}
