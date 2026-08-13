use serde::Serialize;

use crate::discovery::{current_user, detect_arch, detect_platform, detect_target};
use crate::manifest::{Manifest, Validation};
use crate::process::{command_succeeds, run_capture};
use crate::repo::resolve_repo;
use crate::state::StateStore;
use crate::tool::{ResolvedTool, ToolResolver, ToolSource, ToolStatus, Toolchain};

/// ツール検出結果のまとめ（後方互換・GUI serialize 用）
///
/// 新しいコードは [`NixHealth`] / [`Toolchain`] を直接参照すること。
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
    /// 実行 OS ユーザー (manifest の username とは独立)
    pub system_user: Option<String>,
    /// 実行ユーザーの HOME
    pub home: Option<String>,
    /// manifest の実行時検証結果 (読めた場合のみ)
    pub validation: Option<Validation>,
    pub tools: ToolsSummary,
    /// Nix の詳細ヘルス（store 接続・flakes 有効・出処）
    pub nix_health: NixHealth,
    /// 解決済み toolchain（GUI が内部で使う・表示用）
    pub toolchain: ToolchainSummary,
    pub applied_revision: Option<String>,
    pub applied_at: Option<String>,
}

/// Nix の詳細ヘルス状態
#[derive(Debug, Clone, Serialize)]
pub struct NixHealth {
    pub installed: bool,
    pub executable: Option<String>,
    pub version: Option<String>,
    pub store_accessible: bool,
    pub flakes_available: bool,
    pub source: Option<ToolSource>,
    pub error: Option<String>,
}

/// Toolchain のサマリ（GUI への表示用・serialize 可能）
#[derive(Debug, Clone, Serialize)]
pub struct ToolchainSummary {
    pub nix: ResolvedToolSummary,
    pub git: ResolvedToolSummary,
    pub homebrew: Option<ResolvedToolSummary>,
    pub nh: Option<ResolvedToolSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedToolSummary {
    pub path: String,
    pub source: ToolSource,
    pub version: Option<String>,
}

impl From<&ResolvedTool> for ResolvedToolSummary {
    fn from(t: &ResolvedTool) -> Self {
        Self {
            path: t.path.to_string_lossy().to_string(),
            source: t.source,
            version: t.version.clone(),
        }
    }
}

/// 診断 Status を構築する。`Toolchain` を1回解決し、NixHealth 検査に使う
pub fn diagnose(cli_repo: Option<&str>) -> Diagnostics {
    let resolver = ToolResolver::new();
    let nix = resolver.resolve_tool_with_version("nix");
    let nh = resolver.resolve_tool_with_version("nh");
    let git = resolver.resolve_tool_with_version("git");
    let homebrew = resolver.resolve_tool_with_version("brew");

    // 後方互換 ToolsSummary
    let tools = ToolsSummary {
        nix: ToolStatus::from(nix.as_ref()),
        nh: ToolStatus::from(nh.as_ref()),
        git: ToolStatus::from(git.as_ref()),
        homebrew: ToolStatus::from(homebrew.as_ref()),
    };

    // Toolchain 構築（Nix/Git が欠けても Diagnostics 自体は返せるように None を埋める）
    let placeholder = ResolvedTool::new(
        std::path::PathBuf::from("/__not_resolved__"),
        ToolSource::Path,
    );
    let tc = Toolchain {
        nix: nix.clone().unwrap_or_else(|| placeholder.clone()),
        git: git.clone().unwrap_or_else(|| placeholder.clone()),
        homebrew: homebrew.clone(),
        nh: nh.clone(),
    };

    let nix_health = nix_health(&tc);

    let target = detect_target();
    let repo_path = resolve_repo(cli_repo);
    let repo_exists = std::path::Path::new(&repo_path).is_dir();

    let (manifest_found, manifest_error, username, validation) = manifest_diagnostics(&repo_path);

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
        system_user: current_user(),
        home: std::env::var("HOME").ok(),
        validation,
        tools,
        nix_health,
        toolchain: ToolchainSummary {
            nix: ResolvedToolSummary::from(&tc.nix),
            git: ResolvedToolSummary::from(&tc.git),
            homebrew: tc.homebrew.as_ref().map(ResolvedToolSummary::from),
            nh: tc.nh.as_ref().map(ResolvedToolSummary::from),
        },
        applied_revision: state.as_ref().and_then(|s| s.applied_revision.clone()),
        applied_at: state.as_ref().and_then(|s| s.applied_at.clone()),
    }
}

