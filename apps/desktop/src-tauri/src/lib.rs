use schneeforge_core::{detect_host, has_git, has_homebrew, has_nix, State};
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

fn load_manifest() -> Option<schneeforge_core::Manifest> {
    let content = std::fs::read_to_string("config.toml").ok()?;
    schneeforge_core::Manifest::parse(&content).ok()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
