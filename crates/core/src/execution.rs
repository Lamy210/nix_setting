use crate::error::{Error, Result};

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

/// One WSL distribution discovered from `wsl.exe` inventory output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WslDistro {
    pub name: String,
    pub is_default: bool,
    pub state: Option<String>,
    pub version: Option<u8>,
}

/// WSL distributions registered on the Windows host.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WslInventory {
    pub distros: Vec<WslDistro>,
}

/// Source that selected the active WSL distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WslSelectionSource {
    Cli,
    Environment,
    Default,
}

/// Deterministically selected WSL2 distribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedWsl {
    pub distro: String,
    pub source: WslSelectionSource,
}

/// Protocol spoken between the native Windows launcher and Linux helper.
pub const BACKEND_PROTOCOL_VERSION: u32 = 1;

/// Machine-readable identity returned by the Linux helper before delegation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BackendInfo {
    pub protocol_version: u32,
    pub app_version: String,
    pub os: String,
    pub arch: String,
}

/// Validate that the selected Linux helper is compatible with this launcher.
pub fn validate_backend_info(info: &BackendInfo, expected_app_version: &str) -> Result<()> {
    if info.protocol_version != BACKEND_PROTOCOL_VERSION {
        return Err(Error::Precondition(format!(
            "WSL helper protocol mismatch: launcher expects {}, helper reports {}; update the WSL helper",
            BACKEND_PROTOCOL_VERSION, info.protocol_version
        )));
    }
    if info.app_version != expected_app_version {
        return Err(Error::Precondition(format!(
            "WSL helper version mismatch: launcher is {expected_app_version}, helper is {}; install the matching SchneeForge version in WSL",
            info.app_version
        )));
    }
    if info.os != "linux" {
        return Err(Error::Precondition(format!(
            "WSL helper reported unsupported execution OS '{}'; Linux is required",
            info.os
        )));
    }
    if !matches!(info.arch.as_str(), "x86_64" | "aarch64") {
        return Err(Error::Precondition(format!(
            "WSL helper reported unsupported architecture '{}'; x86_64 or aarch64 is required",
            info.arch
        )));
    }
    Ok(())
}

/// Validate an explicit Windows-launcher `--repo` value without translating it.
pub fn validate_wsl_repo_path(repo: &str) -> Result<()> {
    let repo = repo.trim();
    let bytes = repo.as_bytes();
    let drive_letter = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    let unc_path = repo.starts_with("\\\\") || repo.starts_with("//");

    if repo.is_empty() || drive_letter || unc_path || repo.contains('\\') || !repo.starts_with('/')
    {
        return Err(Error::Precondition(format!(
            "Windows --repo must be an absolute Linux path inside WSL (for example /home/user/project), got '{repo}'"
        )));
    }
    Ok(())
}

/// Construct direct `wsl.exe` argv without any shell interpolation.
pub fn build_wsl_argv(distro: &str, forwarded: &[String]) -> Vec<String> {
    let mut args = Vec::with_capacity(forwarded.len() + 4);
    args.extend([
        "-d".to_owned(),
        distro.to_owned(),
        "--".to_owned(),
        "schneeforge".to_owned(),
    ]);
    args.extend(forwarded.iter().cloned());
    args
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

/// Decode `wsl.exe` output which can be UTF-8 or UTF-16LE depending on the
/// Windows/WSL invocation context.
pub fn decode_wsl_output(bytes: &[u8]) -> Result<String> {
    if bytes.is_empty() {
        return Ok(String::new());
    }

    let has_utf16le_bom = bytes.starts_with(&[0xff, 0xfe]);
    let payload = if has_utf16le_bom { &bytes[2..] } else { bytes };
    let pair_count = payload.len() / 2;
    let nul_high_bytes = payload
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|byte| **byte == 0)
        .count();
    let looks_utf16le = has_utf16le_bom
        || (payload.len().is_multiple_of(2) && pair_count >= 2 && nul_high_bytes * 2 >= pair_count);

    let decoded = if looks_utf16le {
        if !payload.len().is_multiple_of(2) {
            return Err(Error::Precondition(
                "invalid UTF-16LE WSL output: odd byte count".into(),
            ));
        }
        let units = payload
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        String::from_utf16(&units)
            .map_err(|e| Error::Precondition(format!("invalid UTF-16LE WSL output: {e}")))?
    } else {
        std::str::from_utf8(bytes)
            .map_err(|e| Error::Precondition(format!("invalid WSL output encoding: {e}")))?
            .to_owned()
    };

    Ok(decoded.replace("\r\n", "\n").replace('\r', "\n"))
}

