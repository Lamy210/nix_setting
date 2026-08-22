//! `schneeforge nix` subcommand handlers (Phase 1: Managed Nix bootstrap)

use std::path::{Path, PathBuf};

use clap::Args;

use schneeforge_core::{
    cache_path, classify_current, default_ownership_path, default_receipt_path, detect_arch,
    detect_platform, existing_nix_detected, installed_binary_path, nix_health,
    repair_action_current, repair_args, run_with_json_logs, secure_plan_dir,
    uninstall_args as build_uninstall_args, JsonLogLine, ManagedNix, ManagedNixError, NoProgress,
    OwnershipRecord, ProgressSink, Receipt, RepairAction, ToolInventory, UpstreamRepair,
};

/// `schneeforge nix` サブコマンド
#[derive(Args, Debug)]
pub struct NixArgs {
    #[command(subcommand)]
    pub command: NixSub,
}

#[derive(clap::Subcommand, Debug)]
pub enum NixSub {
    /// Managed Nix を install (preflight → download → verify → plan → install)
    Install(InstallArgs),
    /// Managed Nix の receipt / nix store / flakes 設定を診断
    Doctor,
    /// Managed Nix を uninstall (nix-darwin 残留時は警告で abort)
    Uninstall(UninstallArgs),
    /// NixStatus 分類に基づいて修復 (stale record 削除 / upstream repair)
    Repair(RepairArgs),
}

#[derive(Args, Debug, Default)]
pub struct InstallArgs {
    /// `nix-installer plan` へ渡す extra-conf 行 (複数指定可)
    #[arg(long = "extra-conf")]
    pub extra_conf: Vec<String>,

    /// preflight 表示だけで終了する (download / plan / install を skipping)
    #[arg(long)]
    pub dry_run: bool,

    /// detailed plan 表示後の最終確認を skip する (automation 用)。
    /// upstream は --no-confirm で呼ぶため、確認責任は SchneeForge 側にある (D8)。
    #[arg(long)]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct UninstallArgs {
    /// `/nix/receipt.json` 以外の receipt path
    #[arg(long)]
    pub receipt: Option<PathBuf>,

    /// SchneeForge ownership record が無い場合も uninstall を続行する
    /// (SchneeForge 経由以外で install された Nix の明示的な削除)
    #[arg(long)]
    pub force: bool,
}

#[derive(Args, Debug, Default)]
pub struct RepairArgs {
    /// 実行内容を表示するのみで file system を変更しない
    #[arg(long)]
    pub dry_run: bool,

    /// upstream `nix-installer repair hooks` (shell profile 修復) を実行する
    #[arg(long)]
    pub hooks: bool,

    /// upstream `nix-installer repair sequoia` (macOS Sequoia の _nixbld 回復)
    /// を実行する
    #[arg(long)]
    pub sequoia: bool,
}

type Result = std::result::Result<(), String>;

