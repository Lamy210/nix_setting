use serde::Serialize;

use crate::discovery::{current_user, detect_arch, detect_platform, detect_target};
use crate::manifest::{Manifest, Validation};
use crate::process::{command_succeeds, run_capture};
use crate::repo::resolve_repo;
use crate::state::StateStore;
use crate::tool::{ResolvedTool, ToolInventory, ToolResolver, ToolSource, ToolStatus};

/// ツール検出結果のまとめ（後方互換・GUI serialize 用）
///
/// 新しいコードは [`NixHealth`] / [`ToolInventory`] を直接参照すること。
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
    /// 解決済み tool inventory（GUI が内部で使う・表示用）
    pub tool_inventory: ToolInventorySummary,
    pub applied_revision: Option<String>,
    pub applied_at: Option<String>,
}

/// Nix の詳細ヘルス状態
///
/// `error` は blocking な問題 (store 到達不能・flakes 無効・binary 無し)。
/// `warning` は blocker ではないが対応推奨の注意 (例: XDG state ディレクトリ欠如)。
/// 両方は同時にセットされ得る (例: store ping 失敗 + XDG state 欠如)。
/// frontend は `error.is_some()` を最優先で表示し、`warning` はその下に info 扱いで出す。
#[derive(Debug, Clone, Serialize)]
pub struct NixHealth {
    pub installed: bool,
    pub executable: Option<String>,
    pub version: Option<String>,
    pub store_accessible: bool,
    pub flakes_available: bool,
    pub source: Option<ToolSource>,
    pub error: Option<String>,
    /// blocker ではないが対応推奨の警告（例: XDG state ディレクトリ欠如）
    pub warning: Option<String>,
}

/// ToolInventory のサマリ（GUI への表示用・serialize 可能）
///
/// 全フィールド `Option` で統一 (Fresh install 環境で未発見ツールがある場合、
/// `None` となる。空文字列のダミーエントリは作らない)。
#[derive(Debug, Clone, Serialize)]
pub struct ToolInventorySummary {
    pub nix: Option<ResolvedToolSummary>,
    pub git: Option<ResolvedToolSummary>,
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

/// 診断 Status を構築する。呼び出し元が解決済みの `ToolInventory` を渡す。
///
/// desktop は `CachedToolInventory` から・CLI は起動時に1回解決した `ToolInventory` から
/// 呼び出すことで、起動直後の PATH 補正 (`fix_path_env::fix`) が反映された
/// 同一の解決結果を Diagnostics と apply 系操作で共有できる。
pub fn diagnose(tc: &ToolInventory, cli_repo: Option<&str>) -> Diagnostics {
    let resolver = ToolResolver::new();
    let homebrew = tc
        .homebrew
        .clone()
        .or_else(|| resolver.resolve_tool_with_version("brew"));
    let nh = tc
        .nh
        .clone()
        .or_else(|| resolver.resolve_tool_with_version("nh"));

    // 後方互換 ToolsSummary
    let tools = ToolsSummary {
        nix: ToolStatus::from(tc.nix.as_ref()),
        nh: ToolStatus::from(nh.as_ref()),
        git: ToolStatus::from(tc.git.as_ref()),
        homebrew: ToolStatus::from(homebrew.as_ref()),
    };

    let nix_health = nix_health(tc);

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
        tool_inventory: ToolInventorySummary {
            nix: tc.nix.as_ref().map(ResolvedToolSummary::from),
            git: tc.git.as_ref().map(ResolvedToolSummary::from),
            homebrew: tc.homebrew.as_ref().map(ResolvedToolSummary::from),
            nh: tc.nh.as_ref().map(ResolvedToolSummary::from),
        },
        applied_revision: state.as_ref().and_then(|s| s.applied_revision.clone()),
        applied_at: state.as_ref().and_then(|s| s.applied_at.clone()),
    }
}

