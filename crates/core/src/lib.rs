pub mod actions;
pub mod discovery;
pub mod error;
pub mod lock;
pub mod manifest;
pub mod operations;
pub mod repo;
pub mod state;
pub mod time;
pub mod tool;

pub use actions::{scan, upgrade, upgrade_captured};
pub use discovery::{
    detect_arch, detect_arch_for, detect_platform, detect_platform_for, detect_target,
    detect_target_for, has_git, has_homebrew, has_nix, which, Architecture, ConfigurationTarget,
    Platform,
};
pub use error::{Error, Result};
pub use lock::{OperationGuard, OperationLock};
pub use manifest::{Manifest, Validation};
pub use operations::{apply, rollback, ApplyResult};
pub use repo::{current_git_revision, resolve_repo, Repo, RepoResolver};
pub use state::{State, StateStore};
pub use time::{days_to_ymd, format_unix_secs, now_iso8601};
pub use tool::{find_executable, version_of, ToolResolver, ToolStatus};
