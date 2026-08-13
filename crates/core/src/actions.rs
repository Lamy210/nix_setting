use crate::discovery::ConfigurationTarget;
use crate::error::{Error, Result};
use crate::process::{run_capture, run_stream};
use crate::tool::Toolchain;
use std::path::Path;

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

/// drop 時に一時 symlink を削除する RAII ガード
struct ActivationLink {
    path: String,
}

impl ActivationLink {
    fn new() -> Self {
        Self {
            path: activation_link(),
        }
    }
}

impl Drop for ActivationLink {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn unsupported(target: &ConfigurationTarget) -> Error {
    Error::UnsupportedPlatform {
        os: target.platform().to_string(),
        arch: target.architecture().to_string(),
    }
}

/// apply: ストリーミング実行 (stdio 継承、リアルタイム出力)
pub(crate) fn apply(target: &ConfigurationTarget, repo: &str, tc: &Toolchain) -> Result<()> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    if target.platform() == crate::discovery::Platform::MacOS {
        run_stream(&tc.nix.path, &darwin_switch_args(repo, target))
    } else {
        apply_linux(target, repo, tc)
    }
}

/// apply_captured: 出力をキャプチャして返す (GUI 用)
pub(crate) fn apply_captured(
    target: &ConfigurationTarget,
    repo: &str,
    tc: &Toolchain,
) -> Result<String> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    if target.platform() == crate::discovery::Platform::MacOS {
        run_capture(&tc.nix.path, &darwin_switch_args(repo, target))
    } else {
        apply_linux_captured(target, repo, tc)
    }
}

/// Linux: activationPackage を build して activate する (nh 非依存)
fn apply_linux(target: &ConfigurationTarget, repo: &str, tc: &Toolchain) -> Result<()> {
    let link = ActivationLink::new();
    run_stream(&tc.nix.path, &linux_build_args(repo, target, &link.path))?;
    run_stream(Path::new(&format!("{}/activate", link.path)), &[])
}

fn apply_linux_captured(
    target: &ConfigurationTarget,
    repo: &str,
    tc: &Toolchain,
) -> Result<String> {
    let link = ActivationLink::new();
    let mut out = run_capture(&tc.nix.path, &linux_build_args(repo, target, &link.path))?;
    let activate = run_capture(Path::new(&format!("{}/activate", link.path)), &[])?;
    if !activate.is_empty() {
        out.push('\n');
        out.push_str(&activate);
    }
    Ok(out)
}

/// macOS rollback の引数 (`nix run --inputs-from <repo> nix-darwin#darwin-rebuild -- --rollback`)
fn darwin_rollback_args(repo: &str) -> Vec<String> {
    vec![
        "run".to_string(),
        "--inputs-from".to_string(),
        repo.to_string(),
        "nix-darwin#darwin-rebuild".to_string(),
        "--".to_string(),
        "--rollback".to_string(),
    ]
}

/// Linux rollback の引数 (`nh home switch --rollback`)
/// rollback は apply 後（nh 導入済み）の便利操作のため nh 依存を許容
fn linux_rollback_args() -> Vec<String> {
    vec![
        "home".to_string(),
        "switch".to_string(),
        "--rollback".to_string(),
    ]
}

/// rollback: ストリーミング実行
pub(crate) fn rollback(target: &ConfigurationTarget, repo: &str, tc: &Toolchain) -> Result<()> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    if target.platform() == crate::discovery::Platform::MacOS {
        run_stream(&tc.nix.path, &darwin_rollback_args(repo))
    } else {
        let nh = tc
            .nh
            .as_ref()
            .ok_or_else(|| Error::Precondition("nh is required for Linux rollback".to_string()))?;
        run_stream(&nh.path, &linux_rollback_args())
    }
}

