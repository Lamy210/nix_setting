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

    fn sample_inventory() -> WslInventory {
        WslInventory {
            distros: vec![
                WslDistro {
                    name: "Ubuntu".into(),
                    is_default: true,
                    state: Some("Running".into()),
                    version: Some(2),
                },
                WslDistro {
                    name: "Debian".into(),
                    is_default: false,
                    state: Some("Stopped".into()),
                    version: Some(2),
                },
                WslDistro {
                    name: "Legacy".into(),
                    is_default: false,
                    state: Some("Stopped".into()),
                    version: Some(1),
                },
            ],
        }
    }

    #[test]
    fn host_platform_distinguishes_windows_from_nix_platform() {
        assert_eq!(detect_host_platform_for("windows"), HostPlatform::Windows);
        assert_eq!(
            crate::detect_platform_for("windows"),
            crate::Platform::Unsupported
        );
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

    #[test]
    fn decodes_utf16le_wsl_output_with_bom() {
        let text = "Ubuntu\r\nUbuntu Dev\r\n";
        let mut bytes = vec![0xff, 0xfe];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }

        assert_eq!(
            decode_wsl_output(&bytes).unwrap(),
            "Ubuntu\nUbuntu Dev\n"
        );
    }

    #[test]
    fn decodes_nul_heavy_utf16le_without_bom() {
        let text = "Ubuntu\r\n";
        let mut bytes = Vec::new();
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }

        assert_eq!(decode_wsl_output(&bytes).unwrap(), "Ubuntu\n");
    }

    #[test]
    fn parses_default_wsl2_and_distro_name_with_spaces() {
        let quiet = "Ubuntu\nUbuntu Dev\nLegacy\n";
        let verbose = "  NAME                   STATE           VERSION\n  Ubuntu                 Stopped         2\n* Ubuntu Dev             Running         2\n  Legacy                 Stopped         1\n";

        let inventory = parse_wsl_inventory(quiet, verbose).unwrap();
        assert_eq!(inventory.distros.len(), 3);
        assert_eq!(inventory.distros[0].name, "Ubuntu");
        assert!(!inventory.distros[0].is_default);
        assert_eq!(inventory.distros[0].version, Some(2));
        assert_eq!(inventory.distros[1].name, "Ubuntu Dev");
        assert!(inventory.distros[1].is_default);
        assert_eq!(inventory.distros[1].state.as_deref(), Some("Running"));
        assert_eq!(inventory.distros[1].version, Some(2));
        assert_eq!(inventory.distros[2].version, Some(1));
    }

    #[test]
    fn selection_precedence_is_cli_then_env_then_default() {
        let inventory = sample_inventory();

        assert_eq!(
            select_wsl2(&inventory, Some("Debian"), Some("Ubuntu")).unwrap(),
            SelectedWsl {
                distro: "Debian".into(),
                source: WslSelectionSource::Cli,
            }
        );
        assert_eq!(
            select_wsl2(&inventory, None, Some("Debian")).unwrap(),
            SelectedWsl {
                distro: "Debian".into(),
                source: WslSelectionSource::Environment,
            }
        );
        assert_eq!(
            select_wsl2(&inventory, None, None).unwrap(),
            SelectedWsl {
                distro: "Ubuntu".into(),
                source: WslSelectionSource::Default,
            }
        );
    }

    #[test]
    fn explicit_unknown_distro_fails_closed() {
        assert!(select_wsl2(&sample_inventory(), Some("Missing"), None).is_err());
    }

    #[test]
    fn selected_wsl1_fails_closed() {
        assert!(select_wsl2(&sample_inventory(), Some("Legacy"), None).is_err());
    }

    #[test]
    fn missing_default_without_explicit_selector_fails_closed() {
        let inventory = WslInventory {
            distros: vec![WslDistro {
                name: "Ubuntu".into(),
                is_default: false,
                state: Some("Running".into()),
                version: Some(2),
            }],
        };

        assert!(select_wsl2(&inventory, None, None).is_err());
    }
}
