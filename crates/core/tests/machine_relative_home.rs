use schneeforge_core::MachineFacts;
use std::fs;
use std::process::{self, Command};

const CHILD: &str = "SCHNEEFORGE_MACHINE_RELATIVE_HOME_CHILD";

#[test]
fn machine_facts_reject_relative_home() {
    if std::env::var_os(CHILD).is_some() {
        let err = MachineFacts::detect().expect_err("relative HOME must be rejected");
        assert!(
            err.to_string().contains("absolute"),
            "relative HOME rejection should explain the absolute-path requirement: {err}"
        );
        return;
    }

    let root = std::env::temp_dir().join(format!(
        "schneeforge-machine-relative-home-{}",
        process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let work = root.join("work");
    fs::create_dir_all(&work).expect("create isolated working directory");

    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("machine_facts_reject_relative_home")
        .arg("--nocapture")
        .current_dir(&work)
        .env(CHILD, "1")
        .env("HOME", "relative-home")
        .output()
        .expect("run isolated machine facts test");

    let _ = fs::remove_dir_all(&root);
    assert!(
        output.status.success(),
        "MachineFacts accepted a CWD-relative HOME:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
