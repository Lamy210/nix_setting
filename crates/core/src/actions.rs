use crate::discovery::ConfigurationTarget;
use crate::error::{Error, Result};
use crate::process::{run_capture, run_stream};
use crate::profile;
use crate::tool::{ResolvedTool, ToolInventory};
use std::path::Path;

/// macOS apply の引数 (`nix run --inputs-from <repo> nix-darwin#darwin-rebuild -- switch --flake <ref>`)
/// `--inputs-from <repo>` で repo の flake.lock に pin された nix-darwin を使う (registry 非依存)
fn darwin_switch_args(repo: &str, target: &ConfigurationTarget) -> Result<Vec<String>> {
    let mut args = vec![
        "run".to_string(),
        "--inputs-from".to_string(),
        repo.to_string(),
    ];
    args.extend(profile::override_args(repo)?);
    args.extend([
        "nix-darwin#darwin-rebuild".to_string(),
        "--".to_string(),
        "switch".to_string(),
        "--flake".to_string(),
        target.switch_ref(repo),
    ]);
    Ok(args)
}

/// Linux apply の build 引数 (`nix build --out-link <link> <ref>`)
fn linux_build_args(repo: &str, target: &ConfigurationTarget, link: &str) -> Result<Vec<String>> {
    let mut args = vec![
        "build".to_string(),
        "--out-link".to_string(),
        link.to_string(),
    ];
    args.extend(profile::override_args(repo)?);
    args.push(target.build_ref(repo));
    Ok(args)
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
pub(crate) fn apply(target: &ConfigurationTarget, repo: &str, tc: &ToolInventory) -> Result<()> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    let nix = tc.require_nix()?;
    if target.platform() == crate::discovery::Platform::MacOS {
        run_stream(&nix.path, &darwin_switch_args(repo, target)?)
    } else {
        apply_linux(target, repo, nix)
    }
}

/// apply_captured: 出力をキャプチャして返す (GUI 用)
pub(crate) fn apply_captured(
    target: &ConfigurationTarget,
    repo: &str,
    tc: &ToolInventory,
) -> Result<String> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    let nix = tc.require_nix()?;
    if target.platform() == crate::discovery::Platform::MacOS {
        run_capture(&nix.path, &darwin_switch_args(repo, target)?)
    } else {
        apply_linux_captured(target, repo, nix)
    }
}

/// Linux: activationPackage を build して activate する (nh 非依存)
fn apply_linux(target: &ConfigurationTarget, repo: &str, nix: &ResolvedTool) -> Result<()> {
    let link = ActivationLink::new();
    run_stream(&nix.path, &linux_build_args(repo, target, &link.path)?)?;
    run_stream(Path::new(&format!("{}/activate", link.path)), &[])
}

fn apply_linux_captured(
    target: &ConfigurationTarget,
    repo: &str,
    nix: &ResolvedTool,
) -> Result<String> {
    let link = ActivationLink::new();
    let mut out = run_capture(&nix.path, &linux_build_args(repo, target, &link.path)?)?;
    let activate = run_capture(Path::new(&format!("{}/activate", link.path)), &[])?;
    if !activate.is_empty() {
        out.push('\n');
        out.push_str(&activate);
    }
    Ok(out)
}