/// Nix のヘルスを検査する。store 接続（`nix store ping`）と flakes 有効性
/// （`nix config show experimental-features`）を subprocess で確認する
pub fn nix_health(tc: &Toolchain) -> NixHealth {
    let nix = &tc.nix;
    if !nix.path.is_file() {
        return NixHealth {
            installed: false,
            executable: None,
            version: None,
            store_accessible: false,
            flakes_available: false,
            source: None,
            error: Some(format!(
                "nix not found in PATH or known locations (source={}). \
                 install: curl -L https://nixos.org/nix/install | sh",
                nix.source
            )),
        };
    }

    let version = nix.version.clone();
    let store_accessible = command_succeeds(&nix.path, &["store".to_string(), "ping".to_string()]);
    let flakes_available = run_capture(
        &nix.path,
        &[
            "config".to_string(),
            "show".to_string(),
            "experimental-features".to_string(),
        ],
    )
    .map(|out| out.contains("flakes"))
    .unwrap_or(false);

    // XDG state フォルダ欠如の検出（Nix installer が作らない有名な罠）
    let xdg_state_ok = xdg_state_profile_dir().map(|d| d.is_dir()).unwrap_or(true);

    let error = if !store_accessible {
        Some("`nix store ping` failed; nix-daemon not running or socket not accessible".to_string())
    } else if !flakes_available {
        Some(
            "experimental-features does not include `flakes`; \
             run `schneeforge doctor` or add `experimental-features = nix-command flakes`"
                .to_string(),
        )
    } else if !xdg_state_ok {
        Some(
            "~/.local/state/nix/profiles not found; \
             run `mkdir -p ~/.local/state/nix/profiles` or reinstall Nix"
                .to_string(),
        )
    } else {
        None
    };

    NixHealth {
        installed: true,
        executable: Some(nix.path.to_string_lossy().to_string()),
        version,
        store_accessible,
        flakes_available,
        source: Some(nix.source),
        error,
    }
}

/// XDG state の Nix profile ディレクトリ（フォルダ欠如検出用）
fn xdg_state_profile_dir() -> Option<std::path::PathBuf> {
    let base = std::env::var("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|_| {
            std::env::var("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state"))
        })
        .ok()?;
    Some(base.join("nix/profiles"))
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
    use crate::bootstrap::preflight;
    use crate::tool::{ResolvedTool, ToolSource};
    use std::path::PathBuf;

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

    #[test]
    fn diagnose_includes_nix_health() {
        let d = diagnose(Some("/definitely/not/a/real/repo"));
        // nix_health フィールドが serialize 可能で、フィールド群が揃っていること
        let json = serde_json::to_string(&d.nix_health).expect("NixHealth must be serializable");
        assert!(json.contains("installed"));
        assert!(json.contains("store_accessible"));
        assert!(json.contains("flakes_available"));
    }

    #[test]
    fn diagnose_includes_toolchain_summary() {
        let d = diagnose(Some("/definitely/not/a/real/repo"));
        // toolchain フィールドが serialize 可能な形で入っている
        let json =
            serde_json::to_string(&d.toolchain).expect("ToolchainSummary must be serializable");
        assert!(json.contains("path"));
        assert!(json.contains("source"));
    }

    #[test]
    fn nix_health_returns_not_installed_when_binary_missing() {
        // 存在しないパスを toolchain に詰めて nix_health を呼ぶ
        let tc = Toolchain {
            nix: ResolvedTool::new(
                PathBuf::from("/__definitely_not_a_real_nix__"),
                ToolSource::SystemProfile,
            ),
            git: ResolvedTool::new(PathBuf::from("/usr/bin/git"), ToolSource::Path),
            homebrew: None,
            nh: None,
        };
        let health = nix_health(&tc);
        assert!(!health.installed);
        assert!(health.executable.is_none());
        assert!(health.source.is_none());
        assert!(health.error.is_some());
        assert!(health.error.unwrap().contains("not found"));
    }

    #[test]
    fn preflight_and_nix_health_share_flakes_detection() {
        // preflight の flakes 判定と nix_health の flakes_available は
        // 同じ `nix config show experimental-features` の出力を parse する。
        // ここでは nix_health が false を返す状況をシミュレート
        let tc = Toolchain {
            nix: ResolvedTool::new(
                PathBuf::from("/__not_a_real_nix__"),
                ToolSource::SystemProfile,
            ),
            git: ResolvedTool::new(PathBuf::from("/usr/bin/git"), ToolSource::Path),
            homebrew: None,
            nh: None,
        };
        let pre = preflight(&tc);
        let health = nix_health(&tc);
        // Nix が見つからない場合、両方とも flakes 利用不可となる
        assert!(!pre.flakes_enabled);
        assert!(!health.flakes_available);
    }
}
