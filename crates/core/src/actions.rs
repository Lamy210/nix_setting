use crate::discovery::{has_git, has_homebrew, has_nix, ConfigurationTarget, Platform};
use crate::error::{Error, Result};
use crate::process::{run_capture, run_stream};

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
pub(crate) fn apply(target: &ConfigurationTarget, flake: &str) -> Result<()> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    let (cmd, args) = switch_command(target, flake);
    run_stream(cmd, &args)
}

/// apply_captured: 出力をキャプチャして返す (GUI 用)
pub(crate) fn apply_captured(target: &ConfigurationTarget, flake: &str) -> Result<String> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    let (cmd, args) = switch_command(target, flake);
    run_capture(cmd, &args)
}

/// rollback: ストリーミング実行
pub(crate) fn rollback(target: &ConfigurationTarget) -> Result<()> {
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
pub(crate) fn rollback_captured(target: &ConfigurationTarget) -> Result<String> {
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

/// upgrade の引数を構築する (`nix flake update --flake <repo>`)
fn upgrade_args(repo: &str) -> Vec<String> {
    vec![
        "flake".to_string(),
        "update".to_string(),
        "--flake".to_string(),
        repo.to_string(),
    ]
}

/// upgrade: `nix flake update` (repo-aware、`--flake <repo>`)
pub(crate) fn upgrade(repo: &str) -> Result<()> {
    run_stream("nix", &upgrade_args(repo))
}

/// upgrade_captured: 出力をキャプチャ (GUI 用)
pub(crate) fn upgrade_captured(repo: &str) -> Result<String> {
    run_capture("nix", &upgrade_args(repo))
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

    #[test]
    fn upgrade_args_are_repo_aware() {
        assert_eq!(
            upgrade_args("/tmp/repo"),
            vec![
                "flake".to_string(),
                "update".to_string(),
                "--flake".to_string(),
                "/tmp/repo".to_string(),
            ]
        );
    }
}
