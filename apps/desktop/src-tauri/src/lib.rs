use schneeforge_core::{
    detect_target, resolve_repo, scan, Diagnostics, PreflightReport, ToolInventory, VerifyReport,
    DEFAULT_REPO_URL,
};
use serde::Serialize;
use std::sync::Mutex;

use schneeforge_core::{escalate_command, existing_nix_detected, is_root, EscalatedOp, ManagedNix};

#[derive(Serialize)]
struct CommandOutput {
    success: bool,
    output: String,
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
    // v2: machine 情報は MachineFacts 検出 (repo の config.toml は読まない)
    match schneeforge_core::MachineFacts::detect() {
        Ok(facts) => out.push_str(&format!("user: {}\n", facts.username)),
        Err(e) => out.push_str(&format!("user: (detection failed: {e})\n")),
    }
    Ok(CommandOutput {
        success: true,
        output: out,
    })
}

/// `run_apply`: host 設定の適用。root 権限が必要なため GUI process 内で
/// 直接は実行せず CLI sidecar を昇格実行する (デグレ #5: sudo の TTY 問題)。
/// lock / state 保存は昇格先 CLI process 内で行われる。
#[tauri::command]
async fn run_apply(app: tauri::AppHandle) -> Result<CommandOutput, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_escalated_cli(
            &app,
            EscalatedOp::Apply,
            "sudo schneeforge apply",
            "apply",
            false,
        )
    })
    .await
    .map_err(|e| format!("task error: {e}"))
}

/// `run_rollback`: 前の世代へ戻す。apply と同じ昇格経路。
#[tauri::command]
async fn run_rollback(app: tauri::AppHandle) -> Result<CommandOutput, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_escalated_cli(
            &app,
            EscalatedOp::Rollback,
            "sudo schneeforge rollback",
            "rollback",
            false,
        )
    })
    .await
    .map_err(|e| format!("task error: {e}"))
}

/// `run_upgrade`: flake.lock の更新。apply と同じ昇格経路。
#[tauri::command]
async fn run_upgrade(app: tauri::AppHandle) -> Result<CommandOutput, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_escalated_cli(
            &app,
            EscalatedOp::Upgrade,
            "sudo schneeforge upgrade",
            "upgrade",
            false,
        )
    })
    .await
    .map_err(|e| format!("task error: {e}"))
}

/// `nix_repair_escalated`: NixStatus 分類に基づく修復 (stale ownership
/// record の削除 / 状態に応じた案内)。削除対象は root 所有のため昇格経路で
/// 実行する。案内のみの状態 (Healthy / Missing 等) は何も変更しないため
/// 確認 dialog なしで呼べる。
#[tauri::command]
async fn nix_repair_escalated(app: tauri::AppHandle) -> Result<CommandOutput, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_escalated_cli(
            &app,
            EscalatedOp::NixRepair,
            "sudo schneeforge nix repair",
            "repair",
            true,
        )
    })
    .await
    .map_err(|e| format!("task error: {e}"))
}