/// `schneeforge nix install`
pub fn run_install(repo_root: &str, args: InstallArgs) -> Result {
    // manifest は repo file 優先 + embedded fallback (fresh machine は
    // repo checkout 無しでも install できる)
    let mn = ManagedNix::load_prefer_repo(Some(Path::new(repo_root)))
        .map_err(|e| format!("load bootstrap-manifest: {e}"))?;

    let preflight = mn.preflight();
    eprintln!("=== Managed Nix install ===");
    eprintln!();
    for line in preflight.summary_lines() {
        eprintln!("{line}");
    }
    eprintln!();

    // dry-run は preview なので、blocking condition があっても info 表示して終了する (D8)
    if args.dry_run {
        if !preflight.supported {
            eprintln!(
                "[dry-run] unsupported platform/arch ({}) のため install は中止されます。",
                preflight.platform
            );
        } else if preflight.existing_nix {
            eprintln!("[dry-run] 既存の Nix が検出されているため install は中止されます。");
        } else {
            eprintln!("[dry-run] preflight のみ完了。download / plan / install は skipping。");
        }
        return Ok(());
    }

    if !preflight.supported {
        return Err(format!(
            "unsupported platform/arch: {} {}",
            preflight.platform, preflight.arch
        ));
    }
    if preflight.existing_nix {
        return Err(
            "existing Nix detected; SchneeForge does not overwrite (ExistingNixDetected)"
                .to_string(),
        );
    }

    if !preflight.is_root {
        eprintln!();
        eprintln!("root 権限が必要です。以下で再実行してください:");
        eprintln!("  sudo schneeforge nix install");
        if !args.extra_conf.is_empty() {
            eprintln!(
                "    --extra-conf '{}'",
                args.extra_conf.join("' --extra-conf '")
            );
        }
        return Err("not running as root (privilege escalation required)".to_string());
    }

    let mut progress = ShellProgress::new();

    // D8: detailed plan 生成 → 表示 → 最終確認 → install。
    // plan は secure_plan_dir 内で生成したもののみを使う (user-supplied plan は
    // 確認と実行の間の差し替え (TOCTOU) を防げないため Phase 1 では受け付けない)
    let plan_dir = secure_plan_dir().map_err(|e| e.to_string())?;
    let plan_file = mn
        .prepare_plan(
            preflight.platform,
            preflight.arch,
            &plan_dir,
            &args.extra_conf,
            &mut progress,
        )
        .map_err(|e| format!("plan generation failed: {e}"))?;

    // detailed plan の内容を表示 (actions の概要)
    print_plan_summary(&plan_file)?;

    // 最終確認。upstream は --no-confirm で呼ぶため、この確認が唯一の gate (D8)
    if !args.yes {
        if !confirm_install()? {
            eprintln!("aborted. `/nix` は変更していません。");
            return Ok(());
        }
    } else {
        eprintln!("[confirm]  --yes 指定のため最終確認を skip します。");
    }

    let (binary, expected_sha) = mn
        .fetch_binary(preflight.platform, preflight.arch)
        .map_err(|e| format!("fetch binary for install: {e}"))?;

    mn.execute_plan(
        preflight.platform,
        preflight.arch,
        &plan_file,
        &binary,
        &mut progress,
    )
    .map_err(|e| format!("install failed: {e}"))?;

    eprintln!("[verify]   reading /nix/receipt.json...");
    match Receipt::load_default() {
        Ok(r) => {
            eprintln!(
                "  receipt version: {}",
                r.version.as_deref().unwrap_or("(unknown)")
            );
            eprintln!("  actions: {}", r.actions.len());
            let _ = plan_file;

            // SchneeForge 経由の install であることを記録 (uninstall 対称性のため)。
            // この record は uninstall safety の根拠なので、書けなければ success にしない。
            // ただし Nix 自体は install 済みのため自動 rollback はしない。
            // installer_sha256 を保存し、uninstall 時に cached binary の trust を再確立する。
            let ownership = OwnershipRecord::new(mn.version(), expected_sha);
            let ownership_path = default_ownership_path();
            if let Err(e) = ownership.write(&ownership_path) {
                eprintln!();
                eprintln!("⚠ Nix の install には成功しましたが、SchneeForge の ownership");
                eprintln!("  metadata を書き込めませんでした: {e}");
                eprintln!("  この状態では SchneeForge はこの Nix を管理対象とみなしません。");
                eprintln!("  以下で状態を確認してください:");
                eprintln!("    schneeforge nix doctor");
                return Err("ownership record write failed after successful install".to_string());
            }
            eprintln!("  ownership: {}", ownership_path.display());
        }
        Err(ManagedNixError::ReceiptNotFound { .. }) => {
            return Err(
                "install 完了後に /nix/receipt.json が見つかりません (ReceiptNotFound)".to_string(),
            );
        }
        Err(e) => return Err(format!("read receipt: {e}")),
    }

    // post-install gate: installer exit 0 + receipt 存在だけでは「動く Nix」を
    // 保証しない (upstream の self-test 失敗は warning のため)。nix_health で
    // binary / store / flakes を確認してから成功を宣言する。
    // 失敗しても自動 rollback はしない (危険操作のため)。
    eprintln!("[verify]   post-install verification...");
    let tc = ToolInventory::discover();
    let health = nix_health(&tc);
    let mut failures = Vec::new();
    if !health.installed {
        failures.push("nix binary not found".to_string());
    }
    if !health.store_accessible {
        failures.push("nix store ping failed".to_string());
    }
    if !health.flakes_available {
        failures.push("experimental-features does not include flakes".to_string());
    }
    if !failures.is_empty() {
        eprintln!("⚠ Nix の install 自体は完了しましたが、post-install 検証に失敗しました:");
        for f in &failures {
            eprintln!("  - {f}");
        }
        eprintln!("  `schneeforge nix doctor` で詳細を確認してください。");
        eprintln!("  (SchneeForge は install 済み Nix の自動 rollback を行いません)");
        return Err("post-install verification failed".to_string());
    }
    eprintln!("  nix binary / store / flakes: OK");

    eprintln!();
    eprintln!("Managed Nix install 完了。`schneeforge nix doctor` で状態を確認してください。");
    Ok(())
}

