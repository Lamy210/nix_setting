use schneeforge_core::state::State;
use std::path::PathBuf;
use std::process::Command;

const CHILD_CASE: &str = "SCHNEEFORGE_STATE_PATH_CHILD";

fn run_isolated(case: &str, xdg_state_home: Option<&str>, home: Option<&str>) {
    let mut command = Command::new(std::env::current_exe().expect("current test executable"));
    command
        .arg("--exact")
        .arg("isolated_state_path_child")
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

    let output = command.output().expect("run isolated state-path test");
    assert!(
        output.status.success(),
        "isolated state-path check failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn state_default_path_relative_home_falls_back_to_current_directory() {
    run_isolated("relative-home", None, Some("relative-home"));
}

#[test]
fn isolated_state_path_child() {
    let Ok(case) = std::env::var(CHILD_CASE) else {
        return;
    };

    let expected = match case.as_str() {
        "relative-home" => PathBuf::from(".").join("schneeforge").join("state.json"),
        other => panic!("unexpected child case: {other}"),
    };

    assert_eq!(State::default_path(), expected);
}