/// `nix_uninstall_escalated`: Managed Nix の削除。破壊的操作のため
/// frontend の確認 dialog を経てのみ呼ばれる (確認責任は GUI 側, D8 と
/// 同じ構図)。`--force` は付与しない — fail-closed の突破は CLI の明示
/// 指定に限定する。
#[tauri::command]
async fn nix_uninstall_escalated(app: tauri::AppHandle) -> Result<CommandOutput, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_escalated_cli(
            &app,
            EscalatedOp::NixUninstall,
            "sudo schneeforge nix uninstall",
            "uninstall",
            false,
        )
    })
    .await
    .map_err(|e| format!("task error: {e}"))
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
async fn machine_facts() -> Result<CommandOutput, String> {
    // v2: machine 情報は repo へ書かず machine input (state dir) で管理する。
    // wizard は検出結果の表示のみ行う (repo は書き換えない)
    tauri::async_runtime::spawn_blocking(|| match schneeforge_core::MachineFacts::detect() {
        Ok(facts) => CommandOutput {
            success: true,
            output: format!(
                "user={} home={} system={} hostname={}",
                facts.username,
                facts.home_directory.display(),
                facts.nix_system_string(),
                facts.hostname
            ),
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
    // source 解決経由: managed source は tag-pinned 取得 + state cache、
    // それ以外は local filesystem 読み取り
    schneeforge_core::load_manifest_for(&repo, &schneeforge_core::StateStore::default()).ok()
}

/// `get_dashboard` (v2 §28): Installed / Available の snapshot を返す。
/// available 解決 (git ls-remote + release metadata fetch) は network を
/// 伴うため blocking 実行し、失敗しても command error にせず
/// `available_error` に理由を載せる (offline でも installed は表示する)。
#[tauri::command]
async fn get_dashboard(
    state: tauri::State<'_, CachedToolInventory>,
) -> Result<schneeforge_core::DashboardSnapshot, String> {
    let tc = state.get_or_discover()?;
    tauri::async_runtime::spawn_blocking(move || {
        let repo_state = schneeforge_core::StateStore::default().load();
        let channel = schneeforge_core::channel_of(repo_state.as_ref());
        let repo_url =
            std::env::var("SCHNEEFORGE_REPO_URL").unwrap_or_else(|_| DEFAULT_REPO_URL.to_string());
        let available = match tc.git.as_ref() {
            Some(git) => schneeforge_core::fetch_available(&repo_url, &channel, git)
                .map_err(|e| e.to_string()),
            None => Err("git not found; cannot resolve available release".to_string()),
        };
        schneeforge_core::snapshot(
            env!("CARGO_PKG_VERSION"),
            repo_state.as_ref(),
            load_manifest().as_ref(),
            available,
        )
    })
    .await
    .map_err(|e| format!("task error: {e}"))
}

/// `get_profiles` (v2 §17 follow-up): manifest の available / default と
/// state の選択を返す。managed source は repo file 取得で network を伴う
/// ため blocking 実行する。manifest が解決できない場合は error を返し、
/// frontend は切替 UI を使用不可表示に落とす (Dashboard 自体の表示は維持)。
#[tauri::command]
async fn get_profiles() -> Result<schneeforge_core::ProfileList, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let repo = resolve_repo(None);
        schneeforge_core::list_profiles(&repo).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("task error: {e}"))
}

/// `set_profile`: manifest の available 検証を行ってから state へ保存する
/// (検証は core `set_selection` に集約、fail-closed)。repo は書き換えず、
/// 選択は次回の apply から反映される。
#[tauri::command]
async fn set_profile(name: String) -> Result<CommandOutput, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let repo = resolve_repo(None);
        match schneeforge_core::set_selection(&repo, &name) {
            Ok(()) => CommandOutput {
                success: true,
                output: format!("profile set to '{name}' (applies from next apply)"),
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

/// `clear_profile`: state の選択を解除し manifest default へ戻す。
#[tauri::command]
async fn clear_profile() -> Result<CommandOutput, String> {
    tauri::async_runtime::spawn_blocking(|| match schneeforge_core::clear_selection() {
        Ok(()) => CommandOutput {
            success: true,
            output: "profile selection cleared (manifest default will be used)".to_string(),
        },
        Err(e) => CommandOutput {
            success: false,
            output: e.to_string(),
        },
    })
    .await
    .map_err(|e| format!("task error: {e}"))
}

// ---------- Managed Nix install (issue #16 / D4 Phase 2, D8 GUI 版) ----------

/// bundle 内の CLI sidecar の path。
/// escalation 先は CLI binary でなければならない — desktop 自身の binary は
/// CLI 引数を解釈しないため (externalBin として tauri が main binary と同じ
/// directory へ配置する)。tauri 2.x は bundle 時に triple suffix を除去して
/// `schneeforge-cli` の名で配置するため、まず suffix 無しを探し、開発環境の
/// cargo target (triple 付きのまま) を fallback とする。
fn cli_sidecar_path() -> Result<std::path::PathBuf, String> {
    let plain = format!("schneeforge-cli{}", std::env::consts::EXE_SUFFIX);
    let tripled = format!(
        "schneeforge-cli-{}{}",
        current_target_triple(),
        std::env::consts::EXE_SUFFIX
    );

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in [&plain, &tripled] {
                let p = dir.join(name);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }
    Err(format!(
        "CLI sidecar ({plain}) が見つかりません。desktop app の bundle が不正です"
    ))
}

/// build target triple (host)。`tauri_utils` への直接依存を避けるため
/// 環境から推定する (cross build は本 project の配布経路に無い)
fn current_target_triple() -> String {
    let os = match std::env::consts::OS {
        "macos" => "apple-darwin",
        "linux" => "unknown-linux-gnu",
        other => other,
    };
    format!("{}-{}", std::env::consts::ARCH, os)
}

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
    // wizard は repo 未 clone 状態でここへ来るべきではないが、fail-closed に
    // manifest 読み込み error を返す (fresh machine で誤って呼ばれた場合)
    if !std::path::Path::new(&repo)
        .join("bootstrap-manifest.toml")
        .exists()
    {
        return CommandOutput {
            success: false,
            output: format!(
                "repository ({repo}) に bootstrap-manifest.toml がありません。\
                 先に wizard で repository を clone してください"
            ),
        };
    }
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
/// root なら sidecar CLI を直接、非 root なら escalation helper
/// (osascript / pkexec) で sidecar CLI の `nix install --yes` を再実行する。
/// policy / ownership 記録 / post-install gate は CLI 側に集約されている。
/// stderr の JSON Lines は `nix-install-progress` event として frontend へ
/// 随時流す (spec: phase が順次表示され UI は応答し続ける)。
#[tauri::command]
async fn nix_install_escalated(app: tauri::AppHandle) -> Result<CommandOutput, String> {
    tauri::async_runtime::spawn_blocking(move || nix_install_escalated_blocking(app))
        .await
        .map_err(|e| format!("task error: {e}"))
}

/// frontend へ流す progress event の payload。
/// `phase` は CLI 側 ProgressSink と同じ label (download/verify/…)、
/// `message` は JSON Lines の本文 (無ければ空)。
#[derive(Clone, serde::Serialize)]
struct NixInstallProgress {
    phase: String,
    message: String,
}

fn nix_install_escalated_blocking(app: tauri::AppHandle) -> CommandOutput {
    if existing_nix_detected() {
        return CommandOutput {
            success: false,
            output: "existing Nix detected; SchneeForge does not overwrite".to_string(),
        };
    }
    run_escalated_cli(
        &app,
        EscalatedOp::NixInstall,
        "sudo schneeforge nix install",
        "install",
        true,
    )
}

/// CLI sidecar を昇格 (非 root) / 直接 (root) 実行する共通 runner。
///
/// - root 実行の GUI (例: 開発中に sudo で起動) は昇格不要で直接実行。
///   NIX_SETTING_DIR は昇格と同じく明示渡しする (root の HOME は違うため)
/// - stderr は JSON Lines として parse し `nix-install-progress` event を
///   frontend へ流す (nix install と apply 系で共通の progress 表示)
/// - `dev_guard` は nix install / nix repair のみ true: debug build で
///   誤って本物の install や record 削除を走らせない `--dry-run` 標識を付ける
fn run_escalated_cli(
    app: &tauri::AppHandle,
    op: EscalatedOp,
    cli_fallback: &str,
    op_label: &str,
    dev_guard: bool,
) -> CommandOutput {
    let cli_bin = match cli_sidecar_path() {
        Ok(p) => p,
        Err(e) => {
            return CommandOutput {
                success: false,
                output: format!("{e}\nCLI で実行してください: {cli_fallback}"),
            }
        }
    };
    let repo_dir = resolve_repo(None);
    let repo_path = std::path::PathBuf::from(&repo_dir);

    let (program, args) = if is_root() {
        let mut args: Vec<String> = match op {
            EscalatedOp::NixInstall => vec!["nix".into(), "install".into(), "--yes".into()],
            EscalatedOp::Apply => vec!["apply".into()],
            EscalatedOp::Rollback => vec!["rollback".into()],
            EscalatedOp::Upgrade => vec!["upgrade".into()],
            EscalatedOp::NixRepair => vec!["nix".into(), "repair".into()],
            EscalatedOp::NixUninstall => vec!["nix".into(), "uninstall".into()],
        };
        if dev_guard && cfg!(debug_assertions) {
            // 開発 build で誤って本物の install を走らせない標識 (E2E では使わない)
            args.push("--dry-run".to_string());
        }
        (cli_bin, args)
    } else {
        match escalate_command(&cli_bin, op, &repo_path) {
            Ok(cmd) => cmd,
            Err(e) => {
                return CommandOutput {
                    success: false,
                    output: format!(
                        "{e}\n昇格 helper が利用できません。CLI で実行してください: {cli_fallback}"
                    ),
                }
            }
        }
    };

    let mut cmd = std::process::Command::new(&program);
    cmd.args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if is_root() {
        // 直接実行時は helper 経由と同じ env を明示する
        cmd.env("NIX_SETTING_DIR", &repo_dir);
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return CommandOutput {
                success: false,
                output: format!(
                "昇格実行の起動に失敗しました ({e}): {}\nCLI で実行してください: {cli_fallback}",
                program.display()
            ),
            }
        }
    };

    // stdout と stderr は別々の thread で読む。CLI は進捗を大量に stderr へ
    // 出力するため、片方ずつ順に読むと反対側の pipe buffer (64KB) 満杯で
    // child が block し相互待ちになる (deadlock)
    let stdout_thread = spawn_line_reader(child.stdout.take());
    let stderr_thread = spawn_progress_reader(child.stderr.take(), app);

    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => {
            return CommandOutput {
                success: false,
                output: format!("wait escalated {op_label}: {e}"),
            }
        }
    };
    let out_log = stdout_thread.join().unwrap_or_default();
    let err_log = stderr_thread.join().unwrap_or_default();
    let log = format!("{out_log}{err_log}");
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
                "{op_label} が失敗しました (exit: {}):\n{}\nCLI での再試行: {cli_fallback}",
                status.code().map(|c| c.to_string()).unwrap_or("?".into()),
                tail
            ),
        }
    }
}

