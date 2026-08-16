use schneeforge_core::{
    detect_target, resolve_repo, scan, ApplyResult, Diagnostics, PreflightReport, StateStore,
    ToolInventory, VerifyReport,
};
use serde::Serialize;
use std::sync::Mutex;

use schneeforge_core::{
    escalate_command, existing_nix_detected, is_root, self_binary_path, EscalatedOp, ManagedNix,
};

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

/// desktop アプリ内で ToolInventory を1回 discover してキャッシュする State。
///
/// `fix_path_env::fix()` が `main()` で呼ばれた後に discover するため、
/// macOS の .app から起動しても PATH 補正後の解決結果が使われる。
#[derive(Default)]
struct CachedToolInventory(Mutex<Option<ToolInventory>>);

impl CachedToolInventory {
    fn get_or_discover(&self) -> Result<ToolInventory, String> {
        let mut lock = self.0.lock().map_err(|e| format!("lock error: {e}"))?;
        if let Some(tc) = lock.as_ref() {
            return Ok(tc.clone());
        }
        // discover() は infallible (未検出ツールは None になる)。Fresh install 環境でも
        // Diagnostics が Nix 無し状態を表示できるよう、Nix が無くてもキャッシュする。
        let tc = ToolInventory::discover();
        *lock = Some(tc.clone());
        Ok(tc)
    }
}

#[tauri::command]
async fn get_status(state: tauri::State<'_, CachedToolInventory>) -> Result<Diagnostics, String> {
    let tc = state.get_or_discover()?;
    tauri::async_runtime::spawn_blocking(move || schneeforge_core::diagnose(&tc, None))
        .await
        .map_err(|e| format!("task error: {e}"))
}

