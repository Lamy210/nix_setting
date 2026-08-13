use crate::discovery::{has_git, has_homebrew, has_nix, ConfigurationTarget, Platform};
use crate::error::{Error, Result};
use std::process::Command;

/// switch コマンドを構築 (共有)
fn switch_command(target: &ConfigurationTarget, flake: &str) -> (&'static str, Vec<String>) {
    if target.platform() == Platform::MacOS {
        (
            "nh",
            vec![
                "darwin".to_string(),
                "switch".to_string(),
                format!("{flake}#darwinConfigurations.{target}"),
            ],
        )
    } else {
        (
            "nh",
            vec![
                "home".to_string(),
                "switch".to_string(),
                format!("{flake}#homeConfigurations.{target}"),
            ],
        )
    }
}

fn unsupported(target: &ConfigurationTarget) -> Error {
    Error::UnsupportedPlatform {
        os: target.platform().to_string(),
        arch: target.architecture().to_string(),
    }
}

/// apply: ストリーミング実行 (stdio 継承、リアルタイム出力)
pub fn apply(target: &ConfigurationTarget, flake: &str) -> Result<()> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    let (cmd, args) = switch_command(target, flake);
    run_stream(cmd, &args)
}

/// apply_captured: 出力をキャプチャして返す (GUI 用)
pub fn apply_captured(target: &ConfigurationTarget, flake: &str) -> Result<String> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    let (cmd, args) = switch_command(target, flake);
    run_capture(cmd, &args)
}

/// rollback: ストリーミング実行
pub fn rollback(target: &ConfigurationTarget) -> Result<()> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    if target.platform() == Platform::MacOS {
        run_stream("darwin-rebuild", &["--rollback".to_string()])
    } else {
        run_stream(
            "nh",
            &[
                "home".to_string(),
                "switch".to_string(),
                "--rollback".to_string(),
            ],
        )
    }
}

/// rollback_captured: 出力をキャプチャ (GUI 用)
pub fn rollback_captured(target: &ConfigurationTarget) -> Result<String> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    if target.platform() == Platform::MacOS {
        run_capture("darwin-rebuild", &["--rollback".to_string()])
    } else {
        run_capture(
            "nh",
            &[
                "home".to_string(),
                "switch".to_string(),
                "--rollback".to_string(),
            ],
        )
    }
}

/// upgrade: ストリーミング実行
pub fn upgrade() -> Result<()> {
    run_stream("nix", &["flake".to_string(), "update".to_string()])
}

/// upgrade_captured: 出力をキャプチャ (GUI 用)
pub fn upgrade_captured() -> Result<String> {
    run_capture("nix", &["flake".to_string(), "update".to_string()])
}

/// scan: 環境スキャン結果を文字列で返す
pub fn scan(target: &ConfigurationTarget) -> String {
    let mut out = String::new();
    out.push_str(&format!("OS:   {}\n", std::env::consts::OS));
    out.push_str(&format!("arch: {}\n", std::env::consts::ARCH));
    out.push_str(&format!("host: {target}\n"));
    out.push_str(&format!("nix:  {}\n", if has_nix() { "yes" } else { "no" }));
    out.push_str(&format!(
        "brew: {}\n",
        if has_homebrew() { "yes" } else { "no" }
    ));
    out.push_str(&format!("git:  {}\n", if has_git() { "yes" } else { "no" }));
    out
}

fn run_stream(cmd: &str, args: &[String]) -> Result<()> {
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

fn run_capture(cmd: &str, args: &[String]) -> Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::detect_target_for;

    #[test]
    fn scan_contains_host() {
        let target = detect_target_for("macos", "aarch64");
        let out = scan(&target);
        assert!(out.contains("macbook-air"));
    }

    #[test]
    fn apply_unsupported_fails() {
        let target = detect_target_for("windows", "x86_64");
        let err = apply(&target, "/tmp/repo").unwrap_err();
        assert_eq!(
            err,
            Error::UnsupportedPlatform {
                os: "unsupported".to_string(),
                arch: "x86_64".to_string(),
            }
        );
    }

    #[test]
    fn rollback_unsupported_fails() {
        let target = detect_target_for("windows", "x86_64");
        assert!(rollback(&target).is_err());
    }

    #[test]
    fn switch_command_macos() {
        let target = detect_target_for("macos", "aarch64");
        let (cmd, args) = switch_command(&target, "/tmp/repo");
        assert_eq!(cmd, "nh");
        assert_eq!(args[0], "darwin");
        assert_eq!(args[1], "switch");
        assert_eq!(args[2], "/tmp/repo#darwinConfigurations.macbook-air");
    }

    #[test]
    fn switch_command_linux() {
        let target = detect_target_for("linux", "x86_64");
        let (cmd, args) = switch_command(&target, "/tmp/repo");
        assert_eq!(cmd, "nh");
        assert_eq!(args[0], "home");
        assert_eq!(args[2], "/tmp/repo#homeConfigurations.linux");
    }
}
