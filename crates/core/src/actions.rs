use crate::discovery::{has_git, has_homebrew, has_nix, Host};
use std::process::Command;

/// apply: ホストを検出して設定を適用 (switch)
/// `flake` はリポジトリのパス (例: "$HOME/nix_setting")
pub fn apply(host: Host, flake: &str) -> Result<String, String> {
    if host == Host::Unsupported {
        return Err("unsupported platform".to_string());
    }
    if host == Host::MacbookAir {
        let target = format!("{flake}#darwinConfigurations.{host}");
        run("nh", &["darwin", "switch", &target])
    } else {
        let target = format!("{flake}#homeConfigurations.{host}");
        run("nh", &["home", "switch", &target])
    }
}

/// rollback: 前の世代へロールバック
pub fn rollback(host: Host) -> Result<String, String> {
    if host == Host::Unsupported {
        return Err("unsupported platform".to_string());
    }
    if host == Host::MacbookAir {
        run("darwin-rebuild", &["--rollback"])
    } else {
        run("nh", &["home", "switch", "--rollback"])
    }
}

/// upgrade: 依存 (flake.lock) を更新
pub fn upgrade() -> Result<String, String> {
    run("nix", &["flake", "update"])
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

fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
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
}
