pub(crate) mod actions;
pub mod bootstrap;
pub mod dashboard;
pub mod diagnostics;
pub mod discovery;
pub mod error;
pub mod execution;
pub mod lock;
pub mod machine;
pub mod managed_nix;
pub mod manifest;
pub(crate) mod nix_features;
#[path = "operations.rs"]
mod operations_impl;
pub mod operations {
    pub use crate::operations_impl::*;

    /// Public source-initialization boundary.
    ///
    /// Persisted managed-source state is authoritative. Validate it before the
    /// implementation is allowed to discover remote tags, fetch release metadata,
    /// or inspect the checkout. Invalid state therefore remains available for
    /// explicit repair instead of being silently replaced by a new source.
    pub fn source_init(
        repo: &str,
        store: &crate::state::StateStore,
        git: &crate::tool::ResolvedTool,
        channel: Option<String>,
        tag: Option<String>,
    ) -> crate::error::Result<crate::operations_impl::SourceInitResult> {
        let state = store.load()?;
        if let Some(source) = state
            .as_ref()
            .and_then(|state| state.source.as_ref())
            .filter(|source| source.managed)
        {
            source.validate_managed_release()?;
        }

        crate::operations_impl::source_init(repo, store, git, channel, tag)
    }

    /// Public dependency-update boundary.
    ///
    /// Persisted managed-source semantics must be handled before checkout
    /// classification. For unmanaged Git checkouts, require Git before entering
    /// the mutation path so release-integrity warnings cannot be silently skipped.
    pub fn deps_update(
        repo: &str,
        tc: &crate::tool::ToolInventory,
        capture: bool,
    ) -> crate::error::Result<Option<String>> {
        let state = crate::state::StateStore::default().load()?;
        let managed = state
            .as_ref()
            .and_then(|state| state.source.as_ref())
            .is_some_and(|source| source.managed);

        if !managed && std::path::Path::new(repo).join(".git").exists() {
            tc.require_git()?;
        }

        crate::operations_impl::deps_update(repo, tc, capture)
    }

    /// Deprecated compatibility alias for [`deps_update`].
    ///
    /// Keep the legacy entry point behind the same source-safety boundary as
    /// `source deps update`: managed sources must be rejected and release
    /// checkouts must receive the canonical warning before any flake mutation.
    pub fn upgrade(
        repo: &str,
        tc: &crate::tool::ToolInventory,
        capture: bool,
    ) -> crate::error::Result<Option<String>> {
        deps_update(repo, tc, capture)
    }

    /// Public verification boundary.
    ///
    /// `HOME` is an implicit filesystem root, so it is usable only when the
    /// process can represent it as a non-empty absolute path. The internal
    /// verifier is intentionally infallible; mask HOME-derived dotfile results
    /// when that precondition is not met so verification cannot depend on CWD.
    pub fn verify(
        repo: &str,
        tc: &crate::tool::ToolInventory,
    ) -> crate::operations_impl::VerifyReport {
        let mut report = crate::operations_impl::verify(repo, tc);
        let valid_home = std::env::var("HOME")
            .ok()
            .filter(|home| !home.is_empty())
            .is_some_and(|home| std::path::Path::new(&home).is_absolute());

        if !valid_home {
            for check in &mut report.checks {
                if matches!(check.name.as_str(), ".zshrc" | ".gitconfig" | "starship.toml") {
                    check.ok = false;
                }
            }
        }

        report
    }
}
pub(crate) mod process;
pub mod profile;
pub mod release_metadata;
pub mod repo;
pub mod self_update;
pub(crate) mod semver;
pub mod source;
pub mod source_files;
pub mod state;
pub mod time;
pub mod tool;

