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
