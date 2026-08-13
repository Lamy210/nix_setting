use schneeforge_core::{
    detect_target, has_git, has_homebrew, has_nix, resolve_repo, scan, StateStore,
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
    let state = StateStore::default().load();
    Status {
        host: detect_target().to_string(),
        user: manifest.as_ref().map(|m| m.user.username.clone()),
        nix: has_nix(),
        homebrew: has_homebrew(),
        git: has_git(),
        applied_revision: state.and_then(|s| s.applied_revision),
    }
}

#[tauri::command]
fn run_scan() -> CommandOutput {
    let target = detect_target();
    let mut out = scan(&target);
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
    match schneeforge_core::apply(&detect_target(), &resolve_repo(None), &StateStore::default(), true)
    {
        Ok(result) => CommandOutput {
            success: true,
            output: result.output.unwrap_or_default(),
        },
        Err(e) => CommandOutput { success: false, output: e.to_string() },
    }
}

#[tauri::command]
fn run_rollback() -> CommandOutput {
    match schneeforge_core::rollback(&detect_target(), &resolve_repo(None), &StateStore::default(), true)
    {
        Ok(result) => CommandOutput {
            success: true,
            output: result.output.unwrap_or_default(),
        },
        Err(e) => CommandOutput { success: false, output: e.to_string() },
    }
}

#[tauri::command]
fn run_upgrade() -> CommandOutput {
    match schneeforge_core::upgrade(&resolve_repo(None), true) {
        Ok(out) => CommandOutput { success: true, output: out.unwrap_or_default() },
        Err(e) => CommandOutput { success: false, output: e.to_string() },
    }
}

fn load_manifest() -> Option<schneeforge_core::Manifest> {
    let repo = resolve_repo(None);
    let content = std::fs::read_to_string(format!("{repo}/config.toml")).ok()?;
    schneeforge_core::Manifest::parse(&content).ok()
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
