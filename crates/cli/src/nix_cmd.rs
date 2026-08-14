//! `schneeforge nix` subcommand handlers (Phase 1: Managed Nix bootstrap)

use std::path::{Path, PathBuf};

use clap::Args;

use schneeforge_core::{
    cache_path, default_ownership_path, default_receipt_path, detect_arch, detect_platform,
    existing_nix_detected, installed_binary_path, nix_health, run_with_json_logs, secure_plan_dir,
    uninstall_args as build_uninstall_args, JsonLogLine, ManagedNix, ManagedNixError, NoProgress,
    OwnershipRecord, ProgressSink, Receipt, ToolInventory,
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
}

#[derive(Args, Debug, Default)]
pub struct InstallArgs {
    /// bootstrap-manifest.toml の代わりに使う plan.json (Phase 1 では通常未指定)
    #[arg(long)]
    pub plan: Option<PathBuf>,

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

type Result = std::result::Result<(), String>;

/// `schneeforge nix install`
pub fn run_install(repo_root: &str, args: InstallArgs) -> Result {
    let mn = ManagedNix::load_from_repo(Path::new(repo_root))
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
        if let Some(plan) = &args.plan {
            eprintln!("    --plan {}", plan.display());
        }
        return Err("not running as root (privilege escalation required)".to_string());
    }

    let mut progress = ShellProgress::new();

    // D8: detailed plan 生成 → 表示 → 最終確認 → install
    let plan_file = match &args.plan {
        Some(p) => {
            if !p.exists() {
                return Err(format!(
                    "plan file not found: {} (PlanFileNotFound)",
                    p.display()
                ));
            }
            eprintln!("[plan]     using user-supplied plan: {}", p.display());
            // user-supplied plan でも download + verify は必須
            let (binary, sha) = mn
                .fetch_binary(preflight.platform, preflight.arch)
                .map_err(|e| format!("fetch binary: {e}"))?;
            eprintln!("[verify]   SHA256 OK: {} ({sha})", binary.display());
            p.clone()
        }
        None => {
            let plan_dir = secure_plan_dir().map_err(|e| e.to_string())?;
            mn.prepare_plan(
                preflight.platform,
                preflight.arch,
                &plan_dir,
                &args.extra_conf,
                &mut progress,
            )
            .map_err(|e| format!("plan generation failed: {e}"))?
        }
    };

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
        eprintln!("  先に `nix run nix-darwin -- uninstall` (macOS) 等で nix-darwin を");
        eprintln!("  取り外してから再実行してください (ADR-0001 Open Question 4)。");
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
        assert!(a.plan.is_none());
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
