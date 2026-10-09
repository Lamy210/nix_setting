use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

use super::installer::{parse_json_line, JsonLogLine};
use super::ManagedNixError;

pub(super) fn install_args(plan_file: &Path) -> Vec<OsString> {
    vec![
        OsString::from("install"),
        plan_file.as_os_str().to_os_string(),
        OsString::from("--logger"),
        OsString::from("json"),
        OsString::from("--no-confirm"),
    ]
}

pub(super) fn uninstall_args(receipt: Option<&Path>) -> Vec<OsString> {
    let mut args = vec![OsString::from("uninstall"), OsString::from("--no-confirm")];
    if let Some(receipt) = receipt {
        args.push(receipt.as_os_str().to_os_string());
    }
    args
}

pub(super) fn run_with_json_logs<F>(
    binary: &Path,
    args: &[OsString],
    mut on_line: F,
) -> Result<(), ManagedNixError>
where
    F: FnMut(&JsonLogLine),
{
    let mut child = Command::new(binary)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ManagedNixError::Io {
            context: format!("spawn {}", binary.display()),
            source: e.to_string(),
        })?;

    const TAIL_LINES: usize = 20;
    let mut tail: std::collections::VecDeque<String> = std::collections::VecDeque::new();

    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    tail.push_back(line.clone());
                    if tail.len() > TAIL_LINES {
                        tail.pop_front();
                    }
                    if let Some(parsed) = parse_json_line(&line) {
                        on_line(&parsed);
                    } else {
                        eprintln!("{line}");
                    }
                }
                Err(e) => {
                    return Err(ManagedNixError::Io {
                        context: "read installer stderr".to_string(),
                        source: e.to_string(),
                    });
                }
            }
        }
    }

    let status = child.wait().map_err(|e| ManagedNixError::Io {
        context: format!("wait {}", binary.display()),
        source: e.to_string(),
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(ManagedNixError::Subprocess {
            exit_status: status.code(),
            stderr_tail: tail.into_iter().collect::<Vec<_>>().join("\n"),
        })
    }
}
