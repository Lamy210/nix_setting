use schneeforge_core::machine::state_dir;
use std::path::PathBuf;
use std::process::Command;

const CHILD_CASE: &str = "SCHNEEFORGE_MACHINE_STATE_PATH_CHILD";

fn run_isolated(case: &str, xdg_state_home: Option<&str>, home: Option<&str>) {
    let mut command = Command::new(std::env::current_exe().expect("current test executable"));
    command
        .arg("--exact")
        .arg("isolated_state_dir_child")
        .arg("--nocapture")
        .env(CHILD_CASE, case);

    match xdg_state_home {
        Some(value) => {
            command.env("XDG_STATE_HOME", value);
        }
        None => {
            command.env_remove("XDG_STATE_HOME");
        }
    }
    match home {
        Some(value) => {
            command.env("HOME", value);
        }
        None => {
            command.env_remove("HOME");
        }
    }

    let output = command.output().expect("run isolated state-dir test");
    assert!(
        output.status.success(),
        "isolated state-dir check failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn state_dir_empty_xdg_state_home_falls_back_to_home() {
    run_isolated("empty-xdg", Some(""), Some("/home/u"));
}

#[test]
fn state_dir_empty_home_falls_back_to_current_directory() {
    run_isolated("empty-home", None, Some(""));
}

#[test]
fn isolated_state_dir_child() {
    let Ok(case) = std::env::var(CHILD_CASE) else {
        return;
    };

    let expected = match case.as_str() {
        "empty-xdg" => PathBuf::from("/home/u/.local/state/schneeforge"),
        "empty-home" => PathBuf::from(".").join("schneeforge"),
        other => panic!("unexpected child case: {other}"),
    };

    assert_eq!(state_dir(), expected);
}
