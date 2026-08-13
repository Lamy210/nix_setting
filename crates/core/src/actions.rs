use crate::discovery::{has_git, has_homebrew, has_nix, Host};
use std::process::Command;

/// switch コマンドを構築 (共有)
fn switch_command(host: Host, flake: &str) -> (&'static str, Vec<String>) {
    if host == Host::MacbookAir {
        (
            "nh",
            vec![
                "darwin".to_string(),
                "switch".to_string(),
                format!("{flake}#darwinConfigurations.{host}"),
            ],
        )
    } else {
        (
            "nh",
            vec![
                "home".to_string(),
                "switch".to_string(),
                format!("{flake}#homeConfigurations.{host}"),
            ],
        )
    }
}

/// apply: ストリーミング実行 (stdio 継承、リアルタイム出力)
pub fn apply(host: Host, flake: &str) -> Result<(), String> {
    if host == Host::Unsupported {
        return Err("unsupported platform".to_string());
    }
    let (cmd, args) = switch_command(host, flake);
    run_stream(cmd, &args)
}

/// apply_captured: 出力をキャプチャして返す (GUI 用)
pub fn apply_captured(host: Host, flake: &str) -> Result<String, String> {
    if host == Host::Unsupported {
        return Err("unsupported platform".to_string());
    }
    let (cmd, args) = switch_command(host, flake);
    run_capture(cmd, &args)
}

/// rollback: ストリーミング実行
pub fn rollback(host: Host) -> Result<(), String> {
    if host == Host::Unsupported {
        return Err("unsupported platform".to_string());
    }
    if host == Host::MacbookAir {
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
pub fn rollback_captured(host: Host) -> Result<String, String> {
    if host == Host::Unsupported {
        return Err("unsupported platform".to_string());
    }
    if host == Host::MacbookAir {
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
pub fn upgrade() -> Result<(), String> {
    run_stream("nix", &["flake".to_string(), "update".to_string()])
}

/// upgrade_captured: 出力をキャプチャ (GUI 用)
pub fn upgrade_captured() -> Result<String, String> {
    run_capture("nix", &["flake".to_string(), "update".to_string()])
}

/// scan: 環境スキャン結果を文字列で返す
pub fn scan(host: Host) -> String {
    let mut out = String::new();
    out.push_str(&format!("OS:   {}\n", std::env::consts::OS));
    out.push_str(&format!("arch: {}\n", std::env::consts::ARCH));
    out.push_str(&format!("host: {host}\n"));
    out.push_str(&format!("nix:  {}\n", if has_nix() { "yes" } else { "no" }));
    out.push_str(&format!(
        "brew: {}\n",
        if has_homebrew() { "yes" } else { "no" }
    ));
    out.push_str(&format!("git:  {}\n", if has_git() { "yes" } else { "no" }));
    out
}

fn run_stream(cmd: &str, args: &[String]) -> Result<(), String> {
    println!("running: {cmd} {}", args.join(" "));
    let status = Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| format!("failed to run {cmd}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{cmd} exited with {}", status.code().unwrap_or(1)))
    }
}

fn run_capture(cmd: &str, args: &[String]) -> Result<String, String> {
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
                Err(if combined.is_empty() {
                    format!("{cmd} exited with {}", out.status.code().unwrap_or(1))
                } else {
                    combined
                })
            }
        }
        Err(e) => Err(format!("failed to run {cmd}: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_contains_host() {
        let out = scan(Host::MacbookAir);
        assert!(out.contains("macbook-air"));
    }

    #[test]
    fn apply_unsupported_fails() {
        assert!(apply(Host::Unsupported, "/tmp/repo").is_err());
    }

    #[test]
    fn rollback_unsupported_fails() {
        assert!(rollback(Host::Unsupported).is_err());
    }

    #[test]
    fn switch_command_macos() {
        let (cmd, args) = switch_command(Host::MacbookAir, "/tmp/repo");
        assert_eq!(cmd, "nh");
        assert_eq!(args[0], "darwin");
        assert_eq!(args[1], "switch");
        assert_eq!(args[2], "/tmp/repo#darwinConfigurations.macbook-air");
    }

    #[test]
    fn switch_command_linux() {
        let (cmd, args) = switch_command(Host::Linux, "/tmp/repo");
        assert_eq!(cmd, "nh");
        assert_eq!(args[0], "home");
        assert_eq!(args[2], "/tmp/repo#homeConfigurations.linux");
    }
}
