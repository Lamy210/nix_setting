use schneeforge_core::execution::HostPlatform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DispatchKind {
    Native,
    LocalHelpOrVersion,
    WindowsDoctor,
    WindowsSelfUpdateUnsupported,
    Delegate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LauncherArgs {
    pub(crate) wsl_distro: Option<String>,
    pub(crate) repo: Option<String>,
    pub(crate) forwarded: Vec<String>,
}

pub(crate) fn classify_dispatch(host: HostPlatform, args: &[String]) -> DispatchKind {
    if !matches!(host, HostPlatform::Windows) {
        return DispatchKind::Native;
    }

    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h" | "--version" | "-V"))
    {
        return DispatchKind::LocalHelpOrVersion;
    }

    match command_name(args) {
        Some("doctor") => DispatchKind::WindowsDoctor,
        Some("self-update") => DispatchKind::WindowsSelfUpdateUnsupported,
        Some("__backend-info") => DispatchKind::LocalHelpOrVersion,
        Some(_) => DispatchKind::Delegate,
        None => DispatchKind::LocalHelpOrVersion,
    }
}

pub(crate) fn parse_launcher_args(args: &[String]) -> Result<LauncherArgs, String> {
    let mut wsl_distro = None;
    let mut repo = None;
    let mut forwarded = Vec::with_capacity(args.len());
    let mut index = 0;
    let mut passthrough = false;

    while index < args.len() {
        let arg = &args[index];
        if passthrough {
            forwarded.push(arg.clone());
            index += 1;
            continue;
        }

        if arg == "--" {
            passthrough = true;
            forwarded.push(arg.clone());
            index += 1;
            continue;
        }

        if arg == "--wsl-distro" {
            let value = args
                .get(index + 1)
                .ok_or_else(|| "--wsl-distro requires a distribution name".to_owned())?;
            if value.trim().is_empty() || value == "--" {
                return Err("--wsl-distro requires a non-empty distribution name".to_owned());
            }
            wsl_distro = Some(value.clone());
            index += 2;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--wsl-distro=") {
            if value.trim().is_empty() {
                return Err("--wsl-distro requires a non-empty distribution name".to_owned());
            }
            wsl_distro = Some(value.to_owned());
            index += 1;
            continue;
        }

        if arg == "--repo" {
            let value = args
                .get(index + 1)
                .ok_or_else(|| "--repo requires a path".to_owned())?;
            repo = Some(value.clone());
            forwarded.push(arg.clone());
            forwarded.push(value.clone());
            index += 2;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--repo=") {
            repo = Some(value.to_owned());
            forwarded.push(arg.clone());
            index += 1;
            continue;
        }

        forwarded.push(arg.clone());
        index += 1;
    }

    Ok(LauncherArgs {
        wsl_distro,
        repo,
        forwarded,
    })
}

pub(crate) fn delegated_exit_code(code: Option<i32>) -> Result<i32, String> {
    code.ok_or_else(|| "delegated WSL command terminated without an exit code".to_owned())
}

fn command_name(args: &[String]) -> Option<&str> {
    let mut index = 0;
    let mut passthrough = false;

    while index < args.len() {
        let arg = &args[index];
        if passthrough {
            return Some(arg.as_str());
        }
        if arg == "--" {
            passthrough = true;
            index += 1;
            continue;
        }
        if matches!(arg.as_str(), "--wsl-distro" | "--repo") {
            index += 2;
            continue;
        }
        if arg.starts_with("--wsl-distro=") || arg.starts_with("--repo=") {
            index += 1;
            continue;
        }
        if arg.starts_with('-') {
            index += 1;
            continue;
        }
        return Some(arg.as_str());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
