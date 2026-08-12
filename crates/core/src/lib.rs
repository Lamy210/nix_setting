pub mod discovery;
pub mod manifest;

pub use discovery::{detect_host, Host, Platform};
pub use manifest::Manifest;
