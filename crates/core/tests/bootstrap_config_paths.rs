#![cfg(unix)]

use schneeforge_core::{enable_flakes, ResolvedTool, ToolInventory, ToolSource};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const CHILD_MODE: &str = "SCHNEEFORGE_BOOTSTRAP_CONFIG_PATH_CHILD";
const FAKE_NIX: &str = "SCHNEEFORGE_BOOTSTRAP_CONFIG_PATH_FAKE_NIX";

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "schneeforge-bootstrap-config-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn fake_nix(dir: &Path) -> PathBuf {
    let path = dir.join("fake-nix");
    fs::write(
        &path,
        "#!/bin/sh\nprintf '%s\\n' 'experimental-features = nix-command'\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    path
}

fn inventory_from_env() -> ToolInventory {
    ToolInventory {
        nix: Some(ResolvedTool::new(
            PathBuf::from(std::env::var_os(FAKE_NIX).expect("fake nix path")),
            ToolSource::Path,
        )),
        git: None,
        homebrew: None,
        nh: None,
    }
}

fn spawn_child(
    test_name: &str,
    mode: &str,
    cwd: &Path,
    home: &str,
    xdg_config_home: Option<&str>,
    fake_nix: &Path,
) -> Output {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .current_dir(cwd)
        .env(CHILD_MODE, mode)
        .env(FAKE_NIX, fake_nix)
        .env("HOME", home);
    match xdg_config_home {
        Some(value) => {
            command.env("XDG_CONFIG_HOME", value);
        }
        None => {
            command.env_remove("XDG_CONFIG_HOME");
        }
    }
    command.output().unwrap()
}

#[test]
fn relative_xdg_config_home_falls_back_to_absolute_home() {
    if std::env::var(CHILD_MODE).as_deref() == Ok("relative-xdg") {
        enable_flakes(&inventory_from_env()).unwrap();
        return;
    }

    let root = temp_dir("relative-xdg");
    let cwd = root.join("cwd");
    let home = root.join("home");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(&home).unwrap();
    let nix = fake_nix(&root);

    let output = spawn_child(
        "relative_xdg_config_home_falls_back_to_absolute_home",
        "relative-xdg",
        &cwd,
        home.to_str().unwrap(),
        Some("relative-config"),
        &nix,
    );

    assert!(
        output.status.success(),
        "child failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        home.join(".config/nix/nix.conf").is_file(),
        "relative XDG_CONFIG_HOME must be ignored in favor of absolute HOME"
    );
    assert!(
        !cwd.join("relative-config/nix/nix.conf").exists(),
        "relative XDG_CONFIG_HOME must never make flakes config CWD-relative"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn relative_home_is_rejected_instead_of_writing_under_cwd() {
    if std::env::var(CHILD_MODE).as_deref() == Ok("relative-home") {
        let err = enable_flakes(&inventory_from_env()).unwrap_err();
        assert!(err.to_string().contains("HOME"), "{err}");
        return;
    }

    let root = temp_dir("relative-home");
    let cwd = root.join("cwd");
    fs::create_dir_all(&cwd).unwrap();
    let nix = fake_nix(&root);

    let output = spawn_child(
        "relative_home_is_rejected_instead_of_writing_under_cwd",
        "relative-home",
        &cwd,
        "relative-home",
        None,
        &nix,
    );

    assert!(
        output.status.success(),
        "child failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !cwd.join("relative-home/.config/nix/nix.conf").exists(),
        "relative HOME must never make flakes config CWD-relative"
    );

    let _ = fs::remove_dir_all(root);
}