#[tauri::command]
fn run_scan(state: tauri::State<'_, CachedToolInventory>) -> Result<CommandOutput, String> {
    let tc = state.get_or_discover()?;
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
async fn run_apply(state: tauri::State<'_, CachedToolInventory>) -> Result<CommandOutput, String> {
    let tc = state.get_or_discover()?;
    let out = tauri::async_runtime::spawn_blocking(move || {
        apply_output(schneeforge_core::apply(
            &detect_target(),
            &resolve_repo(None),
            &StateStore::default(),
            &tc,
            true,
        ))
    })
    .await
    .map_err(|e| format!("task error: {e}"))?;
    Ok(out)
}

#[tauri::command]
async fn run_rollback(
    state: tauri::State<'_, CachedToolInventory>,
) -> Result<CommandOutput, String> {
    let tc = state.get_or_discover()?;
    let out = tauri::async_runtime::spawn_blocking(move || {
        apply_output(schneeforge_core::rollback(
            &detect_target(),
            &resolve_repo(None),
            &StateStore::default(),
            &tc,
            true,
        ))
    })
    .await
    .map_err(|e| format!("task error: {e}"))?;
    Ok(out)
}

#[tauri::command]
async fn run_upgrade(
    state: tauri::State<'_, CachedToolInventory>,
) -> Result<CommandOutput, String> {
    let tc = state.get_or_discover()?;
    let repo = resolve_repo(None);
    let out = tauri::async_runtime::spawn_blocking(move || {
        option_output(schneeforge_core::upgrade(&repo, &tc, true))
    })
    .await
    .map_err(|e| format!("task error: {e}"))?;
    Ok(out)
}

#[tauri::command]
async fn run_preflight(
    state: tauri::State<'_, CachedToolInventory>,
) -> Result<PreflightReport, String> {
    let tc = state.get_or_discover()?;
    tauri::async_runtime::spawn_blocking(move || schneeforge_core::preflight(&tc))
        .await
        .map_err(|e| format!("task error: {e}"))
}

#[tauri::command]
async fn run_generate_config(username: String) -> Result<CommandOutput, String> {
    let repo = resolve_repo(None);
    tauri::async_runtime::spawn_blocking(move || {
        match schneeforge_core::generate_config(&repo, &username) {
            Ok(()) => CommandOutput {
                success: true,
                output: "config.toml を生成しました".to_string(),
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
async fn run_clone_repo(
    url: String,
    state: tauri::State<'_, CachedToolInventory>,
) -> Result<CommandOutput, String> {
    let tc = state.get_or_discover()?;
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
async fn run_plan(state: tauri::State<'_, CachedToolInventory>) -> Result<CommandOutput, String> {
    let tc = state.get_or_discover()?;
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
fn run_verify(state: tauri::State<'_, CachedToolInventory>) -> Result<VerifyReport, String> {
    let tc = state.get_or_discover()?;
    Ok(schneeforge_core::verify(&resolve_repo(None), &tc))
}

fn load_manifest() -> Option<schneeforge_core::Manifest> {
    let repo = resolve_repo(None);
    schneeforge_core::Manifest::load(&repo).ok()
}

// ---------- Managed Nix install (issue #16 / D4 Phase 2, D8 GUI 版) ----------

/// `nix_prepare_plan`: root 不要の plan preview。
/// core の `ManagedNix::prepare_plan()` (policy 集約済み) を自 process で呼び、
/// detailed plan 行を返す。install 実行は別 command の確認後。
#[tauri::command]
async fn nix_prepare_plan() -> Result<CommandOutput, String> {
    tauri::async_runtime::spawn_blocking(nix_prepare_plan_blocking)
        .await
        .map_err(|e| format!("task error: {e}"))
}

fn nix_prepare_plan_blocking() -> CommandOutput {
    if existing_nix_detected() {
        return CommandOutput {
            success: false,
            output: "existing Nix detected; SchneeForge does not overwrite".to_string(),
        };
    }
    let repo = resolve_repo(None);
    let mn = match ManagedNix::load_from_repo(std::path::Path::new(&repo)) {
        Ok(mn) => mn,
        Err(e) => {
            return CommandOutput {
                success: false,
                output: format!("load bootstrap-manifest: {e}"),
            }
        }
    };
    let preflight = mn.preflight();
    if !preflight.supported {
        return CommandOutput {
            success: false,
            output: format!(
                "unsupported platform/arch: {} {}",
                preflight.platform, preflight.arch
            ),
        };
    }
    let plan_dir = match schneeforge_core::secure_plan_dir() {
        Ok(d) => d,
        Err(e) => {
            return CommandOutput {
                success: false,
                output: format!("resolve plan dir: {e}"),
            }
        }
    };
    let mut progress = GuiCollectProgress::default();
    let plan_file = match mn.prepare_plan(
        preflight.platform,
        preflight.arch,
        &plan_dir,
        &[],
        &mut progress,
    ) {
        Ok(p) => p,
        Err(e) => {
            return CommandOutput {
                success: false,
                output: format!("plan generation failed: {e}"),
            }
        }
    };
    let summary = match schneeforge_core::summarize_plan(&plan_file) {
        Ok(lines) => lines.join("\n"),
        Err(e) => {
            return CommandOutput {
                success: false,
                output: format!("read plan summary: {e}"),
            }
        }
    };
    CommandOutput {
        success: true,
        output: format!("{}\n\n{}", progress.take_log(), summary),
    }
}

/// `nix_install_escalated`: GUI の最終確認済みを前提に install を実行する。
/// root なら自 process と同一 binary を直接、非 root なら escalation helper
/// (osascript / pkexec) で `schneeforge nix install --yes` を再実行する。
/// policy / ownership 記録 / post-install gate は CLI 側に集約されている。
#[tauri::command]
async fn nix_install_escalated() -> Result<CommandOutput, String> {
    tauri::async_runtime::spawn_blocking(nix_install_escalated_blocking)
        .await
        .map_err(|e| format!("task error: {e}"))
}

fn nix_install_escalated_blocking() -> CommandOutput {
    if existing_nix_detected() {
        return CommandOutput {
            success: false,
            output: "existing Nix detected; SchneeForge does not overwrite".to_string(),
        };
    }

    let self_bin = match self_binary_path() {
        Ok(p) => p,
        Err(e) => {
            return CommandOutput {
                success: false,
                output: format!("resolve schneeforge binary: {e}"),
            }
        }
    };

    // root 実行の GUI (例: 開発中に sudo で起動) は昇格不要で直接実行
    let (program, args) = if is_root() {
        let mut args = vec!["nix".to_string(), "install".to_string(), "--yes".to_string()];
        if cfg!(debug_assertions) {
            // 開発 build で誤って本物の install を走らせない標識 (E2E では使わない)
            args.push("--dry-run".to_string());
        }
        (self_bin, args)
    } else {
        match escalate_command(&self_bin, EscalatedOp::NixInstall) {
            Ok(cmd) => cmd,
            Err(e) => {
                return CommandOutput {
                    success: false,
                    output: format!(
                        "{e}\n昇格 helper が利用できません。CLI で実行してください: sudo schneeforge nix install"
                    ),
                }
            }
        }
    };

    let mut cmd = std::process::Command::new(&program);
    cmd.args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return CommandOutput {
                success: false,
                output: format!(
                    "昇格実行の起動に失敗しました ({e}): {}\nCLI で実行してください: sudo schneeforge nix install",
                    program.display()
                ),
            }
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    // stdout/stderr を別 thread で読み取り、phase 行 ([phase] … / [verify] …) を
    // progress log として集める (GUI は完了後に一括表示。行 stream は別 change)
    let reader = std::thread::spawn(move || {
        use std::io::{BufRead, BufReader, Read};
        let mut log = String::new();
        let streams: Vec<Box<dyn Read>> = vec![
            stdout.map(|s| Box::new(s) as Box<dyn Read>),
            stderr.map(|s| Box::new(s) as Box<dyn Read>),
        ]
        .into_iter()
        .flatten()
        .collect();
        for stream in streams {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                log.push_str(&line);
                log.push('\n');
            }
        }
        log
    });

    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => {
            return CommandOutput {
                success: false,
                output: format!("wait escalated install: {e}"),
            }
        }
    };
    let log = reader.join().unwrap_or_default();
    if status.success() {
        CommandOutput {
            success: true,
            output: log,
        }
    } else {
        let tail: String = log.lines().rev().take(20).collect::<Vec<_>>().join("\n");
        CommandOutput {
            success: false,
            output: format!(
                "install が失敗しました (exit: {}):\n{}\nCLI での再試行: sudo schneeforge nix install",
                status.code().map(|c| c.to_string()).unwrap_or("?".into()),
                tail
            ),
        }
    }
}

/// plan preview の進捗収集用 ProgressSink (画面表示は完了後の一括)
#[derive(Default)]
struct GuiCollectProgress {
    log: String,
}

impl GuiCollectProgress {
    fn take_log(&mut self) -> String {
        std::mem::take(&mut self.log)
    }
}

impl schneeforge_core::ProgressSink for GuiCollectProgress {
    fn on_phase(&mut self, phase: schneeforge_core::InstallPhase) {
        let label = match phase {
            schneeforge_core::InstallPhase::Download => "download",
            schneeforge_core::InstallPhase::Verify => "verify",
            schneeforge_core::InstallPhase::Privilege => "privilege",
            schneeforge_core::InstallPhase::Plan => "plan",
            schneeforge_core::InstallPhase::Install => "install",
            schneeforge_core::InstallPhase::PostInstall => "post-install",
        };
        self.log.push_str(&format!("[phase] {label}\n"));
    }
    fn on_log(&mut self, line: &schneeforge_core::JsonLogLine) {
        if let Some(level) = &line.level {
            let msg = line
                .fields
                .as_ref()
                .and_then(|f| f.message.as_deref())
                .unwrap_or("");
            let span = line
                .spans
                .as_ref()
                .and_then(|s| s.first())
                .map(|s| s.name.as_str())
                .unwrap_or("");
            if !msg.is_empty() || !span.is_empty() {
                self.log.push_str(&format!("  {level:<5} {span}: {msg}\n"));
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CachedToolInventory::default())
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
            run_verify,
            nix_prepare_plan,
            nix_install_escalated
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// wizard (stepPrereq) が読む `pre.*` field が PreflightReport の serde keys と
    /// 一致することを検証する。コマンド名 test と違い、応答 field は JS で undefined
    /// (falsy) に落ちるため実行時 error にならず「常に NG」化する (rc.3 で実際に発生)。
    #[test]
    fn wizard_reads_preflight_report_fields() {
        let js = include_str!("../../dist/main.js");

        // backend 側の serialize keys (serde は field 名そのまま)
        let report = schneeforge_core::preflight(&ToolInventory {
            nix: None,
            git: None,
            homebrew: None,
            nh: None,
        });
        let keys: Vec<String> = serde_json::to_value(&report)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();

        // frontend の `pre.<field>` 参照を抽出
        let used: Vec<String> = js
            .split("pre.")
            .skip(1)
            .filter_map(|rest| {
                let field: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                (!field.is_empty()).then_some(field)
            })
            .collect();
        let used: Vec<String> = {
            let mut u = used;
            u.sort();
            u.dedup();
            u
        };

        assert!(!used.is_empty(), "wizard should read some pre.* fields");
        for f in &used {
            assert!(
                keys.contains(f),
                "frontend reads `pre.{f}` but PreflightReport serializes {keys:?}"
            );
        }
    }

    /// wizard (stepRepo) の既定 repository URL が core の DEFAULT_REPO_URL と一致
    /// することを検証する。frontend の既定値と backend の fallback がずれると
    /// 「空入力で clone した場合」と「既定値を明示入力した場合」で別 repository
    /// を見ることになる (fork 利用者の誤誘導になる) ため静的に突合する。
    #[test]
    fn wizard_default_repo_url_matches_core() {
        let js = include_str!("../../dist/main.js");

        let default_marker = "DEFAULT_REPO_URL = \"";
        let frontend_url: String = js
            .split(default_marker)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_default()
            .to_string();

        assert!(
            !frontend_url.is_empty(),
            "stepRepo should define DEFAULT_REPO_URL"
        );
        assert_eq!(
            frontend_url,
            schneeforge_core::DEFAULT_REPO_URL,
            "frontend default repo URL must match core DEFAULT_REPO_URL"
        );
    }

    /// wizard (stepPrereq) は legacy な `curl | sh` での Nix 導入を案内しない。
    /// この案内で入れた Nix は SchneeForge の ownership 管理外になり
    /// uninstall 対称性が崩れるため (gui-diagnostics spec)。
    #[test]
    fn wizard_does_not_suggest_legacy_curl_install() {
        let js = include_str!("../../dist/main.js");
        assert!(
            !js.contains("nixos.org/nix/install"),
            "wizard must not suggest the legacy curl | sh install"
        );
        assert!(
            js.contains("schneeforge nix install"),
            "wizard should suggest the Managed Nix install instead"
        );
    }

    /// wizard (stepPrereq) は get_status 応答の `nix_status` (NixStatus 分類) を
    /// 参照する。Diagnostics が当該 field を serialize することと、frontend が
    /// 読むことの両方が揃って初めて表示が機能するため静的に突合する。
    #[test]
    fn wizard_reads_nix_status_field() {
        let js = include_str!("../../dist/main.js");
        assert!(
            js.contains("nix_status"),
            "stepPrereq should read status.nix_status from get_status"
        );

        // backend 側: Diagnostics が nix_status を serialize すること
        let tc = ToolInventory {
            nix: None,
            git: None,
            homebrew: None,
            nh: None,
        };
        let d = schneeforge_core::diagnose(&tc, None);
        let json = serde_json::to_value(&d).unwrap();
        assert!(
            json.get("nix_status").is_some(),
            "Diagnostics must serialize nix_status"
        );
    }

    /// Managed Nix の GUI install flow (issue #16) は D8 の 2 段階確認を
    /// frontend で持つ: nix_prepare_plan (plan preview) を表示してから
    /// nix_install_escalated (確認済みの install) を呼ぶ。逆順や preview 無し
    /// 実行に regress しないよう静的に検証する。
    #[test]
    fn wizard_nix_install_flow_is_two_phase() {
        let js = include_str!("../../dist/main.js");
        let preview = js.find("nix_prepare_plan").expect("plan preview invoke");
        let execute = js.find("nix_install_escalated").expect("install invoke");
        assert!(
            preview < execute,
            "nix_prepare_plan must be invoked before nix_install_escalated (D8)"
        );
        // 確認 gate: plan 表示と install 実行の間にユーザー操作 (導入する button) がある
        let confirm_btn = js.find("導入する").expect("confirm button label");
        assert!(
            preview < confirm_btn && confirm_btn < execute,
            "install must wait for explicit user confirmation between preview and execute"
        );
    }

    /// escalation 失敗時の CLI fallback 案内 (`sudo schneeforge nix install`) が
    /// wizard に存在する。pkexec 未導入環境などで GUI 導入が使えない場合の
    /// 回復経路を保証する (gui-operations spec)。
    #[test]
    fn wizard_keeps_cli_fallback_for_escalation_failures() {
        let js = include_str!("../../dist/main.js");
        assert!(
            js.contains("sudo schneeforge nix install"),
            "wizard must keep the CLI fallback guidance"
        );
        assert!(
            js.contains("cliFallbackNote"),
            "failure paths should render the CLI fallback note"
        );
    }
}
