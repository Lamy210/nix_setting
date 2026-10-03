use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::discovery::detect_target;
use crate::error::{Error, Result};
use crate::nix_features::has_required_flake_features;
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

fn append_config_line(path: &Path, line: &str) -> Result<()> {
    let needs_separator = match std::fs::read(path) {
        Ok(content) => !content.is_empty() && !content.ends_with(b"\n"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => {
            return Err(Error::Io(format!("read {}: {e}", path.display())));
        }
    };

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| Error::Io(format!("open {}: {e}", path.display())))?;

    use std::io::Write;
    if needs_separator {
        file.write_all(b"\n")
            .map_err(|e| Error::Io(format!("write {}: {e}", path.display())))?;
    }
    file.write_all(line.as_bytes())
        .map_err(|e| Error::Io(format!("write {}: {e}", path.display())))
}

fn nix_config_path_from(
    xdg_config_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<PathBuf> {
    if let Some(xdg_config_home) = xdg_config_home.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(xdg_config_home).join("nix").join("nix.conf"));
    }

    if let Some(home) = home.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(home)
            .join(".config")
            .join("nix")
            .join("nix.conf"));
    }

    Err(Error::Precondition(
        "cannot enable flakes: XDG_CONFIG_HOME or HOME must be set".to_string(),
    ))
}

/// nix.conf に experimental-features (nix-command flakes) を追記する
///
/// flakes 有効化は Nix を必要とする操作 (run_capture で現在の設定を確認するため)。
pub fn enable_flakes(tc: &ToolInventory) -> Result<()> {
    let nix = tc.require_nix()?;

    // nix.conf の文字列ではなく、resolved Nix が実際に認識している設定を
    // authoritative source とする。コメント中の "flakes" や nix-command 欠落を
    // 有効状態として扱わない。
    let current = run_capture(
        &nix.path,
        &[
            "config".to_string(),
            "show".to_string(),
            "experimental-features".to_string(),
        ],
    )?;
    if has_required_flake_features(&current) {
        return Ok(());
    }

    let conf = nix_config_path_from(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )?;
    if let Some(parent) = conf.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::Io(format!("create_dir: {e}")))?;
    }
    append_config_line(&conf, "extra-experimental-features = nix-command flakes\n")
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
                .map(|out| has_required_flake_features(&out))
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

fn ensure_setup_nix_and_flakes_with(
    tc: &ToolInventory,
    mut inspect: impl FnMut(&ToolInventory) -> PreflightReport,
    mut enable: impl FnMut(&ToolInventory) -> Result<()>,
) -> Result<()> {
    let pre = inspect(tc);
    if !pre.nix_installed {
        return Err(Error::Precondition(
            "Nix is not installed; install it with `schneeforge nix install` (Managed Nix)"
                .to_string(),
        ));
    }
    if pre.flakes_enabled {
        return Ok(());
    }

    enable(tc)?;

    // bootstrap-flow contract: flakes を有効化した後は再診断し、実際に有効に
    // なったことを確認してから apply へ進む。
    let post = inspect(tc);
    if !post.nix_installed {
        return Err(Error::Precondition(
            "Nix became unavailable while enabling flakes; run `schneeforge doctor`".to_string(),
        ));
    }
    if !post.flakes_enabled {
        return Err(Error::Precondition(
            "flakes are still disabled after updating nix.conf; verify with `nix config show experimental-features`"
                .to_string(),
        ));
    }
    Ok(())
}

/// 初回セットアップ: Nix 確認 → flakes 有効化 + 再診断 → apply
pub fn setup(repo: &str, store: &StateStore, tc: &ToolInventory) -> Result<ApplyResult> {
    ensure_setup_nix_and_flakes_with(tc, preflight, enable_flakes)?;
    let target = detect_target();
    apply(&target, repo, store, tc, false)
}

