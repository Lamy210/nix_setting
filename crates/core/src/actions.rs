use crate::discovery::{has_git, has_homebrew, has_nix, ConfigurationTarget, Platform};
use crate::error::{Error, Result};
use crate::process::{run_capture, run_stream};

/// macOS apply の引数 (`nix run --inputs-from <repo> nix-darwin#darwin-rebuild -- switch --flake <ref>`)
/// `--inputs-from <repo>` で repo の flake.lock に pin された nix-darwin を使う (registry 非依存)
fn darwin_switch_args(repo: &str, target: &ConfigurationTarget) -> Vec<String> {
    vec![
        "run".to_string(),
        "--inputs-from".to_string(),
        repo.to_string(),
        "nix-darwin#darwin-rebuild".to_string(),
        "--".to_string(),
        "switch".to_string(),
        "--flake".to_string(),
        target.switch_ref(repo),
    ]
}

/// Linux apply の build 引数 (`nix build --out-link <link> <ref>`)
fn linux_build_args(repo: &str, target: &ConfigurationTarget, link: &str) -> Vec<String> {
    vec![
        "build".to_string(),
        "--out-link".to_string(),
        link.to_string(),
        target.build_ref(repo),
    ]
}

/// activationPackage を build する際の一時 symlink パス
fn activation_link() -> String {
    std::env::temp_dir()
        .join(format!("schneeforge-activate-{}", std::process::id()))
        .to_string_lossy()
        .to_string()
}

fn unsupported(target: &ConfigurationTarget) -> Error {
    Error::UnsupportedPlatform {
        os: target.platform().to_string(),
        arch: target.architecture().to_string(),
    }
}

/// apply: ストリーミング実行 (stdio 継承、リアルタイム出力)
pub(crate) fn apply(target: &ConfigurationTarget, repo: &str) -> Result<()> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    if target.platform() == Platform::MacOS {
        run_stream("nix", &darwin_switch_args(repo, target))
    } else {
        apply_linux(target, repo)
    }
}

/// apply_captured: 出力をキャプチャして返す (GUI 用)
pub(crate) fn apply_captured(target: &ConfigurationTarget, repo: &str) -> Result<String> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    if target.platform() == Platform::MacOS {
        run_capture("nix", &darwin_switch_args(repo, target))
    } else {
        apply_linux_captured(target, repo)
    }
}

/// Linux: activationPackage を build して activate する (nh 非依存)
fn apply_linux(target: &ConfigurationTarget, repo: &str) -> Result<()> {
    let link = activation_link();
    run_stream("nix", &linux_build_args(repo, target, &link))?;
    let result = run_stream(&format!("{link}/activate"), &[]);
    let _ = std::fs::remove_file(&link);
    result
}

fn apply_linux_captured(target: &ConfigurationTarget, repo: &str) -> Result<String> {
    let link = activation_link();
    let mut out = run_capture("nix", &linux_build_args(repo, target, &link))?;
    let activate = run_capture(&format!("{link}/activate"), &[])?;
    let _ = std::fs::remove_file(&link);
    if !activate.is_empty() {
        out.push('\n');
        out.push_str(&activate);
    }
    Ok(out)
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
        let args = darwin_switch_args("/tmp/repo", &target);
        assert_eq!(args[0], "run");
        assert_eq!(args[1], "--inputs-from");
        assert_eq!(args[2], "/tmp/repo");
        assert_eq!(args[3], "nix-darwin#darwin-rebuild");
        assert_eq!(args[4], "--");
        assert_eq!(args[5], "switch");
        assert_eq!(args[6], "--flake");
        assert_eq!(args[7], "/tmp/repo#darwinConfigurations.macbook-air");
    }

    #[test]
    fn switch_command_linux() {
        let target = detect_target_for("linux", "x86_64");
        let args = linux_build_args("/tmp/repo", &target, "/tmp/link");
        assert_eq!(args[0], "build");
        assert_eq!(args[1], "--out-link");
        assert_eq!(args[2], "/tmp/link");
        assert_eq!(
            args[3],
            "/tmp/repo#homeConfigurations.linux.activationPackage"
        );
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

    #[test]
    fn apply_is_nh_free() {
        let mac = detect_target_for("macos", "aarch64");
        let mac_args = darwin_switch_args("/tmp/repo", &mac);
        assert!(!mac_args.iter().any(|a| a == "nh"));
        assert!(mac_args.iter().any(|a| a == "nix-darwin#darwin-rebuild"));

        let linux = detect_target_for("linux", "x86_64");
        let linux_args = linux_build_args("/tmp/repo", &linux, "/tmp/link");
        assert!(!linux_args.iter().any(|a| a == "nh"));
    }
}
