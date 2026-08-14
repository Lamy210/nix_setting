//! `schneeforge nix` subcommand handlers (Phase 1: Managed Nix bootstrap)

use std::path::{Path, PathBuf};

use clap::Args;

use schneeforge_core::{
    cache_path, default_receipt_path, detect_arch, detect_platform, has_nix, planner_name,
    run_with_json_logs, uninstall_args as build_uninstall_args, JsonLogLine, ManagedNix,
    ManagedNixError, NoProgress, ProgressSink, Receipt,
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

    /// root 未実行時に即座に終了せず、処理を継続しようとする (Phase 1 では推奨しない)
    #[arg(long)]
    pub allow_non_root: bool,
}

#[derive(Args, Debug)]
pub struct UninstallArgs {
    /// `/nix/receipt.json` 以外の receipt path
    #[arg(long)]
    pub receipt: Option<PathBuf>,
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

    if args.dry_run {
        eprintln!("[dry-run] preflight のみ完了。download / plan / install は skipping。");
        return Ok(());
    }

    if !preflight.is_root && !args.allow_non_root {
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
        return Err(
            "not running as root (privilege escalation required)".to_string(),
        );
    }

    eprintln!(
        "[download] resolving asset for {} {}...",
        preflight.platform, preflight.arch
    );
    let binary = mn
        .fetch_binary(preflight.platform, preflight.arch)
        .map_err(|e| format!("fetch binary: {e}"))?;
    eprintln!("[verify]   SHA256 OK: {}", binary.display());

    let planner = planner_name(preflight.platform, preflight.arch)
        .map_err(|e| format!("resolve planner: {e}"))?;

    let plan_file = match &args.plan {
        Some(p) => {
            if !p.exists() {
                return Err(format!(
                    "plan file not found: {} (PlanFileNotFound)",
                    p.display()
                ));
            }
            eprintln!("[plan]     using user-supplied plan: {}", p.display());
            p.clone()
        }
        None => {
            let plan_dir = std::env::temp_dir().join("schneeforge-managed-nix");
            std::fs::create_dir_all(&plan_dir).map_err(|e| format!("create plan dir: {e}"))?;
            let plan_file = plan_dir.join(format!("plan-{}.json", mn.version()));
            eprintln!("[plan]     generating plan (planner={planner})...");
            mn.generate_plan(&binary, planner, &plan_file, &args.extra_conf)
                .map_err(|e| format!("generate plan: {e}"))?;
            plan_file
        }
    };

    eprintln!("[install]  invoking nix-installer (stderr → JSON Lines)...");
    let mut progress = ShellProgress::new();
    if let Err(e) = mn.run_install(&binary, &plan_file, &mut progress) {
        return Err(format!("install failed: {e}"));
    }

    eprintln!("[verify]   reading /nix/receipt.json...");
    match Receipt::load_default() {
        Ok(r) => {
            eprintln!(
                "  receipt version: {}",
                r.version.as_deref().unwrap_or("(unknown)")
            );
            eprintln!("  actions: {}", r.actions.len());
        }
        Err(ManagedNixError::ReceiptNotFound { .. }) => {
            return Err(
                "install 完了後に /nix/receipt.json が見つかりません (ReceiptNotFound)"
                    .to_string(),
            );
        }
        Err(e) => return Err(format!("read receipt: {e}")),
    }
    eprintln!();
    eprintln!("Managed Nix install 完了。`schneeforge nix doctor` で状態を確認してください。");
    Ok(())
}

/// `schneeforge nix doctor`
pub fn run_doctor() -> Result {
    println!("=== schneeforge nix doctor ===");
    println!();

    println!("[environment]");
    println!("  platform: {}", detect_platform());
    println!("  arch:     {}", detect_arch());
    println!("  has_nix:  {}", has_nix());
    println!();

    println!("[receipt]");
    let receipt_path = default_receipt_path();
    match Receipt::load(&receipt_path) {
        Ok(r) => {
            println!("  path:    {}", receipt_path.display());
            println!(
                "  version: {}",
                r.version.as_deref().unwrap_or("(missing)")
            );
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
    match std::process::Command::new("nix").args(["store", "ping"]).output() {
        Ok(out) if out.status.success() => println!("  nix store ping: ok"),
        Ok(out) => {
            println!("  nix store ping: failed (exit {:?})", out.status.code());
            if !out.stderr.is_empty() {
                let tail = String::from_utf8_lossy(&out.stderr);
                if let Some(last) = tail.lines().last() {
                    println!("    stderr (last line): {last}");
                }
            }
        }
        Err(_) => println!("  nix store ping: (nix not found on PATH)"),
    }

    match std::process::Command::new("nix")
        .args(["config", "show", "experimental-features"])
        .output()
    {
        Ok(out) if out.status.success() => {
            let val = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let has_flakes = val
                .split_whitespace()
                .any(|f| f == "flakes" || f == "nix-command");
            println!("  experimental-features: {val}");
            println!("  flakes enabled: {has_flakes}");
        }
        _ => println!("  experimental-features: (unavailable)"),
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

    let local = Path::new("/nix/nix-installer");
    let binary = if local.exists() {
        local.to_path_buf()
    } else {
        eprintln!("(note) /nix/nix-installer が見つかりません。cached binary を探します。");
        cached_binary_for_receipt(&receipt_path)?
    };

    eprintln!("invoking upstream uninstall...");
    let uninstall_args = build_uninstall_args(Some(&receipt_path));
    let mut noop = NoProgress;
    run_with_json_logs(&binary, &uninstall_args, |line| noop.on_log(line))
        .map_err(|e| format!("uninstall: {e}"))?;

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
        eprintln!("[phase] {phase:?}");
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
        let p = Probe::try_parse_from([
            "probe",
            "uninstall",
            "--receipt",
            "/nix/receipt.json",
        ])
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
        assert!(!a.allow_non_root);
        assert!(a.plan.is_none());
        assert!(a.extra_conf.is_empty());
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
