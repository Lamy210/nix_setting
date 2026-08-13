pub mod actions;
pub mod discovery;
pub mod manifest;
pub mod state;

pub use actions::{
    apply, apply_captured, rollback, rollback_captured, scan, upgrade, upgrade_captured,
};
pub use discovery::{
    detect_host, detect_host_for, has_git, has_homebrew, has_nix, which, Host, Platform,
};
pub use manifest::Manifest;
pub use state::State;