/// plan JSON から人間可読な概要を表示する (D8: Detailed Plan step)。
fn print_plan_summary(plan_file: &Path) -> Result {
    let lines = schneeforge_core::summarize_plan(plan_file).map_err(|e| e.to_string())?;
    eprintln!();
    eprintln!("=== Detailed plan ===");
    for line in lines {
        eprintln!("  {line}");
    }
    eprintln!();
    Ok(())
}

/// install の最終確認。TTY でのみ prompt を出し、非 TTY では安全側に fail する。
/// stdin が閉じている CI 環境で hang しない。
fn confirm_install() -> std::result::Result<bool, String> {
    use std::io::{IsTerminal, Read};

    if !std::io::stdin().is_terminal() {
        eprintln!("⚠ 非 interactive 環境では確認を取れません。");
        eprintln!("  自動化で実行する場合は --yes を指定してください。");
        return Err("cannot confirm installation without a TTY (use --yes)".to_string());
    }

    eprint!("この内容で install しますか? [y/N] ");
    let _ = std::io::Write::flush(&mut std::io::stderr());

    let mut buf = [0u8; 64];
    let n = std::io::stdin()
        .read(&mut buf)
        .map_err(|e| format!("read confirmation: {e}"))?;
    let answer = String::from_utf8_lossy(&buf[..n]).trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

/// `schneeforge nix doctor`
///
/// `ToolInventory` を受け取って `schneeforge_core::nix_health` で nix 関連を診断する。
/// (ToolInventory 経由で解決した nix binary を使うことで、tool-resolution spec の
///  「文字列リテラル spawn 禁止」に従う)
pub fn run_doctor(tc: Option<&ToolInventory>) -> Result {
    println!("=== schneeforge nix doctor ===");
    println!();

    // [status]: NixStatus 4 状態分類 (issue #15)。
    // ping 成否は既存の [nix runtime] 診断 (nix_health) と同じ解決済み binary から
    // 取る。Nix 未解決環境では nix_health が store_accessible = false を返すため
    // そのまま使う (Missing 分類は marker の有無だけで決まるため影響しない)。
    let store_ping_ok = tc
        .map(nix_health)
        .map(|h| h.store_accessible)
        .unwrap_or(false);
    let report = classify_current(store_ping_ok);
    println!("[status]");
    println!("  status:      {}", report.status.label());
    println!("  next action: {}", report.status.next_action());
    if let Some(mismatch) = &report.mismatch {
        println!("  mismatch:    {mismatch}");
    }
    println!();

    println!("[environment]");
    println!("  platform: {}", detect_platform());
    println!("  arch:     {}", detect_arch());
    println!("  has_nix:  {}", existing_nix_detected());
    println!();

    println!("[receipt]");
    let receipt_path = default_receipt_path();
    match Receipt::load(&receipt_path) {
        Ok(r) => {
            println!("  path:    {}", receipt_path.display());
            println!("  version: {}", r.version.as_deref().unwrap_or("(missing)"));
            println!("  actions: {}", r.actions.len());
            if let Some(planner) = &r.planner {
                if let Some(s) = planner.get("planner").and_then(|v| v.as_str()) {
                    println!("  planner: {s}");
                }
            }
        }
        Err(ManagedNixError::ReceiptNotFound { .. }) => {
            println!("  receipt not found at {}", receipt_path.display());
            println!("  → Managed Nix は未 install の可能性があります (schneeforge nix install)");
        }
        Err(e) => {
            println!("  receipt parse error: {e}");
        }
    }
    println!();

    println!("[nix runtime]");
    match tc {
        Some(inv) => {
            let h = nix_health(inv);
            println!("  installed:        {}", h.installed);
            if let Some(v) = &h.version {
                println!("  version:          {v}");
            }
            if let Some(exe) = &h.executable {
                println!("  executable:       {exe}");
            }
            println!("  store accessible: {}", h.store_accessible);
            println!("  flakes available: {}", h.flakes_available);
            if let Some(err) = &h.error {
                println!("  error:            {err}");
            }
            if let Some(w) = &h.warning {
                println!("  warning:          {w}");
            }
        }
        None => {
            println!("  (ToolInventory 未解決のため skip)")
        }
    }
    Ok(())
}

/// `schneeforge nix uninstall`
pub fn run_uninstall(args: UninstallArgs) -> Result {
    eprintln!("=== schneeforge nix uninstall ===");
    eprintln!();

    let receipt_path = args.receipt.clone().unwrap_or_else(default_receipt_path);
    if !receipt_path.exists() {
        return Err(format!(
            "receipt not found: {} (ReceiptNotFound)",
            receipt_path.display()
        ));
    }

    // SchneeForge 経由で install されたものか確認 (install 拒否 policy との対称性)。
    // record が無い = 用户が nix-installer 直接等で入れた Nix。既定では abort。
    let ownership_path = default_ownership_path();
    let ownership = match OwnershipRecord::load(&ownership_path) {
        Ok(rec) => {
            eprintln!(
                "ownership: SchneeForge managed (installer {})",
                rec.installer_version
            );
            Some(rec)
        }
        Err(ManagedNixError::OwnershipNotFound { .. }) if !args.force => {
            eprintln!("⚠ SchneeForge の ownership record が見つかりません:");
            eprintln!("  {}", ownership_path.display());
            eprintln!();
            eprintln!("  この Nix は SchneeForge 経由で install されたものではありません。");
            eprintln!("  (SchneeForge は既存 Nix の上書き install を拒否するため、対応する");
            eprintln!("   ownership record が存在しません)");
            eprintln!();
            eprintln!("  どうしても SchneeForge で uninstall したい場合は:");
            eprintln!("    sudo schneeforge nix uninstall --force");
            return Err("no SchneeForge ownership record (NotManagedBySchneeForge)".to_string());
        }
        Err(ManagedNixError::OwnershipNotFound { .. }) => {
            eprintln!("⚠ ownership record がありませんが --force により続行します。");
            None
        }
        Err(e) => return Err(format!("read ownership record: {e}")),
    };

    // custom receipt は ownership record の upstream_receipt と一致しなければ
    // 受け付けない (valid な ownership を別 receipt への root 実行に転用させない)
    if let Some(rec) = &ownership {
        if !args.force && receipt_path != rec.upstream_receipt {
            return Err(format!(
                "--receipt {} は SchneeForge の ownership record が指す {} と一致しません。\
                 既定の receipt のみ uninstall できます (回避するには --force)",
                receipt_path.display(),
                rec.upstream_receipt.display()
            ));
        }
    }

    if has_nix_darwin_markers() {
        eprintln!("⚠ nix-darwin の markers を検出しました。");
        eprintln!("  SchneeForge は現在 nix-darwin の自動取り外しをサポートしません。");
        eprintln!("  先に nix-darwin 公式 uninstaller で nix-darwin を取り外してから");
        eprintln!("  再実行してください:");
        eprintln!("    sudo nix --extra-experimental-features \"nix-command flakes\" \\");
        eprintln!("      run nix-darwin#darwin-uninstaller");
        eprintln!("  (/install 済みの場合は `sudo darwin-uninstaller` も可)");
        return Err("nix-darwin detected; uninstall aborted (D6 policy)".to_string());
    }

    if !schneeforge_core::is_root() {
        eprintln!("root 権限が必要です。以下で再実行してください:");
        eprintln!("  sudo schneeforge nix uninstall");
        return Err("not running as root".to_string());
    }

    let local = installed_binary_path();
    let binary = if local.exists() {
        local
    } else {
        eprintln!("(note) /nix/nix-installer が見つかりません。cached binary を探します。");
        let cached = cached_binary_for_receipt(&receipt_path)?;
        // root で実行する外部 binary は毎回 trust を再確立する:
        // ownership record が保存した installer SHA256 と再計算 hash を比較する。
        // SHA が無い record は既定で abort (fail-closed)。--force のみ突破。
        let sha_ok = match &ownership {
            Some(rec) => {
                let actual = schneeforge_core::sha256_hex(&cached)
                    .map_err(|e| format!("hash cached installer: {e}"))?;
                if actual == rec.installer_sha256 {
                    eprintln!("  cached installer SHA256 verified (ownership record 一致)");
                    true
                } else {
                    eprintln!(
                        "cached installer {} の SHA256 が ownership record と一致しません。",
                        cached.display()
                    );
                    false
                }
            }
            None => {
                eprintln!("⚠ ownership record が無いため cached binary を検証できません。");
                false
            }
        };
        if !sha_ok && !args.force {
            return Err(
                "cached installer の SHA256 検証に失敗しました (fail-closed)。続行するには \
                 cache を削除して再 install するか、--force で明示的に突破してください"
                    .to_string(),
            );
        }
        if !sha_ok && args.force {
            eprintln!("⚠ --force により SHA256 検証失敗を突破して続行します。");
        }
        cached
    };

    eprintln!("invoking upstream uninstall...");
    let uninstall_args = build_uninstall_args(Some(&receipt_path));
    let mut noop = NoProgress;
    run_with_json_logs(&binary, &uninstall_args, |line| noop.on_log(line))
        .map_err(|e| format!("uninstall: {e}"))?;

    // ownership record も削除 (Nix が無くなったので管理対象でも無くなる)
    if let Err(e) = OwnershipRecord::remove(&ownership_path) {
        eprintln!("⚠ ownership record の削除に失敗しました: {e}");
    }

    eprintln!();
    eprintln!("upstream uninstall 完了。`/nix` が残っている場合は手動で確認してください。");
    Ok(())
}

/// `schneeforge nix repair`
///
/// NixStatus 分類に基づいて修復 action を決定する。SchneeForge 単独で安全に
/// 実行できるのは stale ownership record の削除のみ。破壊的な uninstall /
/// 再 install は案内表示に留める (spec: state-driven 修復)。
pub fn run_repair(args: RepairArgs, tc: Option<&ToolInventory>) -> Result {
    eprintln!("=== schneeforge nix repair ===");
    eprintln!();

    // upstream repair (hooks / sequoia) は状態分類と独立した明示 option
    if args.hooks || args.sequoia {
        return run_upstream_repair(args);
    }

    let store_ping_ok = tc
        .map(nix_health)
        .map(|h| h.store_accessible)
        .unwrap_or(false);
    let action = repair_action_current(store_ping_ok);
    let ownership_path = default_ownership_path();

    match action {
        RepairAction::RemoveStaleOwnership => {
            // marker が一切無い = Nix 実態が無い。record を消すだけで
            // Missing へ復帰する (receipt も同時に確認して案内)
            eprintln!("ownership record は存在しますが /nix 配下の Nix が見つかりません。");
            eprintln!("  (uninstall が途中で失敗した可能性があります)");
            eprintln!();
            eprintln!("  削除対象: {}", ownership_path.display());
            if args.dry_run {
                eprintln!();
                eprintln!("[dry-run] 所有権 record の削除は実行しませんでした。");
                eprintln!("  実行するには: schneeforge nix repair");
                return Ok(());
            }
            OwnershipRecord::remove(&ownership_path)
                .map_err(|e| format!("remove stale ownership record: {e}"))?;
            eprintln!();
            eprintln!("stale ownership record を削除しました。状態は Missing に戻りました。");
            eprintln!("  再 install する場合: sudo schneeforge nix install");
        }
        RepairAction::SuggestUninstall => {
            eprintln!("Nix の marker / receipt は揃っていますが、runtime 検証に失敗しています");
            eprintln!("  (nix store ping が失敗)。修復手段:");
            eprintln!();
            eprintln!("  1. sudo schneeforge nix uninstall");
            eprintln!("  2. sudo schneeforge nix install");
            eprintln!();
            eprintln!("  (SchneeForge は破壊的な uninstall を repair から自動実行しません)");
        }
        RepairAction::SuggestManualCleanup => {
            eprintln!("installation marker は残っていますが receipt が読めません。");
            eprintln!("  この状態では upstream (nix-installer) も revert できません。");
            eprintln!();
            eprintln!("  手動での cleanup 手順:");
            eprintln!("    1. sudo schneeforge nix uninstall --force");
            eprintln!("       (receipt が無いため --force が必要です)");
            eprintln!("    2. /nix 配下と build users が残っていれば手動で削除");
            eprintln!("       Linux: sudo userdel nixbld1..nixbldN / sudo groupdel nixbld");
            eprintln!("       macOS: sudo dscl . -delete /Users/_nixbld1..");
            eprintln!("    3. sudo schneeforge nix install");
            eprintln!();
            eprintln!("  (SchneeForge は /nix 配下や build users の削除を自動実行しません)");
        }
        RepairAction::NoActionNeeded => {
            eprintln!("Nix は Healthy です。対応は不要です。");
        }
        RepairAction::SuggestInstall => {
            eprintln!("Nix は Missing (未導入) です。install してください:");
            eprintln!("  sudo schneeforge nix install");
        }
    }
    Ok(())
}

/// upstream `nix-installer repair {hooks|sequoia}` を wrap する。
/// 修復 logic は upstream 側のものをそのまま使う (uninstall と同じ委譲方針)。
fn run_upstream_repair(args: RepairArgs) -> Result {
    let targets: Vec<UpstreamRepair> = [
        args.hooks.then_some(UpstreamRepair::Hooks),
        args.sequoia.then_some(UpstreamRepair::Sequoia),
    ]
    .into_iter()
    .flatten()
    .collect();

    let binary = installed_binary_path();
    if !binary.exists() {
        return Err(format!(
            "{} が見つかりません。upstream repair は install 済み環境でのみ利用できます",
            binary.display()
        ));
    }

    for target in &targets {
        let cli_args = repair_args(*target);
        eprintln!(
            "invoking upstream: nix-installer {} ...",
            cli_args.join(" ")
        );
        if args.dry_run {
            eprintln!("[dry-run] upstream 呼び出しは実行しませんでした。");
            continue;
        }
        let mut noop = NoProgress;
        run_with_json_logs(&binary, &cli_args, |line| noop.on_log(line))
            .map_err(|e| format!("upstream repair {}: {e}", target.subcommand()))?;
        eprintln!("upstream repair {} 完了。", target.subcommand());
    }
    Ok(())
}

struct ShellProgress;

impl ShellProgress {
    fn new() -> Self {
        Self
    }
}

impl ProgressSink for ShellProgress {
    fn on_phase(&mut self, phase: schneeforge_core::InstallPhase) {
        // Debug 出力ではなく人間可読な phase 名を表示
        let label = match phase {
            schneeforge_core::InstallPhase::Download => "download",
            schneeforge_core::InstallPhase::Verify => "verify",
            schneeforge_core::InstallPhase::Privilege => "privilege",
            schneeforge_core::InstallPhase::Plan => "plan",
            schneeforge_core::InstallPhase::Install => "install",
            schneeforge_core::InstallPhase::PostInstall => "post-install",
        };
        eprintln!("[phase] {label}");
    }
    fn on_log(&mut self, line: &JsonLogLine) {
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
                eprintln!("  {level:<5} {span}: {msg}");
            }
        }
    }
}