/// macOS rollback の引数 (`nix run --inputs-from <repo> nix-darwin#darwin-rebuild -- --rollback`)
fn darwin_rollback_args(repo: &str) -> Result<Vec<String>> {
    let mut args = vec![
        "run".to_string(),
        "--inputs-from".to_string(),
        repo.to_string(),
    ];
    args.extend(profile::override_args(repo)?);
    args.extend([
        "nix-darwin#darwin-rebuild".to_string(),
        "--".to_string(),
        "--rollback".to_string(),
    ]);
    Ok(args)
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
pub(crate) fn rollback(target: &ConfigurationTarget, repo: &str, tc: &ToolInventory) -> Result<()> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    if target.platform() == crate::discovery::Platform::MacOS {
        let nix = tc.require_nix()?;
        run_stream(&nix.path, &darwin_rollback_args(repo)?)
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
    tc: &ToolInventory,
) -> Result<String> {
    if !target.is_supported() {
        return Err(unsupported(target));
    }
    if target.platform() == crate::discovery::Platform::MacOS {
        let nix = tc.require_nix()?;
        run_capture(&nix.path, &darwin_rollback_args(repo)?)
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
pub(crate) fn upgrade(repo: &str, tc: &ToolInventory) -> Result<()> {
    let nix = tc.require_nix()?;
    run_stream(&nix.path, &upgrade_args(repo))
}

/// upgrade_captured: 出力をキャプチャ (GUI 用)
pub(crate) fn upgrade_captured(repo: &str, tc: &ToolInventory) -> Result<String> {
    let nix = tc.require_nix()?;
    run_capture(&nix.path, &upgrade_args(repo))
}

/// scan: 環境スキャン結果を文字列で返す。Nix/Git が未解決でも落ちない
pub fn scan(target: &ConfigurationTarget, tc: &ToolInventory) -> String {
    let mut out = String::new();
    out.push_str(&format!("OS:   {}\n", std::env::consts::OS));
    out.push_str(&format!("arch: {}\n", std::env::consts::ARCH));
    out.push_str(&format!("host: {target}\n"));
    match tc.nix.as_ref() {
        Some(nix) => out.push_str(&format!(
            "nix:  {} ({}, {})\n",
            nix.path.display(),
            nix.source,
            nix.version.as_deref().unwrap_or("unknown")
        )),
        None => out.push_str("nix:  not found\n"),
    }
    if let Some(brew) = &tc.homebrew {
        out.push_str(&format!(
            "brew: {} ({})\n",
            brew.path.display(),
            brew.version.as_deref().unwrap_or("unknown")
        ));
    } else {
        out.push_str("brew: not found\n");
    }
    match tc.git.as_ref() {
        Some(git) => out.push_str(&format!(
            "git:  {} ({})\n",
            git.path.display(),
            git.version.as_deref().unwrap_or("unknown")
        )),
        None => out.push_str("git:  not found\n"),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::detect_target_for;
    use crate::tool::{ResolvedTool, ToolSource};
    use std::path::PathBuf;

    /// profile 解決に必要な schneeforge.toml を持つ repo dir を作る
    fn manifest_repo(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("schneeforge-actions-repo-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("schneeforge.toml"),
            "schema = 1\n[profiles]\ndefault = \"developer\"\navailable = [\"minimal\", \"developer\"]\n",
        )
        .unwrap();
        dir.to_string_lossy().to_string()
    }

    fn dummy_tc() -> ToolInventory {
        ToolInventory {
            nix: Some(ResolvedTool::new(
                PathBuf::from("/usr/local/bin/nix"),
                ToolSource::Homebrew,
            )),
            git: Some(ResolvedTool::new(
                PathBuf::from("/usr/bin/git"),
                ToolSource::Path,
            )),
            homebrew: None,
            nh: None,
        }
    }

    #[test]
    fn scan_contains_host() {
        let target = detect_target_for("macos", "aarch64");
        let out = scan(&target, &dummy_tc());
        assert!(out.contains("darwin-aarch64"));
        assert!(out.contains("/usr/local/bin/nix"));
    }

    #[test]
    fn scan_reports_missing_brew() {
        let target = detect_target_for("linux", "x86_64");
        let out = scan(&target, &dummy_tc());
        assert!(out.contains("brew: not found"));
    }

    #[test]
    fn scan_reports_missing_nix_and_git_for_fresh_machine() {
        // Fresh install 環境では nix / git が未解決でも scan は落ちない
        let target = detect_target_for("macos", "aarch64");
        let out = scan(
            &target,
            &ToolInventory {
                nix: None,
                git: None,
                homebrew: None,
                nh: None,
            },
        );
        assert!(out.contains("nix:  not found"));
        assert!(out.contains("git:  not found"));
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
    fn apply_returns_nix_not_found_when_nix_missing() {
        let target = detect_target_for("linux", "x86_64");
        let tc = ToolInventory {
            nix: None,
            git: Some(ResolvedTool::new(
                PathBuf::from("/usr/bin/git"),
                ToolSource::Path,
            )),
            homebrew: None,
            nh: None,
        };
        let err = apply(&target, "/tmp/repo", &tc).unwrap_err();
        assert!(
            err.to_string().contains("nix not found"),
            "expected nix-not-found message, got: {err}"
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
    fn rollback_macos_returns_nix_not_found_when_nix_missing() {
        let target = detect_target_for("macos", "aarch64");
        let tc = ToolInventory {
            nix: None,
            git: Some(ResolvedTool::new(
                PathBuf::from("/usr/bin/git"),
                ToolSource::Path,
            )),
            homebrew: None,
            nh: None,
        };
        let err = rollback(&target, "/tmp/repo", &tc).unwrap_err();
        assert!(err.to_string().contains("nix not found"));
    }

    #[test]
    fn rollback_macos_uses_resolved_nix() {
        // macOS rollback は tc.nix.path を使う。ここでは引数組み立てまで検証
        let repo = manifest_repo("rollback-pinned");
        let args = darwin_rollback_args(&repo).unwrap();
        assert_eq!(args[1], "--inputs-from");
        assert_eq!(args[2], repo);
    }

    #[test]
    fn switch_command_macos() {
        let target = detect_target_for("macos", "aarch64");
        let repo = manifest_repo("switch-macos");
        let args = darwin_switch_args(&repo, &target).unwrap();
        assert_eq!(args[0], "run");
        assert_eq!(args[1], "--inputs-from");
        assert_eq!(args[2], repo);
        // machine / profile の 2 組の --override-input が並ぶ
        let oi = args.iter().position(|a| a == "--override-input").unwrap();
        assert_eq!(args[oi + 1], "machine");
        let last_oi = args.iter().rposition(|a| a == "--override-input").unwrap();
        assert_eq!(args[last_oi + 1], "profile");
        assert_eq!(args[last_oi + 3], "nix-darwin#darwin-rebuild");
        assert_eq!(args[last_oi + 4], "--");
        assert_eq!(args[last_oi + 5], "switch");
        assert_eq!(args[last_oi + 6], "--flake");
        assert!(args
            .last()
            .unwrap()
            .contains("#darwinConfigurations.darwin-aarch64"));
    }

    #[test]
    fn switch_command_linux() {
        let target = detect_target_for("linux", "x86_64");
        let repo = manifest_repo("switch-linux");
        let args = linux_build_args(&repo, &target, "/tmp/link").unwrap();
        assert_eq!(args[0], "build");
        assert_eq!(args[1], "--out-link");
        assert_eq!(args[2], "/tmp/link");
        assert!(args
            .iter()
            .any(|a| a == &format!("{repo}#homeConfigurations.linux.activationPackage")));
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
    fn upgrade_returns_nix_not_found_when_nix_missing() {
        let tc = ToolInventory {
            nix: None,
            git: None,
            homebrew: None,
            nh: None,
        };
        let err = upgrade("/tmp/repo", &tc).unwrap_err();
        assert!(err.to_string().contains("nix not found"));
    }

    #[test]
    fn apply_is_nh_free() {
        let repo = manifest_repo("nh-free");
        let mac = detect_target_for("macos", "aarch64");
        let mac_args = darwin_switch_args(&repo, &mac).unwrap();
        assert!(!mac_args.iter().any(|a| a == "nh"));
        assert!(mac_args.iter().any(|a| a == "nix-darwin#darwin-rebuild"));

        let linux = detect_target_for("linux", "x86_64");
        let linux_args = linux_build_args(&repo, &linux, "/tmp/link").unwrap();
        assert!(!linux_args.iter().any(|a| a == "nh"));
    }

    #[test]
    fn darwin_rollback_is_pinned_and_repo_aware() {
        let repo = manifest_repo("rollback-pinned");
        let args = darwin_rollback_args(&repo).unwrap();
        assert_eq!(args[1], "--inputs-from");
        assert_eq!(args[2], repo);
        let last_oi = args.iter().rposition(|a| a == "--override-input").unwrap();
        assert_eq!(args[last_oi + 1], "profile");
        assert_eq!(args[last_oi + 3], "nix-darwin#darwin-rebuild");
        assert_eq!(args[last_oi + 5], "--rollback");
    }

    #[test]
    fn override_args_point_at_state_dir() {
        // profile 解決は manifest を要求するため repo を用意する
        let dir = std::env::temp_dir().join("schneeforge-actions-override-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("schneeforge.toml"),
            "schema = 1\n[profiles]\ndefault = \"minimal\"\navailable = [\"minimal\"]\n",
        )
        .unwrap();
        let args = profile::override_args(dir.to_string_lossy().as_ref()).unwrap();
        let machine_idx = args
            .iter()
            .position(|a| a == "machine")
            .expect("machine input name present");
        assert_eq!(args[machine_idx - 1], "--override-input");
        let machine_path = &args[machine_idx + 1];
        // file 指す path input は path: URL 形式であること (nix 2.35 の要件)
        assert!(
            machine_path.starts_with("path:"),
            "must use path: URL form: {machine_path}"
        );
        assert!(machine_path.ends_with("machine.nix"));
        let profile_idx = args
            .iter()
            .position(|a| a == "profile")
            .expect("profile input name present");
        assert_eq!(args[profile_idx - 1], "--override-input");
        let profile_path = &args[profile_idx + 1];
        assert!(profile_path.starts_with("path:"));
        assert!(profile_path.ends_with("profile.nix"));
        let file = machine_path.trim_start_matches("path:");
        assert!(std::path::Path::new(file).is_file(), "exists: {file}");
        let profile_file = profile_path.trim_start_matches("path:");
        assert!(std::fs::read_to_string(profile_file)
            .unwrap()
            .contains("minimal"));
    }

    #[test]
    fn switch_and_build_args_carry_machine_override() {
        let dir = std::env::temp_dir().join("schneeforge-actions-args-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("schneeforge.toml"),
            "schema = 1\n[profiles]\ndefault = \"minimal\"\navailable = [\"minimal\"]\n",
        )
        .unwrap();
        let repo = dir.to_string_lossy().to_string();

        let mac = detect_target_for("macos", "aarch64");
        let mac_args = darwin_switch_args(&repo, &mac).unwrap();
        let oi = mac_args
            .iter()
            .position(|a| a == "--override-input")
            .expect("override present");
        assert_eq!(mac_args[oi + 1], "machine");

        let linux = detect_target_for("linux", "x86_64");
        let linux_args = linux_build_args(&repo, &linux, "/tmp/link").unwrap();
        assert!(linux_args.iter().any(|a| a == "--override-input"));
        // flake ref は最後のまま
        assert!(linux_args.last().unwrap().starts_with(&format!("{repo}#")));
    }
}
