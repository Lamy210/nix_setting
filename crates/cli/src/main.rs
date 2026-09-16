mod windows_backend;

use std::ffi::OsString;

use schneeforge_core::execution::{
    detect_host_platform_for, BackendInfo, HostPlatform, BACKEND_PROTOCOL_VERSION,
};

#[allow(dead_code)]
mod native_cli {
    include!("legacy_main.rs");

    use std::ffi::OsString;

    use clap::{Arg, CommandFactory, FromArgMatches};

    pub(super) fn run(args: Vec<OsString>) {
        let mut command = Cli::command();
        command = command.arg(
            Arg::new("wsl-distro")
                .long("wsl-distro")
                .value_name("DISTRO")
                .global(true)
                .help("Windows launcher: select the WSL2 distribution"),
        );
        let matches = command.get_matches_from(args);
        let _wsl_distro = matches.get_one::<String>("wsl-distro").cloned();
        let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
        let repo = schneeforge_core::resolve_repo(cli.repo.as_deref());

        // Keep the existing native dispatch unchanged. Windows interception is
        // layered in front of this boundary before repo/tool/state discovery.
        let result = match cli.command {
            Cmd::Doctor => with_tool_inventory(|tc| doctor(&repo, tc), &repo),
            Cmd::Scan => with_tool_inventory(|tc| scan(&repo, tc), &repo),
            Cmd::Setup => with_tool_inventory(|tc| setup(&repo, tc), &repo),
            Cmd::Status => status(&repo),
            Cmd::Profile(args) => run_profile(args.command, &repo),
            Cmd::Plan => with_tool_inventory(|tc| plan(&repo, tc), &repo),
            Cmd::Apply => with_tool_inventory(|tc| apply(&repo, tc), &repo),
            Cmd::Rollback => with_tool_inventory(|tc| rollback(&repo, tc), &repo),
            Cmd::Update => with_tool_inventory(|tc| update(&repo, tc), &repo),
            Cmd::Source(args) => {
                with_tool_inventory(|tc| run_source(args.command, &repo, tc), &repo)
            }
            Cmd::Upgrade => with_tool_inventory(|tc| upgrade(&repo, tc), &repo),
            Cmd::Sync => with_tool_inventory(|tc| sync(&repo, tc), &repo),
            Cmd::Verify => with_tool_inventory(|tc| verify(&repo, tc), &repo),
            Cmd::Uninstall => uninstall(),
            Cmd::SelfUpdate => with_tool_inventory(self_update, &repo),
            Cmd::Nix(nix_args) => run_nix(nix_args.command, &repo),
        };
        if let Err(error) = result {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}

fn main() {
    let args = std::env::args_os().collect::<Vec<_>>();
    if is_backend_info_probe(&args) {
        print_backend_info();
        return;
    }

    let host = detect_host_platform_for(std::env::consts::OS);
    if host == HostPlatform::Windows {
        let launcher_args = match windows_launcher_args(&args) {
            Ok(args) => args,
            Err(error) => {
                eprintln!("error: {error}");
                std::process::exit(1);
            }
        };
        let env_selector = std::env::var("SCHNEEFORGE_WSL_DISTRO").ok();
        match windows_backend::dispatch(host, &launcher_args, env_selector.as_deref()) {
            Ok(windows_backend::EarlyDispatch::ContinueNative) => {}
            Ok(windows_backend::EarlyDispatch::Doctor(report)) => {
                print!("{report}");
                return;
            }
            Ok(windows_backend::EarlyDispatch::Exit(code)) => std::process::exit(code),
            Err(error) => {
                eprintln!("error: {error}");
                std::process::exit(1);
            }
        }
    }

    native_cli::run(args);
}

fn windows_launcher_args(args: &[OsString]) -> Result<Vec<String>, String> {
    args.iter()
        .skip(1)
        .map(|arg| {
            arg.clone().into_string().map_err(|_| {
                "Windows launcher arguments must be valid Unicode before WSL delegation".to_owned()
            })
        })
        .collect()
}

fn is_backend_info_probe(args: &[OsString]) -> bool {
    args.len() == 2 && args[1] == "__backend-info"
}

fn print_backend_info() {
    let info = BackendInfo {
        protocol_version: BACKEND_PROTOCOL_VERSION,
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
    };
    match serde_json::to_string(&info) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("error: failed to serialize backend info: {error}");
            std::process::exit(1);
        }
    }
}