pub use actions::scan;
pub use bootstrap::{
    clone_repo, doctor, enable_flakes, generate_config, preflight, setup, DoctorReport,
    PreflightReport, DEFAULT_REPO_URL,
};
pub use dashboard::{
    channel_of, compare_versions, fetch_available, installed_info, latest_tag_from_ls_remote,
    remote_tags, snapshot, version_is_newer, DashboardSnapshot, InstalledInfo,
};
pub use diagnostics::{
    diagnose, nix_health, Diagnostics, NixHealth, ResolvedToolSummary, ToolInventorySummary,
    ToolsSummary,
};
pub use discovery::{
    current_user, detect_arch, detect_arch_for, detect_platform, detect_platform_for,
    detect_target, detect_target_for, has_git, has_homebrew, has_nix, which, Architecture,
    ConfigurationTarget, Platform,
};
pub use error::{Error, Result};
pub use lock::{OperationGuard, OperationLock};
pub use machine::{
    default_machine_nix_path, state_dir, write_machine_input, MachineFacts, OperatingSystem,
};
pub use managed_nix::{
    cache_path, classify, classify_current, default_ownership_path, default_receipt_path, download,
    download_text, escalate_command, existing_nix_detected, install_args, installed_binary_path,
    is_root, parse_json_line, parse_sha256_sums, plan_args, planner_name, repair_action,
    repair_action_current, repair_args, run_with_json_logs, run_with_json_logs_capture_stdout,
    secure_plan_dir, sha256_hex, summarize_plan, uninstall_args, verify_file, verify_sha256,
    BootstrapManifest, EscalatedOp, InstallPhase, JsonLogLine, ManagedNix, ManagedNixError,
    ManagedNixSection, NixStatus, NoProgress, OwnershipRecord, PreflightSummary, ProgressSink,
    Provider, Receipt, RepairAction, Sha256ByArch, StatusProbe, StatusReport, UpstreamRepair,
};
pub use manifest::{Manifest, Validation};
pub use operations::{
    apply, deps_update, dispatch_update, plan, plan_target, rollback, source_init, source_sync,
    sync, update, upgrade, verify, ApplyResult, PlanResult, UpdateAction, UpdateResult,
    VerifyCheck, VerifyReport,
};
pub use profile::{
    clear_selection, default_profile_nix_path, list as list_profiles, override_args,
    resolve as resolve_profile, save_selection, set_selection, write_profile_input, ProfileList,
};
pub use release_metadata::{channel_for_version, release_page_url, ReleaseMetadata};
pub use repo::{current_git_revision, resolve_repo, Repo, RepoResolver};
pub use self_update::{
    current_platform_asset, expected_sha256, plan as plan_self_update, platform_asset,
    release_asset_url, run as run_self_update, verify_and_replace, SelfUpdateAction,
    SelfUpdatePlan, SelfUpdateStatus,
};
pub use source::{
    classify_release_tag, effective_ref, github_slug, latest_tag_for_channel, repo_url, SourceKind,
    SourceResolver, SourceState,
};
pub use source_files::load_manifest_for;
pub use state::{State, StateStore};
pub use time::{days_to_ymd, format_unix_secs, now_iso8601};
pub use tool::{
    find_executable, version_of, ResolvedTool, ToolInventory, ToolRequirementError, ToolResolver,
    ToolSource, ToolStatus,
};

#[cfg(test)]
mod public_boundary_tests {
    use super::*;
    use std::path::PathBuf;

    fn inventory_without_git() -> ToolInventory {
        ToolInventory {
            nix: Some(ResolvedTool::new(
                PathBuf::from("/usr/bin/true"),
                ToolSource::Path,
            )),
            git: None,
            homebrew: None,
            nh: None,
        }
    }

    #[test]
    fn deps_update_requires_git_when_repo_is_a_git_checkout() {
        let dir =
            std::env::temp_dir().join(format!("schneeforge-deps-git-guard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".git")).unwrap();

        let err = deps_update(dir.to_str().unwrap(), &inventory_without_git(), true).unwrap_err();
        assert!(matches!(err, Error::Precondition(_)), "{err}");
        assert!(err.to_string().contains("git not found"), "{err}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn deps_update_keeps_non_git_local_source_usable_without_git() {
        let dir = std::env::temp_dir().join(format!(
            "schneeforge-deps-local-no-git-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let output = deps_update(dir.to_str().unwrap(), &inventory_without_git(), true).unwrap();
        assert_eq!(output.as_deref(), Some(""));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
