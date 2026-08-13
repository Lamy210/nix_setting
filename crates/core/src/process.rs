use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

/// コマンドが存在し実行可能かを確認する (出力は破棄)
///
/// `cmd` は解決済みの絶対パス（`Toolchain` 経由）を渡すこと。
pub fn command_succeeds(cmd: &Path, args: &[String]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// コマンドを stdio 継承で実行する (リアルタイム出力)
///
/// `cmd` は解決済みの絶対パス（`Toolchain` 経由）を渡すこと。
pub fn run_stream(cmd: &Path, args: &[String]) -> Result<()> {
    println!("running: {} {}", cmd.display(), args.join(" "));
    let status = Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| Error::Command {
            command: cmd.display().to_string(),
            detail: format!("failed to run: {e}"),
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Command {
            command: cmd.display().to_string(),
            detail: format!("exited with {}", status.code().unwrap_or(1)),
        })
    }
}

/// コマンドを実行し、stdout/stderr をキャプチャして返す
///
/// `cmd` は解決済みの絶対パス（`Toolchain` 経由）を渡すこと。
pub fn run_capture(cmd: &Path, args: &[String]) -> Result<String> {
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
                    command: cmd.display().to_string(),
                    detail: if combined.is_empty() {
                        format!("exited with {}", out.status.code().unwrap_or(1))
                    } else {
                        combined
                    },
                })
            }
        }
        Err(e) => Err(Error::Command {
            command: cmd.display().to_string(),
            detail: format!("failed to run: {e}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_succeeds_with_real_command() {
        // /bin/true はUnix系OSで普遍的に存在
        let ok = command_succeeds(Path::new("/bin/true"), &[]);
        assert!(ok);
    }

    #[test]
    fn command_succeeds_with_failing_command() {
        let ok = command_succeeds(Path::new("/bin/false"), &[]);
        assert!(!ok);
    }

    #[test]
    fn command_succeeds_with_nonexistent_command() {
        let ok = command_succeeds(Path::new("/__definitely_not_a_real_binary__"), &[]);
        assert!(!ok);
    }

    #[test]
    fn run_capture_with_echo() {
        let out = run_capture(Path::new("/bin/echo"), &["hello".to_string()]).unwrap();
        assert_eq!(out.trim(), "hello");
    }

    #[test]
    fn run_capture_failing_command_returns_error() {
        let result = run_capture(Path::new("/bin/false"), &[]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            Error::Command { command, .. } => assert_eq!(command, "/bin/false"),
            _ => panic!("expected Command error"),
        }
    }

    #[test]
    fn run_stream_with_echo() {
        // /bin/echo は stdout へ出力して exit 0 するので stream も成功する
        let result = run_stream(Path::new("/bin/echo"), &["test".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn run_stream_failing_command_returns_error() {
        let result = run_stream(Path::new("/bin/false"), &[]);
        assert!(result.is_err());
    }

    #[test]
    fn run_stream_nonexistent_command_returns_error() {
        let result = run_stream(Path::new("/__no_such_binary__"), &[]);
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Command { command, detail } => {
                assert_eq!(command, "/__no_such_binary__");
                assert!(detail.contains("failed to run"));
            }
            _ => panic!("expected Command error"),
        }
    }
}
