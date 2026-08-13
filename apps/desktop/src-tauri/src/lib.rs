use schneeforge_core::{
    detect_target, resolve_repo, scan, ApplyResult, Diagnostics, PreflightReport, StateStore,
    VerifyReport,
};
use serde::Serialize;

#[derive(Serialize)]
struct CommandOutput {
    success: bool,
    output: String,
}

fn apply_output(r: schneeforge_core::Result<ApplyResult>) -> CommandOutput {
    match r {
        Ok(r) => CommandOutput {
            success: true,
            output: r.output.unwrap_or_default(),
        },
        Err(e) => CommandOutput {
            success: false,
            output: e.to_string(),
        },
    }
}

fn option_output(r: schneeforge_core::Result<Option<String>>) -> CommandOutput {
    match r {
        Ok(out) => CommandOutput {
            success: true,
            output: out.unwrap_or_default(),
        },
        Err(e) => CommandOutput {
            success: false,
            output: e.to_string(),
        },
    }
}

#[tauri::command]
async fn get_status() -> Result<Diagnostics, String> {
    tauri::async_runtime::spawn_blocking(|| schneeforge_core::diagnose(None))
        .await
        .map_err(|e| format!("task error: {e}"))
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
async fn run_apply() -> CommandOutput {
    tauri::async_runtime::spawn_blocking(|| {
        apply_output(schneeforge_core::apply(
            &detect_target(),
            &resolve_repo(None),
            &StateStore::default(),
            true,
        ))
    })
    .await
    .unwrap_or_else(|e| CommandOutput {
        success: false,
        output: format!("task error: {e}"),
    })
}

#[tauri::command]
async fn run_rollback() -> CommandOutput {
    tauri::async_runtime::spawn_blocking(|| {
        apply_output(schneeforge_core::rollback(
            &detect_target(),
            &resolve_repo(None),
            &StateStore::default(),
            true,
        ))
    })
    .await
    .unwrap_or_else(|e| CommandOutput {
        success: false,
        output: format!("task error: {e}"),
    })
}

#[tauri::command]
async fn run_upgrade() -> CommandOutput {
    tauri::async_runtime::spawn_blocking(|| {
        option_output(schneeforge_core::upgrade(&resolve_repo(None), true))
    })
    .await
    .unwrap_or_else(|e| CommandOutput {
        success: false,
        output: format!("task error: {e}"),
    })
}

#[tauri::command]
async fn run_preflight() -> Result<PreflightReport, String> {
    tauri::async_runtime::spawn_blocking(schneeforge_core::preflight)
        .await
        .map_err(|e| format!("task error: {e}"))
}

#[tauri::command]
async fn run_generate_config(username: String) -> CommandOutput {
    let repo = resolve_repo(None);
    tauri::async_runtime::spawn_blocking(move || match schneeforge_core::generate_config(&repo, &username)
    {
        Ok(()) => CommandOutput {
            success: true,
            output: "config.toml を生成しました".to_string(),
        },
        Err(e) => CommandOutput {
            success: false,
            output: e.to_string(),
        },
    })
    .await
    .unwrap_or_else(|e| CommandOutput {
        success: false,
        output: format!("task error: {e}"),
    })
}

#[tauri::command]
async fn run_clone_repo(url: String) -> CommandOutput {
    let dest = resolve_repo(None);
    tauri::async_runtime::spawn_blocking(move || match schneeforge_core::clone_repo(&url, &dest) {
        Ok(out) => CommandOutput {
            success: true,
            output: out,
        },
        Err(e) => CommandOutput {
            success: false,
            output: e.to_string(),
        },
    })
    .await
    .unwrap_or_else(|e| CommandOutput {
        success: false,
        output: format!("task error: {e}"),
    })
}

#[tauri::command]
async fn run_plan() -> CommandOutput {
    let repo = resolve_repo(None);
    tauri::async_runtime::spawn_blocking(move || match schneeforge_core::plan(&repo, true) {
        Ok(r) => CommandOutput {
            success: true,
            output: r.output.unwrap_or_default(),
        },
        Err(e) => CommandOutput {
            success: false,
            output: e.to_string(),
        },
    })
    .await
    .unwrap_or_else(|e| CommandOutput {
        success: false,
        output: format!("task error: {e}"),
    })
}

#[tauri::command]
fn run_verify() -> VerifyReport {
    schneeforge_core::verify(&resolve_repo(None))
}

fn load_manifest() -> Option<schneeforge_core::Manifest> {
    let repo = resolve_repo(None);
    schneeforge_core::Manifest::load(&repo).ok()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_status,
            run_scan,
            run_apply,
            run_rollback,
            run_upgrade,
            run_preflight,
            run_generate_config,
            run_clone_repo,
            run_plan,
            run_verify
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    /// frontend の invoke コマンド名と backend の generate_handler 登録名が一致することを検証する
    /// (button → IPC の mapping がずれたままにならないよう静的クロスチェック)
    #[test]
    fn frontend_commands_match_backend() {
        let js = include_str!("../../dist/main.js");
        let rs = include_str!("lib.rs");

        let mut frontend: Vec<String> = js
            .split("invoke(\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .map(|s| s.to_string())
            .collect();
        frontend.sort();
        frontend.dedup();

        let marker = ["tauri::", "generate_handler!["].concat();
        let mut backend: Vec<String> = rs
            .split(&marker)
            .nth(1)
            .and_then(|s| s.split(']').next())
            .map(|block| {
                block
                    .lines()
                    .map(|l| l.trim().trim_end_matches(',').to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        backend.sort();

        assert_eq!(
            frontend, backend,
            "frontend invoke() names must match backend generate_handler commands"
        );
    }
}

