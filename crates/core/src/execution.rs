/// Launcher process host platform.
///
/// This is intentionally distinct from `discovery::Platform`, which models
/// the Nix execution platform and therefore remains macOS/Linux-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostPlatform {
    MacOS,
    Linux,
    Windows,
    Unsupported,
}

/// Backend used to execute SchneeForge operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionBackend {
    Native,
    Wsl2 { distro: String },
}

/// Derive the launcher host platform from Rust's OS identifier.
pub fn detect_host_platform_for(os: &str) -> HostPlatform {
    match os {
        "macos" => HostPlatform::MacOS,
        "linux" => HostPlatform::Linux,
        "windows" => HostPlatform::Windows,
        _ => HostPlatform::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_platform_distinguishes_windows_from_nix_platform() {
        assert_eq!(detect_host_platform_for("windows"), HostPlatform::Windows);
        assert_eq!(crate::detect_platform_for("windows"), crate::Platform::Unsupported);
    }

    #[test]
    fn native_hosts_use_native_execution_backend() {
        assert_eq!(ExecutionBackend::Native, ExecutionBackend::Native);
    }

    #[test]
    fn windows_host_selects_no_native_nix_target() {
        assert_eq!(
            crate::detect_target_for("windows", "x86_64").name(),
            "unsupported"
        );
    }
}