/// rollback_captured: 出力をキャプチャ (GUI 用)
pub(crate) fn rollback_captured(
    target: &ConfigurationTarget,
    repo: &str,
    tc: &Toolchain,
) -> Result<String> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    if target.platform() == crate::discovery::Platform::MacOS {
        run_capture(&tc.nix.path, &darwin_rollback_args(repo))
    } else {
        let nh = tc
            .nh
            .as_ref()
            .ok_or_else(|| Error::Precondition("nh is required for Linux rollback".to_string()))?;
        run_capture(&nh.path, &linux_rollback_args())
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
pub(crate) fn upgrade(repo: &str, tc: &Toolchain) -> Result<()> {
    run_stream(&tc.nix.path, &upgrade_args(repo))
}

/// upgrade_captured: 出力をキャプチャ (GUI 用)
pub(crate) fn upgrade_captured(repo: &str, tc: &Toolchain) -> Result<String> {
    run_capture(&tc.nix.path, &upgrade_args(repo))
}

/// scan: 環境スキャン結果を文字列で返す
pub fn scan(target: &ConfigurationTarget, tc: &Toolchain) -> String {
    let mut out = String::new();
    out.push_str(&format!("OS:   {}\n", std::env::consts::OS));
    out.push_str(&format!("arch: {}\n", std::env::consts::ARCH));
    out.push_str(&format!("host: {target}\n"));
    out.push_str(&format!(
        "nix:  {} ({}, {})\n",
        tc.nix.path.display(),
        tc.nix.source,
        tc.nix.version.as_deref().unwrap_or("unknown")
    ));
    if let Some(brew) = &tc.homebrew {
        out.push_str(&format!(
            "brew: {} ({})\n",
            brew.path.display(),
            brew.version.as_deref().unwrap_or("unknown")
        ));
    } else {
        out.push_str("brew: not found\n");
    }
    out.push_str(&format!(
        "git:  {} ({})\n",
        tc.git.path.display(),
        tc.git.version.as_deref().unwrap_or("unknown")
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::detect_target_for;
    use crate::tool::{ResolvedTool, ToolSource};
    use std::path::PathBuf;

    fn dummy_tc() -> Toolchain {
        Toolchain {
            nix: ResolvedTool::new(PathBuf::from("/usr/local/bin/nix"), ToolSource::Homebrew),
            git: ResolvedTool::new(PathBuf::from("/usr/bin/git"), ToolSource::Path),
            homebrew: None,
            nh: None,
        }
    }

    #[test]
    fn scan_contains_host() {
        let target = detect_target_for("macos", "aarch64");
        let out = scan(&target, &dummy_tc());
        assert!(out.contains("macbook-air"));
        assert!(out.contains("/usr/local/bin/nix"));
    }

    #[test]
    fn scan_reports_missing_brew() {
        let target = detect_target_for("linux", "x86_64");
        let out = scan(&target, &dummy_tc());
        assert!(out.contains("brew: not found"));
    }

    #[test]
    fn apply_unsupported_fails() {
        let target = detect_target_for("windows", "x86_64");
        let err = apply(&target, "/tmp/repo", &dummy_tc()).unwrap_err();
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
        assert!(rollback(&target, "/tmp/repo", &dummy_tc()).is_err());
    }

    #[test]
    fn rollback_linux_without_nh_returns_precondition_error() {
        let target = detect_target_for("linux", "x86_64");
        let err = rollback(&target, "/tmp/repo", &dummy_tc()).unwrap_err();
        assert!(matches!(err, Error::Precondition(_)));
    }

    #[test]
    fn rollback_macos_uses_resolved_nix() {
        // macOS rollback は tc.nix.path を使う。ここでは引数組み立てまで検証
        let args = darwin_rollback_args("/tmp/repo");
        assert_eq!(args[1], "--inputs-from");
        assert_eq!(args[2], "/tmp/repo");
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

    #[test]
    fn darwin_rollback_is_pinned_and_repo_aware() {
        let args = darwin_rollback_args("/tmp/repo");
        assert_eq!(args[1], "--inputs-from");
        assert_eq!(args[2], "/tmp/repo");
        assert_eq!(args[3], "nix-darwin#darwin-rebuild");
        assert_eq!(args[5], "--rollback");
    }
}