/// pipe を行単位で読み切り String を返す reader thread を起こす
fn spawn_line_reader<S: std::io::Read + Send + 'static>(
    stream: Option<S>,
) -> std::thread::JoinHandle<String> {
    use std::io::{BufRead, BufReader};
    std::thread::spawn(move || {
        let mut log = String::new();
        if let Some(stream) = stream {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                log.push_str(&line);
                log.push('\n');
            }
        }
        log
    })
}

/// CLI の stderr (JSON Lines) を parse し、`nix-install-progress` event を
/// frontend へ emit しながら行 log も返す reader thread。
/// phase の判定は CLI 側 ProgressSink と同じ変換を使うため、GUI 表示と
/// CLI 実行時の表示が一致する。
fn spawn_progress_reader<S: std::io::Read + Send + 'static>(
    stream: Option<S>,
    app: &tauri::AppHandle,
) -> std::thread::JoinHandle<String> {
    use std::io::{BufRead, BufReader};
    let app = app.clone();
    std::thread::spawn(move || {
        let mut log = String::new();
        let Some(stream) = stream else {
            return log;
        };
        let mut last_phase = String::new();
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            log.push_str(&line);
            log.push('\n');
            if let Some(parsed) = schneeforge_core::parse_json_line(&line) {
                let message = parsed
                    .fields
                    .as_ref()
                    .and_then(|f| f.message.clone())
                    .unwrap_or_default();
                // span 名 (Step 等) を phase として流す。message しか無い行は
                // 直前の phase のまま (phase 遷移行だけが span を持つ)
                if let Some(span) = parsed.spans.as_ref().and_then(|s| s.last()) {
                    if span.name != last_phase {
                        last_phase = span.name.clone();
                    }
                }
                let progress = NixInstallProgress {
                    phase: last_phase.clone(),
                    message,
                };
                let _ = tauri::Emitter::emit(&app, "nix-install-progress", &progress);
            }
        }
        log
    })
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
            get_dashboard,
            get_profiles,
            set_profile,
            clear_profile,
            run_scan,
            run_apply,
            run_rollback,
            run_upgrade,
            run_preflight,
            machine_facts,
            run_clone_repo,
            run_plan,
            run_verify,
            nix_prepare_plan,
            nix_install_escalated,
            nix_repair_escalated,
            nix_uninstall_escalated
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

    /// v2 §17: get_status は実効 profile (`profile`) と state の明示選択
    /// (`selected_profile`) を serialize する。frontend は `profile` を
    /// 表示に使うため、両 field が揃って選択反映が機能する。
    #[test]
    fn status_serializes_profile_fields() {
        let tc = ToolInventory {
            nix: None,
            git: None,
            homebrew: None,
            nh: None,
        };
        let d = schneeforge_core::diagnose(&tc, None);
        let json = serde_json::to_value(&d).unwrap();
        assert!(
            json.get("profile").is_some(),
            "Diagnostics must serialize profile (effective)"
        );
        assert!(
            json.get("selected_profile").is_some(),
            "Diagnostics must serialize selected_profile (state selection)"
        );

        // frontend は実効 profile を表示する
        let js = include_str!("../../dist/main.js");
        assert!(
            js.contains("s.profile"),
            "frontend should display the effective profile"
        );
    }

    /// v2 §28: get_dashboard の応答は frontend が参照する key を serialize
    /// する。rc.3 と同じ「JS の undefined は falsy 化して実行時 error に
    /// ならない」事故を Dashboard でも防ぐ静的検証。
    #[test]
    fn dashboard_snapshot_serializes_frontend_keys() {
        let snap = schneeforge_core::snapshot("0.2.0", None, None, Err("offline".to_string()));
        let json = serde_json::to_value(&snap).unwrap();
        for key in [
            "installed",
            "available",
            "available_error",
            "update_available",
        ] {
            assert!(
                json.get(key).is_some(),
                "DashboardSnapshot must serialize {key}"
            );
        }
        let installed = json.get("installed").unwrap();
        for key in [
            "version",
            "profile",
            "channel",
            "applied_revision",
            "applied_at",
        ] {
            assert!(
                installed.get(key).is_some(),
                "InstalledInfo must serialize {key}"
            );
        }

        // frontend はこれらの key を参照する
        let js = include_str!("../../dist/main.js");
        for needle in [
            "d.installed.version",
            "d.installed.profile",
            "d.installed.channel",
            "d.available.version",
            "d.available.systems",
            "d.available_error",
            "d.update_available",
        ] {
            assert!(
                js.contains(needle),
                "frontend should reference `{needle}` in the dashboard render"
            );
        }

        // 参照先の DOM 要素が index.html に存在する (無いと常時 catch に落ちる)
        let html = include_str!("../../dist/index.html");
        for id in [
            "dash-installed",
            "dash-profile",
            "dash-channel",
            "dash-available",
            "dash-update",
        ] {
            assert!(
                html.contains(&format!("id=\"{id}\"")),
                "index.html must have #{id} for the dashboard render"
            );
        }
    }

    /// v2 §17 follow-up: GUI からの profile 切替は `ProfileList` の
    /// serialize key・frontend の invoke / key 参照・切替 UI の DOM id が
    /// 揃って機能する。rc.3 と同じ「JS は undefined が falsy 化して実行時
    /// error にならない」事故を静的検証で防ぐ。
    #[test]
    fn profile_switching_contract_matches_frontend() {
        let list = schneeforge_core::ProfileList {
            available: vec![],
            default: None,
            selected: None,
        };
        let json = serde_json::to_value(&list).unwrap();
        for key in ["available", "default", "selected"] {
            assert!(json.get(key).is_some(), "ProfileList must serialize {key}");
        }

        // frontend はこれらの command / key を参照する
        let js = include_str!("../../dist/main.js");
        for needle in [
            "get_profiles",
            "set_profile",
            "clear_profile",
            "p.available",
            "p.default",
            "p.selected",
            // 切替は state のみを変えるため、反映は次回の apply である旨の案内が必須
            "次回の「適用」から反映",
        ] {
            assert!(
                js.contains(needle),
                "frontend should reference `{needle}` for profile switching"
            );
        }

        // 参照先の DOM 要素が index.html に存在する
        let html = include_str!("../../dist/index.html");
        for id in [
            "dash-profile-select",
            "profile-set",
            "profile-clear",
            "profile-note",
        ] {
            assert!(
                html.contains(&format!("id=\"{id}\"")),
                "index.html must have #{id} for the profile switch UI"
            );
        }
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

    /// wizard (stepUser) は username を入力させず machine 情報の検出結果を
    /// 表示する。v2 では machine 情報は MachineFacts (state dir の machine
    /// input) で管理され、repo への書き込み (config.toml 生成) は行わない。
    /// 入力 field が復活すると repo を書き換える旧 flow への regress になる。
    #[test]
    fn wizard_user_step_is_detection_only() {
        let js = include_str!("../../dist/main.js");
        let step = js
            .split("async function stepUser")
            .nth(1)
            .expect("stepUser must exist");
        let body = step.split("\n}").next().unwrap_or(step);
        assert!(
            !body.contains("id=\"username\""),
            "stepUser must not render a username input (MachineFacts detection only)"
        );
        assert!(
            !body.contains("run_generate_config"),
            "stepUser must not generate config.toml in the repo"
        );
        assert!(
            !js.contains("run_generate_config"),
            "no frontend path should invoke run_generate_config anymore"
        );
        assert!(
            body.contains("machine_facts"),
            "stepUser should display the detected machine facts"
        );
    }

    /// wizard (stepPrereq) は Managed Nix 導入の案内を `status.repo_exists` で
    /// gate する。repo 未 clone では install が manifest 解決に失敗するため
    /// (backend も fail-closed で拒否する)、frontend は先に repo step へ
    /// 誘導しなければならない。
    #[test]
    fn wizard_gates_nix_install_on_repo_exists() {
        let js = include_str!("../../dist/main.js");
        assert!(
            js.contains("status.repo_exists"),
            "stepPrereq must check status.repo_exists before offering the Managed Nix install"
        );
        // gate の分岐よりも install flow の呼び出しが後になっている
        // (gate が dead code 化していないことの静的確認)
        let gate = js.find("status.repo_exists").expect("repo gate");
        let install = js
            .find("stepNixInstall(box, actions)")
            .expect("install flow");
        assert!(
            gate < install,
            "repo_exists gate must come before invoking the install flow"
        );
    }

    /// escalation 先は CLI sidecar binary でなければならない。GUI 自身の binary は
    /// CLI 引数を解釈しないため、`current_exe` を昇格しても install が実行されない
    /// (review 指摘 A)。sidecar 解決が `cli_sidecar_path` 経由であることと、
    /// externalBin 設定が tauri.conf.json に存在することを静的に検証する。
    #[test]
    fn escalation_targets_cli_sidecar_not_gui_binary() {
        let rs = include_str!("lib.rs");
        assert!(
            rs.contains("fn cli_sidecar_path()"),
            "escalation must resolve the CLI sidecar binary"
        );
        let conf = include_str!("../tauri.conf.json");
        assert!(
            conf.contains("\"binaries/schneeforge-cli\""),
            "tauri.conf.json must declare the CLI sidecar via externalBin"
        );
        let build = include_str!("../build.rs");
        assert!(
            build.contains("schneeforge-cli-{triple}"),
            "build.rs must stage the sidecar with the target triple suffix"
        );
        // tauri 2.x は bundle 時に triple suffix を除去するため、runtime は
        // suffix 無しの名前も解決できなければならない (DMG gate が実際の配置名)
        assert!(
            rs.contains("\"schneeforge-cli{}\""),
            "runtime resolution must try the unsuffixed bundle name first"
        );
    }

    /// 昇格先に NIX_SETTING_DIR が渡る構造になっていること。root 環境では HOME が
    /// 変わり user の repo が解決できなくなるため (review 指摘 B)。helper 経由は
    /// core 側、root 直接実行は desktop 側、それぞれで env を明示する。
    #[test]
    fn escalation_passes_repo_dir_via_env() {
        let rs = include_str!("lib.rs");
        // root 直接実行 path で env を明示している
        assert!(
            rs.contains("NIX_SETTING_DIR"),
            "direct (root) execution must set NIX_SETTING_DIR explicitly"
        );
        // escalate_command は repo_dir 引数を取る (core が env へ組み立てる)
        assert!(
            rs.contains("escalate_command(&cli_bin, EscalatedOp::NixInstall, &repo_path)"),
            "escalation must pass the repo dir to the helper command builder"
        );
        let core = include_str!("../../../../crates/core/src/managed_nix/escalate.rs");
        assert!(
            core.contains("NIX_SETTING_DIR"),
            "core escalate must pass NIX_SETTING_DIR to the elevated process"
        );
    }

    /// install 実行中の progress が event で frontend へ流れる構造になっている
    /// こと (issue #16 作業項目: stderr JSON Lines parse → phase map の GUI 流用)。
    /// backend の emit 名と frontend の listen 名が一致しないと progress が
    /// 表示されず完了後の一括表示に静かに退化するため静的に突合する。
    #[test]
    fn install_progress_streams_via_matching_event_names() {
        let rs = include_str!("lib.rs");
        let js = include_str!("../../dist/main.js");
        const EVENT: &str = "nix-install-progress";
        assert!(
            rs.contains(&format!("\"{EVENT}\"")),
            "backend must emit the progress event"
        );
        assert!(
            rs.contains("parse_json_line"),
            "backend must parse the CLI's JSON Lines stderr"
        );
        assert!(
            js.contains(&format!("listen(\"{EVENT}\"")),
            "frontend must listen to the progress event"
        );
    }

    /// apply / rollback / upgrade は GUI process 内で直接 core を呼ばず
    /// CLI sidecar の昇格実行へ集約されていること (デグレ #5: sudo の TTY
    /// 問題)。直接 `schneeforge_core::apply` 等を呼ぶと root 昇格されない
    /// まま activation が失敗するため静的に検証する。
    #[test]
    fn apply_rollback_upgrade_run_via_escalated_sidecar() {
        let rs = include_str!("lib.rs");
        // handler は run_escalated_cli (sidecar 昇格) 経由であること
        for (op, marker) in [
            (
                EscalatedOp::Apply,
                "run_escalated_cli(&app, EscalatedOp::Apply",
            ),
            (EscalatedOp::Rollback, "EscalatedOp::Rollback"),
            (EscalatedOp::Upgrade, "EscalatedOp::Upgrade"),
        ] {
            let _ = op;
            assert!(
                rs.contains(marker),
                "apply-family commands must run via the escalated sidecar: {marker}"
            );
        }
        // GUI process 内の直接 core 呼び出しは残っていないこと
        // (nix_prepare_plan の root 不要な plan preview は対象外)。
        // marker は test 自身の文字列と区別するため quote 付きで検索する
        for marker in [
            "schneeforge_core::apply(",
            "schneeforge_core::rollback(",
            "schneeforge_core::upgrade(",
        ] {
            let quoted = format!("\"{marker}\"");
            let call_sites: Vec<usize> = rs
                .match_indices(marker)
                .map(|(i, _)| i)
                .filter(|&i| rs.get(i.saturating_sub(1)..i + marker.len() + 1) != Some(&quoted[..]))
                .collect();
            assert!(
                call_sites.is_empty(),
                "apply-family commands must not call {marker} in the GUI process"
            );
        }
    }

    /// 昇格失敗時の CLI fallback 案内が apply 系にも存在すること。
    /// pkexec 未導入環境などで GUI 操作が使えない場合の回復経路。
    #[test]
    fn apply_family_keeps_cli_fallback_guidance() {
        let rs = include_str!("lib.rs");
        assert!(rs.contains("\"sudo schneeforge apply\""));
        assert!(rs.contains("\"sudo schneeforge rollback\""));
        assert!(rs.contains("\"sudo schneeforge upgrade\""));
    }

    /// nix repair / uninstall も apply 系と同じ sidecar 昇格経路で実行
    /// されること (stale ownership record 削除と upstream uninstall は
    /// root が必要)。GUI process 内での直接実行が無いことを静的に検証する。
    #[test]
    fn nix_repair_uninstall_run_via_escalated_sidecar() {
        let rs = include_str!("lib.rs");
        assert!(
            rs.contains("EscalatedOp::NixRepair"),
            "nix repair must run via the escalated sidecar"
        );
        assert!(
            rs.contains("EscalatedOp::NixUninstall"),
            "nix uninstall must run via the escalated sidecar"
        );
        // GUI から --force を付けた uninstall を組み立てていないこと
        // (fail-closed の突破は CLI の明示指定に限定)
        assert!(
            !rs.contains("\"nix\", \"uninstall\", \"--force\""),
            "GUI must not bypass the ownership check with --force"
        );
    }

    /// repair / uninstall の CLI fallback 案内が存在すること。
    #[test]
    fn nix_repair_uninstall_keep_cli_fallback_guidance() {
        let rs = include_str!("lib.rs");
        assert!(rs.contains("\"sudo schneeforge nix repair\""));
        assert!(rs.contains("\"sudo schneeforge nix uninstall\""));
    }

    /// wizard の修復ボタンと Ready 画面の削除ボタンが backend command を
    /// invoke していること (button → IPC の mapping ずれ検知)。
    #[test]
    fn frontend_invokes_nix_repair_uninstall_commands() {
        let js = include_str!("../../dist/main.js");
        assert!(
            js.contains("invoke(\"nix_repair_escalated\")"),
            "wizard repair button must invoke nix_repair_escalated"
        );
        assert!(
            js.contains("invoke(\"nix_uninstall_escalated\")"),
            "ready-view uninstall button must invoke nix_uninstall_escalated"
        );
        // uninstall は確認を経てのみ実行されること
        assert!(
            js.contains("confirm("),
            "uninstall must be behind a confirmation"
        );
    }
}
