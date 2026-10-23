use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

/// コマンドが存在し実行可能かを確認する (出力は破棄)
///
/// `cmd` は解決済みの絶対パス（`ToolInventory` 経由）を渡すこと。
pub fn command_succeeds<S: AsRef<OsStr>>(cmd: &Path, args: &[S]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn display_args<S: AsRef<OsStr>>(args: &[S]) -> String {
    args.iter()
        .map(|arg| arg.as_ref().to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

/// コマンドを stdio 継承で実行する (リアルタイム出力)
///
/// `cmd` は解決済みの絶対パス（`ToolInventory` 経由）を渡すこと。
/// argv は OS-native のまま `Command` へ渡し、表示時のみ lossy 変換する。
pub fn run_stream<S: AsRef<OsStr>>(cmd: &Path, args: &[S]) -> Result<()> {
    println!("running: {} {}", cmd.display(), display_args(args));
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
/// `cmd` は解決済みの絶対パス（`ToolInventory` 経由）を渡すこと。
/// argv は OS-native のまま `Command` へ渡す。
pub fn run_capture<S: AsRef<OsStr>>(cmd: &Path, args: &[S]) -> Result<String> {
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

    /// test 内で使う「必ず exit 0 する」実 binary。
    /// /bin/true は nix build の checkPhase sandbox に存在しないため使わない。
    /// /usr/bin/env は coreutils ではなく env 自体なので sandbox にもある
    fn true_bin() -> std::path::PathBuf {
        let bin = std::path::PathBuf::from("/usr/bin/env");
        if bin.is_file() {
            bin
        } else {
            // 念のため PATH 探索に fallback
            std::path::PathBuf::from("env")
        }
    }

    #[test]
    fn command_succeeds_with_real_command() {
        let ok = command_succeeds(&true_bin(), &[] as &[String]);
        assert!(ok);
    }

    #[test]
    fn command_succeeds_with_failing_command() {
        let ok = command_succeeds(Path::new("/bin/false"), &[] as &[String]);
        assert!(!ok);
    }

    #[test]
    fn command_succeeds_with_nonexistent_command() {
        let ok = command_succeeds(
            Path::new("/__definitely_not_a_real_binary__"),
            &[] as &[String],
        );
        assert!(!ok);
    }

    #[test]
    fn run_capture_with_echo() {
        let out = run_capture(Path::new("/bin/echo"), &["hello".to_string()]).unwrap();
        assert_eq!(out.trim(), "hello");
    }

    #[test]
    fn run_capture_failing_command_returns_error() {
        let result = run_capture(Path::new("/bin/false"), &[] as &[String]);
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
        let result = run_stream(Path::new("/bin/false"), &[] as &[String]);
        assert!(result.is_err());
    }

    #[test]
    fn run_stream_nonexistent_command_returns_error() {
        let result = run_stream(Path::new("/__no_such_binary__"), &[] as &[String]);
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Command { command, detail } => {
                assert_eq!(command, "/__no_such_binary__");
                assert!(detail.contains("failed to run"));
            }
            _ => panic!("expected Command error"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn run_capture_preserves_non_utf8_argument_bytes() {
        use std::ffi::OsString;
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        use std::os::unix::fs::PermissionsExt;

        let dir =
            std::env::temp_dir().join(format!("sf-process-native-argv-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("capture-argv");
        let output = dir.join("argv.bin");
        std::fs::write(&script, "#!/bin/sh\nprintf '%s' \"$2\" > \"$1\"\n").unwrap();
        let mut permissions = std::fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).unwrap();

        let native = OsString::from_vec(b"repo-\xff-path".to_vec());
        run_capture(&script, &[output.clone().into_os_string(), native.clone()]).unwrap();

        assert_eq!(
            std::fs::read(&output).unwrap(),
            native.as_os_str().as_bytes()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
