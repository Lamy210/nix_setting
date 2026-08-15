use std::path::PathBuf;

use serde::Serialize;

use crate::discovery::detect_target;
use crate::error::{Error, Result};
use crate::manifest::{Manifest, User};
use crate::operations::{apply, ApplyResult};
use crate::process::{command_succeeds, run_capture};
use crate::state::StateStore;
use crate::tool::ToolInventory;

/// システム診断情報
#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub os: String,
    pub arch: String,
    pub nix: bool,
    pub homebrew: bool,
    pub git: bool,
    pub host: String,
}

/// システム / Nix / ホスト互換性を診断する
///
/// Nix/Git が未解決の場合でも成功し、各フラグは discover 時の発見可否と
/// 現在の実行可能ファイル状態を反映する (`verify` と同じ基準)。
/// これにより Fresh install 環境でも Doctor が診断結果を出力できる。
pub fn doctor(tc: &ToolInventory) -> DoctorReport {
    DoctorReport {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        nix: tc.nix.as_ref().is_some_and(|t| t.path.is_file()),
        homebrew: tc.homebrew.is_some(),
        git: tc.git.as_ref().is_some_and(|t| t.path.is_file()),
        host: detect_target().name().to_string(),
    }
}

/// nix.conf に experimental-features (nix-command flakes) を追記する
///
/// flakes 有効化は Nix を必要とする操作 (run_capture で現在の設定を確認するため)。
pub fn enable_flakes(tc: &ToolInventory) -> Result<()> {
    let nix = tc.require_nix()?;
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|_| PathBuf::from("."));
    let conf = base.join("nix").join("nix.conf");

    // 解決済み nix を使って現在の設定を確認し、既に flakes が入っていれば何もしない
    if let Ok(content) = std::fs::read_to_string(&conf) {
        if content.contains("flakes") {
            return Ok(());
        }
    }
    // 念のため resolved nix 経由で現在の有効設定も確認（ファイルと実際の挙動が一致しないケース）
    let current =
        run_capture(&nix.path, &["config".to_string(), "show".to_string()]).unwrap_or_default();
    if current.contains("flakes") {
        return Ok(());
    }

    if let Some(parent) = conf.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::Io(format!("create_dir: {e}")))?;
    }
    let line = "experimental-features = nix-command flakes\n";
    match std::fs::OpenOptions::new().append(true).open(&conf) {
        Ok(mut f) => {
            use std::io::Write;
            f.write_all(line.as_bytes())
                .map_err(|e| Error::Io(format!("write {}: {e}", conf.display())))?;
        }
        Err(_) => {
            std::fs::write(&conf, line)
                .map_err(|e| Error::Io(format!("write {}: {e}", conf.display())))?;
        }
    }
    Ok(())
}

/// 初回セットアップ前の前提条件チェック結果（nix と flakes を分離）
#[derive(Debug, Clone, Serialize)]
pub struct PreflightReport {
    pub nix_installed: bool,
    pub flakes_enabled: bool,
    pub git_installed: bool,
}

impl PreflightReport {
    pub fn is_ok(&self) -> bool {
        self.nix_installed && self.flakes_enabled && self.git_installed
    }
}

/// Nix / Git / flakes が実際に動作するかを確認する。
///
/// `nix` / `flakes` を別状態として返す（Nix 未検出と flakes 無効を区別するため）。
/// Nix / Git が未解決の場合は対応フラグが false になる（Preflight 自体は infallible）。
/// flakes 判定は `<resolved_nix> config show experimental-features` の出力を parse する。
pub fn preflight(tc: &ToolInventory) -> PreflightReport {
    let nix_installed = tc
        .nix
        .as_ref()
        .map(|n| command_succeeds(&n.path, &["--version".to_string()]))
        .unwrap_or(false);
    let git_installed = tc
        .git
        .as_ref()
        .map(|g| command_succeeds(&g.path, &["--version".to_string()]))
        .unwrap_or(false);

    let flakes_enabled = if nix_installed {
        tc.nix
            .as_ref()
            .map(|nix| {
                run_capture(
                    &nix.path,
                    &[
                        "config".to_string(),
                        "show".to_string(),
                        "experimental-features".to_string(),
                    ],
                )
                .map(|out| out.contains("flakes"))
                .unwrap_or(false)
            })
            .unwrap_or(false)
    } else {
        false
    };

    PreflightReport {
        nix_installed,
        flakes_enabled,
        git_installed,
    }
}

