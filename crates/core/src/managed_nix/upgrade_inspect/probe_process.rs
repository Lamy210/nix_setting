//! Bounded Unix subprocess capture for Phase A inspection.
//!
//! The caller must establish executable provenance and local-only behavior
//! before use. This is not a network or filesystem sandbox and is not yet
//! connected to the CLI. A kernel-blocked spawn/kill/wait may exceed the
//! observation deadline; this is not a hard real-time guarantee.

use std::io::{self, Read};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::{parse_client_version, NixClientVersion, Observation, ObservationReason};
use crate::tool::ResolvedTool;

const TIMEOUT: Duration = Duration::from_secs(10);
const STDOUT_LIMIT: usize = 128;
const STDERR_LIMIT: usize = 4096;
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Probe only a caller-verified Nix executable resolved without a version
/// subprocess. An absolute path is necessary but not sufficient evidence
/// of trust. Never use ToolInventory::discover's unbounded version result.
/// No inherited environment, stdin, shell or arbitrary arguments.
pub fn probe_client_version(tool: Option<&ResolvedTool>) -> Observation<NixClientVersion> {
    let Some(tool) = tool else {
        return Observation::Unknown {
            reason: ObservationReason::ToolMissing,
        };
    };
    if !tool.path.is_absolute() {
        return Observation::Error {
            reason: ObservationReason::ProbeFailed,
        };
    }
    let mut command = Command::new(&tool.path);
    command.arg("--version");
    match capture(&mut command, TIMEOUT) {
        Ok(stdout) => parse_client_version(&stdout),
        Err(reason) => Observation::Error { reason },
    }
}

