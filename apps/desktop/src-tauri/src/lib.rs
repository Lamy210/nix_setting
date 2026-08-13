use schneeforge_core::{
    detect_target, resolve_repo, scan, ApplyResult, Diagnostics, PreflightReport, StateStore,
    Toolchain, VerifyReport,
};
use serde::Serialize;
use std::sync::Mutex;

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

/// desktop アプリ内で Toolchain を1回解決してキャッシュする State。
///
/// `fix_path_env::fix()` が `main()` で呼ばれた後に resolve するため、
/// macOS の .app から起動しても PATH 補正後の解決結果が使われる。
#[derive(Default)]
struct CachedToolchain(Mutex<Option<Toolchain>>);

impl CachedToolchain {
    fn get_or_resolve(&self) -> Result<Toolchain, String> {
        let mut lock = self.0.lock().map_err(|e| format!("lock error: {e}"))?;
        if let Some(tc) = lock.as_ref() {
            return Ok(tc.clone());
        }
        let tc = Toolchain::resolve().map_err(|e| e.to_string())?;
        *lock = Some(tc.clone());
        Ok(tc)
    }
}

#[tauri::command]
async fn get_status(state: tauri::State<'_, CachedToolchain>) -> Result<Diagnostics, String> {
    let tc = state.get_or_resolve()?;
    tauri::async_runtime::spawn_blocking(move || schneeforge_core::diagnose(&tc, None))
        .await
        .map_err(|e| format!("task error: {e}"))
}

#[tauri::command]
fn run_scan(state: tauri::State<'_, CachedToolchain>) -> Result<CommandOutput, String> {
    let tc = state.get_or_resolve()?;
    let target = detect_target();
    let mut out = scan(&target, &tc);
    if let Some(m) = load_manifest() {
        out.push_str(&format!("user: {}\n", m.user.username));
    } else {
        out.push_str("user: (config.toml not found)\n");
    }
    Ok(CommandOutput {
        success: true,
        output: out,
    })
}

#[tauri::command]
async fn run_apply(state: tauri::State<'_, CachedToolchain>) -> Result<CommandOutput, String> {
    let tc = state.get_or_resolve()?;
    tauri::async_runtime::spawn_blocking(move || {
        apply_output(schneeforge_core::apply(
            &detect_target(),
            &resolve_repo(None),
            &StateStore::default(),
            &tc,
            true,
        ))
    })
    .await
    .map(Some)
    .unwrap_or(None)
    .ok_or_else(|| "task error".to_string())
}

#[tauri::command]
async fn run_rollback(state: tauri::State<'_, CachedToolchain>) -> Result<CommandOutput, String> {
    let tc = state.get_or_resolve()?;
    tauri::async_runtime::spawn_blocking(move || {
        apply_output(schneeforge_core::rollback(
            &detect_target(),
            &resolve_repo(None),
            &StateStore::default(),
            &tc,
            true,
        ))
    })
    .await
    .map(Some)
    .unwrap_or(None)
    .ok_or_else(|| "task error".to_string())
}

#[tauri::command]
async fn run_upgrade(state: tauri::State<'_, CachedToolchain>) -> Result<CommandOutput, String> {
    let tc = state.get_or_resolve()?;
    let repo = resolve_repo(None);
    tauri::async_runtime::spawn_blocking(move || {
        option_output(schneeforge_core::upgrade(&repo, &tc, true))
    })
    .await
    .map(Some)
    .unwrap_or(None)
    .ok_or_else(|| "task error".to_string())
}

#[tauri::command]
async fn run_preflight(state: tauri::State<'_, CachedToolchain>) -> Result<PreflightReport, String> {
    let tc = state.get_or_resolve()?;
    tauri::async_runtime::spawn_blocking(move || schneeforge_core::preflight(&tc))
        .await
        .map_err(|e| format!("task error: {e}"))
}

#[tauri::command]
async fn run_generate_config(username: String) -> Result<CommandOutput, String> {
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
    .map_err(|e| format!("task error: {e}"))
}

#[tauri::command]
async fn run_clone_repo(
    url: String,
    state: tauri::State<'_, CachedToolchain>,
) -> Result<CommandOutput, String> {
    let tc = state.get_or_resolve()?;
    let dest = resolve_repo(None);
    tauri::async_runtime::spawn_blocking(move || {
        match schneeforge_core::clone_repo(&url, &dest, &tc) {
            Ok(out) => CommandOutput {
                success: true,
                output: out,
            },
            Err(e) => CommandOutput {
                success: false,
                output: e.to_string(),
            },
        }
    })
    .await
    .map_err(|e| format!("task error: {e}"))
}

#[tauri::command]
async fn run_plan(state: tauri::State<'_, CachedToolchain>) -> Result<CommandOutput, String> {
    let tc = state.get_or_resolve()?;
    let repo = resolve_repo(None);
    tauri::async_runtime::spawn_blocking(move || match schneeforge_core::plan(&repo, &tc, true) {
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
    .map_err(|e| format!("task error: {e}"))
}

#[tauri::command]
fn run_verify(state: tauri::State<'_, CachedToolchain>) -> Result<VerifyReport, String> {
    let tc = state.get_or_resolve()?;
    Ok(schneeforge_core::verify(&resolve_repo(None), &tc))
}

fn load_manifest() -> Option<schneeforge_core::Manifest> {
    let repo = resolve_repo(None);
    schneeforge_core::Manifest::load(&repo).ok()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CachedToolchain::default())
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
