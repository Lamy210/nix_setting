pub mod discovery;
pub mod manifest;
pub mod state;

pub use discovery::{detect_host, has_git, has_homebrew, has_nix, which, Host, Platform};
pub use manifest::Manifest;
pub use state::State;