/// Nix のヘルスを検査する。store 接続（`nix store ping`）と flakes 有効性
/// （`nix config show experimental-features`）を subprocess で確認する
///
/// Nix が未解決の場合は `installed: false` の NixHealth を返す（panic しない）。
pub fn nix_health(tc: &ToolInventory) -> NixHealth {
    let Some(nix) = tc.nix.as_ref() else {
        return NixHealth {
            installed: false,
            executable: None,
            version: None,
            store_accessible: false,
            flakes_available: false,
            source: None,
            error: Some(
                "nix not found in PATH or known locations. \
                 install: curl -L https://nixos.org/nix/install | sh"
                    .to_string(),
            ),
            warning: None,
        };
    };
    if !nix.path.is_file() {
        return NixHealth {
            installed: false,
            executable: None,
            version: None,
            store_accessible: false,
            flakes_available: false,
            source: Some(nix.source),
            error: Some(format!(
                "nix resolved to {} but the file is not executable; \
                 reinstall Nix or fix SCHNEEFORGE_NIX_BIN",
                nix.path.display()
            )),
            warning: None,
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

    // XDG state フォルダ欠如は警告扱い（error にはしない）。Nix installer が
    // 作らない有名な罠だが、ユーザーが自前で運用している場合は正常に動くため。
    let xdg_state_missing = xdg_state_profile_dir()
        .map(|d| !d.is_dir())
        .unwrap_or(false);

    let error = if !store_accessible {
        Some("`nix store ping` failed; nix-daemon not running or socket not accessible".to_string())
    } else if !flakes_available {
        Some(
            "experimental-features does not include `flakes`; \
             run `schneeforge doctor` or add `experimental-features = nix-command flakes`"
                .to_string(),
        )
    } else {
        None
    };

    let warning = if xdg_state_missing {
        Some(
            "~/.local/state/nix/profiles not found; \
             if Nix profile commands fail, run `mkdir -p ~/.local/state/nix/profiles`"
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
        warning,
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

    fn dummy_tc() -> ToolInventory {
        ToolInventory {
            nix: Some(ResolvedTool::new(
                PathBuf::from("/__definitely_not_a_real_nix__"),
                ToolSource::SystemProfile,
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
    fn diagnose_nonexistent_repo() {
        let tc = dummy_tc();
        let d = diagnose(&tc, Some("/definitely/not/a/real/repo"));
        assert!(!d.repo_exists);
        assert!(!d.manifest_found);
        assert!(d.manifest_error.is_some());
        assert_eq!(d.username, None);
        assert_eq!(d.validation, None);
    }

    #[test]
    fn diagnose_reports_platform_and_architecture() {
        let tc = dummy_tc();
        let d = diagnose(&tc, Some("/definitely/not/a/real/repo"));
        assert!(!d.platform.is_empty());
        assert!(!d.architecture.is_empty());
        assert!(!d.host.is_empty());
    }

    #[test]
    fn diagnose_includes_nix_health() {
        let tc = dummy_tc();
        let d = diagnose(&tc, Some("/definitely/not/a/real/repo"));
        // nix_health フィールドが serialize 可能で、フィールド群が揃っていること
        let json = serde_json::to_string(&d.nix_health).expect("NixHealth must be serializable");
        assert!(json.contains("installed"));
        assert!(json.contains("store_accessible"));
        assert!(json.contains("flakes_available"));
    }

    #[test]
    fn diagnose_includes_tool_inventory_summary() {
        let tc = dummy_tc();
        let d = diagnose(&tc, Some("/definitely/not/a/real/repo"));
        // tool_inventory フィールドが serialize 可能な形で入っている
        let json = serde_json::to_string(&d.tool_inventory)
            .expect("ToolInventorySummary must be serializable");
        assert!(json.contains("path"));
        assert!(json.contains("source"));
    }

    #[test]
    fn nix_health_returns_not_installed_when_binary_missing() {
        let tc = dummy_tc();
        let health = nix_health(&tc);
        assert!(!health.installed);
        assert!(health.executable.is_none());
        // ダミー Nix は解決済みだが実体が無い → source は保持しつつ not-executable エラー
        assert!(health.source.is_some());
        assert!(health.error.is_some());
        assert!(health.error.unwrap().contains("not executable"));
    }

    #[test]
    fn nix_health_returns_not_installed_when_unresolved() {
        // Fresh install 環境 (Nix 未解決) でも nix_health は panic しない
        let tc = ToolInventory {
            nix: None,
            git: None,
            homebrew: None,
            nh: None,
        };
        let health = nix_health(&tc);
        assert!(!health.installed);
        assert!(health.executable.is_none());
        assert!(health.error.is_some());
        assert!(health.error.unwrap().contains("not found"));
    }

    #[test]
    fn preflight_and_nix_health_share_flakes_detection() {
        // preflight の flakes 判定と nix_health の flakes_available は
        // 同じ `nix config show experimental-features` の出力を parse する。
        // ここでは nix_health が false を返す状況をシミュレート
        let tc = dummy_tc();
        let pre = preflight(&tc);
        let health = nix_health(&tc);
        // Nix が見つからない場合、両方とも flakes 利用不可となる
        assert!(!pre.flakes_enabled);
        assert!(!health.flakes_available);
    }
}
