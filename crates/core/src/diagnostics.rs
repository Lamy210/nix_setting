use serde::Serialize;

use crate::discovery::{current_user, detect_arch, detect_platform, detect_target};
use crate::manifest::{Manifest, Validation};
use crate::repo::resolve_repo;
use crate::state::StateStore;
use crate::tool::{ToolResolver, ToolStatus};

/// ツール検出結果のまとめ
#[derive(Debug, Clone, Serialize)]
pub struct ToolsSummary {
    pub nix: ToolStatus,
    pub nh: ToolStatus,
    pub git: ToolStatus,
    pub homebrew: ToolStatus,
}

/// GUI / CLI が取得する診断 Status
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostics {
    /// ConfigurationTarget 名 (例: "macbook-air")
    pub host: String,
    /// OS (Platform) — ConfigurationTarget とは独立
    pub platform: String,
    /// CPU arch — ConfigurationTarget とは独立
    pub architecture: String,
    /// 解決済み repository path
    pub repo_path: String,
    /// repository が存在するか
    pub repo_exists: bool,
    /// config.toml が読み込めたか
    pub manifest_found: bool,
    /// config.toml の読み込み・parse エラー原因
    pub manifest_error: Option<String>,
    /// manifest の username (読めた場合のみ)
    pub username: Option<String>,
    /// manifest の実行時検証結果 (読めた場合のみ)
    pub validation: Option<Validation>,
    pub tools: ToolsSummary,
    pub applied_revision: Option<String>,
    pub applied_at: Option<String>,
}

/// 診断 Status を構築する
pub fn diagnose(cli_repo: Option<&str>) -> Diagnostics {
    let target = detect_target();
    let repo_path = resolve_repo(cli_repo);
    let repo_exists = std::path::Path::new(&repo_path).is_dir();

    let (manifest_found, manifest_error, username, validation) = manifest_diagnostics(&repo_path);

    let resolver = ToolResolver::new();
    let tools = ToolsSummary {
        nix: resolver.resolve_with_version("nix"),
        nh: resolver.resolve_with_version("nh"),
        git: resolver.resolve_with_version("git"),
        homebrew: resolver.resolve_with_version("brew"),
    };

    let state = StateStore::default().load();

    Diagnostics {
        host: target.name().to_string(),
        platform: detect_platform().to_string(),
        architecture: detect_arch().to_string(),
        repo_path,
        repo_exists,
        manifest_found,
        manifest_error,
        username,
        validation,
        tools,
        applied_revision: state.as_ref().and_then(|s| s.applied_revision.clone()),
        applied_at: state.as_ref().and_then(|s| s.applied_at.clone()),
    }
}

fn manifest_diagnostics(
    repo_path: &str,
) -> (bool, Option<String>, Option<String>, Option<Validation>) {
    match Manifest::load(repo_path) {
        Ok(m) => {
            let validation = m.validate(current_user().as_deref());
            (true, None, Some(m.user.username.clone()), Some(validation))
        }
        Err(e) => (false, Some(e.to_string()), None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnose_nonexistent_repo() {
        let d = diagnose(Some("/definitely/not/a/real/repo"));
        assert!(!d.repo_exists);
        assert!(!d.manifest_found);
        assert!(d.manifest_error.is_some());
        assert_eq!(d.username, None);
        assert_eq!(d.validation, None);
    }

    #[test]
    fn diagnose_reports_platform_and_architecture() {
        let d = diagnose(Some("/definitely/not/a/real/repo"));
        assert!(!d.platform.is_empty());
        assert!(!d.architecture.is_empty());
        assert!(!d.host.is_empty());
    }
}
