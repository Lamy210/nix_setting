pub(crate) mod actions;
pub mod bootstrap;
pub mod dashboard;
pub mod diagnostics;
pub mod discovery;
pub mod error;
pub mod lock;
pub mod machine;
pub mod managed_nix;
pub mod manifest;
pub mod operations;
pub(crate) mod process;
pub mod profile;
pub mod release_metadata;
pub mod repo;
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
pub use release_metadata::{channel_for_version, ReleaseMetadata};
pub use repo::{current_git_revision, resolve_repo, Repo, RepoResolver};
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
