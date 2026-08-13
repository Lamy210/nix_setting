pub mod actions;
pub mod discovery;
pub mod manifest;
pub mod state;

pub use actions::{apply, rollback, scan, upgrade};
pub use discovery::{
    detect_host, detect_host_for, has_git, has_homebrew, has_nix, which, Host, Platform,
};
pub use manifest::Manifest;
pub use state::State;
