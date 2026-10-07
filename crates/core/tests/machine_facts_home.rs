use schneeforge_core::machine::MachineFacts;
use std::process::Command;

const CHILD_CASE: &str = "SCHNEEFORGE_MACHINE_FACTS_HOME_CHILD";

#[test]
fn relative_home_is_rejected_for_machine_facts() {
    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("isolated_machine_facts_home_child")
        .arg("--nocapture")
        .env(CHILD_CASE, "relative-home")
        .env("HOME", "relative-home")
        .output()
        .expect("run isolated machine-facts HOME test");

    assert!(
        output.status.success(),
        "isolated machine-facts HOME check failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn isolated_machine_facts_home_child() {
    let Ok(case) = std::env::var(CHILD_CASE) else {
        return;
    };
    assert_eq!(case, "relative-home");

    let err = MachineFacts::detect().expect_err("relative HOME must be rejected");
    assert!(
        err.to_string().contains("HOME") && err.to_string().contains("absolute"),
        "unexpected error: {err}"
    );
}