/// Combine quiet and verbose WSL inventory output.
///
/// The quiet listing is authoritative for distribution names. Verbose output
/// only enriches those names with default/state/version metadata, which avoids
/// splitting distribution names on whitespace.
pub fn parse_wsl_inventory(quiet: &str, verbose: &str) -> Result<WslInventory> {
    let mut inventory = WslInventory {
        distros: quiet
            .lines()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| WslDistro {
                name: name.to_owned(),
                is_default: false,
                state: None,
                version: None,
            })
            .collect(),
    };

    if inventory.distros.is_empty() {
        return Err(Error::Precondition(
            "no WSL distributions are registered".into(),
        ));
    }

    for raw_line in verbose.lines() {
        let mut line = raw_line.trim_start();
        if line.is_empty() || (line.contains("NAME") && line.contains("VERSION")) {
            continue;
        }

        let is_default = line.starts_with('*');
        if is_default {
            line = line[1..].trim_start();
        }

        let matched_name = inventory
            .distros
            .iter()
            .filter_map(|distro| {
                line.strip_prefix(&distro.name).and_then(|suffix| {
                    (suffix.is_empty() || suffix.chars().next().is_some_and(char::is_whitespace))
                        .then_some(distro.name.as_str())
                })
            })
            .max_by_key(|name| name.len())
            .map(str::to_owned);

        let Some(name) = matched_name else {
            continue;
        };
        let suffix = line[name.len()..].trim();
        let fields = suffix.split_whitespace().collect::<Vec<_>>();
        let version = fields.last().and_then(|field| field.parse::<u8>().ok());
        let state = if version.is_some() && fields.len() > 1 {
            Some(fields[..fields.len() - 1].join(" "))
        } else if version.is_none() && !fields.is_empty() {
            Some(fields.join(" "))
        } else {
            None
        };

        if let Some(distro) = inventory
            .distros
            .iter_mut()
            .find(|distro| distro.name == name)
        {
            distro.is_default = is_default;
            distro.state = state;
            distro.version = version;
        }
    }

    Ok(inventory)
}

/// Select one registered WSL2 distro using CLI > environment > WSL default.
pub fn select_wsl2(
    inventory: &WslInventory,
    cli_selector: Option<&str>,
    env_selector: Option<&str>,
) -> Result<SelectedWsl> {
    let explicit = cli_selector
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| (name, WslSelectionSource::Cli))
        .or_else(|| {
            env_selector
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(|name| (name, WslSelectionSource::Environment))
        });

    let (selected_name, source) = match explicit {
        Some(selection) => selection,
        None => {
            let default = inventory
                .distros
                .iter()
                .find(|distro| distro.is_default)
                .ok_or_else(|| {
                    Error::Precondition(
                        "no default WSL distribution is selected; use --wsl-distro".into(),
                    )
                })?;
            (default.name.as_str(), WslSelectionSource::Default)
        }
    };

    let distro = inventory
        .distros
        .iter()
        .find(|distro| distro.name == selected_name)
        .ok_or_else(|| {
            Error::Precondition(format!(
                "WSL distribution '{selected_name}' is not registered"
            ))
        })?;

    match distro.version {
        Some(2) => Ok(SelectedWsl {
            distro: distro.name.clone(),
            source,
        }),
        Some(version) => Err(Error::Precondition(format!(
            "WSL distribution '{}' uses WSL{version}; WSL2 is required",
            distro.name
        ))),
        None => Err(Error::Precondition(format!(
            "cannot determine WSL version for distribution '{}'",
            distro.name
        ))),
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

        assert_eq!(decode_wsl_output(&bytes).unwrap(), "Ubuntu\nUbuntu Dev\n");
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
