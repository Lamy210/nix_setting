use schneeforge_core::{
    apply, detect_host, has_git, has_homebrew, has_nix, rollback, scan, upgrade, State,
};
use serde::Serialize;

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
    let mut out = scan(host);
    if let Some(m) = load_manifest() {
        out.push_str(&format!("user: {}\n", m.user.username));
    } else {
        out.push_str("user: (config.toml not found)\n");
    }
    CommandOutput {
        success: true,
        output: out,
    }
}

#[tauri::command]
fn run_apply() -> CommandOutput {
    match apply(detect_host(), &resolve_repo()) {
        Ok(out) => CommandOutput { success: true, output: out },
        Err(e) => CommandOutput { success: false, output: e },
    }
}

#[tauri::command]
fn run_rollback() -> CommandOutput {
    match rollback(detect_host()) {
        Ok(out) => CommandOutput { success: true, output: out },
        Err(e) => CommandOutput { success: false, output: e },
    }
}

#[tauri::command]
fn run_upgrade() -> CommandOutput {
    match upgrade() {
        Ok(out) => CommandOutput { success: true, output: out },
        Err(e) => CommandOutput { success: false, output: e },
    }
}

fn load_manifest() -> Option<schneeforge_core::Manifest> {
    let repo = resolve_repo();
    let content = std::fs::read_to_string(format!("{repo}/config.toml")).ok()?;
    schneeforge_core::Manifest::parse(&content).ok()
}

fn resolve_repo() -> String {
    if let Ok(r) = std::env::var("NIX_SETTING_DIR") {
        return r;
    }
    if let Ok(home) = std::env::var("HOME") {
        return format!("{home}/nix_setting");
    }
    ".".to_string()
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
