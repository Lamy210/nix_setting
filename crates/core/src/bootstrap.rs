use std::path::PathBuf;

use crate::discovery::{detect_target, has_git, has_homebrew, has_nix};
use crate::error::{Error, Result};
use crate::operations::{apply, ApplyResult};
use crate::process::command_succeeds;
use crate::state::StateStore;

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
pub fn doctor() -> DoctorReport {
    DoctorReport {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        nix: has_nix(),
        homebrew: has_homebrew(),
        git: has_git(),
        host: detect_target().name().to_string(),
    }
}

/// nix.conf に experimental-features (nix-command flakes) を追記する
pub fn enable_flakes() -> Result<()> {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|_| PathBuf::from("."));
    let conf = base.join("nix").join("nix.conf");

    if let Ok(content) = std::fs::read_to_string(&conf) {
        if content.contains("flakes") {
            return Ok(());
        }
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

/// 初回セットアップ前の前提条件チェック結果
#[derive(Debug, Clone)]
pub struct PreflightReport {
    pub nix: bool,
    pub git: bool,
    pub flakes: bool,
}

impl PreflightReport {
    pub fn is_ok(&self) -> bool {
        self.nix && self.git && self.flakes
    }
}

/// Nix / Git / flakes が実際に動作するかを確認する
pub fn preflight() -> PreflightReport {
    PreflightReport {
        nix: command_succeeds("nix", &["--version".to_string()]),
        git: command_succeeds("git", &["--version".to_string()]),
        flakes: command_succeeds("nix", &["flake".to_string(), "--help".to_string()]),
    }
}

/// 初回セットアップ: Nix 確認 → flakes 有効化 → apply
pub fn setup(repo: &str, store: &StateStore) -> Result<ApplyResult> {
    let pre = preflight();
    if !pre.nix {
        return Err(Error::Precondition(
            "Nix is not installed (install: curl -L https://nixos.org/nix/install | sh)"
                .to_string(),
        ));
    }
    if !pre.flakes {
        enable_flakes()?;
    }
    let target = detect_target();
    apply(&target, repo, store, false)
}

/// state ファイルを削除する。削除した場合は true
pub fn uninstall(store: &StateStore) -> Result<bool> {
    let path = store.path();
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|e| Error::Io(format!("remove {}: {e}", path.display())))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_report_has_host() {
        let report = doctor();
        assert!(!report.host.is_empty());
    }

    #[test]
    fn uninstall_missing_state_returns_false() {
        let store =
            StateStore::new(std::env::temp_dir().join("schneeforge-uninstall-missing.json"));
        assert!(!uninstall(&store).unwrap());
    }

    #[test]
    fn preflight_report_is_ok_when_all_present() {
        let report = PreflightReport {
            nix: true,
            git: true,
            flakes: true,
        };
        assert!(report.is_ok());
    }

    #[test]
    fn preflight_report_fails_when_flakes_missing() {
        let report = PreflightReport {
            nix: true,
            git: true,
            flakes: false,
        };
        assert!(!report.is_ok());
    }
}