// Until try_wait reaps the leader, its PID cannot be reused. Never signal
// a process group after reaping the leader. On failure terminate the group
// and reap the direct child; a descendant that escaped the group is outside
// this guarantee. Successful probes must not spawn background descendants.
struct ChildGuard {
    child: Child,
    reaped: bool,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !self.reaped {
            // SAFETY: process_group(0) established this child's own group.
            // The positive child PID fits pid_t on supported Unix hosts.
            // The leader has not been reaped by this guard.
            unsafe {
                libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL);
            }
            // Best effort direct-child fallback if group termination failed.
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn set_nonblocking(fd: RawFd) -> io::Result<()> {
    // SAFETY: fd is a live pipe descriptor owned by the caller.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: set status flags on the same live descriptor.
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn io_reason(error: io::Error) -> ObservationReason {
    if error.kind() == io::ErrorKind::PermissionDenied {
        ObservationReason::PermissionDenied
    } else {
        ObservationReason::ProbeFailed
    }
}

/// Read at most one fixed-size chunk per iteration, so a busy stream cannot
/// starve the other stream or the deadline check. EOF is distinct from EAGAIN.
fn drain(
    reader: &mut impl Read,
    bytes: &mut Vec<u8>,
    limit: usize,
) -> Result<bool, ObservationReason> {
    let mut buffer = [0_u8; 512];
    let remaining = (limit - bytes.len() + 1).min(buffer.len());
    match reader.read(&mut buffer[..remaining]) {
        Ok(0) => Ok(true),
        Ok(count) if count > limit - bytes.len() => Err(ObservationReason::OutputLimitExceeded),
        Ok(count) => {
            bytes.extend_from_slice(&buffer[..count]);
            Ok(false)
        }
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(io_reason(error)),
    }
}

fn capture(command: &mut Command, timeout: Duration) -> Result<Vec<u8>, ObservationReason> {
    let start = Instant::now();
    let child = command
        .env_clear()
        .env("LC_ALL", "C")
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(io_reason)?;
    let mut guard = ChildGuard {
        child,
        reaped: false,
    };
    let mut stdout = guard
        .child
        .stdout
        .take()
        .ok_or(ObservationReason::ProbeFailed)?;
    let mut stderr = guard
        .child
        .stderr
        .take()
        .ok_or(ObservationReason::ProbeFailed)?;
    set_nonblocking(stdout.as_raw_fd()).map_err(io_reason)?;
    set_nonblocking(stderr.as_raw_fd()).map_err(io_reason)?;

    let mut out = Vec::with_capacity(STDOUT_LIMIT);
    let mut err = Vec::with_capacity(STDERR_LIMIT);
    let mut stdout_eof = false;
    let mut stderr_eof = false;

    loop {
        if start.elapsed() >= timeout {
            return Err(ObservationReason::TimedOut);
        }
        if !stdout_eof {
            stdout_eof = drain(&mut stdout, &mut out, STDOUT_LIMIT)?;
        }
        if !stderr_eof {
            stderr_eof = drain(&mut stderr, &mut err, STDERR_LIMIT)?;
        }
        // A descendant may retain either pipe after its parent exits. Do
        // not reap the leader until both pipes reach EOF; the deadline can
        // then still terminate its process group without PID-reuse risk.
        if stdout_eof && stderr_eof {
            if let Some(status) = guard.child.try_wait().map_err(io_reason)? {
                guard.reaped = true;
                return if start.elapsed() >= timeout {
                    Err(ObservationReason::TimedOut)
                } else if status.success() {
                    Ok(out)
                } else {
                    Err(ObservationReason::ProbeFailed)
                };
            }
        }
        thread::sleep(POLL_INTERVAL.min(timeout.saturating_sub(start.elapsed())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolSource;
    use std::path::PathBuf;

    // Fixed test-only commands exercise real Unix pipe and process behavior.
    // Production probe_client_version never invokes a shell.
    fn fixture(script: &str, timeout: Duration) -> Result<Vec<u8>, ObservationReason> {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", script]);
        capture(&mut command, timeout)
    }

    #[test]
    fn stdout_only_is_parsed_and_stderr_is_not_returned() {
        let out = fixture(
            "printf 'nix (Nix) 2.35.4\\n'; printf 'private-token' >&2",
            TIMEOUT,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(parse_client_version(&out)).unwrap(),
            serde_json::json!({"state":"known","value":"2.35.4"})
        );
    }

    #[test]
    fn nonzero_exit_cannot_publish_a_valid_version() {
        assert_eq!(
            fixture("printf 'nix (Nix) 2.35.4\\n'; exit 7", TIMEOUT),
            Err(ObservationReason::ProbeFailed)
        );
    }

    #[test]
    fn both_streams_have_independent_limits() {
        // Exact boundary is accepted; one extra byte must fail.
        assert_eq!(
            fixture("printf '%0128d' 0; printf '%04096d' 0 >&2", TIMEOUT)
                .unwrap()
                .len(),
            STDOUT_LIMIT
        );
        for script in ["printf '%0129d' 0", "printf '%04097d' 0 >&2"] {
            assert_eq!(
                fixture(script, TIMEOUT),
                Err(ObservationReason::OutputLimitExceeded)
            );
        }
    }

    #[test]
    fn silent_running_child_is_stopped_at_deadline() {
        assert_eq!(
            fixture("while :; do :; done", Duration::from_millis(200)),
            Err(ObservationReason::TimedOut)
        );
    }

    #[test]
    fn exited_parent_with_inherited_pipe_is_not_waited_on_forever() {
        // The background shell holds stdout/stderr open after the leader exits.
        assert_eq!(
            fixture("(while :; do :; done) & exit 0", Duration::from_millis(200)),
            Err(ObservationReason::TimedOut)
        );
    }

    #[test]
    fn child_environment_and_stdin_are_not_inherited() {
        let mut command = Command::new("/bin/sh");
        command
            .args([
                "-c",
                "test -z \"$SF_PROBE_SECRET\" && ! read -r line && printf safe",
            ])
            .env("SF_PROBE_SECRET", "private-token");
        assert_eq!(capture(&mut command, TIMEOUT).unwrap(), b"safe");
    }

    #[test]
    fn tool_missing_relative_and_spawn_failure_are_distinct() {
        assert_eq!(
            probe_client_version(None),
            Observation::Unknown {
                reason: ObservationReason::ToolMissing
            }
        );
        for path in ["relative/nix", "/__sf_missing_probe_binary__"] {
            let tool = ResolvedTool::new(PathBuf::from(path), ToolSource::Path);
            assert_eq!(
                probe_client_version(Some(&tool)),
                Observation::Error {
                    reason: ObservationReason::ProbeFailed
                }
            );
        }
    }
}
