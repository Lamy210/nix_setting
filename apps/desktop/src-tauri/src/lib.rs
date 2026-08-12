use schneeforge_core::{detect_host, has_git, has_homebrew, has_nix, Host, State};
use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
struct Status {
    host: String,
    user: Option<String>,
    nix: bool,
    homebrew: bool,
    git: bool,
    applied_revision: Option<String>,
}

#[derive(Serialize)]
struct CommandOutput {
    success: bool,
    output: String,
}

#[tauri::command]
fn get_status() -> Status {
    let manifest = load_manifest();
    let state = State::load(&State::default_path());
    Status {
        host: detect_host().to_string(),
        user: manifest.as_ref().map(|m| m.user.username.clone()),
        nix: has_nix(),
        homebrew: has_homebrew(),
        git: has_git(),
        applied_revision: state.and_then(|s| s.applied_revision),
    }
}

#[tauri::command]
fn run_scan() -> CommandOutput {
    let host = detect_host();
    let manifest = load_manifest();
    let mut out = String::new();
    out.push_str(&format!("OS:   {}\n", std::env::consts::OS));
    out.push_str(&format!("arch: {}\n", std::env::consts::ARCH));
    out.push_str(&format!("host: {host}\n"));
    out.push_str(&format!("nix:  {}\n", if has_nix() { "yes" } else { "no" }));
    out.push_str(&format!(
        "brew: {}\n",
        if has_homebrew() { "yes" } else { "no" }
    ));
    match manifest {
        Some(m) => out.push_str(&format!("user: {}\n", m.user.username)),
        None => out.push_str("user: (config.toml not found)\n"),
    }
    CommandOutput {
        success: true,
        output: out,
    }
}

#[tauri::command]
fn run_apply() -> CommandOutput {
    let host = detect_host();
    if host == Host::Unsupported {
        return CommandOutput {
            success: false,
            output: "unsupported platform".to_string(),
        };
    }
    let result = if host == Host::MacbookAir {
        run_nix([
            "run",
            "nix-darwin",
            "--",
            "switch",
            "--flake",
            &format!(".#{host}"),
        ])
    } else {
        run_nix([
            "run",
            "nixpkgs#home-manager",
            "--",
            "switch",
            "--flake",
            &format!(".#{host}"),
        ])
    };
    CommandOutput {
        success: result.0,
        output: result.1,
    }
}

#[tauri::command]
fn run_rollback() -> CommandOutput {
    let host = detect_host();
    if host == Host::Unsupported {
        return CommandOutput {
            success: false,
            output: "unsupported platform".to_string(),
        };
    }
    let result = if host == Host::MacbookAir {
        run_command("darwin-rebuild", ["--rollback"])
    } else {
        run_nix(["run", "nixpkgs#home-manager", "--", "switch", "--rollback"])
    };
    CommandOutput {
        success: result.0,
        output: result.1,
    }
}

#[tauri::command]
fn run_upgrade() -> CommandOutput {
    let result = run_nix(["flake", "update"]);
    CommandOutput {
        success: result.0,
        output: result.1,
    }
}

fn load_manifest() -> Option<schneeforge_core::Manifest> {
    let content = std::fs::read_to_string("config.toml").ok()?;
    schneeforge_core::Manifest::parse(&content).ok()
}

fn run_nix<I, S>(args: I) -> (bool, String)
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    run_command("nix", args)
}

fn run_command<I, S>(cmd: &str, args: I) -> (bool, String)
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    match Command::new(cmd).args(args).output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = if stderr.is_empty() {
                stdout
            } else {
                format!("{stdout}\n{stderr}")
            };
            (out.status.success(), combined)
        }
        Err(e) => (false, format!("failed to run {cmd}: {e}")),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            run_scan,
            run_apply,
            run_rollback,
            run_upgrade
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
