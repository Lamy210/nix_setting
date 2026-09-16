use std::process::Command;

use schneeforge_core::execution::{
    build_wsl_argv, decode_wsl_output, parse_wsl_inventory, select_wsl2, validate_backend_info,
    validate_wsl_repo_path, BackendInfo, HostPlatform,
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessOutput {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

trait WslRuntime {
    fn capture(&mut self, argv: &[String]) -> Result<ProcessOutput, String>;
    fn status(&mut self, argv: &[String]) -> Result<Option<i32>, String>;
}

struct SystemWslRuntime;

impl WslRuntime for SystemWslRuntime {
    fn capture(&mut self, argv: &[String]) -> Result<ProcessOutput, String> {
        let output = Command::new("wsl.exe")
            .args(argv)
            .output()
            .map_err(|error| format!("failed to execute wsl.exe: {error}"))?;
        Ok(ProcessOutput {
            code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    fn status(&mut self, argv: &[String]) -> Result<Option<i32>, String> {
        Command::new("wsl.exe")
            .args(argv)
            .status()
            .map(|status| status.code())
            .map_err(|error| format!("failed to execute wsl.exe: {error}"))
    }
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

fn capture_checked<R: WslRuntime>(
    runtime: &mut R,
    argv: &[String],
    operation: &str,
) -> Result<ProcessOutput, String> {
    let output = runtime.capture(argv)?;
    if output.code == Some(0) {
        return Ok(output);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "{operation} failed with exit code {:?}: {}",
        output.code,
        stderr.trim()
    ))
}

fn delegate_with_runtime<R: WslRuntime>(
    runtime: &mut R,
    parsed: &LauncherArgs,
    env_selector: Option<&str>,
    expected_app_version: &str,
) -> Result<i32, String> {
    if let Some(repo) = parsed.repo.as_deref() {
        validate_wsl_repo_path(repo).map_err(|error| error.to_string())?;
    }

    let quiet_args = vec!["--list".to_owned(), "--quiet".to_owned()];
    let verbose_args = vec!["--list".to_owned(), "--verbose".to_owned()];
    let quiet = capture_checked(runtime, &quiet_args, "WSL distribution listing")?;
    let verbose = capture_checked(runtime, &verbose_args, "WSL verbose distribution listing")?;
    let quiet = decode_wsl_output(&quiet.stdout).map_err(|error| error.to_string())?;
    let verbose = decode_wsl_output(&verbose.stdout).map_err(|error| error.to_string())?;
    let inventory = parse_wsl_inventory(&quiet, &verbose).map_err(|error| error.to_string())?;
    let selected = select_wsl2(
        &inventory,
        parsed.wsl_distro.as_deref(),
        env_selector,
    )
    .map_err(|error| error.to_string())?;

    let helper_args = build_wsl_argv(&selected.distro, &["__backend-info".to_owned()]);
    let helper = capture_checked(runtime, &helper_args, "WSL helper compatibility probe")?;
    let helper = decode_wsl_output(&helper.stdout).map_err(|error| error.to_string())?;
    let helper = serde_json::from_str::<BackendInfo>(helper.trim())
        .map_err(|error| format!("invalid WSL helper compatibility response: {error}"))?;
    validate_backend_info(&helper, expected_app_version).map_err(|error| error.to_string())?;

    let delegated_args = build_wsl_argv(&selected.distro, &parsed.forwarded);
    delegated_exit_code(runtime.status(&delegated_args)?)
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
    use std::collections::VecDeque;

    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn output(code: i32, stdout: &str) -> ProcessOutput {
        ProcessOutput {
            code: Some(code),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    #[derive(Default)]
    struct FakeRuntime {
        captures: VecDeque<(Vec<String>, ProcessOutput)>,
        statuses: VecDeque<(Vec<String>, Option<i32>)>,
        seen: Vec<Vec<String>>,
    }

    impl FakeRuntime {
        fn with_capture(mut self, expected: &[&str], result: ProcessOutput) -> Self {
            self.captures.push_back((args(expected), result));
            self
        }

        fn with_status(mut self, expected: &[&str], code: Option<i32>) -> Self {
            self.statuses.push_back((args(expected), code));
            self
        }
    }

    impl WslRuntime for FakeRuntime {
        fn capture(&mut self, argv: &[String]) -> Result<ProcessOutput, String> {
            self.seen.push(argv.to_vec());
            let (expected, result) = self
                .captures
                .pop_front()
                .expect("unexpected capture call");
            assert_eq!(argv, expected);
            Ok(result)
        }

        fn status(&mut self, argv: &[String]) -> Result<Option<i32>, String> {
            self.seen.push(argv.to_vec());
            let (expected, code) = self.statuses.pop_front().expect("unexpected status call");
            assert_eq!(argv, expected);
            Ok(code)
        }
    }

    fn compatible_backend_json(version: &str) -> String {
        format!(
            r#"{{"protocol_version":1,"app_version":"{version}","os":"linux","arch":"x86_64"}}"#
        )
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
            args(&["--repo", "/home/alice/nix setting", "apply", "a b;$(x)"])
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

    #[test]
    fn delegation_runs_inventory_handshake_and_command_in_order() {
        let version = env!("CARGO_PKG_VERSION");
        let helper = compatible_backend_json(version);
        let mut runtime = FakeRuntime::default()
            .with_capture(&["--list", "--quiet"], output(0, "Ubuntu Dev\nDebian\n"))
            .with_capture(
                &["--list", "--verbose"],
                output(0, "* Ubuntu Dev Running 2\n  Debian Stopped 2\n"),
            )
            .with_capture(
                &["-d", "Ubuntu Dev", "--", "schneeforge", "__backend-info"],
                output(0, &helper),
            )
            .with_status(
                &[
                    "-d",
                    "Ubuntu Dev",
                    "--",
                    "schneeforge",
                    "apply",
                    "a b;$(x)",
                ],
                Some(23),
            );
        let parsed = LauncherArgs {
            wsl_distro: Some("Ubuntu Dev".into()),
            repo: None,
            forwarded: args(&["apply", "a b;$(x)"]),
        };

        assert_eq!(
            delegate_with_runtime(&mut runtime, &parsed, None, version).unwrap(),
            23
        );
        assert!(runtime.captures.is_empty());
        assert!(runtime.statuses.is_empty());
    }

    #[test]
    fn windows_repo_path_is_rejected_before_wsl_is_invoked() {
        let mut runtime = FakeRuntime::default();
        let parsed = LauncherArgs {
            wsl_distro: None,
            repo: Some(r"C:\src\nix_setting".into()),
            forwarded: args(&["status"]),
        };

        assert!(delegate_with_runtime(
            &mut runtime,
            &parsed,
            None,
            env!("CARGO_PKG_VERSION")
        )
        .is_err());
        assert!(runtime.seen.is_empty());
    }

    #[test]
    fn wsl1_is_rejected_before_helper_probe() {
        let mut runtime = FakeRuntime::default()
            .with_capture(&["--list", "--quiet"], output(0, "Legacy\n"))
            .with_capture(
                &["--list", "--verbose"],
                output(0, "* Legacy Running 1\n"),
            );
        let parsed = LauncherArgs {
            wsl_distro: None,
            repo: None,
            forwarded: args(&["status"]),
        };

        assert!(delegate_with_runtime(
            &mut runtime,
            &parsed,
            None,
            env!("CARGO_PKG_VERSION")
        )
        .is_err());
        assert!(runtime.captures.is_empty());
        assert!(runtime.statuses.is_empty());
    }

    #[test]
    fn helper_version_mismatch_fails_before_delegated_command() {
        let helper = compatible_backend_json("9.9.9");
        let mut runtime = FakeRuntime::default()
            .with_capture(&["--list", "--quiet"], output(0, "Ubuntu\n"))
            .with_capture(
                &["--list", "--verbose"],
                output(0, "* Ubuntu Running 2\n"),
            )
            .with_capture(
                &["-d", "Ubuntu", "--", "schneeforge", "__backend-info"],
                output(0, &helper),
            );
        let parsed = LauncherArgs {
            wsl_distro: None,
            repo: None,
            forwarded: args(&["apply"]),
        };

        assert!(delegate_with_runtime(
            &mut runtime,
            &parsed,
            None,
            env!("CARGO_PKG_VERSION")
        )
        .is_err());
        assert!(runtime.statuses.is_empty());
    }

    #[test]
    fn environment_selector_is_used_when_cli_selector_is_absent() {
        let version = env!("CARGO_PKG_VERSION");
        let helper = compatible_backend_json(version);
        let mut runtime = FakeRuntime::default()
            .with_capture(&["--list", "--quiet"], output(0, "Ubuntu\nDebian\n"))
            .with_capture(
                &["--list", "--verbose"],
                output(0, "* Ubuntu Running 2\n  Debian Stopped 2\n"),
            )
            .with_capture(
                &["-d", "Debian", "--", "schneeforge", "__backend-info"],
                output(0, &helper),
            )
            .with_status(
                &["-d", "Debian", "--", "schneeforge", "status"],
                Some(0),
            );
        let parsed = LauncherArgs {
            wsl_distro: None,
            repo: None,
            forwarded: args(&["status"]),
        };

        assert_eq!(
            delegate_with_runtime(&mut runtime, &parsed, Some("Debian"), version).unwrap(),
            0
        );
    }
}
