#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use schneeforge_core::{escalate_command, EscalatedOp};

fn non_utf8_absolute_path(prefix: &[u8]) -> PathBuf {
    let mut bytes = b"/tmp/".to_vec();
    bytes.extend_from_slice(prefix);
    bytes.push(0xff);
    PathBuf::from(OsString::from_vec(bytes))
}

#[test]
fn escalation_rejects_non_utf8_cli_binary_path() {
    let cli = non_utf8_absolute_path(b"schneeforge-cli-");

    let err = escalate_command(&cli, EscalatedOp::Apply, Path::new("/tmp/schneeforge-repo"))
        .expect_err("non-UTF8 CLI path must be rejected before privilege escalation");

    assert!(
        err.to_string().contains("CLI binary path must be valid UTF-8"),
        "unexpected error: {err}"
    );
}

#[test]
fn escalation_rejects_non_utf8_repo_path() {
    let repo = non_utf8_absolute_path(b"schneeforge-repo-");

    let err = escalate_command(Path::new("/usr/bin/schneeforge"), EscalatedOp::Apply, &repo)
        .expect_err("non-UTF8 repository path must be rejected before privilege escalation");

    assert!(
        err.to_string().contains("repo dir must be valid UTF-8"),
        "unexpected error: {err}"
    );
}