fn has_nix_darwin_markers() -> bool {
    if cfg!(target_os = "macos") {
        if let Ok(home) = std::env::var("HOME") {
            let marker = PathBuf::from(home).join(".nix-darwin");
            if marker.exists() {
                return true;
            }
        }
        if Path::new("/run/current-system").exists() {
            return true;
        }
    }
    false
}

fn cached_binary_for_receipt(receipt: &Path) -> std::result::Result<PathBuf, String> {
    let r = Receipt::load(receipt).map_err(|e| format!("read receipt: {e}"))?;
    let version = r.version.as_deref().ok_or("receipt missing version")?;
    let cache = cache_path(version).map_err(|e| format!("cache path: {e}"))?;
    if !cache.exists() {
        return Err(format!(
            "cached installer for version {version} not found at {}",
            cache.display()
        ));
    }
    Ok(cache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct Probe {
        #[command(subcommand)]
        cmd: NixSub,
    }

    #[test]
    fn parse_install_dry_run() {
        let p = Probe::try_parse_from(["probe", "install", "--dry-run"]).unwrap();
        match p.cmd {
            NixSub::Install(a) => assert!(a.dry_run),
            _ => panic!("expected Install"),
        }
    }

    #[test]
    fn parse_uninstall_receipt() {
        let p = Probe::try_parse_from(["probe", "uninstall", "--receipt", "/nix/receipt.json"])
            .unwrap();
        match p.cmd {
            NixSub::Uninstall(a) => {
                assert_eq!(a.receipt.unwrap(), PathBuf::from("/nix/receipt.json"));
            }
            _ => panic!("expected Uninstall"),
        }
    }

    #[test]
    fn parse_doctor() {
        let p = Probe::try_parse_from(["probe", "doctor"]).unwrap();
        matches!(p.cmd, NixSub::Doctor);
    }

    #[test]
    fn install_args_default() {
        let a = InstallArgs::default();
        assert!(!a.dry_run);
        assert!(!a.yes);
        assert!(a.extra_conf.is_empty());
    }

    #[test]
    fn install_rejects_allow_non_root_flag() {
        // spec: root でなければ sudo 再実行を案内して停止。
        // --allow-non-root は削除済みで、使われたら error になる
        let res = Probe::try_parse_from(["probe", "install", "--allow-non-root"]);
        assert!(res.is_err());
    }

    #[test]
    fn install_rejects_plan_flag() {
        // user-supplied plan は「確認表示 → 実行」の間の差し替え (TOCTOU) を
        // 防げないため削除済み。plan は SchneeForge が secure dir へ生成する
        let res = Probe::try_parse_from(["probe", "install", "--plan", "/tmp/plan.json"]);
        assert!(res.is_err());
    }

    #[test]
    fn parse_install_extra_conf_multiple() {
        let p = Probe::try_parse_from([
            "probe",
            "install",
            "--extra-conf",
            "a = 1",
            "--extra-conf",
            "b = 2",
        ])
        .unwrap();
        if let NixSub::Install(a) = p.cmd {
            assert_eq!(a.extra_conf, vec!["a = 1".to_string(), "b = 2".to_string()]);
        } else {
            panic!("expected Install");
        }
    }
}