/// 初回セットアップ: Nix 確認 → flakes 有効化 → apply
pub fn setup(repo: &str, store: &StateStore, tc: &ToolInventory) -> Result<ApplyResult> {
    let pre = preflight(tc);
    if !pre.nix_installed {
        return Err(Error::Precondition(
            "Nix is not installed (install: curl -L https://nixos.org/nix/install | sh)"
                .to_string(),
        ));
    }
    if !pre.flakes_enabled {
        enable_flakes(tc)?;
    }
    let target = detect_target();
    apply(&target, repo, store, tc, false)
}

/// config.toml を生成する (schema=1, user.username=<username>)
pub fn generate_config(repo: &str, username: &str) -> Result<()> {
    if username.trim().is_empty() || username.chars().any(|c| c.is_control()) {
        return Err(Error::Precondition("invalid username".to_string()));
    }
    let manifest = Manifest {
        schema: 1,
        user: User {
            username: username.to_string(),
        },
    };
    let content =
        toml::to_string(&manifest).map_err(|e| Error::Io(format!("serialize config.toml: {e}")))?;
    let path = std::path::Path::new(repo).join("config.toml");
    std::fs::write(&path, content).map_err(|e| Error::Io(format!("write {}: {e}", path.display())))
}

/// repository を clone する。clone 出力を返す
///
/// clone は Git を必要とする操作。
pub fn clone_repo(url: &str, dest: &str, tc: &ToolInventory) -> Result<String> {
    if url.trim().is_empty() || url.starts_with('-') || url.contains("::") {
        return Err(Error::Precondition("invalid repository URL".to_string()));
    }
    let git = tc.require_git()?;
    run_capture(
        &git.path,
        &["clone".to_string(), url.to_string(), dest.to_string()],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{ResolvedTool, ToolSource};
    use std::path::PathBuf;

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
    fn preflight_report_is_ok_when_all_present() {
        let report = PreflightReport {
            nix_installed: true,
            flakes_enabled: true,
            git_installed: true,
        };
        assert!(report.is_ok());
    }

    #[test]
    fn preflight_report_fails_when_flakes_missing() {
        let report = PreflightReport {
            nix_installed: true,
            flakes_enabled: false,
            git_installed: true,
        };
        assert!(!report.is_ok());
    }

    #[test]
    fn preflight_report_fails_when_nix_missing() {
        let report = PreflightReport {
            nix_installed: false,
            flakes_enabled: false,
            git_installed: true,
        };
        assert!(!report.is_ok());
    }

    #[test]
    fn preflight_report_distinguishes_nix_and_flakes() {
        // Nix あり・flakes 無し は別状態
        let report = PreflightReport {
            nix_installed: true,
            flakes_enabled: false,
            git_installed: true,
        };
        assert!(report.nix_installed);
        assert!(!report.flakes_enabled);
    }

    #[test]
    fn generate_config_writes_parseable_toml() {
        let dir = std::env::temp_dir().join("schneeforge-config-gen");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = dir.to_string_lossy().to_string();

        generate_config(&repo, "alice").unwrap();

        let content = std::fs::read_to_string(format!("{repo}/config.toml")).unwrap();
        let manifest = Manifest::parse(&content).unwrap();
        assert_eq!(manifest.schema, 1);
        assert_eq!(manifest.user.username, "alice");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn generate_config_rejects_invalid_username() {
        assert!(generate_config("/tmp", "").is_err());
        assert!(generate_config("/tmp", "a\nb").is_err());
    }

    #[test]
    fn clone_repo_rejects_invalid_url() {
        let tc = dummy_tc();
        assert!(clone_repo("", "/tmp/dest", &tc).is_err());
        assert!(clone_repo("-malicious", "/tmp/dest", &tc).is_err());
        assert!(clone_repo("proto:://evil", "/tmp/dest", &tc).is_err());
    }

    #[test]
    fn clone_repo_returns_git_not_found_when_git_missing() {
        // Git 未解決の環境では clone_repo は GitNotFound (Precondition) で弾かれる
        let tc = ToolInventory {
            nix: Some(ResolvedTool::new(
                PathBuf::from("/usr/local/bin/nix"),
                ToolSource::Homebrew,
            )),
            git: None,
            homebrew: None,
            nh: None,
        };
        let err = clone_repo("https://example.com/repo.git", "/tmp/dest", &tc).unwrap_err();
        assert!(
            err.to_string().contains("git not found"),
            "expected git-not-found message, got: {err}"
        );
    }
}
