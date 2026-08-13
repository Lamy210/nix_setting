use std::process::Command;

use crate::error::{Error, Result};

/// コマンドが存在し実行可能かを確認する (出力は破棄)
pub fn command_succeeds(cmd: &str, args: &[String]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// コマンドを stdio 継承で実行する (リアルタイム出力)
pub fn run_stream(cmd: &str, args: &[String]) -> Result<()> {
    println!("running: {cmd} {}", args.join(" "));
    let status = Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| Error::Command {
            command: cmd.to_string(),
            detail: format!("failed to run: {e}"),
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Command {
            command: cmd.to_string(),
            detail: format!("exited with {}", status.code().unwrap_or(1)),
        })
    }
}

/// コマンドを実行し、stdout/stderr をキャプチャして返す
pub fn run_capture(cmd: &str, args: &[String]) -> Result<String> {
    match Command::new(cmd).args(args).output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = if stderr.is_empty() {
                stdout
            } else {
                format!("{stdout}\n{stderr}")
            };
            if out.status.success() {
                Ok(combined)
            } else {
                Err(Error::Command {
                    command: cmd.to_string(),
                    detail: if combined.is_empty() {
                        format!("exited with {}", out.status.code().unwrap_or(1))
                    } else {
                        combined
                    },
                })
            }
        }
        Err(e) => Err(Error::Command {
            command: cmd.to_string(),
            detail: format!("failed to run: {e}"),
        }),
    }
}
