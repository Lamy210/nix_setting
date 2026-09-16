#[cfg(test)]
mod tests {
    use super::*;
    use schneeforge_core::execution::HostPlatform;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn native_hosts_bypass_windows_dispatch() {
        assert_eq!(
            classify_dispatch(HostPlatform::Linux, &args(&["apply"])),
            DispatchKind::Native
        );
        assert_eq!(
            classify_dispatch(HostPlatform::MacOS, &args(&["doctor"])),
            DispatchKind::Native
        );
    }

    #[test]
    fn windows_classifies_local_and_delegated_commands_before_native_startup() {
        assert_eq!(
            classify_dispatch(HostPlatform::Windows, &args(&["--help"])),
            DispatchKind::LocalHelpOrVersion
        );
        assert_eq!(
            classify_dispatch(HostPlatform::Windows, &args(&["--version"])),
            DispatchKind::LocalHelpOrVersion
        );
        assert_eq!(
            classify_dispatch(HostPlatform::Windows, &args(&["doctor"])),
            DispatchKind::WindowsDoctor
        );
        assert_eq!(
            classify_dispatch(HostPlatform::Windows, &args(&["self-update"])),
            DispatchKind::WindowsSelfUpdateUnsupported
        );
        assert_eq!(
            classify_dispatch(HostPlatform::Windows, &args(&["apply"])),
            DispatchKind::Delegate
        );
    }

    #[test]
    fn launcher_parser_consumes_selector_and_preserves_forwarded_argv() {
        let parsed = parse_launcher_args(&args(&[
            "--wsl-distro",
            "Ubuntu Dev",
            "--repo",
            "/home/alice/nix setting",
            "apply",
            "a b;$(x)",
        ]))
        .unwrap();

        assert_eq!(parsed.wsl_distro.as_deref(), Some("Ubuntu Dev"));
        assert_eq!(parsed.repo.as_deref(), Some("/home/alice/nix setting"));
        assert_eq!(
            parsed.forwarded,
            args(&[
                "--repo",
                "/home/alice/nix setting",
                "apply",
                "a b;$(x)"
            ])
        );
    }

    #[test]
    fn launcher_parser_supports_equals_forms() {
        let parsed = parse_launcher_args(&args(&[
            "--wsl-distro=Debian",
            "--repo=/srv/schneeforge",
            "status",
        ]))
        .unwrap();

        assert_eq!(parsed.wsl_distro.as_deref(), Some("Debian"));
        assert_eq!(parsed.repo.as_deref(), Some("/srv/schneeforge"));
        assert_eq!(
            parsed.forwarded,
            args(&["--repo=/srv/schneeforge", "status"])
        );
    }

    #[test]
    fn delegated_exit_status_is_preserved() {
        assert_eq!(delegated_exit_code(Some(23)).unwrap(), 23);
        assert_eq!(delegated_exit_code(Some(0)).unwrap(), 0);
        assert!(delegated_exit_code(None).is_err());
    }
}
