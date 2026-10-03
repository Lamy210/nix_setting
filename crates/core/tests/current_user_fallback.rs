use std::process::Command;

const CHILD_ENV: &str = "SCHNEEFORGE_CURRENT_USER_FALLBACK_CHILD";

#[test]
fn current_user_falls_back_to_logname_when_user_is_empty() {
    if std::env::var_os(CHILD_ENV).is_some() {
        assert_eq!(schneeforge_core::current_user().as_deref(), Some("alice"));
        return;
    }

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "current_user_falls_back_to_logname_when_user_is_empty",
            "--nocapture",
        ])
        .env(CHILD_ENV, "1")
        .env("USER", "")
        .env("LOGNAME", "alice")
        .status()
        .expect("spawn isolated current_user regression test");

    assert!(
        status.success(),
        "current_user should ignore an empty USER and fall back to LOGNAME"
    );
}