/// config.toml を生成する (旧 API・v2 では machine input へ移行済み)。
/// 後方互換のため残すが username は machine.nix 生成に置き換わる
pub fn generate_config(_repo: &str, username: &str) -> Result<()> {
    if username.trim().is_empty() || username.chars().any(|c| c.is_control()) {
        return Err(Error::Precondition("invalid username".to_string()));
    }
    // machine 情報は repo へ書かない (v2)。machine input は apply 時に生成される
    Ok(())
}

/// repository を clone する。clone 出力を返す
///
/// `url` が空のときは既定 repository (upstream) を使う。既定値は
/// `SCHNEEFORGE_REPO_URL` 環境変数で上書きできる (install.sh の
/// `REPO_URL="${SCHNEEFORGE_REPO_URL:-...}"` と同じ規約。fork を使う
/// ユーザーは環境変数か wizard 入力で上書きする)。
/// clone は Git を必要とする操作。
pub const DEFAULT_REPO_URL: &str = "https://github.com/Lamy210/nix_setting.git";

pub fn clone_repo(url: &str, dest: &str, tc: &ToolInventory) -> Result<String> {
    let effective_url = if url.trim().is_empty() {
        crate::source::repo_url()
    } else {
        url.to_string()
    };
    if effective_url.trim().is_empty()
        || effective_url.starts_with('-')
        || effective_url.contains("::")
    {
        return Err(Error::Precondition("invalid repository URL".to_string()));
    }
    let git = tc.require_git()?;
    run_capture(
        &git.path,
        &[
            "clone".to_string(),
            effective_url.to_string(),
            dest.to_string(),
        ],
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
    fn enable_flakes_propagates_effective_config_inspection_failure() {
        let tc = ToolInventory {
            nix: Some(ResolvedTool::new(
                PathBuf::from("/__schneeforge_missing_nix_binary__"),
                ToolSource::Path,
            )),
            git: None,
            homebrew: None,
            nh: None,
        };

        let result = enable_flakes(&tc);

        assert!(matches!(result, Err(Error::Command { .. })));
    }

    #[test]
    fn nix_config_path_prefers_nonempty_xdg_config_home() {
        let path = nix_config_path_from(
            Some(std::ffi::OsString::from("/tmp/xdg-config")),
            Some(std::ffi::OsString::from("/home/alice")),
        )
        .unwrap();

        assert_eq!(path, PathBuf::from("/tmp/xdg-config/nix/nix.conf"));
    }

    #[test]
    fn nix_config_path_treats_empty_xdg_as_unset_and_uses_home() {
        let path = nix_config_path_from(
            Some(std::ffi::OsString::new()),
            Some(std::ffi::OsString::from("/home/alice")),
        )
        .unwrap();

        assert_eq!(path, PathBuf::from("/home/alice/.config/nix/nix.conf"));
    }

    #[test]
    fn nix_config_path_rejects_missing_or_empty_home_instead_of_using_cwd() {
        for (xdg, home) in [
            (None, None),
            (Some(std::ffi::OsString::new()), None),
            (None, Some(std::ffi::OsString::new())),
        ] {
            let err = nix_config_path_from(xdg, home).unwrap_err();
            assert!(matches!(err, Error::Precondition(_)), "{err}");
            assert!(err.to_string().contains("HOME"), "{err}");
        }
    }

    #[test]
    fn append_config_line_separates_unterminated_existing_content() {
        let dir = std::env::temp_dir().join(format!(
            "schneeforge-nix-config-append-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let conf = dir.join("nix.conf");
        std::fs::write(&conf, "experimental-features = ca-derivations").unwrap();

        append_config_line(&conf, "extra-experimental-features = nix-command flakes\n").unwrap();

        let content = std::fs::read_to_string(&conf).unwrap();
        assert_eq!(
            content,
            "experimental-features = ca-derivations\nextra-experimental-features = nix-command flakes\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_config_line_creates_new_file_without_leading_separator() {
        let dir = std::env::temp_dir().join(format!(
            "schneeforge-nix-config-create-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let conf = dir.join("nix.conf");

        append_config_line(&conf, "extra-experimental-features = nix-command flakes\n").unwrap();

        assert_eq!(
            std::fs::read_to_string(&conf).unwrap(),
            "extra-experimental-features = nix-command flakes\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
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

    fn preflight_state(nix_installed: bool, flakes_enabled: bool) -> PreflightReport {
        PreflightReport {
            nix_installed,
            flakes_enabled,
            git_installed: true,
        }
    }

    #[test]
    fn setup_preconditions_recheck_flakes_after_enable() {
        let tc = dummy_tc();
        let mut inspections = 0;
        let mut enables = 0;

        ensure_setup_nix_and_flakes_with(
            &tc,
            |_| {
                inspections += 1;
                if inspections == 1 {
                    preflight_state(true, false)
                } else {
                    preflight_state(true, true)
                }
            },
            |_| {
                enables += 1;
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(
            inspections, 2,
            "flakes enable must be followed by re-diagnosis"
        );
        assert_eq!(enables, 1);
    }

    #[test]
    fn setup_preconditions_fail_if_flakes_remain_disabled() {
        let tc = dummy_tc();
        let mut inspections = 0;
        let mut enables = 0;

        let err = ensure_setup_nix_and_flakes_with(
            &tc,
            |_| {
                inspections += 1;
                preflight_state(true, false)
            },
            |_| {
                enables += 1;
                Ok(())
            },
        )
        .unwrap_err();

        assert_eq!(
            inspections, 2,
            "failure must be based on post-enable diagnosis"
        );
        assert_eq!(enables, 1);
        assert!(
            err.to_string().contains("flakes are still disabled"),
            "{err}"
        );
    }

    #[test]
    fn setup_preconditions_skip_enable_when_flakes_already_work() {
        let tc = dummy_tc();
        let mut inspections = 0;
        let mut enables = 0;

        ensure_setup_nix_and_flakes_with(
            &tc,
            |_| {
                inspections += 1;
                preflight_state(true, true)
            },
            |_| {
                enables += 1;
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(inspections, 1);
        assert_eq!(enables, 0);
    }

    #[test]
    fn setup_guides_missing_nix_to_managed_nix() {
        let dir = std::env::temp_dir().join(format!(
            "schneeforge-setup-managed-nix-guidance-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = StateStore::new(dir.join("state.json"));
        let tc = ToolInventory {
            nix: None,
            git: None,
            homebrew: None,
            nh: None,
        };

        let err = setup("/nonexistent/repo", &store, &tc).unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("schneeforge nix install"),
            "missing Nix guidance must use Managed Nix: {message}"
        );
        assert!(
            !message.contains("curl"),
            "missing Nix guidance must not recommend curl|sh: {message}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn generate_config_no_longer_writes_repo() {
        // v2: machine 情報は repo へ書かない。file は作成されない
        let dir = std::env::temp_dir().join("schneeforge-config-gen-v2");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = dir.to_string_lossy().to_string();

        generate_config(&repo, "alice").unwrap();

        assert!(!dir.join("config.toml").exists());
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
        assert!(clone_repo("-malicious", "/tmp/dest", &tc).is_err());
        assert!(clone_repo("proto:://evil", "/tmp/dest", &tc).is_err());
    }

    #[test]
    fn clone_repo_empty_url_falls_back_to_default() {
        // 空文字は error ではなく既定 repository への fallback。dummy_tc の
        // git (/usr/bin/git) が実在する環境では実際に clone が走ってしまうため、
        // destination を必ず失敗する path にして「URL validation は通過した」
        // (既定値が採用された) ことを git 実行段階の失敗で判定する
        let tc = dummy_tc();
        let err = clone_repo("", "/proc/no-such-dir/dest", &tc).unwrap_err();
        assert!(
            !err.to_string().contains("invalid repository URL"),
            "empty URL should fall back to default, got: {err}"
        );
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

    #[test]
    fn default_repo_url_is_upstream() {
        assert_eq!(
            DEFAULT_REPO_URL,
            "https://github.com/Lamy210/nix_setting.git"
        );
    }
}
