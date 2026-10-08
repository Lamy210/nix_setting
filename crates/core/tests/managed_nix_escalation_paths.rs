#![cfg(any(target_os = "linux", target_os = "macos"))]

use schneeforge_core::{escalate_command, EscalatedOp};
use std::path::Path;

#[test]
fn escalation_rejects_relative_cli_binary() {
    let error = escalate_command(
        Path::new("schneeforge"),
        EscalatedOp::Apply,
        Path::new("/tmp/nix_setting"),
    )
    .expect_err("privileged execution must reject a CWD-relative CLI binary");

    let message = error.to_string();
    assert!(
        message.contains("CLI binary") || message.contains("escalation"),
        "unexpected error: {message}"
    );
}

#[test]
fn escalation_rejects_relative_repo_directory() {
    let error = escalate_command(
        Path::new("/usr/local/bin/schneeforge"),
        EscalatedOp::Apply,
        Path::new("relative-repo"),
    )
    .expect_err("privileged execution must reject a CWD-relative repository path");

    let message = error.to_string();
    assert!(
        message.contains("repo dir") || message.contains("escalation"),
        "unexpected error: {message}"
    );
}
